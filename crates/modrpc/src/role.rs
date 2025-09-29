use core::future::Future;

use crate::{
    EndpointAddr, InterfaceBuilder, LocalSpawner, PacketSender, RoleSetup, RuntimeHandle,
    WorkerContext, context_map::ModrpcContextTag,
};

pub trait InterfaceSchema {
    fn new(ib: &mut InterfaceBuilder) -> Self;
}

pub trait InterfaceRole: 'static {
    type Interface: InterfaceSchema + Send + Sync;
    type Config: mproto::Owned;
    type Init: mproto::Owned;
    type Stubs;
    type Hooks: Clone + 'static;

    fn setup_worker(
        i: &Self::Interface,
        ii: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks);
}

pub trait RoleStartFn<Role: InterfaceRole>: FnOnce(RoleWorkerContext<Role>) {}

impl<F, Role> RoleStartFn<Role> for F
where
    Role: InterfaceRole,
    F: FnOnce(RoleWorkerContext<Role>),
{
}

pub struct RoleWorkerContext<'a, Role: InterfaceRole> {
    pub rt: &'a RuntimeHandle,
    pub setup: &'a mut RoleSetup<'a>,
    pub stubs: Role::Stubs,
    pub hooks: &'a Role::Hooks,
    pub config: &'a Role::Config,
    pub init: &'a Role::Init,
}

impl<Role: InterfaceRole> RoleWorkerContext<'_, Role> {
    pub fn raw_spawner(&self) -> &LocalSpawner {
        &self.role_spawner().spawner
    }

    pub fn role_spawner(&self) -> &RoleSpawner {
        &self.setup.role_spawner()
    }

    pub fn role_shutdown_signal(&self) -> &bab::SignalTree {
        self.setup.role_shutdown_signal()
    }
}

#[derive(Clone)]
pub struct RoleSpawner {
    spawner: LocalSpawner,
    interval_fn: for<'a> fn(
        core::time::Duration,
        &'a mut dyn FnMut(),
    ) -> core::pin::Pin<Box<dyn Future<Output = ()> + 'a>>,
    worker_shutdown_signal: bab::SignalTree,
    role_shutdown_signal: bab::SignalTree,
}

impl RoleSpawner {
    pub fn raw_spawner(&self) -> &LocalSpawner {
        &self.spawner
    }

    pub fn spawn(&self, future: impl core::future::Future<Output = ()> + 'static) {
        let worker_shutdown_signal = self.worker_shutdown_signal.clone();
        let role_shutdown_signal = self.role_shutdown_signal.clone();
        self.spawner
            .spawn(async move {
                futures_lite::future::or(
                    future,
                    // TODO: we probably want to create a single thread-local shutdown signal per-role and
                    // spawn a separate task to notify the signal when either of these signals are
                    // notified. Or better yet, integrate ispawn with executors' task cancellation
                    // support (providing shims where there is no built-in task cancellation).
                    //
                    // In the meantime, waitq makes polling these super cheap since they will have
                    // thread-local waiter registrations.
                    futures_lite::future::or(
                        role_shutdown_signal.wait(),
                        worker_shutdown_signal.wait(),
                    ),
                )
                .await;
            })
            .unwrap();
    }

    pub fn on_shutdown(&self, future: impl core::future::Future<Output = ()> + 'static) {
        let worker_shutdown_signal = self.worker_shutdown_signal.clone();
        let role_shutdown_signal = self.role_shutdown_signal.clone();
        self.spawner
            .spawn(async move {
                futures_lite::future::or(
                    role_shutdown_signal.wait(),
                    worker_shutdown_signal.wait(),
                )
                .await;
                future.await;
            })
            .unwrap();
    }

    pub fn spawn_interval_loop<'a>(
        &self,
        interval_duration: core::time::Duration,
        mut f: impl FnMut() + 'static,
    ) {
        let interval_fn = self.interval_fn;
        self.spawn(async move {
            (interval_fn)(interval_duration, &mut f).await;
        });
    }
}

pub fn run_role_worker<Role>(
    rt: &RuntimeHandle,
    plane_id: u32,
    role_id: u64,
    spawner: LocalSpawner,
    worker_cx: &WorkerContext,
    worker_shutdown_signal: bab::SignalTree,
    role_shutdown_signal: bab::SignalTree,
    endpoint_addr: EndpointAddr,
    packet_sender: PacketSender,
    config: &<Role as InterfaceRole>::Config,
    init: &<Role as InterfaceRole>::Init,
    start_fn: impl RoleStartFn<Role>,
) -> <Role as InterfaceRole>::Hooks
where
    Role: InterfaceRole,
    <Role as InterfaceRole>::Init: mproto::Encode,
{
    let role_spawner = RoleSpawner {
        spawner: spawner,
        interval_fn: worker_cx.interval_fn,
        worker_shutdown_signal,
        role_shutdown_signal: role_shutdown_signal.clone(),
    };
    // XXX: spawning this here is somewhat load-bearing for perf - this will be the first task
    // registered as a waiter for the role's shutdown signal, so it will be registered in waitq's
    // thread-safe queue, which is fine because this task will only be woken at shutdown. This
    // means all other tasks on this thread spawned by the role or application will have
    // thread-local waitq registrations and so will be cheap to poll.
    //
    // This papers over the unforunate fact that the shutdown signals are polled every time a
    // long-running task spawned via RoleSpawner or WorkerSpawner is polled to make progress.
    //
    // See a related comment in RoleSpawner::spawn.
    role_spawner.on_shutdown({
        let pp = worker_cx.pp.clone();
        let locals = worker_cx.locals.clone();
        async move {
            pp.remove_plane(plane_id);
            locals
                .borrow_mut()
                .shutdown_tag(ModrpcContextTag::Role(role_id));
        }
    });

    let mut ib = InterfaceBuilder::new();
    let interface = Role::Interface::new(&mut ib);

    let role_handle = {
        let endpoint_addr = endpoint_addr.clone();

        let role_name = core::any::type_name::<Role>()
            // Strip off rust module qualification
            .rsplit("::")
            .next()
            .unwrap()
            // modrpc-codegen always generates interface role type names as
            // <interface name><role name>Role
            .strip_suffix("Role")
            .unwrap();
        let mut setup = RoleSetup::new(
            role_name,
            role_spawner.clone(),
            worker_cx,
            packet_sender,
            plane_id,
            role_id,
            endpoint_addr,
            role_shutdown_signal.clone(),
        );
        let (stubs, role_handle) = Role::setup_worker(&interface, &mut setup, config, init);
        start_fn(RoleWorkerContext {
            rt,
            setup: &mut setup,
            stubs,
            hooks: &role_handle,
            config,
            init,
        });

        role_handle
    };

    role_handle
}

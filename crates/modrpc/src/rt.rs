use std::{
    cell::{RefCell, UnsafeCell},
    mem::MaybeUninit,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    InterfaceRole, LocalSpawner, Packet, PacketSender, SingleChannelSender,
    context_map::{ContextMap, LocalContextMap, ModrpcContextTag},
    endpoint_proto::EndpointAddr,
    packet_processor::PacketProcessor,
    role::{RoleStartFn, run_role_worker},
    transport::{TransportBuilder, TransportContext, TransportHandle},
    worker::{
        WorkerCommand, WorkerContext, WorkerId, WorkerSpawner, build_inter_worker_batchers,
        spawn_worker,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WorkerGroup {
    first_worker_index: u16,
    worker_count: u16,
}

impl WorkerGroup {
    pub fn worker_count(&self) -> u16 {
        self.worker_count
    }
}

struct LocalWorker {
    spawner: LocalSpawner,
    shutdown_signal: bab::SignalTree,
    local_packet_tx: localq::mpsc::Sender<Packet>,
    locals: Rc<RefCell<LocalContextMap>>,
}

struct BuilderLocalWorker {
    local_worker: LocalWorker,
    local_packet_rx: localq::mpsc::Receiver<Packet>,
}

pub struct RuntimeBuilder {
    worker_groups: Vec<WorkerGroup>,
    local_worker: Option<BuilderLocalWorker>,
    next_worker_index: u16,
}

impl RuntimeBuilder {
    /// Start building a runtime that is not used on the current thread.
    pub fn new_no_local() -> Self {
        Self {
            worker_groups: Vec::new(),
            local_worker: None,
            next_worker_index: 0,
        }
    }

    /// Start building a runtime can be used on the current thread.
    pub fn new_with_local(spawner: LocalSpawner) -> Self {
        // TODO configurable capacity
        let (local_packet_tx, local_packet_rx) = localq::mpsc::channel(64);
        let locals = Rc::new(RefCell::new(LocalContextMap::new()));
        let shutdown_signal = bab::SignalTree::new();

        Self {
            worker_groups: Vec::new(),
            local_worker: Some(BuilderLocalWorker {
                local_worker: LocalWorker {
                    spawner,
                    shutdown_signal,
                    local_packet_tx,
                    locals,
                },
                local_packet_rx,
            }),
            next_worker_index: 1,
        }
    }

    pub fn new_worker_group(&mut self, worker_count: u16) -> WorkerGroup {
        let worker_group = WorkerGroup {
            first_worker_index: self.next_worker_index,
            worker_count,
        };
        self.next_worker_index += worker_count;

        self.worker_groups.push(worker_group);

        worker_group
    }

    pub fn start<E: modrpc_executor::ModrpcExecutor>(
        self,
    ) -> (RuntimeHandle, RuntimeShutdownHandle) {
        let rt_shutdown_signal = bab::SignalTree::new();
        let globals = Arc::new(Mutex::new(ContextMap::new()));
        let mut worker_handles = Vec::new();
        // Start at 1 to account for a potential local worker.
        let remote_worker_count = self
            .worker_groups
            .iter()
            .map(|g| g.worker_count)
            .sum::<u16>();
        let worker_count = if self.local_worker.is_some() {
            1 + remote_worker_count
        } else {
            remote_worker_count
        };

        let command_queue_capacity = 8;
        let mut worker_command_receivers = vec![];

        // Create handle for local worker.
        if let Some(BuilderLocalWorker { local_worker, .. }) = &self.local_worker {
            let (command_tx, command_rx) = burstq::mpmc(command_queue_capacity);
            worker_command_receivers.push(Some(command_rx));
            worker_handles.push(WorkerHandle {
                command_tx,
                shutdown_signal: local_worker.shutdown_signal.clone(),
            });

            rt_shutdown_signal.add_child(local_worker.shutdown_signal.clone());
        }

        // Create handles for remote workers.
        for _ in 0..remote_worker_count {
            let (command_tx, command_rx) = burstq::mpmc(command_queue_capacity);

            let shutdown_signal = bab::SignalTree::new();
            rt_shutdown_signal.add_child(shutdown_signal.clone());

            worker_command_receivers.push(Some(command_rx));
            worker_handles.push(WorkerHandle {
                command_tx,
                shutdown_signal: shutdown_signal.clone(),
            });
        }

        // Create the runtime handle.
        let rt = RuntimeHandle {
            inner: Arc::new(RuntimeHandleInner {
                next_role_id: AtomicU64::new(0),
                globals: globals.clone(),
                worker_handles,
                worker_contexts: (0..thid::DEFAULT_MAX_THREADS)
                    .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
                    .collect(),
            }),
        };

        // TODO configurable
        let inter_worker_queue_capacity = 256;
        let max_inter_worker_batch_size = 32;

        let mut inter_worker_senders = Vec::with_capacity(worker_count as usize);
        let mut inter_worker_receivers = Vec::with_capacity(worker_count as usize);
        for _ in 0..worker_count {
            let (inter_worker_tx, inter_worker_rx) = burstq::mpmc(inter_worker_queue_capacity);
            inter_worker_senders.push(inter_worker_tx);
            inter_worker_receivers.push(Some(inter_worker_rx));
        }

        // Create handle for local worker and start it.
        if let Some(BuilderLocalWorker {
            local_worker,
            local_packet_rx,
        }) = self.local_worker
        {
            let inter_worker_batcher_senders = build_inter_worker_batchers(
                &WorkerSpawner {
                    spawner: local_worker.spawner.clone(),
                    shutdown_signal: local_worker.shutdown_signal.clone(),
                    interval_fn: WorkerContext::interval_loop::<E>,
                },
                WorkerId(0),
                max_inter_worker_batch_size,
                &inter_worker_senders,
            );
            let pp = Rc::new(PacketProcessor::new(
                local_worker.local_packet_tx.clone(),
                inter_worker_batcher_senders,
            ));
            let worker_context = WorkerContext::new::<E>(
                rt.clone(),
                local_worker.spawner.clone(),
                local_worker.shutdown_signal.clone(),
                WorkerId(0),
                worker_count,
                pp,
                local_worker.local_packet_tx.clone(),
                local_worker.locals.clone(),
                globals.clone(),
            );

            spawn_worker::<E>(
                worker_context.clone(),
                local_packet_rx,
                worker_command_receivers[0]
                    .take()
                    .expect("worker command receiver"),
                inter_worker_receivers[0]
                    .take()
                    .expect("inter-worker packet receiver"),
            );

            let map_worker_context = unsafe {
                &mut *rt.inner.worker_contexts[thid::ThreadId::current().as_usize()].get()
            };
            map_worker_context.write(worker_context);
        }

        let mut worker_join_handles = Vec::new();
        for worker_group in &self.worker_groups {
            for i in 0..worker_group.worker_count {
                let worker_index = worker_group.first_worker_index + i;
                let worker_id = WorkerId(worker_index);

                let rt = rt.clone();
                let globals = globals.clone();
                let inter_worker_senders = inter_worker_senders.clone();
                let (startup_done_tx, startup_done_rx) = oneshot::channel();

                let command_rx = worker_command_receivers[worker_index as usize]
                    .take()
                    .expect("worker command receiver");
                let inter_rx = inter_worker_receivers[worker_index as usize]
                    .take()
                    .expect("inter-worker packet receiver");

                let join_handle = std::thread::Builder::new()
                    .name(format!("worker-{}", worker_id.0))
                    .spawn(move || {
                        let mut ex = E::new();
                        // TODO configurable capacity
                        let (local_packet_tx, local_packet_rx) = localq::mpsc::channel(64);
                        let locals = Rc::new(RefCell::new(LocalContextMap::new()));

                        let shutdown_signal = rt.inner.worker_handles[worker_index as usize]
                            .shutdown_signal
                            .clone();

                        let inter_worker_batcher_senders = build_inter_worker_batchers(
                            &WorkerSpawner {
                                spawner: ex.spawner(),
                                shutdown_signal: shutdown_signal.clone(),
                                interval_fn: WorkerContext::interval_loop::<E>,
                            },
                            worker_id,
                            max_inter_worker_batch_size,
                            &inter_worker_senders,
                        );
                        let pp = Rc::new(PacketProcessor::new(
                            local_packet_tx.clone(),
                            inter_worker_batcher_senders,
                        ));

                        let worker_context = WorkerContext::new::<E>(
                            rt.clone(),
                            ex.spawner(),
                            shutdown_signal.clone(),
                            worker_id,
                            worker_count,
                            pp,
                            local_packet_tx,
                            locals,
                            globals,
                        );

                        let map_worker_context = unsafe {
                            &mut *rt.inner.worker_contexts[thid::ThreadId::current().as_usize()]
                                .get()
                        };
                        map_worker_context.write(worker_context.clone());

                        spawn_worker::<E>(
                            worker_context.clone(),
                            local_packet_rx,
                            command_rx,
                            inter_rx,
                        );

                        startup_done_tx
                            .send(())
                            .expect("modrpc::RuntimeBuilder::start worker startup oneshot send");

                        probius::enter_component(&format!("worker-{}", worker_id.0), move || {
                            // TODO this is not sufficient - need to add a local semaphore to wait
                            // for all spawned tasks to be polled to completion.
                            ex.run_until(shutdown_signal.wait_owned());
                        });

                        // Drop RuntimeHandle worker_contexts entry
                        unsafe {
                            let map_worker_context = &mut *rt.inner.worker_contexts
                                [thid::ThreadId::current().as_usize()]
                            .get();
                            map_worker_context.assume_init_drop();
                        }
                    })
                    .expect("spawn modrpc worker thread");

                // Ensure the worker_contexts map entry was inserted.
                startup_done_rx
                    .recv()
                    .expect("modrpc::RuntimeBuilder::start worker startup oneshot recv");

                worker_join_handles.push(join_handle);
            }
        }

        let rt_shutdown = RuntimeShutdownHandle {
            rt_shutdown_signal,
            worker_join_handles,
        };

        (rt, rt_shutdown)
    }
}

pub struct RuntimeShutdownHandle {
    rt_shutdown_signal: bab::SignalTree,
    worker_join_handles: Vec<std::thread::JoinHandle<()>>,
}

impl RuntimeShutdownHandle {
    pub async fn shutdown(self) {
        // TODO wait for local worker to shutdown

        self.rt_shutdown_signal.notify();

        for worker_join_handle in self.worker_join_handles {
            worker_join_handle.join().unwrap();
        }
    }
}

#[derive(Clone)]
pub struct RuntimeHandle {
    inner: Arc<RuntimeHandleInner>,
}

pub struct RuntimeHandleInner {
    next_role_id: AtomicU64,
    globals: Arc<Mutex<ContextMap>>,
    worker_handles: Vec<WorkerHandle>,
    // safety: can only be accessed immutably, and WorkerContexts can only be accessed from their
    // respective thread.
    worker_contexts: Vec<UnsafeCell<MaybeUninit<WorkerContext>>>,
}

// safety: see comments in the RuntimeHandle struct definition
unsafe impl Send for RuntimeHandle {}
unsafe impl Sync for RuntimeHandle {}

impl RuntimeHandle {
    pub fn single_threaded<E: modrpc_executor::ModrpcExecutor>(
        ex: &mut E,
    ) -> (Self, RuntimeShutdownHandle) {
        RuntimeBuilder::new_with_local(ex.spawner()).start::<E>()
    }

    pub fn globals(&self) -> &Arc<Mutex<ContextMap>> {
        &self.inner.globals
    }

    pub fn get_worker(&self, worker_id: WorkerId) -> &WorkerHandle {
        &self.inner.worker_handles[worker_id.0 as usize]
    }

    pub fn local_worker_context(&self) -> Option<&WorkerContext> {
        self.inner
            .worker_contexts
            .get(thid::ThreadId::current().as_usize())
            .map(|worker_cx| unsafe { (&*worker_cx.get()).assume_init_ref() })
    }

    pub async fn add_transport(&self, builder: impl TransportBuilder) -> TransportHandle {
        let transport_handle = builder.start_transport(TransportContext { rt: self }).await;

        if let Some(flush_sender) = transport_handle.writer_flush_sender() {
            // Spawn a task on all workers to periodically flush written buffers.

            // TODO need shutdown logic
            self.run_on_all_workers(move |worker_cx| {
                use crate::flush_batcher::FlushBatcherStatus;

                let flush_batcher =
                    TransportHandle::get_local_flush_batcher(worker_cx, &flush_sender);
                let mut sleeper = worker_cx.new_sleeper();
                worker_cx.spawn(async move {
                    loop {
                        // Wait until there is data to flush
                        flush_batcher.wait().await;

                        loop {
                            match flush_batcher.handle_flush() {
                                FlushBatcherStatus::Snooze { duration } => {
                                    sleeper.as_mut().snooze(duration);
                                    core::future::poll_fn(|cx| sleeper.as_mut().poll_sleep(cx))
                                        .await;
                                }
                                FlushBatcherStatus::FlushNow => {
                                    flush_sender.flush();
                                    break;
                                }
                                FlushBatcherStatus::DoNotFlush => break,
                            }
                        }
                    }
                });
            })
            .await;
        }

        transport_handle
    }

    pub async fn run_on_all_workers(
        &self,
        f: impl FnOnce(&WorkerContext) + Clone + Send + 'static,
    ) {
        // Note worker_handles also contains a handle for the local worker.
        for worker in &self.inner.worker_handles {
            worker.run_once(f.clone()).await;
        }
    }

    pub async fn run_on_all_other_workers(
        &self,
        f: impl FnOnce(&WorkerContext) + Clone + Send + 'static,
    ) {
        let local_worker_id = self.local_worker_context()
            .expect("modrpc::RuntimeHandle::run_on_all_other_workers must be called from a modrpc worker")
            .worker_id();

        // Note worker_handles also contains a handle for the local worker.
        for (worker_id, worker) in self.inner.worker_handles.iter().enumerate() {
            if local_worker_id.0 as usize == worker_id {
                // Skip the local worker's handle
                continue;
            }
            worker.run_once(f.clone()).await;
        }
    }

    pub fn start_role<Role>(&self, config: RoleConfig<Role>) -> StartRoleHandle<Role>
    where
        Role: InterfaceRole,
        Role::Init: Sync,
    {
        let role_shutdown_signal = bab::SignalTree::new();

        // Shutdown the role when its transport shuts down
        config
            .transport
            .shutdown_signal
            .add_child(role_shutdown_signal.clone());

        StartRoleHandle {
            rt: self.clone(),
            config,
            role_shutdown_signal,
            role_id: self.inner.next_role_id.fetch_add(1, Ordering::Relaxed),
            has_spawned_globals_shutdown: false,
        }
    }
}

#[derive(Clone)]
pub struct WorkerHandle {
    command_tx: burstq::Sender<WorkerCommand>,
    shutdown_signal: bab::SignalTree,
}

impl WorkerHandle {
    pub async fn run_once<R: Send + 'static>(
        &self,
        f: impl FnOnce(&WorkerContext) -> R + Send + 'static,
    ) -> R {
        let (result_tx, result_rx) = oneshot::channel();
        let command = WorkerCommand(Box::new(move |worker_cx| {
            let _ = result_tx.send(f(worker_cx));
        }));
        // TODO janky
        self.command_tx
            .send(1, move |mut w| unsafe {
                w.write_at(0, command);
            })
            .await
            .unwrap();
        result_rx.await.unwrap()
    }
}

#[derive(Clone)]
pub enum TopicChannels {
    SingleChannel {
        channel_id: u32,
    },
    MultiChannel {
        // TODO
    },
}

impl TopicChannels {
    fn create_packet_sender(
        &self,
        worker_cx: &WorkerContext,
        transport: &TransportHandle,
        endpoint_addr: EndpointAddr,
    ) -> PacketSender {
        match self {
            TopicChannels::SingleChannel { channel_id } => {
                let (writer, flush_batcher) = transport.new_writer(worker_cx, *channel_id);
                SingleChannelSender {
                    endpoint_addr,
                    tx_channel: writer,
                    pp: worker_cx.pp.clone(),
                    flush_batcher,
                }
                .into()
            }
            TopicChannels::MultiChannel {} => {
                todo!();
            }
        }
    }
}

pub struct RoleConfig<Role: InterfaceRole> {
    pub plane_id: u32,
    pub endpoint_addr: EndpointAddr,
    // Specifies which transport packets will be sent out on, and allowlists this plane
    // on the transport's rx half.
    pub transport: TransportHandle,
    pub topic_channels: TopicChannels,
    pub config: Role::Config,
    // TODO new mproto::LazyBuf<Role::Init> struct backed by byteview
    pub init: Role::Init,
}

pub struct StartRoleHandle<Role: InterfaceRole> {
    pub rt: RuntimeHandle,
    pub config: RoleConfig<Role>,
    pub role_shutdown_signal: bab::SignalTree,

    role_id: u64,
    has_spawned_globals_shutdown: bool,
}

impl<Role: InterfaceRole> StartRoleHandle<Role>
where
    Role::Config: Clone + Send,
    Role::Init: Clone + Send,
{
    pub async fn on_worker_group(
        mut self,
        worker_group: WorkerGroup,
        role_start_fn: impl RoleStartFn<Role> + Clone + Send + Sync + 'static,
    ) -> Self {
        let start_worker = worker_group.first_worker_index;
        let end_worker = start_worker + worker_group.worker_count;
        for worker_index in start_worker..end_worker {
            let spawn_globals_shutdown = !self.has_spawned_globals_shutdown;
            self.has_spawned_globals_shutdown = true;

            let rt = self.rt.clone();
            let worker = &self.rt.inner.worker_handles[worker_index as usize];
            let role_start_fn = role_start_fn.clone();
            let role_shutdown_signal = self.role_shutdown_signal.clone();
            let role_id = self.role_id;

            let plane_id = self.config.plane_id;
            let endpoint_addr = self.config.endpoint_addr;
            let transport = self.config.transport.clone();
            let topic_channels = self.config.topic_channels.clone();
            let config: Role::Config = self.config.config.clone();
            let init: Role::Init = self.config.init.clone();

            worker
                .run_once(move |worker_cx| {
                    let packet_sender =
                        topic_channels.create_packet_sender(&worker_cx, &transport, endpoint_addr);

                    let _role_handle = run_role_worker::<Role>(
                        &rt,
                        plane_id,
                        role_id,
                        worker_cx.spawner.clone(),
                        &worker_cx,
                        worker_cx.shutdown_signal.clone(),
                        role_shutdown_signal.clone(),
                        endpoint_addr,
                        packet_sender,
                        &config,
                        &init,
                        role_start_fn,
                    );

                    // Make the first worker to start running the role spawn a task to remove any
                    // globals the role creates.
                    if spawn_globals_shutdown {
                        let globals = worker_cx.globals.clone();
                        worker_cx
                            .spawner
                            .spawn(async move {
                                role_shutdown_signal.wait().await;
                                globals
                                    .lock()
                                    .expect("modrpc globals map poisoned")
                                    .shutdown_tag(ModrpcContextTag::Role(role_id));
                            })
                            .expect("modrpc spawn role globals shutdown");
                    }
                })
                .await;
        }

        self
    }

    pub fn local(mut self, role_start_fn: impl RoleStartFn<Role>) -> Role::Hooks {
        let Some(worker_cx) = &self.rt.local_worker_context() else {
            panic!("Can't start a role locally in a thread that isn't a modrpc worker");
        };

        let RoleConfig {
            plane_id,
            endpoint_addr,
            transport,
            topic_channels,
            config,
            init,
        } = self.config;

        let packet_sender =
            topic_channels.create_packet_sender(&worker_cx, &transport, endpoint_addr);
        let hooks = run_role_worker::<Role>(
            &self.rt,
            plane_id,
            self.role_id,
            worker_cx.spawner.clone(),
            &worker_cx,
            worker_cx.shutdown_signal.clone(),
            self.role_shutdown_signal.clone(),
            endpoint_addr,
            packet_sender,
            &config,
            &init,
            role_start_fn,
        );

        // Make the first worker to start running the role spawn a task to remove any
        // globals the role creates.
        if !self.has_spawned_globals_shutdown {
            let role_shutdown_signal = self.role_shutdown_signal;
            let globals = worker_cx.globals.clone();
            worker_cx
                .spawner
                .spawn(async move {
                    role_shutdown_signal.wait().await;
                    globals
                        .lock()
                        .expect("modrpc globals map poisoned")
                        .shutdown_tag(ModrpcContextTag::Role(self.role_id));
                })
                .expect("modrpc spawn role globals shutdown");

            self.has_spawned_globals_shutdown = true;
        }

        hooks
    }
}

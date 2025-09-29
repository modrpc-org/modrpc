use std::rc::Rc;
use std::sync::{Arc, Mutex};

use core::{cell::RefCell, future::Future, hash::Hash, pin::Pin, time::Duration};

use futures_lite::future;

use crate::{
    EndpointAddr, LocalSpawner, Packet, RoleSetup, RuntimeHandle, SendPacket,
    context_map::{ContextClass, ContextMap, LocalContextMap, ModrpcContextTag},
    packet_processor::{
        PACKET_PROCESSOR_SOURCE_INTER_WORKER, PACKET_PROCESSOR_SOURCE_NEW, PacketProcessor,
        PacketProcessorSource,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WorkerId(pub u16);

impl WorkerId {
    pub fn local() -> Self {
        WorkerId(0)
    }
}

pub struct WorkerSpawner {
    pub(crate) spawner: LocalSpawner,
    pub(crate) interval_fn:
        for<'a> fn(Duration, &'a mut dyn FnMut()) -> Pin<Box<dyn Future<Output = ()> + 'a>>,
    pub(crate) shutdown_signal: bab::SignalTree,
}

impl WorkerSpawner {
    pub fn raw_spawner(&self) -> &LocalSpawner {
        &self.spawner
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + 'static) {
        let shutdown_signal = self.shutdown_signal.clone();
        self.spawner
            .spawn(future::or(future, shutdown_signal.wait_owned()))
            .unwrap();
    }

    pub fn spawn_interval_loop<'a>(
        &self,
        interval_duration: Duration,
        mut f: impl FnMut() + 'static,
    ) {
        let interval_fn = self.interval_fn;
        self.spawn(async move {
            (interval_fn)(interval_duration, &mut f).await;
        });
    }
}

#[derive(Clone)]
pub struct WorkerContext {
    pub(crate) rt: RuntimeHandle,
    pub(crate) spawner: LocalSpawner,
    pub(crate) shutdown_signal: bab::SignalTree,
    pub(crate) worker_id: WorkerId,
    pub(crate) worker_count: u16,
    pub(crate) pp: Rc<PacketProcessor>,
    pub(crate) local_packet_tx: localq::mpsc::Sender<Packet>,
    pub(crate) locals: Rc<RefCell<LocalContextMap>>,
    pub(crate) globals: Arc<Mutex<ContextMap>>,
    pub(crate) interval_fn:
        for<'a> fn(Duration, &'a mut dyn FnMut()) -> Pin<Box<dyn Future<Output = ()> + 'a>>,
    pub(crate) sleep_fn: fn(Duration) -> Pin<Box<dyn Future<Output = ()>>>,
    pub(crate) new_sleeper_fn: fn() -> Pin<Box<dyn modrpc_executor::Sleeper>>,
}

impl WorkerContext {
    pub fn new<E: modrpc_executor::ModrpcExecutor>(
        rt: RuntimeHandle,
        spawner: LocalSpawner,
        shutdown_signal: bab::SignalTree,
        worker_id: WorkerId,
        worker_count: u16,
        pp: Rc<PacketProcessor>,
        local_packet_tx: localq::mpsc::Sender<Packet>,
        locals: Rc<RefCell<LocalContextMap>>,
        globals: Arc<Mutex<ContextMap>>,
    ) -> Self {
        Self {
            rt,
            spawner,
            shutdown_signal,
            worker_id,
            worker_count,
            pp,
            local_packet_tx,
            locals,
            globals,
            interval_fn: Self::interval_loop::<E>,
            sleep_fn: Self::sleep_dyn::<E>,
            new_sleeper_fn: Self::new_sleeper_dyn::<E>,
        }
    }

    pub fn rt(&self) -> &RuntimeHandle {
        &self.rt
    }

    pub fn spawner(&self) -> WorkerSpawner {
        WorkerSpawner {
            spawner: self.spawner.clone(),
            interval_fn: self.interval_fn,
            shutdown_signal: self.shutdown_signal.clone(),
        }
    }

    pub fn worker_id(&self) -> WorkerId {
        self.worker_id
    }

    pub fn worker_count(&self) -> u16 {
        self.worker_count
    }

    pub fn local_packet_tx(&self) -> &localq::mpsc::Sender<Packet> {
        &self.local_packet_tx
    }

    pub async fn sleep(&self, duration: Duration) {
        (self.sleep_fn)(duration).await
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + 'static) {
        let shutdown_signal = self.shutdown_signal.clone();
        self.spawner
            .spawn(future::or(future, shutdown_signal.wait_owned()))
            .unwrap();
    }

    pub fn new_sleeper(&self) -> Pin<Box<dyn modrpc_executor::Sleeper>> {
        (self.new_sleeper_fn)()
    }

    fn new_sleeper_dyn<E: modrpc_executor::ModrpcExecutor>()
    -> Pin<Box<dyn modrpc_executor::Sleeper>> {
        Box::pin(E::new_sleeper())
    }

    fn sleep_dyn<'a, E: modrpc_executor::ModrpcExecutor>(
        duration: Duration,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(E::sleep(duration))
    }

    pub(crate) fn interval_loop<'a, E: modrpc_executor::ModrpcExecutor>(
        interval_duration: Duration,
        f: &'a mut dyn FnMut(),
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        use modrpc_executor::Interval;

        let mut interval = E::interval(interval_duration);
        Box::pin(async move {
            loop {
                interval.tick().await;
                f();
            }
        })
    }

    pub fn spawn_interval_loop<'a>(
        &self,
        interval_duration: Duration,
        mut f: impl FnMut() + 'static,
    ) {
        let interval_fn = self.interval_fn;
        self.spawn(async move {
            (interval_fn)(interval_duration, &mut f).await;
        });
    }

    pub fn spawn_traced(
        &self,
        name: &str,
        flush_interval: Duration,
        f: impl core::ops::AsyncFnOnce(&probius::TraceSource) + 'static,
    ) {
        let interval_fn = self.interval_fn;
        let tracer = probius::new_trace_source(name);
        self.spawn(async move {
            future::or(
                f(&tracer),
                (interval_fn)(flush_interval, &mut || {
                    tracer.flush_aggregate_full();
                }),
            )
            .await
        });
    }

    pub fn with_local<T, U>(
        &self,
        key: T::Key,
        params: &T::Params,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        T: ContextClass + 'static,
    {
        let tag = ModrpcContextTag::Worker(self.worker_id);
        self.locals.borrow_mut().with(tag, key, params, f)
    }

    pub fn with_local_fn<K, T, U>(
        &self,
        key: K,
        constructor: impl FnOnce() -> T,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        K: Eq + core::hash::Hash,
        T: 'static,
    {
        let tag = ModrpcContextTag::Worker(self.worker_id);
        self.locals.borrow_mut().with_fn(tag, key, constructor, f)
    }

    pub fn with_global<T, U>(
        &self,
        key: T::Key,
        params: &T::Params,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        T: ContextClass + Send + 'static,
        T::Key: Send,
    {
        let tag = ModrpcContextTag::Worker(self.worker_id);
        self.globals
            .lock()
            .expect("BUG: modrpc globals lock is poisoned.")
            .with(tag, key, params, f)
    }

    pub fn add_handler(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        handler: impl FnMut(EndpointAddr, &Packet) + 'static,
    ) {
        self.pp
            .add_handler(topic_name, source_filter, plane_id, topic, handler);
    }

    pub fn add_local_queue(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        queue: localq::mpsc::Sender<Packet>,
    ) {
        self.pp
            .add_local_queue(topic_name, source_filter, plane_id, topic, queue);
    }

    pub fn route_to_local_queue(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        handler: impl FnMut(EndpointAddr, &Packet) -> Option<localq::mpsc::Sender<Packet>> + 'static,
    ) {
        self.pp
            .add_route_to_local_queue(topic_name, source_filter, plane_id, topic, handler);
    }

    pub fn add_redirect_to_worker(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        to_worker: WorkerId,
    ) {
        self.pp
            .to_worker(topic_name, source_filter, plane_id, topic, to_worker);
    }

    pub fn route_to_worker(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        handler: impl FnMut(EndpointAddr, &Packet) -> Option<WorkerId> + 'static,
    ) {
        self.pp
            .add_route_to_worker(topic_name, source_filter, plane_id, topic, handler);
    }

    /// Get an async closure that can inject packets on this worker. Intended to be used by custom
    /// Transport implementations.
    pub fn get_packet_processor(&self) -> impl AsyncFn(&Packet) + 'static {
        let pp = self.pp.clone();
        async move |p| pp.handle_packet(PACKET_PROCESSOR_SOURCE_NEW, p).await
    }
}

pub(crate) struct WorkerCommand(pub Box<dyn FnOnce(&WorkerContext) + Send>);

pub(crate) fn spawn_worker<E: modrpc_executor::ModrpcExecutor>(
    worker_cx: WorkerContext,
    mut local_packet_rx: localq::mpsc::Receiver<Packet>,
    command_rx: burstq::Receiver<WorkerCommand>,
    inter_rx: burstq::Receiver<SendPacket>,
) {
    use core::mem::MaybeUninit;

    // Spawn task to process incoming worker commands
    worker_cx.spawn({
        let worker_cx = worker_cx.clone();
        async move {
            let mut batch: Vec<MaybeUninit<_>> = (0..8).map(|_| MaybeUninit::uninit()).collect();

            while let Ok(n) = command_rx
                .recv(8, |r| {
                    for (i, f) in r.into_iter().enumerate() {
                        batch[i].write(f);
                    }
                })
                .await
            {
                for i in 0..n {
                    let WorkerCommand(f) = unsafe { batch[i].assume_init_read() };
                    f(&worker_cx);
                }
            }
        }
    });

    // Spawn task to process packets created by this worker.
    worker_cx.spawn_traced("local-rx", Duration::from_millis(1000), {
        let pp = worker_cx.pp.clone();
        async move |tracer| {
            while let Ok(packet) = local_packet_rx.recv().await {
                tracer
                    .trace_future(async {
                        probius::trace_label("receive-packet");
                        probius::trace_metric("packet_size", packet.len() as i64);

                        // TODO
                        //pp.handle_packet(PACKET_PROCESSOR_SOURCE_NEW, &packet).await;
                    })
                    .await;

                pp.handle_packet(PACKET_PROCESSOR_SOURCE_NEW, &packet).await;
            }
        }
    });

    // Spawn task to process incoming inter-worker packets
    worker_cx.spawn_traced("inter-worker-rx", Duration::from_millis(1000), {
        let worker_cx = worker_cx.clone();
        async move |tracer| {
            let batch_size = 32;
            let mut batch: Vec<MaybeUninit<_>> =
                (0..batch_size).map(|_| MaybeUninit::uninit()).collect();

            while let Ok(n) = inter_rx
                .recv(batch_size, |r| {
                    for (i, f) in r.into_iter().enumerate() {
                        batch[i].write(f);
                    }
                })
                .await
            {
                tracer.trace(|| {
                    probius::trace_metric("batch_size", n as i64);
                });

                for i in 0..n {
                    let packet = unsafe { batch[i].assume_init_read() };
                    let packet = packet.receive();
                    worker_cx
                        .pp
                        .handle_packet(PACKET_PROCESSOR_SOURCE_INTER_WORKER, &packet)
                        .await;
                }
            }
        }
    });

    // Spawn task to flush PacketProcessor trace sources
    let mut sleeper = worker_cx.new_sleeper();
    worker_cx.spawn({
        let pp = worker_cx.pp.clone();
        async move {
            loop {
                sleeper.as_mut().snooze(Duration::from_millis(1000));
                pp.flush_traces().await;
                core::future::poll_fn(|cx| sleeper.as_mut().poll_sleep(cx)).await;
            }
        }
    });
}

struct GlobalQueue<K> {
    tx: burstq::Sender<SendPacket>,
    rx: burstq::Receiver<SendPacket>,
    k: core::marker::PhantomData<K>,
}

impl<K: Eq + Hash + Send + 'static> ContextClass for GlobalQueue<K> {
    type Key = K;
    type Params = GlobalQueueParams;

    fn new(params: &Self::Params) -> Self {
        let (tx, rx) = burstq::mpmc(params.capacity);
        GlobalQueue {
            tx,
            rx,
            k: core::marker::PhantomData,
        }
    }
}

pub struct GlobalQueueParams {
    capacity: usize,
}

pub fn get_global_queue_sender<K: Eq + Hash + Send + 'static>(
    cx: &RoleSetup,
    key: K,
    capacity: usize,
    tx_burst_size: usize,
) -> localq::mpsc::Sender<Packet> {
    let global_tx = cx.with_global(
        key,
        &GlobalQueueParams { capacity },
        |q: &mut GlobalQueue<K>| q.tx.clone(),
    );

    let (local_tx, local_rx) = localq::mpsc::channel::<Packet>(tx_burst_size);
    cx.role_spawner()
        .spawn(run_burstq_batcher(global_tx, local_rx));

    local_tx
}

pub fn get_global_queue_receiver<K: Eq + Hash + Send + 'static>(
    cx: &RoleSetup,
    key: K,
    capacity: usize,
) -> burstq::Receiver<SendPacket> {
    cx.with_global(
        key,
        &GlobalQueueParams { capacity },
        |q: &mut GlobalQueue<K>| q.rx.clone(),
    )
}

pub async fn run_burstq_batcher(
    shared_tx: burstq::Sender<SendPacket>,
    mut local_rx: localq::mpsc::Receiver<Packet>,
) {
    while let Ok(first_packet) = local_rx.recv().await {
        let batch_size = local_rx.len() + 1;
        let _ = shared_tx
            .send(batch_size, |mut w| unsafe {
                w.write_at(0, first_packet.send());
                for i in 1..w.len() {
                    w.write_at(i, local_rx.try_recv().unwrap().send());
                }
            })
            .await;
    }
}

pub fn build_inter_worker_batchers(
    spawner: &WorkerSpawner,
    local_worker_id: WorkerId,
    max_batch_size: usize,
    global_senders: &[burstq::Sender<SendPacket>],
) -> Vec<Option<localq::mpsc::Sender<Packet>>> {
    let mut senders = vec![];

    for (worker_index, shared_tx) in global_senders.iter().enumerate() {
        if worker_index == local_worker_id.0 as usize {
            senders.push(None);
            continue;
        }
        let (local_tx, local_rx) = localq::mpsc::channel::<Packet>(max_batch_size);
        senders.push(Some(local_tx));

        spawner.spawn(run_burstq_batcher(shared_tx.clone(), local_rx));
    }

    senders
}

use std::future::Future;

use crate::{
    ContextClass, Packet, RoleSpawner,
    context_map::ModrpcContextTag,
    endpoint_proto::{EndpointAddr, TransmitPacket},
    interface_builder::InterfaceEvent,
    load_balancer,
    packet_processor::{
        PACKET_PROCESSOR_SOURCE_ANY, PACKET_PROCESSOR_SOURCE_INTER_WORKER,
        PACKET_PROCESSOR_SOURCE_NEW,
    },
    packet_sender::PacketSender,
    worker::WorkerContext,
};

pub struct RoleSetup<'a> {
    spawner: RoleSpawner,
    packet_sender: PacketSender,
    plane_id: u32,
    role_id: u64,
    endpoint_addr: EndpointAddr,
    worker_cx: &'a WorkerContext,
    shutdown_signal: bab::SignalTree,
    object_path: Vec<&'static str>,
}

impl<'a> RoleSetup<'a> {
    pub fn new(
        root_object_name: &'static str,
        spawner: RoleSpawner,
        worker_cx: &'a WorkerContext,
        packet_sender: PacketSender,
        plane_id: u32,
        role_id: u64,
        endpoint_addr: EndpointAddr,
        shutdown_signal: bab::SignalTree,
    ) -> Self {
        Self {
            spawner,
            worker_cx,
            packet_sender,
            plane_id,
            role_id,
            endpoint_addr,
            shutdown_signal,
            object_path: vec![root_object_name],
        }
    }

    pub fn endpoint_addr(&self) -> EndpointAddr {
        self.endpoint_addr
    }

    pub fn worker_id(&self) -> u16 {
        self.worker_cx.worker_id().0
    }

    pub fn plane_id(&self) -> u32 {
        self.plane_id
    }

    pub fn role_id(&self) -> u64 {
        self.role_id
    }

    pub fn role_spawner(&self) -> &RoleSpawner {
        &self.spawner
    }

    pub fn role_shutdown_signal(&self) -> &bab::SignalTree {
        &self.shutdown_signal
    }

    pub fn worker_context(&self) -> &WorkerContext {
        self.worker_cx
    }

    pub fn push_object_path(&mut self, name: &'static str) {
        self.object_path.push(name);
    }

    pub fn pop_object_path(&mut self) {
        self.object_path.pop();
    }

    pub fn get_object_path(&self, end: &'static str) -> String {
        let mut path = "".to_string();
        for object_name in &self.object_path {
            path += object_name;
            path += ".";
        }
        path += end;
        path
    }

    pub fn event_tx<T>(&self, spec: InterfaceEvent<T>) -> EventTx<T>
    where
        T: mproto::Encode + for<'d> mproto::Decode<'d>,
    {
        EventTx::new(self.packet_sender.clone(), self.plane_id, spec.topic)
    }

    pub fn event_rx<T: mproto::Owned>(&self, spec: InterfaceEvent<T>) -> EventRxBuilder<T> {
        EventRxBuilder::new(spec, self.get_object_path(spec.name))
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
        let tag = ModrpcContextTag::Role(self.role_id);
        self.worker_cx.locals.borrow_mut().with(tag, key, params, f)
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
        let tag = ModrpcContextTag::Role(self.role_id);
        self.worker_cx
            .locals
            .borrow_mut()
            .with_fn(tag, key, constructor, f)
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
        let tag = ModrpcContextTag::Role(self.role_id);
        self.worker_cx
            .globals
            .lock()
            .expect("BUG: modrpc globals lock is poisoned.")
            .with(tag, key, params, f)
    }
}

pub struct EventTx<T> {
    packet_sender: PacketSender,
    plane_id: u32,
    topic: u32,
    _payload_type: std::marker::PhantomData<T>,
}

impl<T: mproto::Encode> EventTx<T> {
    pub fn new(packet_sender: PacketSender, plane_id: u32, topic: u32) -> Self {
        Self {
            packet_sender,
            plane_id,
            topic,
            _payload_type: std::marker::PhantomData,
        }
    }

    pub fn plane_id(&self) -> u32 {
        self.plane_id
    }

    pub fn topic(&self) -> u32 {
        self.topic
    }

    pub fn try_send(&self, payload: impl mproto::Encode + mproto::Compatible<T>) -> bool {
        self.packet_sender
            .try_send(self.plane_id, self.topic, payload)
    }

    pub async fn send(&self, payload: impl mproto::Encode + mproto::Compatible<T>) {
        self.packet_sender
            .send(self.plane_id, self.topic, payload)
            .await;
    }

    pub async unsafe fn send_buffer(&self, buffer: bab::BufferPtr) {
        self.packet_sender
            .send_buffer(self.plane_id, self.topic, buffer)
            .await;
    }

    pub fn cast<NewT>(self) -> EventTx<NewT>
    where
        NewT: mproto::Encode + mproto::Compatible<T>,
    {
        EventTx {
            packet_sender: self.packet_sender,
            plane_id: self.plane_id,
            topic: self.topic,
            _payload_type: std::marker::PhantomData,
        }
    }
}

impl EventTx<()> {
    pub async fn send_raw(&self, len: usize, f: impl FnOnce(&mut [u8])) {
        self.packet_sender
            .send_raw(self.plane_id, self.topic, len, f)
            .await;
    }
}

impl<T> PartialEq for EventTx<T> {
    fn eq(&self, other: &Self) -> bool {
        self.topic == other.topic && self.packet_sender.id() == other.packet_sender.id()
    }
}
impl<T> Eq for EventTx<T> {}

impl<T> std::hash::Hash for EventTx<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.topic.hash(state);
        self.packet_sender.id().hash(state);
    }
}

impl<T> Clone for EventTx<T> {
    fn clone(&self) -> Self {
        Self {
            packet_sender: self.packet_sender.clone(),
            plane_id: self.plane_id,
            topic: self.topic,
            _payload_type: std::marker::PhantomData,
        }
    }
}

pub async fn handle_events_async<T: mproto::Owned>(
    tracer: &probius::TraceSource,
    mut rx: localq::mpsc::Receiver<Packet>,
    mut handler: impl AsyncFnMut(EndpointAddr, T::Lazy<'_>),
) where
    for<'a> T: mproto::Decode<'a> + 'a,
{
    while let Ok(payload_buf) = rx.recv().await {
        tracer
            .trace_future(async {
                probius::trace_label("receive-packet");
                probius::trace_metric("packet_size", payload_buf.len() as i64);
                probius::trace_branch_start();

                // Decode packet header
                let Ok(header) = mproto::decode_value::<TransmitPacket>(payload_buf.as_ref())
                else {
                    probius::trace_label("invalid-header");
                    probius::trace_branch_end();
                    return;
                };
                let header_len = mproto::encoded_len(&header);
                let source = header.source;

                // Decode payload
                let Ok(payload) = mproto::decode_value(&payload_buf.as_ref()[header_len..]) else {
                    probius::trace_label("invalid-payload");
                    probius::trace_branch_end();
                    return;
                };

                // Handle event
                probius::trace_label("call-handler");
                handler(source, payload).await;

                probius::trace_branch_end();
            })
            .await;
    }
}

pub async fn handle_untyped_async(
    tracer: &probius::TraceSource,
    mut rx: localq::mpsc::Receiver<Packet>,
    mut handler: impl AsyncFnMut(EndpointAddr, Packet),
) {
    while let Ok(packet) = rx.recv().await {
        tracer
            .trace_future(async {
                probius::trace_label("receive-packet");
                probius::trace_metric("packet_size", packet.len() as i64);
                probius::trace_branch_start();

                // Decode packet header
                let Ok(header) = mproto::decode_value::<TransmitPacket>(packet.as_ref()) else {
                    probius::trace_label("invalid-header");
                    probius::trace_branch_end();
                    return;
                };

                // Handle event
                probius::trace_label("call-handler");
                handler(header.source, packet).await;

                probius::trace_branch_end();
            })
            .await;
    }
}

pub trait AsyncHandler {
    type Input<'a>: mproto::Decode<'a>
    where
        Self: 'a;
    type Context<'a>
    where
        Self: 'a;
    type Output;
    type Future<'a>: Future<Output = Self::Output> + 'a
    where
        Self: 'a;

    fn call<'a>(
        &'a mut self,
        context: Self::Context<'a>,
        input: Self::Input<'a>,
    ) -> Self::Future<'a>;
}

#[must_use]
pub struct EventRxBuilder<T> {
    spec: InterfaceEvent<T>,
    object_path: String,
}

impl<T> Clone for EventRxBuilder<T> {
    fn clone(&self) -> Self {
        Self {
            object_path: self.object_path.clone(),
            spec: self.spec,
        }
    }
}

impl<T: mproto::Owned> EventRxBuilder<T> {
    pub fn new(spec: InterfaceEvent<T>, object_path: String) -> Self {
        Self { spec, object_path }
    }

    pub fn inline<'a>(
        self,
        setup: &'a RoleSetup<'a>,
        mut event_handler: impl FnMut(EndpointAddr, T) + 'static,
    ) -> InlineRxRouterSetup<'a, impl FnMut(EndpointAddr, &Packet) + 'static> {
        InlineRxRouterSetup {
            setup,
            object_path: self.object_path,
            topic: self.spec.topic,
            f: move |source, packet: &Packet| {
                use mproto::BaseLen;

                let Ok(payload) = mproto::decode_value(&packet[TransmitPacket::BASE_LEN..]) else {
                    return;
                };

                event_handler(source, payload)
            },
        }
    }

    pub fn inline_lazy<'a>(
        self,
        setup: &'a RoleSetup<'a>,
        mut event_handler: impl FnMut(EndpointAddr, T::Lazy<'_>) + 'static,
    ) -> InlineRxRouterSetup<'a, impl FnMut(EndpointAddr, &Packet) + 'static> {
        InlineRxRouterSetup {
            setup,
            object_path: self.object_path,
            topic: self.spec.topic,
            f: move |source, packet: &Packet| {
                use mproto::BaseLen;

                let Ok(payload) = mproto::decode_value(&packet[TransmitPacket::BASE_LEN..]) else {
                    return;
                };

                event_handler(source, payload)
            },
        }
    }

    pub fn inline_untyped<'a>(
        self,
        setup: &'a RoleSetup<'a>,
        event_handler: impl FnMut(EndpointAddr, &Packet) + 'static,
    ) -> InlineRxRouterSetup<'a, impl FnMut(EndpointAddr, &Packet) + 'static> {
        InlineRxRouterSetup {
            setup,
            object_path: self.object_path,
            topic: self.spec.topic,
            f: event_handler,
        }
    }

    pub fn route_to_local_queue<'a>(
        self,
        setup: &'a RoleSetup<'a>,
        handler: impl FnMut(EndpointAddr, &Packet) -> Option<localq::mpsc::Sender<Packet>> + 'static,
    ) {
        setup.worker_context().route_to_local_queue(
            &self.object_path,
            PACKET_PROCESSOR_SOURCE_ANY,
            setup.plane_id,
            self.spec.topic,
            handler,
        );
    }

    pub fn route_to_worker<'a>(
        self,
        setup: &'a RoleSetup<'a>,
        handler: impl FnMut(EndpointAddr, &Packet) -> Option<crate::WorkerId> + 'static,
    ) {
        setup.worker_context().route_to_worker(
            &self.object_path,
            // It only makes sense to route novel packets to another worker, not packets already
            // redirected to us by another worker.
            PACKET_PROCESSOR_SOURCE_NEW,
            setup.plane_id,
            self.spec.topic,
            handler,
        );
    }

    pub fn queued<'a>(
        self,
        setup: &'a RoleSetup<'a>,
        handler: impl AsyncFnMut(EndpointAddr, T::Lazy<'_>) + 'static,
    ) -> QueuedEventRxRouterSetup<'a> {
        let (queue_tx, queue_rx) = localq::mpsc::channel(64);

        setup.worker_context().spawn_traced(
            &self.object_path,
            core::time::Duration::from_millis(1000),
            async move |tracer| handle_events_async(tracer, queue_rx, handler).await,
        );

        QueuedEventRxRouterSetup {
            setup,
            object_path: self.object_path,
            topic: self.spec.topic,
            queue_tx,
        }
    }

    pub fn queued_untyped<'a>(
        self,
        setup: &'a RoleSetup<'a>,
        event_handler: impl AsyncFnMut(EndpointAddr, Packet) + 'static,
    ) -> QueuedEventRxRouterSetup<'a> {
        let (queue_tx, queue_rx) = localq::mpsc::channel(64);

        setup.worker_context().spawn_traced(
            &self.object_path,
            core::time::Duration::from_millis(1000),
            async move |tracer| handle_untyped_async(tracer, queue_rx, event_handler).await,
        );

        QueuedEventRxRouterSetup {
            setup,
            object_path: self.object_path,
            topic: self.spec.topic,
            queue_tx,
        }
    }

    pub fn proxy_load_balance(self, setup: &RoleSetup) {
        load_balancer::proxy_load_balancer(
            setup,
            setup.plane_id,
            self.spec.topic,
            self.object_path,
        );
    }
}

#[must_use]
pub struct QueuedEventRxRouterSetup<'a> {
    setup: &'a RoleSetup<'a>,
    object_path: String,
    topic: u32,
    queue_tx: localq::mpsc::Sender<Packet>,
}

impl<'a> QueuedEventRxRouterSetup<'a> {
    pub fn load_balance(self) {
        load_balancer::spawn_load_balancer(
            &self.setup,
            load_balancer::LoadBalancerConfig {
                rx_burst_size: 32,
                tx_burst_size: 32,
            },
            self.setup.plane_id,
            self.topic,
            self.object_path,
            self.queue_tx,
        );
    }

    pub fn subscribe(self) {
        self.setup.worker_context().add_local_queue(
            &self.object_path,
            PACKET_PROCESSOR_SOURCE_ANY,
            self.setup.plane_id,
            self.topic,
            self.queue_tx,
        );

        add_topic_subscription(
            self.setup.worker_cx,
            &self.object_path,
            self.setup.plane_id,
            self.topic,
        );
    }

    pub fn local(self) {
        self.setup.worker_context().add_local_queue(
            &self.object_path,
            PACKET_PROCESSOR_SOURCE_ANY,
            self.setup.plane_id,
            self.topic,
            self.queue_tx,
        );
    }
}

#[must_use]
pub struct InlineRxRouterSetup<'a, F> {
    setup: &'a RoleSetup<'a>,
    object_path: String,
    topic: u32,
    f: F,
}

impl<'a, F> InlineRxRouterSetup<'a, F>
where
    F: FnMut(EndpointAddr, &Packet) + 'static,
{
    pub fn subscribe(self) {
        self.setup.worker_context().add_handler(
            &self.object_path,
            PACKET_PROCESSOR_SOURCE_ANY,
            self.setup.plane_id,
            self.topic,
            self.f,
        );

        add_topic_subscription(
            self.setup.worker_cx,
            &self.object_path,
            self.setup.plane_id,
            self.topic,
        );
    }

    pub fn local(self) {
        self.setup.worker_context().add_handler(
            &self.object_path,
            PACKET_PROCESSOR_SOURCE_ANY,
            self.setup.plane_id,
            self.topic,
            self.f,
        );
    }

    pub fn from_inter_worker(self) {
        self.setup.worker_context().add_handler(
            &self.object_path,
            PACKET_PROCESSOR_SOURCE_INTER_WORKER,
            self.setup.plane_id,
            self.topic,
            self.f,
        );
    }
}

struct TopicSubscriptions {
    worker_mask: u64,
}

impl ContextClass for TopicSubscriptions {
    type Key = (u32, u32); // (plane id, topic)
    type Params = ();

    fn new(_: &Self::Params) -> Self {
        TopicSubscriptions { worker_mask: 0 }
    }
}

pub fn add_topic_subscription(
    worker_cx: &WorkerContext,
    object_path: &str,
    plane_id: u32,
    topic: u32,
) {
    let worker_id = worker_cx.worker_id();
    let needs_handler = worker_cx.with_global(
        (plane_id, topic),
        &(),
        |topic_subscriptions: &mut TopicSubscriptions| {
            let local_worker_mask = 1 << worker_id.0;
            if topic_subscriptions.worker_mask & local_worker_mask != 0 {
                false
            } else {
                topic_subscriptions.worker_mask |= local_worker_mask;
                true
            }
        },
    );

    if needs_handler {
        // Subscribe the current worker to receive packets for this topic unconditionally.

        // TODO this is probably too janky for use-cases where there's a high rate of new planes
        // being established (e.g. lots of new connections coming in, and a separate plane per
        // connection). Could we have an unbounded fire-and-forget command queue based on an
        // intrusive linked list? Or otherwise make plane setup be async?
        let object_path = object_path.to_string();
        pollster::block_on(worker_cx.rt().run_on_all_other_workers(move |worker_cx| {
            worker_cx.add_redirect_to_worker(
                &object_path,
                // Only broadcast packets that are new to this endpoint to avoid re-broadcasting.
                PACKET_PROCESSOR_SOURCE_NEW,
                plane_id,
                topic,
                worker_id,
            );
        }));
    }
}

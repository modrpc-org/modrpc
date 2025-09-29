use core::cell::{Cell, RefCell, UnsafeCell};
use std::collections::{HashMap, HashSet};

use bab::Packet;
use mproto::Decode;
use probius::TraceSource;

use crate::{EndpointAddr, TransmitPacket, WorkerId};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PacketProcessorSource(u8);

/// All packets will match this mask regardless of the source.
pub const PACKET_PROCESSOR_SOURCE_ANY: PacketProcessorSource = PacketProcessorSource(u8::MAX);
/// Packets that are being seen for the first time by this endpoint - a packet that matches this
/// mask was either locally-generated on the current worker or just arrived via a transport.
pub const PACKET_PROCESSOR_SOURCE_NEW: PacketProcessorSource = PacketProcessorSource(1 << 0);
/// Packets arriving on an inter-worker queue will match this mask.
pub const PACKET_PROCESSOR_SOURCE_INTER_WORKER: PacketProcessorSource =
    PacketProcessorSource(1 << 1);

struct Node {
    source_filter: PacketProcessorSource,
    next: Option<Box<Node>>,
    body: NodeBody,
}

enum NodeBody {
    Inline {
        handler: Box<UnsafeCell<dyn FnMut(EndpointAddr, &Packet)>>,
    },
    LocalQueue {
        queue: localq::mpsc::Sender<Packet>,
    },
    RouteToLocalQueue {
        handler: Box<
            UnsafeCell<dyn FnMut(EndpointAddr, &Packet) -> Option<localq::mpsc::Sender<Packet>>>,
        >,
    },
    ToWorker {
        to_worker_id: WorkerId,
    },
    RouteToWorker {
        handler: Box<UnsafeCell<dyn FnMut(EndpointAddr, &Packet) -> Option<WorkerId>>>,
    },
}

struct NodeChain {
    head: Node,
    tracer: TraceSource,
    is_in_handler: Cell<bool>,
    plane_next_topic: Option<u32>,
}

enum Mutation {
    AddNode {
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        tracer: Option<TraceSource>,
        body: NodeBody,
    },
    RemovePlane {
        plane_id: u32,
    },
}

pub struct PacketProcessor {
    handlers: localq::RwLock<HashMap<(u32, u32), NodeChain>>,
    existing_topics: RefCell<HashSet<(u32, u32)>>,
    mutations: UnsafeCell<Vec<Mutation>>,
    // plane ID -> most recently added topic
    // used to track the head node in a per-plane stack of NodeChain. To remove all handlers for a
    // plane we walk the stack.
    plane_topic_chain: RefCell<HashMap<u32, u32>>,
    local_worker_queue: localq::mpsc::Sender<Packet>,
    inter_worker_senders: Vec<Option<localq::mpsc::Sender<Packet>>>,
}

impl PacketProcessor {
    pub fn new(
        local_worker_queue: localq::mpsc::Sender<Packet>,
        inter_worker_senders: Vec<Option<localq::mpsc::Sender<Packet>>>,
    ) -> Self {
        PacketProcessor {
            handlers: localq::RwLock::new(HashMap::new()),
            existing_topics: RefCell::new(HashSet::new()),
            mutations: UnsafeCell::new(Vec::new()),
            plane_topic_chain: RefCell::new(HashMap::new()),
            local_worker_queue,
            inter_worker_senders,
        }
    }

    pub fn add_handler(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        handler: impl FnMut(EndpointAddr, &Packet) + 'static,
    ) {
        self.add_node(
            topic_name,
            source_filter,
            plane_id,
            topic,
            NodeBody::Inline {
                handler: Box::new(UnsafeCell::new(handler)),
            },
        );
    }

    pub fn add_local_queue(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        queue: localq::mpsc::Sender<Packet>,
    ) {
        self.add_node(
            topic_name,
            source_filter,
            plane_id,
            topic,
            NodeBody::LocalQueue { queue },
        );
    }

    pub fn add_route_to_local_queue(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        handler: impl FnMut(EndpointAddr, &Packet) -> Option<localq::mpsc::Sender<Packet>> + 'static,
    ) {
        self.add_node(
            topic_name,
            source_filter,
            plane_id,
            topic,
            NodeBody::RouteToLocalQueue {
                handler: Box::new(UnsafeCell::new(handler)),
            },
        );
    }

    pub fn to_worker(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        to_worker_id: WorkerId,
    ) {
        self.add_node(
            topic_name,
            source_filter,
            plane_id,
            topic,
            NodeBody::ToWorker { to_worker_id },
        );
    }

    pub fn add_route_to_worker(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        handler: impl FnMut(EndpointAddr, &Packet) -> Option<WorkerId> + 'static,
    ) {
        self.add_node(
            topic_name,
            source_filter,
            plane_id,
            topic,
            NodeBody::RouteToWorker {
                handler: Box::new(UnsafeCell::new(handler)),
            },
        );
    }

    pub async fn handle_packet(&self, source: PacketProcessorSource, packet: &Packet) {
        let header = TransmitPacket::decode(&mproto::DecodeCursor::new(packet))
            .expect("decode packet header");

        probius::trace_branch_start();

        let handlers = self.handlers.read().await;
        let Some(node_chain) = handlers.get(&(header.plane_id, header.topic)) else {
            probius::trace_metric("no_handlers", 1);
            return;
        };

        if node_chain.is_in_handler.get() {
            // If a handler for a topic can generate a new packet for itself, we can't allow the
            // handler to be called recursively - doing so would create concurrent `&mut FnMut()`
            // aliases which would be UB.
            probius::trace_metric("recursive_handler", 1);
            let _ = self.local_worker_queue.send(packet.clone()).await;
            return;
        }

        probius::trace_branch_end();

        node_chain
            .tracer
            .trace_future(async {
                probius::trace_metric("packet_size", packet.len() as i64);

                let mut next_node = Some(&node_chain.head);
                let mut sent_to_workers = 0;

                while let Some(node) = next_node {
                    next_node = node.next.as_deref();
                    if source.0 & node.source_filter.0 == 0 {
                        continue;
                    }

                    match &node.body {
                        NodeBody::Inline { handler } => {
                            node_chain.is_in_handler.replace(true);

                            // SAFETY: the `is_in_handler` check guards against recursively calling a
                            // single `FnMut()` topic handler.
                            let handler = unsafe { &mut *handler.get() };
                            handler(header.source, packet);

                            node_chain.is_in_handler.replace(false);
                        }
                        NodeBody::LocalQueue { queue } => {
                            probius::trace_label("local_queue");
                            let _ = queue.send(packet.clone()).await;
                        }
                        NodeBody::RouteToLocalQueue { handler } => {
                            node_chain.is_in_handler.replace(true);

                            // SAFETY: the `is_in_handler` check guards against recursively calling a
                            // single `FnMut()` topic handler.
                            let handler = unsafe { &mut *handler.get() };
                            let maybe_queue = handler(header.source, packet);

                            node_chain.is_in_handler.replace(false);

                            if let Some(queue) = maybe_queue {
                                let _ = queue.send(packet.clone()).await;
                            }
                        }
                        NodeBody::ToWorker { to_worker_id } => {
                            probius::trace_label("to_worker");

                            let mask = 1 << to_worker_id.0;
                            if sent_to_workers & mask == 0 {
                                sent_to_workers |= mask;
                                if let Some(Some(queue)) =
                                    self.inter_worker_senders.get(to_worker_id.0 as usize)
                                {
                                    let _ = queue.send(packet.clone()).await;
                                }
                            }
                        }
                        NodeBody::RouteToWorker { handler } => {
                            node_chain.is_in_handler.replace(true);

                            // SAFETY: the `is_in_handler` check guards against recursively calling a
                            // single `FnMut()` topic handler.
                            let handler = unsafe { &mut *handler.get() };
                            let maybe_to_worker_id = handler(header.source, packet);

                            node_chain.is_in_handler.replace(false);

                            if let Some(to_worker_id) = maybe_to_worker_id {
                                let mask = 1 << to_worker_id.0;
                                if sent_to_workers & mask == 0 {
                                    sent_to_workers |= mask;
                                    if let Some(Some(queue)) =
                                        self.inter_worker_senders.get(to_worker_id.0 as usize)
                                    {
                                        let _ = queue.send(packet.clone()).await;
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .await;
        drop(handlers);

        // SAFETY: `&mut self.mutations` never held across .await point.
        let mutations = unsafe { &mut *self.mutations.get() };
        if mutations.len() > 0 {
            let mut handlers = self.handlers.write().await;
            for mutation in mutations.drain(..) {
                match mutation {
                    Mutation::AddNode {
                        source_filter,
                        plane_id,
                        topic,
                        tracer,
                        body,
                    } => {
                        self._add_node(&mut handlers, source_filter, plane_id, topic, tracer, body);
                    }
                    Mutation::RemovePlane { plane_id } => {
                        self._remove_plane(&mut handlers, plane_id);
                    }
                }
            }
        }
    }

    pub async fn flush_traces(&self) {
        let handlers = self.handlers.read().await;
        for node_chain in handlers.values() {
            node_chain.tracer.flush_aggregate_full();
        }
    }

    pub fn remove_plane(&self, plane_id: u32) {
        if let Some(mut handlers) = self.handlers.try_write() {
            self._remove_plane(&mut handlers, plane_id);
        } else {
            // SAFETY: `&mut self.mutations` never held across .await point.
            let mutations = unsafe { &mut *self.mutations.get() };
            mutations.push(Mutation::RemovePlane { plane_id });
        }
    }

    fn _remove_plane(&self, handlers: &mut HashMap<(u32, u32), NodeChain>, plane_id: u32) {
        let mut existing_topics = self.existing_topics.borrow_mut();
        let mut plane_topic_chain = self.plane_topic_chain.borrow_mut();
        let mut plane_topic_next = plane_topic_chain.remove(&plane_id);

        while let Some(remove_topic) = plane_topic_next {
            existing_topics.remove(&(plane_id, remove_topic));
            let node_chain = handlers
                .remove(&(plane_id, remove_topic))
                .expect("PacketProcessor::remove_plane missing topic");
            plane_topic_next = node_chain.plane_next_topic;
        }
    }

    fn add_node(
        &self,
        topic_name: &str,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        body: NodeBody,
    ) {
        let tracer = if self.existing_topics.borrow_mut().insert((plane_id, topic)) {
            // This will be the first handler for this topic.
            Some(probius::new_trace_source(topic_name))
        } else {
            None
        };

        if let Some(mut handlers) = self.handlers.try_write() {
            self._add_node(&mut handlers, source_filter, plane_id, topic, tracer, body);
        } else {
            // SAFETY: `&mut self.mutations` never held across .await point.
            let mutations = unsafe { &mut *self.mutations.get() };
            mutations.push(Mutation::AddNode {
                source_filter,
                plane_id,
                topic,
                tracer,
                body,
            });
        }
    }

    fn _add_node(
        &self,
        handlers: &mut HashMap<(u32, u32), NodeChain>,
        source_filter: PacketProcessorSource,
        plane_id: u32,
        topic: u32,
        tracer: Option<TraceSource>,
        body: NodeBody,
    ) {
        use std::collections::hash_map::Entry;

        // Note the plane topic chains are stacks, not queues

        match handlers.entry((plane_id, topic)) {
            Entry::Occupied(entry) => {
                debug_assert!(tracer.is_none());

                let chain = entry.into_mut();
                let prev_head = core::mem::replace(
                    &mut chain.head,
                    Node {
                        source_filter,
                        next: None,
                        body,
                    },
                );
                chain.head.next = Some(Box::new(prev_head));
            }
            Entry::Vacant(entry) => {
                let tracer = tracer.expect("PacketProcessor::_add_node missing tracer");

                // Link this topic in the plane's topic chain.
                let mut plane_topic_chain = self.plane_topic_chain.borrow_mut();
                let prev_plane_topic_head = plane_topic_chain.insert(plane_id, topic);

                // Create the topic's NodeChain
                entry.insert(NodeChain {
                    head: Node {
                        source_filter,
                        next: None,
                        body,
                    },
                    tracer,
                    is_in_handler: Cell::new(false),
                    plane_next_topic: prev_plane_topic_head,
                });
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_packet_processor() {
        let (local_tx, _) = localq::mpsc::channel(1);
        let pp = PacketProcessor::new(local_tx, vec![]);
        let buffer_pool = bab::HeapBufferPool::new(64, 2, 2);
        let buffer = buffer_pool.try_acquire().unwrap();

        pp.add_handler(
            "test-topic",
            PACKET_PROCESSOR_SOURCE_NEW,
            0,
            0,
            move |_source, _packet| {},
        );

        unsafe {
            buffer.initialize_rc(1, 0, 0);
        }
        let packet = unsafe { Packet::new(buffer, 0, 64) };
        pollster::block_on(pp.handle_packet(PACKET_PROCESSOR_SOURCE_NEW, &packet));
    }
}

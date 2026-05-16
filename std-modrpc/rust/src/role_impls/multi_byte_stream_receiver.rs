use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use crate::{
    proto::MultiStreamId,
    role_impls::multi_byte_stream_sender::{
        EOF_FLAG,
        INDEX_MASK,
        MULTI_BYTE_STREAM_HEADER_LEN,
    },
    MultiByteStreamInitState,
    MultiByteStreamReceiverConfig,
    MultiByteStreamReceiverHooks,
    MultiByteStreamReceiverStubs,
};

struct StreamState {
    // Blobs by their start index
    blobs: RefCell<HashMap<u64, modrpc::Packet>>,

    current_blob_start: Cell<u64>,
    consume_cursor: Cell<u64>,

    // Set once the EOF sentinel is observed.
    eof: Cell<bool>,
    // Byte index at which the stream terminates - i.e. the highest `start_index + len` seen prior
    // to the EOF packet. We progress this incrementally as blobs arrive.
    eof_at: Cell<u64>,

    // A bit clunky, but rather than precisely track and wake waiting tasks by the byte range they
    // are waiting for, just wake every waiting task whenever new bytes come in.
    waiters: localq::WaiterQueue,
}

impl StreamState {
    fn new() -> Self {
        Self {
            blobs: RefCell::new(HashMap::new()),
            current_blob_start: Cell::new(0),
            consume_cursor: Cell::new(0),
            eof: Cell::new(false),
            eof_at: Cell::new(0),
            waiters: localq::WaiterQueue::new(),
        }
    }
}

struct BrokerState {
    streams: RefCell<HashMap<MultiStreamId, Rc<StreamState>>>,
}

#[derive(Clone)]
pub struct MultiByteStreamReceiver {
    broker_state: Rc<BrokerState>,
}

pub struct MultiByteStreamReceiverBuilder {
    name: &'static str,
    stubs: MultiByteStreamReceiverStubs,
    broker_state: Rc<BrokerState>,
}

impl MultiByteStreamReceiverBuilder {
    pub fn new(
        name: &'static str,
        _hooks: MultiByteStreamReceiverHooks,
        stubs: MultiByteStreamReceiverStubs,
        _config: &MultiByteStreamReceiverConfig,
        _init: MultiByteStreamInitState,
    ) -> Self {
        Self {
            name,
            stubs,
            broker_state: Rc::new(BrokerState {
                streams: RefCell::new(HashMap::new()),
            }),
        }
    }

    pub fn create_handle(
        &self,
        _setup: &modrpc::RoleSetup,
    ) -> MultiByteStreamReceiver {
        MultiByteStreamReceiver {
            broker_state: self.broker_state.clone(),
        }
    }

    pub fn build(
        self,
        setup: &modrpc::RoleSetup,
    ) {
        use mproto::BaseLen;

        let name = self.name;
        let broker_state = self.broker_state;
        self.stubs.blob
            .inline_untyped(setup, move |_source, packet| {
                if packet.len() < modrpc::TransmitPacket::BASE_LEN + MULTI_BYTE_STREAM_HEADER_LEN {
                    return;
                }
                packet.advance(modrpc::TransmitPacket::BASE_LEN);

                // Read MultiStreamId (12B) + start_index (8B)
                let owner_bytes: [u8; 8] = packet[0..8].try_into().unwrap();
                let id_bytes: [u8; 4] = packet[8..12].try_into().unwrap();
                let start_index_bytes: [u8; 8] = packet[12..20].try_into().unwrap();

                let stream_id = MultiStreamId {
                    owner: u64::from_le_bytes(owner_bytes),
                    id: u32::from_le_bytes(id_bytes),
                };
                let raw_start_index = u64::from_le_bytes(start_index_bytes);
                let is_eof = raw_start_index & EOF_FLAG != 0;
                let start_index = raw_start_index & INDEX_MASK;

                // Strip the header
                packet.advance(MULTI_BYTE_STREAM_HEADER_LEN);

                let Some(stream_state) =
                    broker_state.streams.borrow().get(&stream_id).cloned()
                else {
                    log::warn!(
                        "MultiByteStreamReceiver unknown stream name={name} \
                         stream_id={stream_id:?}"
                    );
                    return;
                };

                if is_eof {
                    // End-of-stream marker. `start_index` is the authoritative final byte
                    // length of the stream.
                    stream_state.eof_at.set(start_index);
                    stream_state.eof.set(true);
                    stream_state.waiters.notify(usize::MAX);
                    return;
                }

                let mut blobs = stream_state.blobs.borrow_mut();
                blobs.entry(start_index).or_insert(packet.clone());

                stream_state.waiters.notify(usize::MAX);
            })
            .subscribe();
    }
}

pub struct ReceiveMultiByteStream {
    stream_id: MultiStreamId,
    state: Rc<StreamState>,
    broker_state: Rc<BrokerState>,
}

impl MultiByteStreamReceiver {
    pub fn new_stream(&self, stream_id: MultiStreamId) -> ReceiveMultiByteStream {
        let state = Rc::new(StreamState::new());
        self.broker_state.streams.borrow_mut()
            .insert(stream_id, state.clone());

        ReceiveMultiByteStream {
            stream_id,
            state,
            broker_state: self.broker_state.clone(),
        }
    }
}

impl Drop for ReceiveMultiByteStream {
    fn drop(&mut self) {
        self.broker_state.streams.borrow_mut().remove(&self.stream_id);
    }
}

impl ReceiveMultiByteStream {
    pub fn stream_id(&self) -> MultiStreamId {
        self.stream_id
    }

    pub fn cursor(&self) -> u64 {
        self.state.consume_cursor.get()
    }

    /// Returns true once the sender has signaled end-of-stream and the consume cursor has caught up
    /// to the final byte index.
    pub fn is_done(&self) -> bool {
        self.state.eof.get() && self.state.consume_cursor.get() >= self.state.eof_at.get()
    }

    pub fn peek(&self) -> Option<modrpc::Packet> {
        let start = self.state.current_blob_start.get();
        let cursor = self.state.consume_cursor.get();
        let blobs = self.state.blobs.borrow();

        if start > cursor {
            return None;
        }

        let blob = blobs.get(&start)?.clone();
        blob.advance((cursor - start) as usize);
        Some(blob)
    }

    pub fn consume(&self, count: u64) -> Option<modrpc::Packet> {
        use std::collections::hash_map::Entry;

        let start = self.state.current_blob_start.get();
        let cursor = self.state.consume_cursor.get();
        let offset_in_blob = cursor - start;
        let mut blobs = self.state.blobs.borrow_mut();

        if start > cursor {
            return None;
        }

        let Entry::Occupied(blob_entry) = blobs.entry(start) else {
            return None;
        };

        let blob =
            if count >= blob_entry.get().len() as u64 - offset_in_blob {
                let blob = blob_entry.remove();
                self.state.current_blob_start.set(start + blob.len() as u64);
                blob
            } else {
                blob_entry.get().clone()
            };
        blob.advance(offset_in_blob as usize);
        blob.set_len(std::cmp::min(blob.len(), count as usize));

        self.state.consume_cursor.set(cursor + blob.len() as u64);

        Some(blob)
    }

    pub fn try_peek_ahead(&self, read_start: u64, read_len: u64) -> Option<modrpc::Packet> {
        let start = self.state.current_blob_start.get();
        let consume_cursor = self.state.consume_cursor.get();

        if consume_cursor > read_start {
            return None;
        }
        if start > read_start {
            return None;
        }

        let mut cursor = start;
        let blobs = self.state.blobs.borrow();
        loop {
            let Some(blob) = blobs.get(&cursor) else {
                return None;
            };

            if cursor + blob.len() as u64 > read_start
                || cursor + blob.len() as u64 == read_start && read_len == 0
            {
                let blob = blob.clone();
                blob.advance((read_start - cursor) as usize);
                blob.set_len(std::cmp::min(blob.len(), read_len as usize));
                return Some(blob);
            }

            cursor += blob.len() as u64;
        }
    }

    pub async fn peek_ahead(&self, read_start: u64, read_len: u64) -> modrpc::Packet {
        self.state.waiters.wait_for(|| self.try_peek_ahead(read_start, read_len)).await
    }

    /// Wait until the sender has signaled end-of-stream and the consumer has drained through
    /// the final byte.
    pub async fn wait_done(&self) {
        self.state.waiters.wait_for(|| if self.is_done() { Some(()) } else { None }).await
    }
}

#[cfg(test)]
mod test {
    use modrpc_executor::ModrpcExecutor;
    use crate::{
        MultiByteStreamInitState,
        MultiByteStreamReceiverConfig,
        MultiByteStreamReceiverRole,
        MultiByteStreamSenderBuilder,
        MultiByteStreamSenderConfig,
        MultiByteStreamSenderRole,
    };
    use super::*;

    fn read_all(rx: &ReceiveMultiByteStream) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(blob) = rx.consume(u64::MAX) {
            out.extend_from_slice(&blob);
        }
        out
    }

    #[test]
    fn test_multi_byte_stream_receiver() {
        let mut ex = modrpc_executor::FuturesExecutor::new();
        let (rt, _rt_shutdown) = modrpc::RuntimeHandle::single_threaded(&mut ex);

        ex.run_until(async move {
            let transport = rt.add_transport(modrpc::LocalTransport {
                buffer_size: 256,
                buffer_pool_batches: 16,
                buffer_pool_batch_size: 16,
            })
            .await;

            let mut mb_sender = None;
            let _ =
                rt.start_role::<MultiByteStreamSenderRole>(modrpc::RoleConfig {
                    plane_id: 0,
                    endpoint_addr: modrpc::EndpointAddr { endpoint: 0 },
                    transport: transport.clone(),
                    topic_channels: modrpc::TopicChannels::SingleChannel { channel_id: 0 },
                    config: MultiByteStreamSenderConfig { },
                    init: MultiByteStreamInitState { },
                })
                .local(|cx| {
                    let builder = MultiByteStreamSenderBuilder::new(
                        "mb_sender", cx.hooks.clone(), cx.stubs, cx.config, cx.init.clone(),
                    );
                    mb_sender = Some(builder.create_handle(cx.setup));
                    builder.build(cx.setup);
                });

            let mut mb_receiver = None;
            let _ =
                rt.start_role::<MultiByteStreamReceiverRole>(modrpc::RoleConfig {
                    plane_id: 0,
                    endpoint_addr: modrpc::EndpointAddr { endpoint: 0 },
                    transport,
                    topic_channels: modrpc::TopicChannels::SingleChannel { channel_id: 0 },
                    config: MultiByteStreamReceiverConfig { },
                    init: MultiByteStreamInitState { },
                })
                .local(|cx| {
                    let builder = MultiByteStreamReceiverBuilder::new(
                        "mb_receiver", cx.hooks.clone(), cx.stubs, cx.config, cx.init.clone(),
                    );
                    mb_receiver = Some(builder.create_handle(cx.setup));
                    builder.build(cx.setup);
                });

            let mb_sender = mb_sender.unwrap();
            let mb_receiver = mb_receiver.unwrap();

            let id_a = MultiStreamId { owner: 1, id: 10 };
            let id_b = MultiStreamId { owner: 1, id: 11 };
            // Same stream ID from a different owner
            let id_c = MultiStreamId { owner: 2, id: 10 };

            let send_a = mb_sender.new_stream(id_a);
            let send_b = mb_sender.new_stream(id_b);
            let send_c = mb_sender.new_stream(id_c);

            let recv_a = mb_receiver.new_stream(id_a);
            let recv_b = mb_receiver.new_stream(id_b);
            let recv_c = mb_receiver.new_stream(id_c);

            // Nothing sent yet.
            assert!(recv_a.peek().is_none());
            assert_eq!(recv_a.cursor(), 0);
            assert!(!recv_a.is_done());

            // Send some bytes interleaved across streams.
            assert_eq!(send_a.send(b"hello").await, 0);
            assert_eq!(send_b.send(b"world").await, 0);
            assert_eq!(send_a.send(b" there").await, 5);
            assert_eq!(send_c.send(b"!!").await, 0);

            // Read across the two blobs on stream A.
            let blob = recv_a.peek_ahead(0, 11).await;
            assert_eq!(&blob[..], b"hello");
            // The first blob ends at byte 5; ask for what's after it.
            let blob = recv_a.peek_ahead(5, 6).await;
            assert_eq!(&blob[..], b" there");

            // Drain stream A and confirm the byte content.
            assert_eq!(read_all(&recv_a), b"hello there");
            assert_eq!(recv_a.cursor(), 11);
            assert!(recv_a.peek().is_none());

            // Verify stream B and C didn't get crosstalk.
            assert_eq!(read_all(&recv_b), b"world");
            assert_eq!(read_all(&recv_c), b"!!");
            assert_eq!(recv_b.cursor(), 5);
            assert_eq!(recv_c.cursor(), 2);

            // EOF after additional data - trailing bytes must be drained before is_done flips.
            send_a.send(b"bye").await;
            send_a.end().await;

            // Spin the executor enough to deliver the trailing packets.
            let blob = recv_a.peek_ahead(11, 3).await;
            assert_eq!(&blob[..], b"bye");

            // EOF has been observed by the broker, but the consumer is not done until it
            // catches up to the final byte.
            assert!(!recv_a.is_done());
            assert_eq!(read_all(&recv_a), b"bye");
            assert_eq!(recv_a.cursor(), 14);
            assert!(recv_a.is_done());
            recv_a.wait_done().await; // should immediately return

            // End stream B without further data - is_done should flip as soon as EOF arrives.
            send_b.end().await;
            recv_b.wait_done().await;
            assert!(recv_b.is_done());
            assert_eq!(recv_b.cursor(), 5);

            // Dropping a ReceiveMultiByteStream removes it from the broker.
            let broker = mb_receiver.broker_state.clone();
            assert_eq!(broker.streams.borrow().len(), 3);
            drop(recv_a);
            assert_eq!(broker.streams.borrow().len(), 2);
            drop(recv_b);
            drop(recv_c);
            assert_eq!(broker.streams.borrow().len(), 0);
        });
    }
}

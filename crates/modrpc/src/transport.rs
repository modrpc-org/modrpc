use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::sync::Arc;

use bab::Packet;

use crate::{
    PacketBundle, RuntimeHandle, TransmitPacket, WorkerContext, flush_batcher::FlushBatcher,
};

pub struct TransportContext<'a> {
    pub rt: &'a RuntimeHandle,
}

pub trait TransportBuilder {
    #[allow(async_fn_in_trait)]
    async fn start_transport(self, cx: TransportContext<'_>) -> TransportHandle;
}

pub struct LocalTransport {
    pub buffer_size: usize,
    pub buffer_pool_batches: usize,
    pub buffer_pool_batch_size: usize,
}

#[derive(Clone)]
pub struct TransportHandle {
    pub buffer_pool: bab::HeapBufferPool,
    pub writer_config: WriterConfig,
    pub shutdown_signal: bab::SignalTree,
}

#[derive(Clone)]
pub enum WriterConfig {
    Shared {
        writer_flush_sender: bab::WriterFlushSender,
        channels: Arc<spin::Mutex<HashMap<u32, bab::SharedWriter>>>,
    },
    LocalFlush {
        writer_flush_sender: bab::WriterFlushSender,
    },
    LocalNoFlush,
}

impl TransportBuilder for LocalTransport {
    async fn start_transport(self, _: TransportContext<'_>) -> TransportHandle {
        let shutdown_signal = bab::SignalTree::new();
        let buffer_pool = bab::HeapBufferPool::new(
            self.buffer_size,
            self.buffer_pool_batches,
            self.buffer_pool_batch_size,
        );

        TransportHandle {
            buffer_pool,
            writer_config: WriterConfig::LocalNoFlush,
            shutdown_signal,
        }
    }
}

impl TransportHandle {
    pub(crate) fn writer_flush_sender(&self) -> Option<bab::WriterFlushSender> {
        match &self.writer_config {
            WriterConfig::Shared {
                writer_flush_sender,
                ..
            } => Some(writer_flush_sender.clone()),
            WriterConfig::LocalFlush {
                writer_flush_sender,
            } => Some(writer_flush_sender.clone()),
            WriterConfig::LocalNoFlush => None,
        }
    }

    pub(crate) fn get_local_flush_batcher(
        worker_cx: &WorkerContext,
        writer_flush_sender: &bab::WriterFlushSender,
    ) -> FlushBatcher {
        // TODO how do we clean these up when the transport shuts down?
        worker_cx.with_local_fn(
            writer_flush_sender.id(),
            || {
                // TODO configurable
                FlushBatcher::new(std::time::Duration::from_micros(100))
            },
            |flush_batcher| flush_batcher.clone(),
        )
    }

    pub(crate) fn new_writer(
        &self,
        worker_cx: &WorkerContext,
        channel_id: u32,
    ) -> (bab::DynWriter, Option<FlushBatcher>) {
        // To allow all endpoints to use the same buffer size for both ingress and egress if
        // desired, ensure that buffers flushed by writers are never larger than
        // `buffer_size - PacketBundle::BASE_LEN` since the PacketBundle header will be a part of
        // the buffer at ingress but not at egress.
        let buffer_tailroom = <PacketBundle as mproto::BaseLen>::BASE_LEN;

        match &self.writer_config {
            WriterConfig::Shared {
                writer_flush_sender,
                channels,
            } => {
                let mut channels = channels.lock();
                let writer = channels.entry(channel_id).or_insert_with(|| {
                    bab::Writer::new_shared(
                        self.buffer_pool.clone(),
                        buffer_tailroom,
                        writer_flush_sender.clone(),
                        channel_id as usize,
                    )
                });
                // bab::SharedWriter::flush_local is a no-op, so the transport-level flusher can be
                // used directly.
                let transport_flush_batcher =
                    Self::get_local_flush_batcher(worker_cx, &writer_flush_sender);
                (writer.clone().to_dyn(), Some(transport_flush_batcher))
            }
            WriterConfig::LocalFlush {
                writer_flush_sender,
            } => {
                let transport_flush_batcher =
                    Self::get_local_flush_batcher(worker_cx, &writer_flush_sender);

                // TODO how do we clean these up when the transport shuts down?
                // Convert this to per-Transport ThreadLocal<HashMap<channel_id, bab::DynWriter>>?
                let (writer, flush_batcher) = worker_cx.with_local_fn(
                    channel_id,
                    || {
                        let writer = bab::Writer::new_local_flush(
                            self.buffer_pool.clone(),
                            buffer_tailroom,
                            writer_flush_sender.clone(),
                            channel_id as usize,
                        );
                        let flush_batcher = Self::spawn_writer_flush_batcher(
                            worker_cx,
                            writer.clone().to_dyn(),
                            transport_flush_batcher,
                        );

                        (writer, Some(flush_batcher))
                    },
                    |w| w.clone(),
                );
                (writer.to_dyn(), flush_batcher)
            }
            WriterConfig::LocalNoFlush => {
                let writer = bab::Writer::new_local_noflush(
                    self.buffer_pool.clone(),
                    buffer_tailroom,
                    channel_id as usize,
                )
                .to_dyn();
                (writer, None)
            }
        }
    }

    fn spawn_writer_flush_batcher(
        worker_cx: &WorkerContext,
        writer: bab::DynWriter,
        transport_flush_batcher: FlushBatcher,
    ) -> FlushBatcher {
        use crate::flush_batcher::FlushBatcherStatus;

        let flush_batcher = FlushBatcher::new(std::time::Duration::from_micros(100));
        let mut sleeper = worker_cx.new_sleeper();
        worker_cx.spawn({
            let flush_batcher = flush_batcher.clone();
            async move {
                loop {
                    // Wait until there is data to flush
                    flush_batcher.wait().await;

                    loop {
                        match flush_batcher.handle_flush() {
                            FlushBatcherStatus::Snooze { duration } => {
                                sleeper.as_mut().snooze(duration);
                                core::future::poll_fn(|cx| sleeper.as_mut().poll_sleep(cx)).await;
                            }
                            FlushBatcherStatus::FlushNow => {
                                writer.flush_local();
                                transport_flush_batcher.schedule_flush();
                                break;
                            }
                            FlushBatcherStatus::DoNotFlush => {
                                break;
                            }
                        }
                    }
                }
            }
        });

        flush_batcher
    }
}

pub fn shatter_packet_bundle(
    packet: Packet,
    offsets: &mut Vec<usize>,
    out_packets: &mut Vec<MaybeUninit<Packet>>,
) -> PacketBundle {
    use mproto::BaseLen;

    offsets.clear();
    out_packets.clear();

    let bundle_header: PacketBundle = mproto::decode_value(&packet[..])
        // TODO error handling
        .expect("decode rx bundle header");

    let mut cursor = PacketBundle::BASE_LEN;
    while cursor < PacketBundle::BASE_LEN + bundle_header.length as usize {
        let packet_header: TransmitPacket = mproto::decode_value(&packet[cursor..])
            // TODO error handling
            .expect("decode rx bundle header");
        offsets.push(cursor);
        cursor += TransmitPacket::BASE_LEN + packet_header.payload_length as usize;
    }

    out_packets.resize_with(offsets.len(), || MaybeUninit::uninit());
    packet.shatter_into(offsets, out_packets);

    bundle_header
}

pub struct ShatterPacketBundle<'a> {
    packet: &'a Packet,
    cursor: usize,
}

impl<'a> ShatterPacketBundle<'a> {
    pub fn new(packet: &'a Packet) -> Self {
        Self { packet, cursor: 0 }
    }
}

impl Iterator for ShatterPacketBundle<'_> {
    type Item = Packet;

    fn next(&mut self) -> Option<Self::Item> {
        use mproto::BaseLen;

        if self.cursor + TransmitPacket::BASE_LEN < self.packet.len() {
            let Ok(packet_header) =
                mproto::decode_value::<TransmitPacket>(&self.packet[self.cursor..])
            else {
                return None;
            };
            let packet = self.packet.clone();
            packet.advance(self.cursor);
            self.cursor += TransmitPacket::BASE_LEN + packet_header.payload_length as usize;

            Some(packet)
        } else {
            None
        }
    }
}

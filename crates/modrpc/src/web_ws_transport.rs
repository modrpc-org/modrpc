use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket};

use futures_lite::future;
use mproto::BaseLen;

use crate::{
    HeapBufferPool, Packet, PacketBundle, TransportBuilder, TransportContext, TransportHandle,
    transport::WriterConfig, web_ws_ingress::WebSocketIngress,
};

pub struct WebSocketTransport {
    pub buffer_pool: HeapBufferPool,
    pub websocket: WebSocket,
}

impl TransportBuilder for WebSocketTransport {
    async fn start_transport(self, cx: TransportContext<'_>) -> TransportHandle {
        let shutdown_signal = bab::SignalTree::new();

        let Some(worker_cx) = cx.rt.local_worker_context() else {
            panic!("gloo_net WebSocket modrpc transport requires a local worker.");
        };

        let (mut ws_tx, ws_rx) = self.websocket.split();

        let (writer_flush_sender, mut writer_flush_receiver) = bab::new_writer_flusher();

        // Spawn task to flush egress packets
        worker_cx.spawn({
            let buffer_pool = self.buffer_pool.clone();
            let shutdown_signal = shutdown_signal.clone();
            let shutdown_notifier = shutdown_signal.clone();
            future::or(
                async move {
                    'flush_loop: loop {
                        for flush in writer_flush_receiver.flush().await {
                            if flush.len() > 0 {
                                let mut buf = vec![0u8; PacketBundle::BASE_LEN + flush.len()];

                                // Fill bundle header
                                mproto::encode_value(
                                    PacketBundle {
                                        channel_id: flush.writer_id() as u32,
                                        length: flush.len() as u16,
                                    },
                                    &mut buf[..PacketBundle::BASE_LEN],
                                );
                                // Copy payload
                                buf[PacketBundle::BASE_LEN..].copy_from_slice(&flush);

                                // Write to socket
                                if let Err(_) = ws_tx.send(Message::Bytes(buf)).await {
                                    break 'flush_loop;
                                }
                            }
                        }
                    }
                    shutdown_notifier.notify();
                },
                async move {
                    let _buffer_pool_thread_guard = buffer_pool.register_thread();
                    shutdown_signal.wait().await;
                    //stream.shutdown(std::net::Shutdown::Both).unwrap();
                },
            )
        });

        // Spawn task to receive ingress packets
        let mut ws_ingress = WebSocketIngress::new(
            ws_rx,
            self.buffer_pool.clone(),
            self.buffer_pool.buffer_size(),
        );
        worker_cx.spawn({
            let shutdown_notifier = shutdown_signal.clone();
            let shutdown_waiter = shutdown_signal.clone();
            let process_packet_fn = worker_cx.get_packet_processor();
            future::or(
                async move {
                    use core::mem::MaybeUninit;

                    let mut shatter_offsets: Vec<usize> = Vec::new();
                    let mut shatter_out_packets: Vec<MaybeUninit<Packet>> = Vec::new();

                    while let Ok(packet_bundle) = ws_ingress.receive().await {
                        let _header = crate::shatter_packet_bundle(
                            packet_bundle,
                            &mut shatter_offsets,
                            &mut shatter_out_packets,
                        );

                        for packet in shatter_out_packets.drain(..) {
                            let packet = unsafe { packet.assume_init() };
                            process_packet_fn(&packet).await;
                        }
                    }

                    shutdown_notifier.notify();
                },
                shutdown_waiter.wait_owned(),
            )
        });

        TransportHandle {
            shutdown_signal,
            buffer_pool: self.buffer_pool,
            writer_config: WriterConfig::LocalFlush {
                writer_flush_sender,
            },
        }
    }
}

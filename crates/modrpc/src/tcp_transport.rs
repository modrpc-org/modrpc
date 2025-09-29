use futures_lite::future;
use mproto::BaseLen;
use tokio::io::AsyncWriteExt;

use crate::{
    HeapBufferPool, Packet, PacketBundle, TcpIngress, TransportBuilder, TransportContext,
    TransportHandle, WorkerId, transport::WriterConfig,
};

pub struct TcpTransport {
    pub buffer_pool: HeapBufferPool,
    pub worker_id: WorkerId,
    pub stream: tokio::net::TcpStream,
}

impl TransportBuilder for TcpTransport {
    async fn start_transport(self, cx: TransportContext<'_>) -> TransportHandle {
        let shutdown_signal = bab::SignalTree::new();

        let (tcp_read, mut tcp_write) = self.stream.into_split();

        let writer_flush_sender = cx
            .rt
            .get_worker(self.worker_id)
            .run_once({
                let shutdown_signal = shutdown_signal.clone();
                let buffer_pool = self.buffer_pool.clone();
                move |worker_cx| {
                    let mut bundle_header_buf = [0u8; PacketBundle::BASE_LEN];

                    let (writer_flush_sender, mut writer_flush_receiver) =
                        bab::new_writer_flusher();

                    // Spawn task to flush egress packets
                    worker_cx.spawn_traced("tcp-tx", core::time::Duration::from_millis(1000), {
                        let buffer_pool = buffer_pool.clone();
                        let shutdown_notifier = shutdown_signal.clone();
                        let shutdown_waiter = shutdown_signal.clone();
                        async move |tracer| {
                            future::or(
                                async move {
                                    let bundle_header_buf = bundle_header_buf.as_mut_slice();

                                    'flush_loop: loop {
                                        for flush in writer_flush_receiver.flush().await {
                                            let start = std::time::Instant::now();

                                            if flush.len() > 0 {
                                                if let Err(_) = tracer
                                                    .trace_future(async {
                                                        probius::trace_label("receive-buffer");
                                                        probius::trace_metric(
                                                            "buffer_size",
                                                            flush.len() as i64,
                                                        );

                                                        // Fill bundle header
                                                        mproto::encode_value(
                                                            PacketBundle {
                                                                channel_id: flush.writer_id()
                                                                    as u32,
                                                                length: flush.len() as u16,
                                                            },
                                                            &mut bundle_header_buf[..],
                                                        );

                                                        // Write to socket
                                                        tcp_write
                                                            .write_all(bundle_header_buf)
                                                            .await?;
                                                        tcp_write.write_all(&flush).await?;

                                                        probius::trace_metric(
                                                            "duration_us",
                                                            (std::time::Instant::now() - start)
                                                                .as_micros()
                                                                as i64,
                                                        );

                                                        Ok::<_, std::io::Error>(())
                                                    })
                                                    .await
                                                {
                                                    break 'flush_loop;
                                                }
                                            }
                                        }
                                    }
                                    shutdown_notifier.notify();
                                },
                                async move {
                                    let _buffer_pool_thread_guard = buffer_pool.register_thread();
                                    shutdown_waiter.wait().await;
                                },
                            )
                            .await
                        }
                    });

                    // Spawn task to receive ingress packets
                    let mut tcp_ingress =
                        TcpIngress::new(tcp_read, buffer_pool.clone(), buffer_pool.buffer_size());
                    worker_cx.spawn_traced("tcp-rx", core::time::Duration::from_millis(1000), {
                        let shutdown_notifier = shutdown_signal.clone();
                        let shutdown_waiter = shutdown_signal.clone();
                        let process_packet_fn = worker_cx.get_packet_processor();
                        let buffer_pool = buffer_pool.clone();
                        async move |tracer| {
                            future::or(
                                async move {
                                    use core::mem::MaybeUninit;

                                    let mut shatter_offsets: Vec<usize> = Vec::new();
                                    let mut shatter_out_packets: Vec<MaybeUninit<Packet>> =
                                        Vec::new();

                                    let mut last_rx_end = std::time::Instant::now();

                                    while let Ok(packet_bundle) = tcp_ingress.receive().await {
                                        let header = crate::shatter_packet_bundle(
                                            packet_bundle,
                                            &mut shatter_offsets,
                                            &mut shatter_out_packets,
                                        );
                                        assert!(
                                            header.length as usize <= buffer_pool.buffer_size()
                                        );

                                        tracer.trace(|| {
                                            probius::trace_label("tcp-rx");
                                            probius::trace_branch(|| {
                                                probius::trace_label("receive-bundle");
                                                probius::trace_metric(
                                                    "bytes",
                                                    header.length as i64,
                                                );
                                                probius::trace_metric(
                                                    "gap_us",
                                                    (std::time::Instant::now() - last_rx_end)
                                                        .as_micros()
                                                        as i64,
                                                );
                                            });
                                        });

                                        for packet in shatter_out_packets.drain(..) {
                                            let packet = unsafe { packet.assume_init() };
                                            tracer
                                                .trace_future(async {
                                                    probius::trace_label("tcp-rx");
                                                    probius::trace_branch_start();
                                                    probius::trace_label("receive-packet");
                                                    probius::trace_branch_start();
                                                    process_packet_fn(&packet).await;
                                                    probius::trace_branch_end();
                                                    probius::trace_branch_end();
                                                })
                                                .await;
                                        }

                                        last_rx_end = std::time::Instant::now();
                                    }

                                    shutdown_notifier.notify();
                                },
                                shutdown_waiter.wait_owned(),
                            )
                            .await
                        }
                    });

                    writer_flush_sender
                }
            })
            .await;

        TransportHandle {
            shutdown_signal,
            buffer_pool: self.buffer_pool,
            writer_config: WriterConfig::LocalFlush {
                writer_flush_sender,
            },
        }
    }
}

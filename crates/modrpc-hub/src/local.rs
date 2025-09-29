use modrpc::{
    WriterConfig,
    TransportBuilder,
    TransportContext,
    TransportHandle,
};

use crate::{
    broadcaster::InPacket,
    BroadcasterHandle,
    ChannelId,
    TransportIndex,
};

pub enum LocalHubTransport {
    Static {
        buffer_pool: bab::HeapBufferPool,
        broadcaster_handle: BroadcasterHandle,
        channel_ids: Vec<u32>,
    },
    Dynamic {
        buffer_pool: bab::HeapBufferPool,
        broadcaster_handle: BroadcasterHandle,
        broadcaster_transport: TransportIndex,
    },
}

impl TransportBuilder for LocalHubTransport {
    async fn start_transport(self, cx: TransportContext<'_>) -> TransportHandle {
        let Some(worker_cx) = cx.rt.local_worker_context() else {
            panic!("modrpc runtime must have a local worker to create a LocalHubTransport.");
        };

        let (buffer_pool, broadcaster_handle, broadcaster_transport) = match self {
            Self::Static { buffer_pool, broadcaster_handle, channel_ids } => {
                let broadcaster_transport =
                    broadcaster_handle.add_local(worker_cx.local_packet_tx().clone()).await;

                for channel_id in channel_ids {
                    broadcaster_handle.add_next_hop_to_channels(
                        broadcaster_transport,
                        vec![
                            (ChannelId { channel_id }, ChannelId { channel_id }),
                        ],
                    )
                    .await;
                }

                (buffer_pool, broadcaster_handle, broadcaster_transport)
            }
            Self::Dynamic { buffer_pool, broadcaster_handle, broadcaster_transport } => {
                (buffer_pool, broadcaster_handle, broadcaster_transport)
            }
        };

        let shutdown_signal = bab::SignalTree::new();
        let broadcaster_sender = broadcaster_handle.in_packet_sender().clone();

        let (writer_flush_sender, mut writer_flush_receiver) = bab::new_writer_flusher();
        let writer_config = WriterConfig::LocalFlush {
            writer_flush_sender: writer_flush_sender.clone(),
        };

        // Spawn task to flush egress packets to the broadcaster
        worker_cx.spawn_traced("local-hub-transport-flush", core::time::Duration::from_millis(1000), {
            let buffer_pool = buffer_pool.clone();
            let shutdown_signal = shutdown_signal.clone();
            let broadcaster_transport = broadcaster_transport;
            async move |tracer| {
                let _buffer_pool_thread_guard = buffer_pool.register_thread();
                futures_lite::future::or(
                    async {
                        loop {
                            for flush in writer_flush_receiver.flush().await {
                                let result = tracer.trace_future(async {
                                    probius::trace_metric("buffer_len", flush.len() as i64);
                                    let in_packet = InPacket {
                                        transport: broadcaster_transport,
                                        channel_id: flush.writer_id() as u32,
                                        packet: flush.into(),
                                    };
                                    probius::trace_branch_start();
                                    if let Err(e) = broadcaster_sender.send(in_packet).await {
                                        probius::trace_label("send_fail_abort");
                                        probius::trace_branch_end();
                                        return Err(e);
                                    }
                                    probius::trace_label("send_success");
                                    probius::trace_branch_end();

                                    Ok(())
                                })
                                .await;

                                if result.is_err() {
                                    break;
                                }
                            }
                        }
                    },
                    shutdown_signal.wait_owned(),
                )
                .await
            }
        });

        TransportHandle {
            buffer_pool,
            writer_config,
            shutdown_signal,
        }
    }
}

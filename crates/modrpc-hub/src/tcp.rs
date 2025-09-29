use modrpc::{
    PacketBundle,
    TcpIngress,
    WorkerContext,
};
use mproto::BaseLen;

use crate::{
    broadcaster::InPacket,
    BroadcasterHandle,
    TransportIndex,
};

pub async fn spawn_tcp_spoke(
    // TODO take a spawner instead of a full context
    worker_context: &WorkerContext,
    broadcaster_handle: BroadcasterHandle,
    buffer_pool: bab::HeapBufferPool,
    stream: tokio::net::TcpStream,
    max_packet_size: usize,
) -> (TransportIndex, bab::SignalTree) {
    let (tcp_read, tcp_write) = stream.into_split();

    let broadcaster_spoke = broadcaster_handle.add_tcp(tcp_write).await;
    let to_broadcaster = broadcaster_handle.in_packet_sender().clone();
    let shutdown_signal = bab::SignalTree::new();

    let mut ingress = TcpIngress::new(
        tcp_read,
        buffer_pool,
        max_packet_size,
    );

    worker_context.spawn(probius::enter_component_async(
        "tcp-ingress-task", {
            let shutdown_signal = shutdown_signal.clone();
            async move {
                let tracer = probius::new_trace_source("loop");

                while let Ok(packet_bundle) = ingress.receive().await {
                    let result: Result<(), localq::mpsc::SendError<_>> = tracer.trace_future(async {
                        let Ok(header) =
                            mproto::decode_value::<PacketBundle>(&packet_bundle[..])
                        else {
                            return Ok(());
                        };
                        packet_bundle.advance(PacketBundle::BASE_LEN);

                        tracer.trace(|| {
                            probius::trace_metric("bundle_payload_size", packet_bundle.len() as i64);
                        });

                        to_broadcaster.send(InPacket {
                            transport: broadcaster_spoke,
                            channel_id: header.channel_id,
                            packet: packet_bundle,
                        })
                        .await?;

                        Ok(())
                    })
                    .await;
                    if let Err(_) = result {
                        break;
                    }
                }

                shutdown_signal.notify();
                broadcaster_handle.remove_transport(broadcaster_spoke).await;
            }
        },
    ));

    (broadcaster_spoke, shutdown_signal)
}

use std::marker::Unpin;

use tokio::io::{AsyncRead, AsyncWrite};
use modrpc::{
    PacketBundle,
    WebSocketIngress,
    WorkerContext,
};
use mproto::BaseLen;

use crate::{
    broadcaster::InPacket,
    BroadcasterHandle,
    TransportIndex,
};

pub async fn spawn_websocket_spoke<S>(
    worker_context: &WorkerContext,
    broadcaster_handle: BroadcasterHandle,
    buffer_pool: bab::HeapBufferPool,
    websocket: tokio_tungstenite::WebSocketStream<S>,
    max_packet_size: usize,
) -> (TransportIndex, bab::SignalTree)
where S: AsyncRead + AsyncWrite + Unpin + Send + 'static
{
    use futures_util::StreamExt;

    let (ws_tx, ws_rx) = websocket.split();
    let broadcaster_spoke = broadcaster_handle.add_ws(Box::new(ws_tx)).await;
    let to_broadcaster = broadcaster_handle.in_packet_sender().clone();
    let shutdown_signal = bab::SignalTree::new();

    let mut ingress = WebSocketIngress::new(
        ws_rx,
        buffer_pool.clone(),
        max_packet_size,
    );

    worker_context.spawn_traced("hub-ws-ingress", core::time::Duration::from_millis(500), {
        let shutdown_signal = shutdown_signal.clone();
        async move |tracer| {
            while let Ok(packet_bundle) = ingress.receive().await {
                let result: Result<(), localq::mpsc::SendError<_>> = tracer.trace_future(async {
                    let Ok(header) =
                        mproto::decode_value::<PacketBundle>(&packet_bundle[..])
                    else {
                        log::info!("failed to decode websocket bundle");
                        return Ok(());
                    };
                    packet_bundle.advance(PacketBundle::BASE_LEN);

                    probius::trace_metric("bundle_payload_size", packet_bundle.len() as i64);

                    to_broadcaster.send(InPacket {
                        transport: broadcaster_spoke,
                        channel_id: header.channel_id,
                        packet: packet_bundle,
                    })
                    .await?;

                    probius::trace_label("send_success");

                    Ok(())
                })
                .await;
                if let Err(e) = result {
                    log::info!("websocket ingress error: {e:?}");
                    break;
                }
            }
            shutdown_signal.notify();
            broadcaster_handle.remove_transport(broadcaster_spoke).await;
        }
    });

    (broadcaster_spoke, shutdown_signal)
}


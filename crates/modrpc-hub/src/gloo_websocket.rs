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

pub async fn spawn_gloo_websocket_spoke(
    worker_context: &WorkerContext,
    broadcaster_handle: BroadcasterHandle,
    buffer_pool: bab::HeapBufferPool,
    websocket: gloo_net::websocket::futures::WebSocket,
    max_packet_size: usize,
) -> TransportIndex {
    use futures_util::StreamExt;

    let (ws_tx, ws_rx) = websocket.split();
    let broadcaster_spoke = broadcaster_handle.add_gloo_ws(Box::new(ws_tx)).await;
    let to_broadcaster = broadcaster_handle.in_packet_sender().clone();

    let mut ingress = WebSocketIngress::new(
        ws_rx,
        buffer_pool.clone(),
        max_packet_size,
    );

    worker_context.spawn(async move {
        while let Ok(packet_bundle) = ingress.receive().await {
            let Ok(header) =
                mproto::decode_value::<PacketBundle>(&packet_bundle[..])
            else {
                continue;
            };

            packet_bundle.advance(PacketBundle::BASE_LEN);

            if let Err(_) =
                to_broadcaster.send(InPacket {
                    transport: broadcaster_spoke,
                    channel_id: header.channel_id,
                    packet: packet_bundle,
                })
                .await
            {
                break;
            }
        }

        broadcaster_handle.remove_transport(broadcaster_spoke).await;
    });

    broadcaster_spoke
}

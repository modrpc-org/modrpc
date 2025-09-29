use gloo_net::websocket::{Message, futures::WebSocket};

use crate::{
    EndpointAddr, HeapBufferPool, InterfaceRole, RoleConfig, RuntimeHandle, TopicChannels,
    TransportHandle, WebSocketTransport, endpoint_proto::PlaneHandshake,
};

pub struct WebSocketConnection<Role: InterfaceRole> {
    pub endpoint: EndpointAddr,
    pub transport: TransportHandle,
    pub init: Role::Init,
    pub role_handle: Role::Hooks,
}

pub async fn web_ws_connect<Role: InterfaceRole>(
    rt: &RuntimeHandle,
    buffer_pool: HeapBufferPool,
    addr: &str,
    config: Role::Config,
) -> Result<WebSocketConnection<Role>, ()>
where
    Role::Config: Clone + Send,
    Role::Init: Clone + std::fmt::Debug + Send + Sync,
{
    use futures_util::StreamExt;

    let mut websocket = WebSocket::open(addr).map_err(|_| ())?;

    let payload = websocket
        .next()
        .await
        .map(|r| r.map_err(|_| ()))
        .unwrap_or(Err(()))?;

    let Message::Bytes(payload_bytes) = payload else {
        return Err(());
    };

    let plane_handshake: PlaneHandshake<Role::Init> =
        mproto::decode_value(&payload_bytes).map_err(|_| ())?;

    let transport = rt
        .add_transport(WebSocketTransport {
            websocket,
            buffer_pool,
        })
        .await;

    let transport_shutdown_signal = transport.shutdown_signal.clone();

    let role_handle = rt
        .start_role::<Role>(RoleConfig {
            plane_id: plane_handshake.plane_id,
            endpoint_addr: plane_handshake.endpoint_addr,
            transport: transport.clone(),
            topic_channels: TopicChannels::SingleChannel {
                channel_id: plane_handshake.plane_id,
            },
            config,
            init: plane_handshake.init.clone(),
        })
        .local(|cx| {
            // Shutdown the role when the transport is shutdown.
            let role_shutdown_signal = cx.role_shutdown_signal().clone();
            cx.raw_spawner().spawn(async move {
                transport_shutdown_signal.wait().await;
                role_shutdown_signal.notify();
            });
        });

    Ok(WebSocketConnection {
        endpoint: plane_handshake.endpoint_addr,
        transport,
        init: plane_handshake.init,
        role_handle,
    })
}

pub async fn web_ws_client_handshake(addr: &str) -> Result<(WebSocket, Vec<u8>), ()> {
    use futures_util::StreamExt;

    let mut websocket = WebSocket::open(addr).map_err(|_| ())?;

    let payload = websocket
        .next()
        .await
        .map(|r| r.map_err(|_| ()))
        .unwrap_or(Err(()))?;

    let Message::Bytes(payload_bytes) = payload else {
        return Err(());
    };

    Ok((websocket, payload_bytes))
}

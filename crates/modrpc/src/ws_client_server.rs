use std::cell::Cell;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    endpoint_proto::PlaneHandshake,
    rt::WorkerGroup,
    EndpointAddr,
    HeapBufferPool,
    InterfaceRole,
    PlaneConfig,
    RoleStartFn,
    RuntimeHandle,
    TransportHandle,
    TcpTransport,
    TopicChannels,
    WorkerId,
};

pub struct WebSocketServer {
    next_plane_id: Cell<u32>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            next_plane_id: Cell::new(0),
        }
    }

    pub async fn accept_local<Role: InterfaceRole>(
        &self,
        rt: &RuntimeHandle,
        buffer_pool: HeapBufferPool,
        tcp_stream: TcpStream,
        start_fn: impl RoleStartFn<Role>,
        init: &Role::Init,
    ) -> std::io::Result<Role::Hooks>
        where Role::Init: Clone + Send + Sync,
    {
        let plane_id = self.next_plane_id.get();
        self.next_plane_id.set(self.next_plane_id.get() + 1);

        let Ok(mut websocket) = tokio_tungstenite::accept_async(tcp_stream).await else {
            todo!();
        };

        Self::handshake(&mut stream, plane_id, EndpointAddr { endpoint: 1 }, init).await?;

        let transport = rt.add_transport(WebSocketTransport {
            websocket,
            buffer_pool,
        })
        .await;

        let mut plane_builder = rt.start_plane::<Role>(PlaneConfig {
            plane_id,
            endpoint_addr: EndpointAddr { endpoint: 0 },
            transport: &transport,
            topic_channels: TopicChannels::SingleChannel { channel_id: 0 },
            init,
        });

        let transport_shutdown_signal = transport.shutdown_signal.clone();
        let role_handle = plane_builder.local(|cx| {
            // Shutdown the plane when the transport is shutdown.
            let role_shutdown_signal = cx.shutdown_signal.clone();
            cx.spawner.spawn(async move {
                transport_shutdown_signal.wait().await;
                role_shutdown_signal.notify();
            });

            start_fn(cx);
        }).await;

        Ok(role_handle)
    }

    async fn handshake<Init: mproto::Owned>(
        stream: &mut TcpStream,
        plane_id: u32,
        endpoint_addr: EndpointAddr,
        init: &Init,
    ) -> std::io::Result<()> {
        // Send connect respose
        let payload = PlaneHandshake { plane_id, endpoint_addr, init };
        let payload_len = mproto::encoded_len(&payload);
        let mut payload_buf = vec![0u8; 2 + payload_len];
        payload_buf[..2].copy_from_slice(&(payload_len as u16).to_le_bytes());
        mproto::encode_value(payload, &mut payload_buf[2..]);
        stream.write_all(&payload_buf[..]).await?;

        Ok(())
    }
}

pub async fn tcp_connect<Role: InterfaceRole>(
    rt: &RuntimeHandle,
    buffer_pool: HeapBufferPool,
    worker_id: WorkerId,
    mut stream: TcpStream,
) -> std::io::Result<(TransportHandle, Role::Hooks)>
    where Role::Init: Clone + std::fmt::Debug + Send + Sync,
{
    stream.set_nodelay(true).unwrap();

    let mut payload_len_bytes = [0u8; 2];
    stream.read_exact(&mut payload_len_bytes).await?;
    let payload_len = u16::from_le_bytes(payload_len_bytes);
    let mut payload_bytes = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload_bytes).await?;
    let plane_handshake: PlaneHandshake<Role::Init> = mproto::decode_value(&payload_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let transport = rt.add_transport(TcpTransport {
        worker_id,
        stream,
        buffer_pool,
    })
    .await;

    let transport_shutdown_signal = transport.shutdown_signal.clone();

    let role_handle = rt.start_plane::<Role>(PlaneConfig {
        plane_id: plane_handshake.plane_id,
        endpoint_addr: plane_handshake.endpoint_addr,
        transport: &transport,
        topic_channels: TopicChannels::SingleChannel { channel_id: 0 },
        init: &plane_handshake.init,
    })
    .local(|cx| {
        // Shutdown the plane when the transport is shutdown.
        let role_shutdown_signal = cx.shutdown_signal.clone();
        cx.spawner.spawn(async move {
            transport_shutdown_signal.wait().await;
            role_shutdown_signal.notify();
        });
    })
    .await;

    Ok((transport, role_handle))
}

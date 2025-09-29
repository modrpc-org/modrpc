use std::cell::Cell;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    EndpointAddr, HeapBufferPool, InterfaceRole, RoleConfig, RoleStartFn, RuntimeHandle,
    TcpTransport, TopicChannels, TransportHandle, WorkerId, endpoint_proto::PlaneHandshake,
    rt::WorkerGroup,
};

pub struct TcpServer {
    next_plane_id: Cell<u32>,
}

impl TcpServer {
    pub fn new() -> Self {
        Self {
            next_plane_id: Cell::new(0),
        }
    }

    pub async fn accept<Role: InterfaceRole>(
        &self,
        rt: &RuntimeHandle,
        buffer_pool: HeapBufferPool,
        worker_id: WorkerId,
        worker_group: Option<WorkerGroup>,
        mut stream: tokio::net::TcpStream,
        start_fn: impl RoleStartFn<Role> + Send + Sync + Clone + 'static,
        config: Role::Config,
        init: Role::Init,
    ) -> std::io::Result<Role::Hooks>
    where
        Role::Config: Clone + Send + Sync,
        Role::Init: Clone + Send + Sync,
    {
        let plane_id = self.next_plane_id.get();
        self.next_plane_id.set(self.next_plane_id.get() + 1);

        Self::handshake(&mut stream, plane_id, EndpointAddr { endpoint: 1 }, &init).await?;

        let transport = rt
            .add_transport(TcpTransport {
                worker_id,
                stream,
                buffer_pool,
            })
            .await;

        let mut plane_builder = rt.start_role::<Role>(RoleConfig {
            plane_id,
            endpoint_addr: EndpointAddr { endpoint: 0 },
            transport,
            topic_channels: TopicChannels::SingleChannel {
                channel_id: plane_id,
            },
            config: config.clone(),
            init,
        });

        if let Some(worker_group) = worker_group {
            plane_builder = plane_builder
                .on_worker_group(worker_group, start_fn.clone())
                .await;
        }

        let role_handle = plane_builder.local(start_fn);

        Ok(role_handle)
    }

    pub async fn accept_local<Role: InterfaceRole>(
        &self,
        rt: &RuntimeHandle,
        buffer_pool: HeapBufferPool,
        mut stream: tokio::net::TcpStream,
        start_fn: impl RoleStartFn<Role>,
        config: Role::Config,
        init: Role::Init,
    ) -> std::io::Result<Role::Hooks>
    where
        Role::Config: Clone + Send + Sync,
        Role::Init: Clone + Send + Sync,
    {
        let plane_id = self.next_plane_id.get();
        self.next_plane_id.set(self.next_plane_id.get() + 1);

        Self::handshake(&mut stream, plane_id, EndpointAddr { endpoint: 1 }, &init).await?;

        let transport = rt
            .add_transport(TcpTransport {
                worker_id: WorkerId::local(),
                stream,
                buffer_pool,
            })
            .await;

        let plane_builder = rt.start_role::<Role>(RoleConfig {
            plane_id,
            endpoint_addr: EndpointAddr { endpoint: 0 },
            transport,
            topic_channels: TopicChannels::SingleChannel {
                channel_id: plane_id,
            },
            config,
            init,
        });

        let role_handle = plane_builder.local(start_fn);

        Ok(role_handle)
    }

    async fn handshake<Init: mproto::Owned>(
        stream: &mut tokio::net::TcpStream,
        plane_id: u32,
        endpoint_addr: EndpointAddr,
        init: &Init,
    ) -> std::io::Result<()> {
        // Send connect respose
        let payload = PlaneHandshake {
            plane_id,
            endpoint_addr,
            init,
        };
        let payload_len = mproto::encoded_len(&payload);
        let mut payload_buf = vec![0u8; 2 + payload_len];
        payload_buf[..2].copy_from_slice(&(payload_len as u16).to_le_bytes());
        mproto::encode_value(payload, &mut payload_buf[2..]);
        stream.write_all(&payload_buf[..]).await?;

        Ok(())
    }
}

pub struct TcpConnection<Role: InterfaceRole> {
    pub endpoint: EndpointAddr,
    pub transport: TransportHandle,
    pub init: Role::Init,
    pub role_handle: Role::Hooks,
}

pub async fn tcp_connect<Role: InterfaceRole>(
    rt: &RuntimeHandle,
    buffer_pool: HeapBufferPool,
    worker_id: WorkerId,
    config: Role::Config,
    mut stream: tokio::net::TcpStream,
) -> std::io::Result<TcpConnection<Role>>
where
    Role::Config: Clone + Send + Sync,
    Role::Init: Clone + std::fmt::Debug + Send + Sync,
{
    let mut payload_len_bytes = [0u8; 2];
    stream.read_exact(&mut payload_len_bytes).await?;
    let payload_len = u16::from_le_bytes(payload_len_bytes);
    let mut payload_bytes = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload_bytes).await?;
    let plane_handshake: PlaneHandshake<Role::Init> = mproto::decode_value(&payload_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let transport = rt
        .add_transport(TcpTransport {
            worker_id,
            stream,
            buffer_pool,
        })
        .await;

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
        .local(|_cx| {});

    Ok(TcpConnection {
        endpoint: plane_handshake.endpoint_addr,
        transport,
        init: plane_handshake.init,
        role_handle,
    })
}

pub async fn tcp_connect_builder<Role: InterfaceRole, R>(
    rt: &RuntimeHandle,
    buffer_pool: HeapBufferPool,
    worker_id: WorkerId,
    config: Role::Config,
    mut stream: tokio::net::TcpStream,
    build_fn: impl AsyncFnOnce(crate::StartRoleHandle<Role>) -> R,
) -> std::io::Result<(EndpointAddr, TransportHandle, R)>
where
    Role::Config: Clone + Send + Sync,
    Role::Init: Clone + std::fmt::Debug + Send + Sync,
{
    let mut payload_len_bytes = [0u8; 2];
    stream.read_exact(&mut payload_len_bytes).await?;
    let payload_len = u16::from_le_bytes(payload_len_bytes);
    let mut payload_bytes = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload_bytes).await?;
    let plane_handshake: PlaneHandshake<Role::Init> = mproto::decode_value(&payload_bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let transport = rt
        .add_transport(TcpTransport {
            worker_id,
            stream,
            buffer_pool,
        })
        .await;

    let start_role_handle = rt.start_role::<Role>(RoleConfig {
        plane_id: plane_handshake.plane_id,
        endpoint_addr: plane_handshake.endpoint_addr,
        transport: transport.clone(),
        topic_channels: TopicChannels::SingleChannel {
            channel_id: plane_handshake.plane_id,
        },
        config,
        init: plane_handshake.init,
    });
    let build_result = build_fn(start_role_handle).await;

    Ok((plane_handshake.endpoint_addr, transport, build_result))
}

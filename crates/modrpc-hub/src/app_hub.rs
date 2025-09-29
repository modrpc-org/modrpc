use std::{
    cell::Cell,
    net::SocketAddr,
    rc::Rc,
};

use modrpc::{EndpointAddr, PlaneHandshake};

use crate::{
    Broadcaster, BroadcasterHandle, ChannelId, LocalHubTransport,
};

#[cfg(feature = "tcp-transport")]
use crate::spawn_tcp_spoke;
#[cfg(feature = "websocket-transport")]
use crate::spawn_websocket_spoke;

pub trait AppHubDelegate {
    type Init<'a>: mproto::Encode;

    #[allow(async_fn_in_trait)]
    async fn client_handshake(
        &self,
        endpoint_addr: modrpc::EndpointAddr,
        handshake_fn: impl for<'a> AsyncFnOnce(Self::Init<'a>) -> std::io::Result<()>,
    ) -> std::io::Result<()>;

    #[allow(async_fn_in_trait)]
    async fn client_disconnected(&self, endpoint_addr: modrpc::EndpointAddr);
}

pub struct AppHubBuilder {
    buffer_pool: modrpc::HeapBufferPool,
    rt: modrpc::RuntimeHandle,
    max_packet_size: usize,
    broadcaster_worker: modrpc::WorkerId,
    #[cfg(feature = "tcp-transport")]
    tcp_bind_addr: Option<SocketAddr>,
    #[cfg(feature = "websocket-transport")]
    websocket_bind_addr: Option<SocketAddr>,
}

impl AppHubBuilder {
    pub fn new(
        buffer_pool: modrpc::HeapBufferPool,
        rt: modrpc::RuntimeHandle,
    ) -> Self {
        let max_packet_size = buffer_pool.buffer_size();
        Self {
            buffer_pool,
            rt,
            max_packet_size,
            broadcaster_worker: modrpc::WorkerId::local(),
            #[cfg(feature = "tcp-transport")]
            tcp_bind_addr: None,
            #[cfg(feature = "websocket-transport")]
            websocket_bind_addr: None,
        }
    }

    pub fn max_packet_size(mut self, max_packet_size: usize) -> Self {
        assert!(max_packet_size <= self.buffer_pool.buffer_size());
        self.max_packet_size = max_packet_size;
        self
    }

    pub fn broadcaster_worker(mut self, worker_id: modrpc::WorkerId) -> Self {
        self.broadcaster_worker = worker_id;
        self
    }

    #[cfg(feature = "tcp-transport")]
    pub fn with_tcp(mut self, bind_addr: SocketAddr) -> Self {
        self.tcp_bind_addr = Some(bind_addr);
        self
    }

    #[cfg(feature = "websocket-transport")]
    pub fn with_websocket(mut self, bind_addr: SocketAddr) -> Self {
        self.websocket_bind_addr = Some(bind_addr);
        self
    }

    pub async fn build<Role, Delegate>(
        self,
        delegate: Delegate,
        config: Role::Config,
        init: Role::Init,
    ) -> modrpc::StartRoleHandle<Role>
    where
        Role: modrpc::InterfaceRole,
        Delegate: AppHubDelegate + 'static,
        for<'a> Delegate::Init<'a>: mproto::Compatible<Role::Init>,
    {
        // TODO support running the broadcaster on a separate thread
        /*let broadcaster_handle = self.rt.get_worker(self.broadcaster_worker).run_once(|worker_cx| {
            let mut broadcaster = Broadcaster::new(64);
            let broadcaster_handle = broadcaster.handle();
            worker_cx.spawn(async move {
                broadcaster.run().await;
            });
            broadcaster_handle
        })
        .await;*/

        let worker_cx = self.rt.local_worker_context()
            .expect("modrpc_hub::AppHubBuilder::build must run on a modrpc worker");

        let mut broadcaster = Broadcaster::new(64);
        let broadcaster_handle = broadcaster.handle();
        worker_cx.spawn(async move {
            broadcaster.run().await;
        });

        let transport = self.rt.add_transport(LocalHubTransport::Static {
            buffer_pool: self.buffer_pool.clone(),
            broadcaster_handle: broadcaster_handle.clone(),
            channel_ids: vec![0x42424242],
        })
        .await;

        let delegate = Rc::new(delegate);
        let next_endpoint_id = Rc::new(Cell::new(1));

        #[cfg(feature = "tcp-transport")]
        if let Some(tcp_bind_addr) = self.tcp_bind_addr {
            spawn_hub_tcp(
                self.buffer_pool.clone(),
                self.rt.clone(),
                broadcaster_handle.clone(),
                self.max_packet_size,
                next_endpoint_id.clone(),
                tcp_bind_addr,
                delegate.clone(),
            );
        }
        #[cfg(feature = "websocket-transport")]
        if let Some(websocket_bind_addr) = self.websocket_bind_addr {
            spawn_hub_websocket(
                self.buffer_pool.clone(),
                self.rt.clone(),
                broadcaster_handle.clone(),
                self.max_packet_size,
                next_endpoint_id.clone(),
                websocket_bind_addr,
                delegate.clone(),
            );
        }

        self.rt.start_role(modrpc::RoleConfig {
            plane_id: 0x42424242,
            endpoint_addr: modrpc::EndpointAddr { endpoint: 0 },
            transport,
            topic_channels: modrpc::TopicChannels::SingleChannel { channel_id: 0x42424242 },
            config,
            init,
        })
    }
}

#[cfg(feature = "tcp-transport")]
fn spawn_hub_tcp(
    buffer_pool: modrpc::HeapBufferPool,
    rt: modrpc::RuntimeHandle,
    broadcaster_handle: BroadcasterHandle,
    max_packet_size: usize,
    next_endpoint_id: Rc<Cell<u64>>,
    bind_addr: SocketAddr,
    delegate: Rc<impl AppHubDelegate + 'static>,
) {
    let worker_spawner = rt.local_worker_context()
        .expect("modrpc_hub::spawn_hub_tcp must run on a modrpc worker")
        .spawner();
    let raw_spawner = worker_spawner.raw_spawner().clone();
    worker_spawner.spawn(async move {
        let worker_cx = rt.local_worker_context()
            .expect("modrpc_hub::spawn_hub_tcp must run on a modrpc worker");

        let listener = tokio::net::TcpListener::bind(bind_addr).await
            .expect("tcp listener");

        log::info!("Serving modrpc_hub on tcp://{bind_addr}");

        loop {
            let (mut stream, client_addr) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to accept client: {}", e);
                    continue;
                }
            };

            log::info!("Accepted modrpc_hub tcp client {client_addr}");

            if let Err(e) = stream.set_nodelay(true) {
                log::warn!("Failed to set_nodelay(true) for tcp client {client_addr}: {e}");
            }

            let endpoint_addr = EndpointAddr { endpoint: next_endpoint_id.get() };
            if let Err(e) =
                delegate.client_handshake(
                    endpoint_addr,
                    async |init_payload| {
                        let plane_id = 0x42424242;
                        tcp_handshake(
                            &mut stream,
                            plane_id,
                            endpoint_addr,
                            init_payload,
                        )
                        .await
                    }
                )
                .await
            {
                log::error!("Failed to handshake with client {client_addr}: {e}");
                continue;
            }
            next_endpoint_id.set(next_endpoint_id.get() + 1);

            let (broadcaster_nexthop, tcp_shutdown) = spawn_tcp_spoke(
                &worker_cx,
                broadcaster_handle.clone(),
                buffer_pool.clone(),
                stream,
                max_packet_size,
            )
            .await;

            broadcaster_handle.add_next_hop_to_channels(
                broadcaster_nexthop,
                vec![
                    (
                        ChannelId { channel_id: 0x42424242 },
                        ChannelId { channel_id: 0x42424242 },
                    ),
                ],
            )
            .await;

            raw_spawner.spawn({
                let delegate = delegate.clone();
                async move {
                    tcp_shutdown.wait().await;
                    delegate.client_disconnected(endpoint_addr).await;
                }
            })
            .expect("modrpc-hub tcp spawn client disconnect handler");
        }
    });
}

#[cfg(feature = "websocket-transport")]
fn spawn_hub_websocket(
    buffer_pool: modrpc::HeapBufferPool,
    rt: modrpc::RuntimeHandle,
    broadcaster_handle: BroadcasterHandle,
    max_packet_size: usize,
    next_endpoint_id: Rc<Cell<u64>>,
    bind_addr: SocketAddr,
    delegate: Rc<impl AppHubDelegate + 'static>,
) {
    let worker_spawner = rt.local_worker_context()
        .expect("modrpc_hub::spawn_hub_websocket must run on a modrpc worker")
        .spawner();
    let raw_spawner = worker_spawner.raw_spawner().clone();
    worker_spawner.spawn(async move {
        let worker_cx = rt.local_worker_context()
            .expect("modrpc_hub::spawn_hub_websocket must run on a modrpc worker");

        let listener = tokio::net::TcpListener::bind(bind_addr).await
            .expect("tcp listener");

        log::info!("Serving modrpc_hub on ws://{bind_addr}");

        loop {
            let (tcp_stream, client_addr) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to accept client: {}", e);
                    continue;
                }
            };

            log::info!("Accepted modrpc_hub websocket client {client_addr}");

            if let Err(e) = tcp_stream.set_nodelay(true) {
                log::warn!("Failed to set_nodelay(true) for websocket client {client_addr}: {e}");
            }

            let Ok(mut websocket) = tokio_tungstenite::accept_async(tcp_stream).await else {
                log::info!("Failed to accept websocket client");
                continue;
            };

            let endpoint_addr = EndpointAddr { endpoint: next_endpoint_id.get() };
            if let Err(e) =
                delegate.client_handshake(
                    endpoint_addr,
                    async |init_payload| {
                        let plane_id = 0x42424242;
                        websocket_handshake(
                            &mut websocket,
                            plane_id,
                            endpoint_addr,
                            init_payload,
                        )
                        .await
                    }
                )
                .await
            {
                log::error!("Failed to handshake with client {client_addr}: {e}");
                continue;
            }
            next_endpoint_id.set(next_endpoint_id.get() + 1);

            log::info!("Handshake with websocket client {client_addr} success");

            let (broadcaster_nexthop, websocket_shutdown) = spawn_websocket_spoke(
                &worker_cx,
                broadcaster_handle.clone(),
                buffer_pool.clone(),
                websocket,
                max_packet_size,
            )
            .await;

            broadcaster_handle.add_next_hop_to_channels(
                broadcaster_nexthop,
                vec![
                    (
                        ChannelId { channel_id: 0x42424242 },
                        ChannelId { channel_id: 0x42424242 },
                    ),
                ],
            )
            .await;

            raw_spawner.spawn({
                let delegate = delegate.clone();
                async move {
                    websocket_shutdown.wait().await;
                    delegate.client_disconnected(endpoint_addr).await;
                }
            })
            .expect("modrpc-hub websocket spawn client disconnect handler");
        }
    });
}

#[cfg(feature = "tcp-transport")]
async fn tcp_handshake(
    stream: &mut tokio::net::TcpStream,
    plane_id: u32,
    endpoint_addr: EndpointAddr,
    init: impl mproto::Encode,
) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt;

    let payload = PlaneHandshake { plane_id, endpoint_addr, init };
    let payload_len = mproto::encoded_len(&payload);
    let mut payload_buf = vec![0u8; 2 + payload_len];
    payload_buf[..2].copy_from_slice(&(payload_len as u16).to_le_bytes());
    mproto::encode_value(payload, &mut payload_buf[2..]);
    stream.write_all(payload_buf[..].into()).await?;

    Ok(())
}

#[cfg(feature = "websocket-transport")]
async fn websocket_handshake(
    websocket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    plane_id: u32,
    endpoint_addr: EndpointAddr,
    init: impl mproto::Encode,
) -> std::io::Result<()> {
    use tokio_tungstenite::tungstenite::protocol::Message;
    use futures_util::sink::SinkExt;

    let payload = PlaneHandshake { plane_id, endpoint_addr, init };
    let payload_len = mproto::encoded_len(&payload);
    let mut payload_buf = vec![0u8; payload_len];
    mproto::encode_value(payload, &mut payload_buf[..]);

    websocket.send(Message::Binary(payload_buf.into())).await
        .map_err(|e| std::io::Error::other(e))?;

    Ok(())
}

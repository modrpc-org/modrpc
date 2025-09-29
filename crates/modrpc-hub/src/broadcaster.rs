use std::collections::HashMap;

#[cfg(feature = "websocket-transport")]
use tokio_tungstenite::tungstenite::{
    Error as WsError,
    protocol::Message as WsMessage,
};
#[cfg(feature = "websocket-transport")]
use futures_util::sink::{Sink, SinkExt};
#[cfg(feature = "gloo-websocket")]
use futures_util::sink::{Sink, SinkExt};

use modrpc::{
    Packet,
    PacketBundle, ShatterPacketBundle,
};

pub struct InPacket {
    pub transport: TransportIndex,
    pub channel_id: u32,
    pub packet: Packet,
}

#[cfg(feature = "websocket-transport")]
pub type WsSinkBox = Box<dyn Sink<WsMessage, Error = WsError> + Send + std::marker::Unpin>;
#[cfg(feature = "gloo-websocket")]
pub type GlooWsSinkBox = Box<dyn Sink<gloo_net::websocket::Message, Error = gloo_net::websocket::WebSocketError> + std::marker::Unpin>;

enum BroadcasterRequest {
    #[cfg(feature = "tcp-transport")]
    AddTcp {
        stream: tokio::net::tcp::OwnedWriteHalf,
        response_tx: oneshot::Sender<TransportIndex>,
    },
    #[cfg(feature = "websocket-transport")]
    AddWs {
        ws_tx: WsSinkBox,
        response_tx: oneshot::Sender<TransportIndex>,
    },
    #[cfg(feature = "gloo-websocket")]
    AddGlooWs {
        ws_tx: GlooWsSinkBox,
        response_tx: oneshot::Sender<TransportIndex>,
    },
    AddLocal {
        tx: localq::mpsc::Sender<Packet>,
        response_tx: oneshot::Sender<TransportIndex>,
    },
    Remove {
        transport: TransportIndex,
        response_tx: oneshot::Sender<()>,
    },
    AddNextHopToChannels {
        next_hop_transport: TransportIndex,
        channel_ids: Vec<(ChannelId, ChannelId)>, // [(local channel ID, remote channel ID)]
        response_tx: oneshot::Sender<()>,
    },
}

#[cfg(feature = "tcp-transport")]
struct TcpTransport {
    stream: tokio::net::tcp::OwnedWriteHalf,
}

#[cfg(feature = "websocket-transport")]
struct WsTransport {
    ws_tx: WsSinkBox,
}

#[cfg(feature = "gloo-websocket")]
struct GlooWsTransport {
    ws_tx: GlooWsSinkBox,
}

struct LocalTransport {
    tx: localq::mpsc::Sender<Packet>,
}

type TransportKey = slotmap::DefaultKey;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum TransportType {
    #[cfg(feature = "tcp-transport")]
    Tcp,
    #[cfg(feature = "websocket-transport")]
    WebSocket,
    #[cfg(feature = "gloo-websocket")]
    GlooWebSocket,
    Local,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TransportIndex {
    transport_type: TransportType,
    transport: TransportKey,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ChannelId {
    pub channel_id: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct NextHop {
    remote_channel_id: ChannelId,
    transport: TransportIndex,
}

const BUNDLE_HEADER_LEN: usize = <PacketBundle as mproto::BaseLen>::BASE_LEN;

pub struct Broadcaster {
    in_packet_receiver: localq::mpsc::Receiver<InPacket>,
    in_packet_sender: localq::mpsc::Sender<InPacket>,

    // Map local channel_id to list of transports to broadcast packet bundles to
    next_hops: HashMap<ChannelId, Vec<NextHop>>,
    transport_local_channel_ids: HashMap<TransportIndex, Vec<ChannelId>>,

    #[cfg(feature = "tcp-transport")]
    tcp_transports: slotmap::SlotMap<TransportKey, TcpTransport>,
    #[cfg(feature = "websocket-transport")]
    ws_transports: slotmap::SlotMap<TransportKey, WsTransport>,
    #[cfg(feature = "gloo-websocket")]
    gloo_ws_transports: slotmap::SlotMap<TransportKey, GlooWsTransport>,
    local_transports: slotmap::SlotMap<TransportKey, LocalTransport>,

    request_tx: localq::mpsc::Sender<BroadcasterRequest>,
    request_rx: localq::mpsc::Receiver<BroadcasterRequest>,
}

impl Broadcaster {
    pub fn new(packet_queue_capacity: usize) -> Self {
        let (in_packet_sender, in_packet_receiver) = localq::mpsc::channel(packet_queue_capacity);
        let (request_tx, request_rx) = localq::mpsc::channel(16);

        Self {
            in_packet_receiver,
            in_packet_sender,

            next_hops: HashMap::new(),
            transport_local_channel_ids: HashMap::new(),

            local_transports: slotmap::SlotMap::new(),
            #[cfg(feature = "tcp-transport")]
            tcp_transports: slotmap::SlotMap::new(),
            #[cfg(feature = "websocket-transport")]
            ws_transports: slotmap::SlotMap::new(),
            #[cfg(feature = "gloo-websocket")]
            gloo_ws_transports: slotmap::SlotMap::new(),

            request_tx,
            request_rx,
        }
    }

    pub fn handle(&self) -> BroadcasterHandle {
        BroadcasterHandle {
            in_packet_sender: self.in_packet_sender.clone(),
            request: self.request_tx.clone(),
        }
    }

    pub fn add_local_transport(&mut self, tx: localq::mpsc::Sender<Packet>) -> TransportIndex {
        let key = self.local_transports.insert(LocalTransport { tx });
        log::debug!("Added Local transport {:?}", key);
        TransportIndex {
            transport_type: TransportType::Local,
            transport: key,
        }
    }

    pub async fn run(&mut self) {
        use futures_util::FutureExt;

        loop {
            futures_util::select! {
                in_packet = self.in_packet_receiver.recv().fuse() => {
                    let Ok(in_packet) = in_packet else { break; };
                    self.handle_in_packet(in_packet).await;
                }
                request = self.request_rx.recv().fuse() => {
                    let Ok(request) = request else { break; };
                    self.handle_request(request).await;
                }
            };
        }
    }

    async fn handle_in_packet(&mut self, in_packet: InPacket) {
        let local_channel_id = ChannelId {
            channel_id: in_packet.channel_id,
        };

        log::trace!(
            "in packet - channel_id={} transport={:?} len={}",
            local_channel_id.channel_id,
            in_packet.transport,
            in_packet.packet.len(),
        );

        if let Some(next_hops) = self.next_hops.get(&local_channel_id) {
            if let Err(_) = Self::broadcast(
                in_packet,
                next_hops,
                #[cfg(feature = "tcp-transport")]
                &mut self.tcp_transports,
                #[cfg(feature = "websocket-transport")]
                &mut self.ws_transports,
                #[cfg(feature = "gloo-websocket")]
                &mut self.gloo_ws_transports,
                &mut self.local_transports,
            ).await {
                // TODO can this even fail?
            }
        } else {
            log::trace!(
                "No next-hops for local-channel-id={:?}",
                local_channel_id,
            );
        };
    }

    async fn remove_transport(&mut self, transport: TransportIndex) {
        log::info!("removing transport {:?}", transport);

        match transport.transport_type {
            #[cfg(feature = "tcp-transport")]
            TransportType::Tcp => {
                self.tcp_transports.remove(transport.transport);
            }
            #[cfg(feature = "websocket-transport")]
            TransportType::WebSocket => {
                self.ws_transports.remove(transport.transport);
            }
            #[cfg(feature = "gloo-websocket")]
            TransportType::GlooWebSocket => {
                self.gloo_ws_transports.remove(transport.transport);
            }
            TransportType::Local => {
                self.local_transports.remove(transport.transport);
            }
        }

        if let Some(local_channel_ids) =
            self.transport_local_channel_ids.remove(&transport)
        {
            for local_channel_id in local_channel_ids {
                log::debug!(
                    "removing channel {:?} next_hops for transport {:?}",
                    local_channel_id,
                    transport,
                );
                if let Some(next_hops) = self.next_hops.get_mut(&local_channel_id) {
                    // Remove this transport as a next-hop from all of the channels it
                    // participated in.
                    next_hops.retain(|next_hop| next_hop.transport != transport);
                } else {
                    // TODO warning?
                }
            }
        } else {
            // TODO warning?
        }
    }

    async fn handle_request(&mut self, request: BroadcasterRequest) {
        match request {
            #[cfg(feature = "tcp-transport")]
            BroadcasterRequest::AddTcp { stream, response_tx } => {
                let key = self.tcp_transports.insert(TcpTransport {
                    stream,
                });
                log::debug!("Added TCP transport {:?}", key);
                let _ = response_tx.send(TransportIndex {
                    transport_type: TransportType::Tcp,
                    transport: key,
                });
            }
            #[cfg(feature = "websocket-transport")]
            BroadcasterRequest::AddWs { ws_tx, response_tx } => {
                let key = self.ws_transports.insert(WsTransport { ws_tx });
                log::debug!("Added WebSocket transport {:?}", key);
                let _ = response_tx.send(TransportIndex {
                    transport_type: TransportType::WebSocket,
                    transport: key,
                });
            }
            #[cfg(feature = "gloo-websocket")]
            BroadcasterRequest::AddGlooWs { ws_tx, response_tx } => {
                let key = self.gloo_ws_transports.insert(GlooWsTransport { ws_tx });
                log::debug!("Added Gloo WebSocket transport {:?}", key);
                let _ = response_tx.send(TransportIndex {
                    transport_type: TransportType::GlooWebSocket,
                    transport: key,
                });
            }
            BroadcasterRequest::AddLocal { tx, response_tx } => {
                let key = self.local_transports.insert(LocalTransport { tx });
                log::debug!("Added Local transport {:?}", key);
                let _ = response_tx.send(TransportIndex {
                    transport_type: TransportType::Local,
                    transport: key,
                });
            }
            BroadcasterRequest::Remove { transport, response_tx } => {
                self.remove_transport(transport).await;
                log::debug!("TransportHub removed transport {:?}", transport);
                let _ = response_tx.send(());
            }
            BroadcasterRequest::AddNextHopToChannels {
                next_hop_transport, channel_ids, response_tx,
            } => {
                log::debug!(
                    "Adding next hop to channels transport={:?}, channel_ids={:?}",
                    next_hop_transport,
                    channel_ids,
                );
                for &(local_channel_id, remote_channel_id) in &channel_ids {
                    let next_hops =
                        self.next_hops.entry(local_channel_id).or_insert(Vec::new());
                    next_hops.push(NextHop {
                        remote_channel_id,
                        transport: next_hop_transport,
                    });
                }

                self.transport_local_channel_ids
                    .entry(next_hop_transport)
                    .or_insert(Vec::new())
                    .extend(channel_ids.iter().map(|(local_channel_id, _)| local_channel_id));

                // Don't care if requester hung up
                let _ = response_tx.send(());
            }
        }
    }

    async fn broadcast(
        in_packet: InPacket,
        next_hops: &[NextHop],
        #[cfg(feature = "tcp-transport")]
        tcp_transports: &mut slotmap::SlotMap<TransportKey, TcpTransport>,
        #[cfg(feature = "websocket-transport")]
        ws_transports: &mut slotmap::SlotMap<TransportKey, WsTransport>,
        #[cfg(feature = "gloo-websocket")]
        gloo_ws_transports: &mut slotmap::SlotMap<TransportKey, GlooWsTransport>,
        local_transports: &mut slotmap::SlotMap<TransportKey, LocalTransport>,
    ) -> std::io::Result<()> {
        for next_hop in next_hops {
            let transport_index = next_hop.transport;

            if transport_index == in_packet.transport {
                // Don't send bundles back to transport they originated from.
                continue;
            }

            log::trace!(
                "[transmitter]   Sending to next-hop - transport={:?} channel={} length={}",
                transport_index,
                next_hop.remote_channel_id.channel_id,
                in_packet.packet.len(),
            );

            match transport_index.transport_type {
                #[cfg(feature = "tcp-transport")]
                TransportType::Tcp => {
                    let bundle_payload = &in_packet.packet[..];

                    // Fill bundle header
                    let mut bundle_header_buf = [0u8; BUNDLE_HEADER_LEN];
                    mproto::encode_value(
                        PacketBundle {
                            channel_id: next_hop.remote_channel_id.channel_id,
                            length: bundle_payload.len() as u16,
                        },
                        &mut bundle_header_buf,
                    );

                    if let Some(tcp_transport) = tcp_transports.get_mut(transport_index.transport) {
                        if let Err(_) =
                            Self::write_tcp_bundle(
                                &mut tcp_transport.stream,
                                &bundle_header_buf,
                                bundle_payload,
                            ).await
                        {
                            log::debug!("TransportHub tcp transport closed: {:?}", transport_index);
                            // Remove transport
                            tcp_transports.remove(transport_index.transport);
                        }
                    }
                }
                #[cfg(feature = "websocket-transport")]
                TransportType::WebSocket => {
                    let bundle_payload = &in_packet.packet[..];
                    let mut message = vec![0u8; BUNDLE_HEADER_LEN + bundle_payload.len()];

                    // Fill bundle header
                    mproto::encode_value(
                        PacketBundle {
                            channel_id: next_hop.remote_channel_id.channel_id,
                            length: bundle_payload.len() as u16,
                        },
                        &mut message[..BUNDLE_HEADER_LEN],
                    );

                    message[BUNDLE_HEADER_LEN..].copy_from_slice(bundle_payload);

                    if let Some(ws_transport) = ws_transports.get_mut(transport_index.transport) {
                        if let Err(_) = ws_transport.ws_tx.send(WsMessage::Binary(message.into())).await {
                            log::debug!("WebSocket transport closed: {:?}", transport_index);
                            // Remove transport
                            ws_transports.remove(transport_index.transport);
                        }
                    }
                }
                #[cfg(feature = "gloo-websocket")]
                TransportType::GlooWebSocket => {
                    let bundle_payload = &in_packet.packet[..];
                    let mut message = vec![0u8; BUNDLE_HEADER_LEN + bundle_payload.len()];

                    // Fill bundle header
                    mproto::encode_value(
                        PacketBundle {
                            channel_id: next_hop.remote_channel_id.channel_id,
                            length: bundle_payload.len() as u16,
                        },
                        &mut message[..BUNDLE_HEADER_LEN],
                    );

                    message[BUNDLE_HEADER_LEN..].copy_from_slice(bundle_payload);

                    if let Some(ws_transport) = gloo_ws_transports.get_mut(transport_index.transport) {
                        if let Err(_) =
                            ws_transport.ws_tx.send(
                                gloo_net::websocket::Message::Bytes(message)
                            )
                            .await
                        {
                            log::debug!("Gloo WebSocket transport closed: {:?}", transport_index);
                            // Remove transport
                            gloo_ws_transports.remove(transport_index.transport);
                        }
                    }
                }
                TransportType::Local => {
                    let Some(local_transport) =
                        local_transports.get(transport_index.transport)
                    else {
                        continue;
                    };

                    for packet in ShatterPacketBundle::new(&in_packet.packet) {
                        if let Err(_) = local_transport.tx.send(packet).await {
                            local_transports.remove(transport_index.transport);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "tcp-transport")]
    async fn write_tcp_bundle(
        stream: &mut tokio::net::tcp::OwnedWriteHalf,
        header: &[u8],
        payload: &[u8],
    ) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        // TODO vectored write
        stream.write_all(header).await?;
        stream.write_all(payload).await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct BroadcasterHandle {
    in_packet_sender: localq::mpsc::Sender<InPacket>,
    request: localq::mpsc::Sender<BroadcasterRequest>,
}

impl BroadcasterHandle {
    pub fn in_packet_sender(&self) -> &localq::mpsc::Sender<InPacket> {
        &self.in_packet_sender
    }

    #[cfg(feature = "tcp-transport")]
    pub async fn add_tcp(
        &self,
        stream: tokio::net::tcp::OwnedWriteHalf,
    ) -> TransportIndex {
        let (response_tx, response_rx) = oneshot::channel();

        self.request.send(BroadcasterRequest::AddTcp {
            stream,
            response_tx,
        })
        .await
        .unwrap();
        let transport_index = response_rx.await.unwrap();

        transport_index
    }

    #[cfg(feature = "websocket-transport")]
    pub async fn add_ws(
        &self,
        ws_tx: WsSinkBox,
    ) -> TransportIndex {
        let (response_tx, response_rx) = oneshot::channel();

        self.request.send(BroadcasterRequest::AddWs {
            ws_tx,
            response_tx,
        })
        .await
        .unwrap();
        let transport_index = response_rx.await.unwrap();

        transport_index
    }

    #[cfg(feature = "gloo-websocket")]
    pub async fn add_gloo_ws(
        &self,
        ws_tx: GlooWsSinkBox,
    ) -> TransportIndex {
        let (response_tx, response_rx) = oneshot::channel();

        self.request.send(BroadcasterRequest::AddGlooWs {
            ws_tx,
            response_tx,
        })
        .await
        .unwrap();
        let transport_index = response_rx.await.unwrap();

        transport_index
    }

    pub async fn add_local(
        &self,
        tx: localq::mpsc::Sender<Packet>,
    ) -> TransportIndex {
        let (response_tx, response_rx) = oneshot::channel();

        self.request.send(BroadcasterRequest::AddLocal {
            tx,
            response_tx,
        })
        .await
        .unwrap();
        let transport_index = response_rx.await.unwrap();

        transport_index
    }

    pub async fn add_next_hop_to_channels(
        &self,
        next_hop_transport: TransportIndex,
        channel_ids: Vec<(ChannelId, ChannelId)>,
    ) {
        let (response_tx, response_rx) = oneshot::channel();

        self.request.send(BroadcasterRequest::AddNextHopToChannels {
            next_hop_transport,
            channel_ids,
            response_tx,
        })
        .await
        .unwrap();
        let _ = response_rx.await.unwrap();
    }

    pub async fn remove_transport(
        &self,
        transport: TransportIndex,
    ) {
        let (response_tx, response_rx) = oneshot::channel();

        self.request.send(BroadcasterRequest::Remove {
            transport,
            response_tx,
        })
        .await
        .unwrap();
        let _ = response_rx.await.unwrap();
    }
}


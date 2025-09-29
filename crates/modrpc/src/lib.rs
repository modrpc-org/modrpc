pub use bab::{BufferPtr, HeapBufferPool, Packet, SendPacket, WriterFlushSender};
pub use ispawn::LocalSpawner;

pub use context_map::ContextClass;
pub use endpoint_proto::{
    EndpointAddr, PacketBundle, PacketBundleLazy, PlaneHandshake, PlaneHandshakeGen,
    PlaneHandshakeLazy, TransmitPacket, TransmitPacketLazy,
};
pub use interface_builder::{InterfaceBuilder, InterfaceEvent};
pub use packet_sender::{MultiChannelSender, PacketSender, SingleChannelSender};
pub use role::{InterfaceRole, InterfaceSchema, RoleSpawner, RoleStartFn, RoleWorkerContext};
pub use role_setup::{AsyncHandler, EventRxBuilder, EventTx, RoleSetup, add_topic_subscription};
pub use rt::{
    RoleConfig, RuntimeBuilder, RuntimeHandle, StartRoleHandle, TopicChannels, WorkerGroup,
    WorkerHandle,
};
pub use transport::{
    LocalTransport, ShatterPacketBundle, TransportBuilder, TransportContext, TransportHandle,
    WriterConfig, shatter_packet_bundle,
};
pub use worker::{WorkerContext, WorkerId};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct InstanceChannel {
    pub instance_id: u32,
    pub channel_id: u32,
}

mod context_map;
mod endpoint_proto;
mod flush_batcher;
mod interface_builder;
mod load_balancer;
mod packet_processor;
mod packet_sender;
mod role;
mod role_setup;
mod rt;
mod transport;
mod worker;

#[cfg(feature = "tcp-transport")]
mod tcp_transport;
#[cfg(feature = "tcp-transport")]
pub use tcp_transport::TcpTransport;

#[cfg(feature = "tcp-transport")]
mod tcp_ingress;
#[cfg(feature = "tcp-transport")]
pub use tcp_ingress::TcpIngress;

#[cfg(feature = "tcp-transport")]
mod tcp_client_server;
#[cfg(feature = "tcp-transport")]
pub use tcp_client_server::{TcpConnection, TcpServer, tcp_connect, tcp_connect_builder};

#[cfg(feature = "ws-transport")]
mod ws_transport;
#[cfg(feature = "ws-transport")]
pub use ws_transport::WebSocketTransport;
#[cfg(feature = "ws-transport")]
mod ws_ingress;
#[cfg(feature = "ws-transport")]
pub use ws_ingress::WebSocketIngress;

#[cfg(feature = "web-ws-transport")]
mod web_ws_ingress;
#[cfg(feature = "web-ws-transport")]
mod web_ws_transport;
#[cfg(feature = "web-ws-transport")]
pub use web_ws_transport::WebSocketTransport;
#[cfg(feature = "web-ws-transport")]
mod web_ws_client;
#[cfg(feature = "web-ws-transport")]
pub use web_ws_client::{WebSocketConnection, web_ws_client_handshake, web_ws_connect};
#[cfg(feature = "web-ws-transport")]
pub use web_ws_ingress::WebSocketIngress;

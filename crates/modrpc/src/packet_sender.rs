use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use bab::DynWriter;

use crate::{
    endpoint_proto::{EndpointAddr, TransmitPacket},
    flush_batcher::FlushBatcher,
    packet_processor::{PACKET_PROCESSOR_SOURCE_NEW, PacketProcessor},
};

pub trait ChannelSelector {
    fn select_channel(&self, topic: u32) -> &DynWriter;
}

pub struct SingleChannelSelector {
    tx_channel: DynWriter,
}

impl ChannelSelector for SingleChannelSelector {
    fn select_channel(&self, _topic: u32) -> &DynWriter {
        &self.tx_channel
    }
}

#[derive(Clone)]
pub struct MultiChannelSelector {
    topic_channels: Rc<RefCell<HashMap<u32, usize>>>,
    channels: Vec<DynWriter>,
}

impl MultiChannelSelector {
    // TODO MultiChannelSelector isn't really tested / supported at the moment
    #[allow(unused)]
    pub fn set_topic_channel(&self, topic: u32, channel_id: usize) {
        let mut topic_channels = self.topic_channels.borrow_mut();
        topic_channels.insert(topic, channel_id);
    }
}

impl ChannelSelector for MultiChannelSelector {
    fn select_channel(&self, topic: u32) -> &DynWriter {
        let channel_id = *self.topic_channels.borrow().get(&topic).unwrap();
        &self.channels[channel_id]
    }
}

#[derive(Clone)]
pub struct SingleChannelSender {
    pub endpoint_addr: EndpointAddr,
    pub tx_channel: DynWriter,
    pub pp: Rc<PacketProcessor>,
    pub flush_batcher: Option<FlushBatcher>,
}

impl Into<PacketSender> for SingleChannelSender {
    fn into(self) -> PacketSender {
        PacketSender {
            inner: Rc::new(PacketSenderInner {
                endpoint_addr: self.endpoint_addr,
                channel_selector: Box::new(SingleChannelSelector {
                    tx_channel: self.tx_channel,
                }),
                pp: self.pp,
                flush_batcher: self.flush_batcher,
            }),
        }
    }
}

#[derive(Clone)]
pub struct MultiChannelSender {
    pub endpoint_addr: EndpointAddr,
    pub topic_channels: HashMap<u32, usize>,
    pub channels: Vec<DynWriter>,
    pub pp: Rc<PacketProcessor>,
    pub flush_batcher: Option<FlushBatcher>,
}

impl Into<PacketSender> for MultiChannelSender {
    fn into(self) -> PacketSender {
        PacketSender {
            inner: Rc::new(PacketSenderInner {
                endpoint_addr: self.endpoint_addr,
                channel_selector: Box::new(MultiChannelSelector {
                    topic_channels: Rc::new(RefCell::new(self.topic_channels.clone())),
                    channels: self.channels.into_iter().map(|x| x.into()).collect(),
                }),
                pp: self.pp,
                flush_batcher: self.flush_batcher,
            }),
        }
    }
}

pub struct PacketSenderInner {
    endpoint_addr: EndpointAddr,
    channel_selector: Box<dyn ChannelSelector>,
    pp: Rc<PacketProcessor>,
    flush_batcher: Option<FlushBatcher>,
}

pub struct PacketSender {
    inner: Rc<PacketSenderInner>,
}

impl PacketSender {
    pub fn id(&self) -> usize {
        &*self.inner as *const _ as usize
    }

    pub fn try_send(&self, plane_id: u32, topic: u32, payload: impl mproto::Encode) -> bool {
        futures_util::FutureExt::now_or_never(self.send(plane_id, topic, payload)).is_some()
    }

    #[inline]
    pub async fn send(&self, plane_id: u32, topic: u32, payload: impl mproto::Encode) {
        let payload_length = mproto::encoded_len(&payload);
        self.send_raw(plane_id, topic, payload_length, |buf| {
            mproto::encode_value(payload, buf);
        })
        .await;
    }

    pub async fn send_raw(&self, plane_id: u32, topic: u32, len: usize, f: impl FnOnce(&mut [u8])) {
        let channel = self.inner.channel_selector.select_channel(topic);

        let header_length = <TransmitPacket as mproto::BaseLen>::BASE_LEN;

        let mut packet = channel.reserve(header_length + len).await;

        let tx_packet = TransmitPacket {
            infra_id: 0,
            plane_id,
            topic,
            source: self.inner.endpoint_addr.clone(),
            payload_length: len as u16,
        };
        mproto::encode_value(tx_packet, &mut *packet);
        f(&mut packet.as_mut()[header_length..]);

        self.inner
            .pp
            .handle_packet(PACKET_PROCESSOR_SOURCE_NEW, &packet.into())
            .await;

        // TODO immediately flush under low-load
        if let Some(flush_batcher) = &self.inner.flush_batcher {
            // TODO configurable
            if channel.unflushed_bytes() >= 32_000 {
                channel.flush();
                flush_batcher.cancel_flush();
            } else {
                flush_batcher.schedule_flush();
            }
        }
    }

    pub async fn send_buffer(&self, plane_id: u32, topic: u32, buffer: bab::BufferPtr) {
        let channel = self.inner.channel_selector.select_channel(topic);

        let header_length = <TransmitPacket as mproto::BaseLen>::BASE_LEN;
        let buffer_length = bab::WriterFlushSender::get_complete_buffer_len(buffer) as usize;
        let payload_length = (buffer_length - header_length) as u16;
        let header = TransmitPacket {
            infra_id: 0,
            plane_id,
            topic,
            source: self.inner.endpoint_addr.clone(),
            payload_length,
        };
        mproto::encode_value(header, unsafe { buffer.slice_mut(0..header_length) });

        let packet = channel.ingest_complete_buffer(buffer, buffer_length);
        self.inner
            .pp
            .handle_packet(PACKET_PROCESSOR_SOURCE_NEW, &packet.into())
            .await;

        // TODO immediately flush under low-load
        if let Some(flush_batcher) = &self.inner.flush_batcher {
            // TODO configurable
            if channel.unflushed_bytes() >= 32_000 {
                channel.flush();
                flush_batcher.cancel_flush();
            } else {
                flush_batcher.schedule_flush();
            }
        }
    }
}

impl Clone for PacketSender {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

use std::{
    cell::RefCell,
    rc::Rc,
};
use modrpc::RoleSetup;

use crate::{
    proto::{StreamInitState, StreamItem, StreamItemLazy, StreamReceiverConfig},
    receive_stream::{ReceiveStream, StreamState},
};

#[derive(Clone)]
pub struct StreamReceiver<T> {
    subscriptions: Rc<Subscriptions>,
    _phantom: core::marker::PhantomData<T>,
}

impl<T: mproto::Owned> StreamReceiver<T> {
    pub fn subscribe(&self, next_seq: Option<u64>) -> StreamSubscription<T> {
        let receive_stream = ReceiveStream::new(next_seq);
        self.subscriptions.stream_states.borrow_mut().push(receive_stream.stream_state().clone());
        StreamSubscription {
            receive_stream,
            subscriptions: self.subscriptions.clone(),
            _phantom: core::marker::PhantomData,
        }
    }
}

pub struct StreamSubscription<T> {
    receive_stream: ReceiveStream,
    subscriptions: Rc<Subscriptions>,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> Drop for StreamSubscription<T> {
    fn drop(&mut self) {
        self.subscriptions.stream_states.borrow_mut()
            .retain(|s| Rc::as_ptr(s) != Rc::as_ptr(self.receive_stream.stream_state()));
    }
}

impl<T: mproto::Owned> StreamSubscription<T> {
    pub async fn next(&mut self) -> mproto::DecodeResult<T> {
        use mproto::BaseLen;

        let packet = self.receive_stream.next_packet().await;

        let stream_item: StreamItemLazy<T> = mproto::decode_value(
            &packet.as_ref()[modrpc::TransmitPacket::BASE_LEN..]
        )?;
        let owned_result = stream_item.payload().map(|i| T::lazy_to_owned(i))??;

        Ok(owned_result)
    }

    pub async fn next_lazy(&mut self)
        -> mproto::DecodeResult<mproto::LazyBuf<T, modrpc::Packet>>
    {
        use mproto::BaseLen;

        let packet = self.receive_stream.next_packet().await;
        packet.advance(modrpc::TransmitPacket::BASE_LEN);

        let stream_item: mproto::LazyBuf<StreamItem<T>, _> = mproto::LazyBuf::new(packet);
        // TODO LazyBuf::try_map
        let payload = stream_item.map(|s| s.payload().unwrap());

        Ok(payload)
    }

    pub fn try_next(&mut self) -> mproto::DecodeResult<Option<T>> {
        use mproto::BaseLen;

        let Some(packet) = self.receive_stream.try_next_packet() else {
            return Ok(None);
        };
        packet.advance(modrpc::TransmitPacket::BASE_LEN);

        let stream_item: StreamItemLazy<T> = mproto::decode_value(&packet)?;
        let payload = stream_item.payload().and_then(|i| T::lazy_to_owned(i))?;

        Ok(Some(payload))
    }

    pub fn try_next_lazy(&mut self)
        -> mproto::DecodeResult<Option<mproto::LazyBuf<T, modrpc::Packet>>>
    {
        use mproto::BaseLen;

        let Some(packet) = self.receive_stream.try_next_packet() else {
            return Ok(None);
        };
        packet.advance(modrpc::TransmitPacket::BASE_LEN);

        let stream_item: mproto::LazyBuf<StreamItem<T>, _> = mproto::LazyBuf::new(packet);
        // TODO LazyBuf::try_map
        let payload = stream_item.map(|s| s.payload().unwrap());

        Ok(Some(payload))
    }
}

struct Subscriptions {
    stream_states: RefCell<Vec<Rc<StreamState>>>,
}

pub struct StreamReceiverBuilder<T> {
    stubs: crate::StreamReceiverStubs<T>,
    subscriptions: Rc<Subscriptions>,
}

impl<T: mproto::Owned> StreamReceiverBuilder<T> {
    pub fn new(
        _name: &'static str,
        _hooks: crate::StreamReceiverHooks<T>,
        stubs: crate::StreamReceiverStubs<T>,
        _config: &StreamReceiverConfig,
        _init: StreamInitState,
    ) -> Self {
        Self {
            stubs,
            subscriptions: Rc::new(Subscriptions {
                stream_states: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> crate::StreamReceiver<T> {
        crate::StreamReceiver {
            subscriptions: self.subscriptions.clone(),
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn build(
        self,
        setup: &RoleSetup,
    ) {
        use mproto::BaseLen;

        let subscriptions = self.subscriptions;
        self.stubs.item.inline_untyped(setup, move |_source, packet| {
            let stream_item_bytes = &packet[modrpc::TransmitPacket::BASE_LEN..];
            let Ok(stream_item) =
                mproto::decode_value::<StreamItemLazy<T>>(stream_item_bytes)
            else {
                return;
            };
            let Ok(seq) = stream_item.seq() else {
                return;
            };

            for stream_state in &mut *subscriptions.stream_states.borrow_mut() {
                let _stream_is_done = stream_state.handle_item(seq, false, packet.clone());
            }
        })
        .subscribe();
    }
}

#[cfg(test)]
mod test {
    use modrpc_executor::ModrpcExecutor;
    use crate::{
        StreamInitState,
        StreamSenderBuilder,
        StreamSenderConfig,
        StreamSenderRole,
        StreamReceiverConfig,
        StreamReceiverRole,
    };
    use super::*;

    #[test]
    fn test_stream_receiver() {
        let mut ex = modrpc_executor::FuturesExecutor::new();
        let (rt, _rt_shutdown) = modrpc::RuntimeHandle::single_threaded(&mut ex);

        ex.run_until(async move {
            let transport = rt.add_transport(modrpc::LocalTransport {
                buffer_size: 256,
                buffer_pool_batches: 16,
                buffer_pool_batch_size: 16,
            })
            .await;

            let mut stream_sender = None;
            let _ =
                rt.start_role::<StreamSenderRole<String>>(modrpc::RoleConfig {
                    plane_id: 0,
                    endpoint_addr: modrpc::EndpointAddr { endpoint: 0 },
                    transport: transport.clone(),
                    topic_channels: modrpc::TopicChannels::SingleChannel { channel_id: 0 },
                    config: StreamSenderConfig { },
                    init: StreamInitState { },
                })
                .local(|cx| {
                    let builder = StreamSenderBuilder::new("stream_sender", cx.hooks.clone(), cx.stubs, cx.config, cx.init.clone());
                    stream_sender = Some(builder.create_handle(cx.setup));
                    builder.build(cx.setup);
                });

            let mut stream_receiver = None;
            let _ =
                rt.start_role::<StreamReceiverRole<String>>(modrpc::RoleConfig {
                    plane_id: 0,
                    endpoint_addr: modrpc::EndpointAddr { endpoint: 0 },
                    transport: transport,
                    topic_channels: modrpc::TopicChannels::SingleChannel { channel_id: 0 },
                    config: StreamReceiverConfig { },
                    init: StreamInitState { },
                })
                .local(|cx| {
                    let builder = StreamReceiverBuilder::new("stream_receiver", cx.hooks.clone(), cx.stubs, cx.config, cx.init.clone());
                    stream_receiver = Some(builder.create_handle(cx.setup));
                    builder.build(cx.setup);
                });

            let mut stream_sender = stream_sender.unwrap();
            let stream_receiver = stream_receiver.unwrap();

            stream_sender.send("asdf").await;

            // Passing None to subscribe will make it accept the first seq it sees as the next seq.
            let mut subscription = stream_receiver.subscribe(None);

            assert!(matches!(subscription.try_next(), Ok(None)));

            stream_sender.send("foo").await;
            stream_sender.send("bar").await;
            stream_sender.send("baz").await;

            assert_eq!(subscription.next().await.unwrap(), "foo");
            assert_eq!(subscription.next().await.unwrap(), "bar");
            assert_eq!(subscription.next().await.unwrap(), "baz");

            assert!(matches!(subscription.try_next(), Ok(None)));

            let subscriptions = stream_receiver.subscriptions.clone();
            assert_eq!(subscriptions.stream_states.borrow().len(), 1);
            drop(subscription);
            assert_eq!(subscriptions.stream_states.borrow().len(), 0);
        });
    }
}

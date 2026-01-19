use core::{
    cell::RefCell,
    marker::PhantomData,
};
use crate::{
    proto::{
        MultiStreamInitState,
        MultiStreamItem,
        MultiStreamItemLazy,
        MultiStreamId,
        MultiStreamReceiverConfig,
    },
    receive_stream::{ReceiveStream, StreamState},
};
use std::collections::HashMap;
use std::rc::Rc;
use modrpc::RoleSetup;

pub enum ReceiveMultiStreamNextError {
    Shutdown,
    DecodeItem(mproto::DecodeError),
}

pub struct ReceiveMultiStream<T> {
    stream_id: MultiStreamId,
    receive_stream: ReceiveStream,
    phantom: PhantomData<T>,
}

struct BrokerState {
    streams: RefCell<HashMap<u32, Rc<StreamState>>>,
}

pub struct MultiStreamReceiver<T> {
    hooks: crate::MultiStreamReceiverHooks<T>,
    broker_state: Rc<BrokerState>,
}

pub struct MultiStreamReceiverBuilder<T> {
    name: &'static str,
    hooks: crate::MultiStreamReceiverHooks<T>,
    stubs: crate::MultiStreamReceiverStubs<T>,

    broker_state: Rc<BrokerState>,
}

impl<T: mproto::Owned> MultiStreamReceiver<T> {
    pub fn new_stream(&self, stream_id: MultiStreamId, next_seq: Option<u64>) -> ReceiveMultiStream<T> {
        let receive_stream = ReceiveStream::new(next_seq);
        self.broker_state.streams.borrow_mut()
            .insert(stream_id.id, receive_stream.stream_state().clone());

        ReceiveMultiStream {
            stream_id,
            receive_stream,
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for MultiStreamReceiver<T> {
    fn clone(&self) -> Self {
        Self {
            hooks: self.hooks.clone(),
            broker_state: self.broker_state.clone(),
        }
    }
}

impl<T: mproto::Owned> ReceiveMultiStream<T> {
    pub fn id(&self) -> MultiStreamId {
        self.stream_id
    }

    pub async fn next(&mut self) -> Result<Option<T>, ReceiveMultiStreamNextError> {
        use mproto::BaseLen;

        let packet = self.receive_stream.next_packet().await;

        let stream_item: MultiStreamItemLazy<T> = mproto::decode_value(
            &packet.as_ref()[modrpc::TransmitPacket::BASE_LEN..]
        )
        .map_err(|e| ReceiveMultiStreamNextError::DecodeItem(e))?;

        let owned_result = stream_item.payload()
            .map_err(|e| ReceiveMultiStreamNextError::DecodeItem(e))?
            .map(|i| T::lazy_to_owned(i))
            .transpose()
            .map_err(|e| ReceiveMultiStreamNextError::DecodeItem(e))?;

        Ok(owned_result)
    }

    pub async fn next_lazy(&mut self)
        -> Result<mproto::LazyBuf<Option<T>, modrpc::Packet>, ReceiveMultiStreamNextError>
    {
        use mproto::BaseLen;

        let packet = self.receive_stream.next_packet().await;
        packet.advance(modrpc::TransmitPacket::BASE_LEN);

        let stream_item: mproto::LazyBuf<MultiStreamItem<T>, _> = mproto::LazyBuf::new(packet);

        Ok(stream_item.map(|s| s.payload().unwrap()))
    }

    pub fn with_try_next<R>(
        &mut self,
        f: impl FnOnce(Option<mproto::DecodeResult<Option<T::Lazy<'_>>>>) -> R,
    ) -> R {
        use mproto::BaseLen;

        let Some(packet) = self.receive_stream.try_next_packet() else {
            return f(None);
        };

        let stream_item =
            match mproto::decode_value::<MultiStreamItemLazy<T>>(
                &packet.as_ref()[modrpc::TransmitPacket::BASE_LEN..]
            ) {
                Ok(x) => x,
                Err(e) => {
                    return f(Some(Err(e)));
                },
            };

        let payload =
            match stream_item.payload() {
                Ok(x) => x,
                Err(e) => {
                    return f(Some(Err(e)));
                },
            };

        f(Some(Ok(payload)))
    }

    pub async fn with_next<'a, Fut, R>(
        &mut self,
        f: impl FnOnce(mproto::DecodeResult<Option<T::Lazy<'_>>>) -> Fut,
    ) -> Option<R>
        where Fut: std::future::Future<Output = R>
    {
        use mproto::BaseLen;

        let packet = self.receive_stream.next_packet().await;

        let stream_item =
            match mproto::decode_value::<MultiStreamItemLazy<T>>(
                &packet.as_ref()[modrpc::TransmitPacket::BASE_LEN..]
            ) {
                Ok(x) => x,
                Err(e) => {
                    return Some(f(Err(e)).await);
                },
            };

        let payload =
            match stream_item.payload() {
                Ok(x) => x,
                Err(e) => {
                    return Some(f(Err(e)).await);
                },
            };

        Some(f(Ok(payload)).await)
    }

    pub async fn with_next_sync<'a, R>(
        &mut self,
        f: impl FnOnce(mproto::DecodeResult<T::Lazy<'_>>) -> R,
    ) -> Option<R> {
        use mproto::BaseLen;

        let packet = self.receive_stream.next_packet().await;

        let stream_item =
            match mproto::decode_value::<MultiStreamItemLazy<T>>(
                &packet.as_ref()[modrpc::TransmitPacket::BASE_LEN..]
            ) {
                Ok(x) => x,
                Err(e) => {
                    return Some(f(Err(e)));
                }
            };

        let payload =
            match stream_item.payload() {
                Ok(Some(x)) => x,
                Ok(None) => {
                    // End of stream
                    return None;
                }
                Err(e) => {
                    return Some(f(Err(e)));
                }
            };

        Some(f(Ok(payload)))
    }

    pub async fn collect(&mut self) -> Result<Vec<T>, ReceiveMultiStreamNextError> {
        let mut collected = Vec::new();
        while let Some(item) = self.next().await? {
            collected.push(item);
        }
        Ok(collected)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MultiStreamTryCollectError<E: std::fmt::Debug> {
    #[error("failed to decode MultiStream item")]
    DecodeError(#[from] mproto::DecodeError),
    #[error("stream sender failed: {0:?}")]
    SenderError(E),
    #[error("plane is shutting down")]
    Shutdown,
}

impl<E: std::fmt::Debug> From<ReceiveMultiStreamNextError> for MultiStreamTryCollectError<E> {
    fn from(other: ReceiveMultiStreamNextError) -> Self {
        match other {
            ReceiveMultiStreamNextError::DecodeItem(e) =>
                MultiStreamTryCollectError::DecodeError(e),
            ReceiveMultiStreamNextError::Shutdown =>
                MultiStreamTryCollectError::Shutdown,
        }
    }
}

impl<T: mproto::Owned, E: mproto::Owned + std::fmt::Debug> ReceiveMultiStream<Result<T, E>> {
    pub async fn try_collect(&mut self) -> Result<Vec<T>, MultiStreamTryCollectError<E>> {
        let mut collected = Vec::new();
        while let Some(item) =
            self.next().await?
                .transpose()
                .map_err(|e| MultiStreamTryCollectError::SenderError(e))?
        {
            collected.push(item);
        }
        Ok(collected)
    }
}

impl<T: mproto::Owned> MultiStreamReceiverBuilder<T> {
    pub fn new(
        name: &'static str,
        hooks: crate::MultiStreamReceiverHooks<T>,
        stubs: crate::MultiStreamReceiverStubs<T>,
        _config: &MultiStreamReceiverConfig,
        _init: MultiStreamInitState,
    ) -> Self {
        Self {
            name, hooks, stubs,
            broker_state: Rc::new(BrokerState {
                streams: RefCell::new(HashMap::new()),
            }),
        }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> crate::MultiStreamReceiver<T> {
        crate::MultiStreamReceiver {
            hooks: self.hooks.clone(),
            broker_state: self.broker_state.clone(),
        }
    }

    pub fn build(
        self,
        setup: &RoleSetup,
    ) {
        use mproto::BaseLen;

        let broker_state = self.broker_state;
        self.stubs.item.inline_untyped(setup, move |_source, packet| {
            let stream_item_bytes = &packet[modrpc::TransmitPacket::BASE_LEN..];
            let (seq, stream_id, shutdown) = {
                let Ok(stream_item) =
                    mproto::decode_value::<MultiStreamItemLazy<T>>(stream_item_bytes)
                else {
                    return;
                };
                let Ok(seq) = stream_item.seq() else {
                    return;
                };
                let Ok(stream_id) = stream_item.stream_id().and_then(|r| r.id()) else {
                    return;
                };
                let Ok(payload) = stream_item.payload() else {
                    return;
                };
                (seq, stream_id, payload.is_none())
            };

            let Some(stream_state) = broker_state.streams.borrow().get(&stream_id).cloned() else {
                log::warn!("Unknown stream_id name={} stream_id={stream_id}", self.name);
                return;
            };

            let stream_is_done = stream_state.handle_item(seq, shutdown, packet.clone());
            if stream_is_done {
                log::debug!("MultiStreamReciever shutdown stream stream_id={stream_id} seq={seq}");
                broker_state.streams.borrow_mut().remove(&stream_id);
            }
        })
        .subscribe();
    }
}


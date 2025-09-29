use core::cell::Cell;
use crate::proto::{
    MultiStreamId,
    MultiStreamInitState,
    MultiStreamItem,
    MultiStreamItemGen,
    MultiStreamSenderConfig,
};
use modrpc::RoleSetup;

pub struct SendMultiStream<T> {
    stream_id: MultiStreamId,
    next_seq: Cell<u64>,
    item_tx: modrpc::EventTx<MultiStreamItem<T>>,
}

pub struct MultiStreamSender<T> {
    hooks: crate::MultiStreamSenderHooks<T>,
}

pub struct MultiStreamSenderBuilder<T> {
    hooks: crate::MultiStreamSenderHooks<T>,
}

impl<T: mproto::Owned> MultiStreamSenderBuilder<T> {
    pub fn new(
        _name: &'static str,
        hooks: crate::MultiStreamSenderHooks<T>,
        _stubs: crate::MultiStreamSenderStubs<T>,
        _config: &MultiStreamSenderConfig,
        _init: MultiStreamInitState,
    ) -> Self {
        Self { hooks }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> MultiStreamSender<T> {
        MultiStreamSender {
            hooks: self.hooks.clone(),
        }
    }

    pub fn build(
        self,
        _setup: &RoleSetup,
    ) {
    }
}

impl<T: mproto::Owned> MultiStreamSender<T> {
    pub fn new_stream(&self, stream_id: MultiStreamId) -> SendMultiStream<T> {
        SendMultiStream {
            stream_id,
            next_seq: Cell::new(0),
            item_tx: self.hooks.item.clone(),
        }
    }
}

impl<T> Clone for MultiStreamSender<T> {
    fn clone(&self) -> Self {
        Self {
            hooks: self.hooks.clone(),
        }
    }
}

impl<T: mproto::Owned> SendMultiStream<T> {
    pub fn stream_id(&self) -> MultiStreamId {
        self.stream_id.clone()
    }

    pub fn try_send(
        &self,
        input: impl mproto::Encode + mproto::Compatible<T>,
    ) -> bool {
        let seq = self.next_seq.replace(self.next_seq.get() + 1);

        self.item_tx.try_send(MultiStreamItemGen {
            stream_id: self.stream_id.clone(),
            seq,
            payload: Some(input),
        })
    }

    pub async fn send(
        &self,
        input: impl mproto::Encode + mproto::Compatible<T>,
    ) {
        let seq = self.next_seq.replace(self.next_seq.get() + 1);

        self.item_tx.send(MultiStreamItemGen {
            stream_id: self.stream_id.clone(),
            seq,
            payload: Some(input),
        })
        .await;
    }

    pub async fn end(self) {
        let seq = self.next_seq.replace(self.next_seq.get() + 1);

        self.item_tx.send(MultiStreamItemGen {
            stream_id: self.stream_id.clone(),
            seq,
            payload: None::<T>,
        })
        .await;
    }
}

// Helpers to play nice with type inference for the fairly common situation where the item type
// is a `Result`.
impl<O: mproto::Owned, E: mproto::Owned> SendMultiStream<Result<O, E>> {
    pub async fn send_ok(&mut self, input: impl mproto::Encode + mproto::Compatible<O>) {
        let seq = self.next_seq.replace(self.next_seq.get() + 1);

        self.item_tx.send(MultiStreamItemGen {
            stream_id: self.stream_id.clone(),
            seq,
            payload: Some(Ok::<_, E>(input)),
        })
        .await;
    }

    pub async fn send_err(&mut self, input: impl mproto::Encode + mproto::Compatible<E>) {
        let seq = self.next_seq.replace(self.next_seq.get() + 1);

        self.item_tx.send(MultiStreamItemGen {
            stream_id: self.stream_id.clone(),
            seq,
            payload: Some(Err::<O, _>(input)),
        })
        .await;
    }
}

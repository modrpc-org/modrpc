use core::cell::Cell;
use crate::proto::{StreamInitState, StreamItemGen, StreamSenderConfig};
use modrpc::RoleSetup;
use std::rc::Rc;

struct State<T> {
    hooks: crate::StreamSenderHooks<T>,
    next_seq: Cell<u64>,
}

#[derive(Clone)]
pub struct StreamSender<T> {
    state: Rc<State<T>>,
}

impl<T: mproto::Owned> StreamSender<T> {
    pub async fn send<U>(&mut self, payload: U)
        where U: mproto::Encode + mproto::Compatible<T>
    {
        let seq = self.state.next_seq.get();
        self.state.next_seq.set(seq + 1);

        self.state.hooks.item.send(StreamItemGen {
            seq,
            payload,
        })
        .await;
    }
}

pub struct StreamSenderBuilder<T> {
    state: Rc<State<T>>,
}

impl<T: mproto::Owned> StreamSenderBuilder<T> {
    pub fn new(
        _name: &'static str,
        hooks: crate::StreamSenderHooks<T>,
        _stubs: crate::StreamSenderStubs<T>,
        _config: &StreamSenderConfig,
        _init: StreamInitState,
    ) -> Self {
        let state = Rc::new(State {
            hooks,
            next_seq: Cell::new(0),
        });
        Self { state }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> crate::StreamSender<T> {
        crate::StreamSender { state: self.state.clone() }
    }

    pub fn build(
        self,
        _setup: &RoleSetup,
    ) {
    }
}


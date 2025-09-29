#![allow(unused_variables)]

use crate::interface::StreamInterface;
use crate::proto::{StreamInitState, StreamItem, StreamSenderConfig};
use modrpc::{EventTx, InterfaceRole, RoleSetup};

pub struct StreamSenderHooks<T> {
    pub item: EventTx<StreamItem<T>>,
    _phantom: std::marker::PhantomData<T>,
}

pub struct StreamSenderStubs<T> {
    _phantom: std::marker::PhantomData<T>,
}

pub struct StreamSenderRole<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: mproto::Owned> InterfaceRole for StreamSenderRole<T> {
    type Interface = StreamInterface<T>;
    type Config = StreamSenderConfig;
    type Init = StreamInitState;
    type Stubs = StreamSenderStubs<T>;
    type Hooks = StreamSenderHooks<T>;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {

        (
            Self::Stubs {
                _phantom: std::marker::PhantomData,
            },
            Self::Hooks {
                item: setup.event_tx(i.item),
                _phantom: std::marker::PhantomData,
            },
        )
    }
}

impl<T> Clone for StreamSenderHooks<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

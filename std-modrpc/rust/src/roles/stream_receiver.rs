#![allow(unused_variables)]

use crate::interface::StreamInterface;
use crate::proto::{StreamInitState, StreamItem, StreamReceiverConfig};
use modrpc::{EventRxBuilder, InterfaceRole, RoleSetup};

pub struct StreamReceiverHooks<T> {
    _phantom: std::marker::PhantomData<T>,
}

pub struct StreamReceiverStubs<T> {
    pub item: EventRxBuilder<StreamItem<T>>,
    _phantom: std::marker::PhantomData<T>,
}

pub struct StreamReceiverRole<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: mproto::Owned> InterfaceRole for StreamReceiverRole<T> {
    type Interface = StreamInterface<T>;
    type Config = StreamReceiverConfig;
    type Init = StreamInitState;
    type Stubs = StreamReceiverStubs<T>;
    type Hooks = StreamReceiverHooks<T>;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {

        (
            Self::Stubs {
                item: setup.event_rx(i.item),
                _phantom: std::marker::PhantomData,
            },
            Self::Hooks {
                _phantom: std::marker::PhantomData,
            },
        )
    }
}

impl<T> Clone for StreamReceiverHooks<T> {
    fn clone(&self) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

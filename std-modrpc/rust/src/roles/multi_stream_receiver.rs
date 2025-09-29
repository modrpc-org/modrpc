#![allow(unused_variables)]

use crate::interface::MultiStreamInterface;
use crate::proto::{MultiStreamInitState, MultiStreamItem, MultiStreamReceiverConfig};
use modrpc::{EventRxBuilder, InterfaceRole, RoleSetup};

pub struct MultiStreamReceiverHooks<T> {
    _phantom: std::marker::PhantomData<T>,
}

pub struct MultiStreamReceiverStubs<T> {
    pub item: EventRxBuilder<MultiStreamItem<T>>,
    _phantom: std::marker::PhantomData<T>,
}

pub struct MultiStreamReceiverRole<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: mproto::Owned> InterfaceRole for MultiStreamReceiverRole<T> {
    type Interface = MultiStreamInterface<T>;
    type Config = MultiStreamReceiverConfig;
    type Init = MultiStreamInitState;
    type Stubs = MultiStreamReceiverStubs<T>;
    type Hooks = MultiStreamReceiverHooks<T>;

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

impl<T> Clone for MultiStreamReceiverHooks<T> {
    fn clone(&self) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

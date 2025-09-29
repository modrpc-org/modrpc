#![allow(unused_variables)]

use crate::interface::MultiStreamInterface;
use crate::proto::{MultiStreamInitState, MultiStreamItem, MultiStreamSenderConfig};
use modrpc::{EventTx, InterfaceRole, RoleSetup};

pub struct MultiStreamSenderHooks<T> {
    pub item: EventTx<MultiStreamItem<T>>,
    _phantom: std::marker::PhantomData<T>,
}

pub struct MultiStreamSenderStubs<T> {
    _phantom: std::marker::PhantomData<T>,
}

pub struct MultiStreamSenderRole<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: mproto::Owned> InterfaceRole for MultiStreamSenderRole<T> {
    type Interface = MultiStreamInterface<T>;
    type Config = MultiStreamSenderConfig;
    type Init = MultiStreamInitState;
    type Stubs = MultiStreamSenderStubs<T>;
    type Hooks = MultiStreamSenderHooks<T>;

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

impl<T> Clone for MultiStreamSenderHooks<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

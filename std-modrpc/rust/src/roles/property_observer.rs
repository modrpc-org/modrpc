#![allow(unused_variables)]

use crate::interface::PropertyInterface;
use crate::proto::{PropertyInitState, PropertyObserverConfig, PropertyUpdate};
use modrpc::{EventRxBuilder, InterfaceRole, RoleSetup};

pub struct PropertyObserverHooks<T> {
    _phantom: std::marker::PhantomData<T>,
}

pub struct PropertyObserverStubs<T> {
    pub update: EventRxBuilder<PropertyUpdate<T>>,
    _phantom: std::marker::PhantomData<T>,
}

pub struct PropertyObserverRole<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: mproto::Owned> InterfaceRole for PropertyObserverRole<T> {
    type Interface = PropertyInterface<T>;
    type Config = PropertyObserverConfig;
    type Init = PropertyInitState<T>;
    type Stubs = PropertyObserverStubs<T>;
    type Hooks = PropertyObserverHooks<T>;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {

        (
            Self::Stubs {
                update: setup.event_rx(i.update),
                _phantom: std::marker::PhantomData,
            },
            Self::Hooks {
                _phantom: std::marker::PhantomData,
            },
        )
    }
}

impl<T> Clone for PropertyObserverHooks<T> {
    fn clone(&self) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

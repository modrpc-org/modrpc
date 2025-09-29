#![allow(unused_variables)]

use crate::interface::PropertyInterface;
use crate::proto::{PropertyInitState, PropertyOwnerConfig, PropertyUpdate};
use modrpc::{EventRxBuilder, EventTx, InterfaceRole, RoleSetup};

pub struct PropertyOwnerHooks<T> {
    pub update: EventTx<PropertyUpdate<T>>,
    _phantom: std::marker::PhantomData<T>,
}

pub struct PropertyOwnerStubs<T> {
    pub update: EventRxBuilder<PropertyUpdate<T>>,
    _phantom: std::marker::PhantomData<T>,
}

pub struct PropertyOwnerRole<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: mproto::Owned> InterfaceRole for PropertyOwnerRole<T> {
    type Interface = PropertyInterface<T>;
    type Config = PropertyOwnerConfig;
    type Init = PropertyInitState<T>;
    type Stubs = PropertyOwnerStubs<T>;
    type Hooks = PropertyOwnerHooks<T>;

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
                update: setup.event_tx(i.update),
                _phantom: std::marker::PhantomData,
            },
        )
    }
}

impl<T> Clone for PropertyOwnerHooks<T> {
    fn clone(&self) -> Self {
        Self {
            update: self.update.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

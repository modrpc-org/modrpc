#![allow(unused_variables)]

use crate::interface::MultiByteStreamInterface;
use crate::proto::{MultiByteStreamInitState, MultiByteStreamReceiverConfig};
use modrpc::{EventRxBuilder, InterfaceRole, RoleSetup};

pub struct MultiByteStreamReceiverHooks {}

pub struct MultiByteStreamReceiverStubs {
    pub blob: EventRxBuilder<()>,
}

pub struct MultiByteStreamReceiverRole {}

impl InterfaceRole for MultiByteStreamReceiverRole {
    type Interface = MultiByteStreamInterface;
    type Config = MultiByteStreamReceiverConfig;
    type Init = MultiByteStreamInitState;
    type Stubs = MultiByteStreamReceiverStubs;
    type Hooks = MultiByteStreamReceiverHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {

        (
            Self::Stubs {
                blob: setup.event_rx(i.blob),
            },
            Self::Hooks {},
        )
    }
}

impl Clone for MultiByteStreamReceiverHooks {
    fn clone(&self) -> Self {
        Self {}
    }
}

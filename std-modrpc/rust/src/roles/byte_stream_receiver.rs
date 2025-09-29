#![allow(unused_variables)]

use crate::interface::ByteStreamInterface;
use crate::proto::{ByteStreamInitState, ByteStreamReceiverConfig};
use modrpc::{EventRxBuilder, InterfaceRole, RoleSetup};

pub struct ByteStreamReceiverHooks {}

pub struct ByteStreamReceiverStubs {
    pub blob: EventRxBuilder<()>,
}

pub struct ByteStreamReceiverRole {}

impl InterfaceRole for ByteStreamReceiverRole {
    type Interface = ByteStreamInterface;
    type Config = ByteStreamReceiverConfig;
    type Init = ByteStreamInitState;
    type Stubs = ByteStreamReceiverStubs;
    type Hooks = ByteStreamReceiverHooks;

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

impl Clone for ByteStreamReceiverHooks {
    fn clone(&self) -> Self {
        Self {}
    }
}

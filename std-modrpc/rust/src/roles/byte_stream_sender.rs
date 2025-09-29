#![allow(unused_variables)]

use crate::interface::ByteStreamInterface;
use crate::proto::{ByteStreamInitState, ByteStreamSenderConfig};
use modrpc::{EventTx, InterfaceRole, RoleSetup};

pub struct ByteStreamSenderHooks {
    pub blob: EventTx<()>,
}

pub struct ByteStreamSenderStubs {}

pub struct ByteStreamSenderRole {}

impl InterfaceRole for ByteStreamSenderRole {
    type Interface = ByteStreamInterface;
    type Config = ByteStreamSenderConfig;
    type Init = ByteStreamInitState;
    type Stubs = ByteStreamSenderStubs;
    type Hooks = ByteStreamSenderHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {

        (
            Self::Stubs {},
            Self::Hooks {
                blob: setup.event_tx(i.blob),
            },
        )
    }
}

impl Clone for ByteStreamSenderHooks {
    fn clone(&self) -> Self {
        Self {
            blob: self.blob.clone(),
        }
    }
}

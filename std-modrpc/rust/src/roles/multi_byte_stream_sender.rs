#![allow(unused_variables)]

use crate::interface::MultiByteStreamInterface;
use crate::proto::{MultiByteStreamInitState, MultiByteStreamSenderConfig};
use modrpc::{EventTx, InterfaceRole, RoleSetup};

pub struct MultiByteStreamSenderHooks {
    pub blob: EventTx<()>,
}

pub struct MultiByteStreamSenderStubs {}

pub struct MultiByteStreamSenderRole {}

impl InterfaceRole for MultiByteStreamSenderRole {
    type Interface = MultiByteStreamInterface;
    type Config = MultiByteStreamSenderConfig;
    type Init = MultiByteStreamInitState;
    type Stubs = MultiByteStreamSenderStubs;
    type Hooks = MultiByteStreamSenderHooks;

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

impl Clone for MultiByteStreamSenderHooks {
    fn clone(&self) -> Self {
        Self {
            blob: self.blob.clone(),
        }
    }
}

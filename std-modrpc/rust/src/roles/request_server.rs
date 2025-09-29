#![allow(unused_variables)]

use crate::interface::RequestInterface;
use crate::proto::{Request, RequestInitState, RequestServerConfig, Response};
use modrpc::{EventRxBuilder, EventTx, InterfaceRole, RoleSetup};

pub struct RequestServerHooks<Req, Resp> {
    pub response: EventTx<Response<Resp>>,
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

pub struct RequestServerStubs<Req, Resp> {
    pub request: EventRxBuilder<Request<Req>>,
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

pub struct RequestServerRole<Req, Resp> {
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

impl<Req: mproto::Owned, Resp: mproto::Owned> InterfaceRole for RequestServerRole<Req, Resp> {
    type Interface = RequestInterface<Req, Resp>;
    type Config = RequestServerConfig;
    type Init = RequestInitState;
    type Stubs = RequestServerStubs<Req, Resp>;
    type Hooks = RequestServerHooks<Req, Resp>;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {

        (
            Self::Stubs {
                request: setup.event_rx(i.request),
                _phantom: std::marker::PhantomData,
            },
            Self::Hooks {
                response: setup.event_tx(i.response),
                _phantom: std::marker::PhantomData,
            },
        )
    }
}

impl<Req, Resp> Clone for RequestServerHooks<Req, Resp> {
    fn clone(&self) -> Self {
        Self {
            response: self.response.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

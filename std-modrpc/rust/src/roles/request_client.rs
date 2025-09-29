#![allow(unused_variables)]

use crate::interface::RequestInterface;
use crate::proto::{Request, RequestClientConfig, RequestInitState, Response};
use modrpc::{EventRxBuilder, EventTx, InterfaceRole, RoleSetup};

pub struct RequestClientHooks<Req, Resp> {
    pub request: EventTx<Request<Req>>,
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

pub struct RequestClientStubs<Req, Resp> {
    pub request: EventRxBuilder<Request<Req>>,
    pub response: EventRxBuilder<Response<Resp>>,
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

pub struct RequestClientRole<Req, Resp> {
    _phantom: std::marker::PhantomData<(Req, Resp)>,
}

impl<Req: mproto::Owned, Resp: mproto::Owned> InterfaceRole for RequestClientRole<Req, Resp> {
    type Interface = RequestInterface<Req, Resp>;
    type Config = RequestClientConfig;
    type Init = RequestInitState;
    type Stubs = RequestClientStubs<Req, Resp>;
    type Hooks = RequestClientHooks<Req, Resp>;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {

        (
            Self::Stubs {
                request: setup.event_rx(i.request),
                response: setup.event_rx(i.response),
                _phantom: std::marker::PhantomData,
            },
            Self::Hooks {
                request: setup.event_tx(i.request),
                _phantom: std::marker::PhantomData,
            },
        )
    }
}

impl<Req, Resp> Clone for RequestClientHooks<Req, Resp> {
    fn clone(&self) -> Self {
        Self {
            request: self.request.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

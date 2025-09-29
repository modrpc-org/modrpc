use crate::proto::{
    RequestLazy,
    RequestInitState,
    RequestServerConfig,
    Response,
    ResponseGen,
};
use modrpc::RoleSetup;

use crate::request_tracker::{RequestTracker, get_request_tracker};

pub struct RequestServer<Req, Resp> {
    name: &'static str,
    worker_id: u16,
    hooks: crate::RequestServerHooks<Req, Resp>,
    tracker: RequestTracker,
    tracing_enabled: bool,
}

pub struct RequestServerBuilder<Req, Resp> {
    pub name: &'static str,
    pub hooks: crate::RequestServerHooks<Req, Resp>,
    pub stubs: crate::RequestServerStubs<Req, Resp>,
    pub init: RequestInitState,
}

impl<
    Req: mproto::Owned,
    Resp: mproto::Owned,
> RequestServerBuilder<Req, Resp> {
    pub fn new(
        name: &'static str,
        hooks: crate::RequestServerHooks<Req, Resp>,
        stubs: crate::RequestServerStubs<Req, Resp>,
        _config: &RequestServerConfig,
        init: RequestInitState,
    ) -> Self {
        Self { name, hooks, stubs, init }
    }

    pub fn create_handle(&self, setup: &RoleSetup) -> RequestServer<Req, Resp> {
        let worker_id = setup.worker_id();
        let tracker = get_request_tracker(setup);

        RequestServer {
            name: self.name,
            worker_id,
            hooks: self.hooks.clone(),
            tracker,
            tracing_enabled: true,
        }
    }

    pub fn build_shared(self, /* todo */) {
        // TODO insert self.hooks.response into the shared state's map of plane_id -> response_tx
        // we'll need to change modrpc::PacketProcessor to lookup handlers based on
        // (infra_id, topic) instead of (plane_id, topic) (and add infra_id to TransmitPacket).
        // In PacketProcessor, if infra_id is non-zero, we'll look up the handler based on
        // infra_id. Otherwise we'll look up based on plane_id.
    }

    pub fn build_replier(
        self,
        setup: &RoleSetup,
        mut handler: impl AsyncFnMut(RequestContext<Resp>, Req::Lazy<'_>) + 'static,
    ) {
        let mut response_tx: modrpc::EventTx<Response<Resp>> = self.hooks.response;
        self.stubs.request
            .queued(setup, async move |source: modrpc::EndpointAddr, request: RequestLazy<Req>| {
                let Ok(request_id) = request.request_id() else { return; };
                let Ok(requester_worker) = request.worker() else { return; };
                let Ok(payload) = request.payload() else { return; };

                handler(
                    RequestContext {
                        source,
                        reply: ResponseSender {
                            response_event_sender: &mut response_tx,
                            request_id,
                            source: source,
                            requester_worker,
                        },
                    },
                    payload,
                )
                .await;
            })
            .load_balance();
    }

    pub fn build(
        self,
        setup: &RoleSetup,
        mut handler: impl AsyncFnMut(modrpc::EndpointAddr, Req::Lazy<'_>) -> Resp + 'static,
    ) {
        let response_tx: modrpc::EventTx<Response<Resp>> = self.hooks.response;
        self.stubs.request.queued(
            setup,
            async move |source: modrpc::EndpointAddr, request: RequestLazy<Req>| {
                let Ok(request_id) = request.request_id() else { return; };
                let Ok(requester_worker) = request.worker() else { return; };
                let Ok(request_payload) = request.payload() else { return; };

                let response = handler(source, request_payload).await;
                response_tx.send(Response {
                    request_id,
                    requester: source.endpoint,
                    requester_worker,
                    payload: response,
                })
                .await;
            },
        )
        .load_balance();
    }

    pub fn build_proxied(self, setup: &RoleSetup) {
        self.stubs.request.proxy_load_balance(setup);
    }
}

impl<Req, Resp> Clone for RequestServer<Req, Resp> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            worker_id: self.worker_id,
            hooks: self.hooks.clone(),
            tracker: self.tracker.clone(),
            tracing_enabled: self.tracing_enabled,
        }
    }
}

pub struct RequestContext<'a, R> {
    pub source: modrpc::EndpointAddr,
    pub reply: ResponseSender<'a, R>,
}

pub struct ResponseSender<'a, T> {
    pub response_event_sender: &'a mut modrpc::EventTx<Response<T>>,
    pub request_id: u32,
    pub source: modrpc::EndpointAddr,
    pub requester_worker: u16,
}

impl<T: mproto::Owned> ResponseSender<'_, T> {
    #[inline]
    pub async fn send(&mut self, response: impl mproto::Encode + mproto::Compatible<T>) {
        self.response_event_sender.send(ResponseGen {
            request_id: self.request_id,
            requester: self.source.endpoint,
            requester_worker: self.requester_worker,
            payload: response,
        }).await;
    }
}

// Helpers to play nice with type inference for the very common situation where the response type
// is a `Result`.
impl<O: mproto::Owned, E: mproto::Owned> ResponseSender<'_, Result<O, E>> {
    #[inline]
    pub async fn send_ok(&mut self, response: impl mproto::Encode + mproto::Compatible<O>) {
        self.response_event_sender.send(ResponseGen {
            request_id: self.request_id,
            requester: self.source.endpoint,
            requester_worker: self.requester_worker,
            payload: Ok::<_, E>(response),
        }).await;
    }

    #[inline]
    pub async fn send_err(&mut self, response: impl mproto::Encode + mproto::Compatible<E>) {
        self.response_event_sender.send(ResponseGen {
            request_id: self.request_id,
            requester: self.source.endpoint,
            requester_worker: self.requester_worker,
            payload: Err::<O, _>(response),
        }).await;
    }
}

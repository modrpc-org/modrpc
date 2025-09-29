use modrpc::RoleSetup;

use crate::{
    proto::{
        RequestClientConfig,
        RequestGen,
        RequestLazy,
        RequestInitState,
        Response,
        ResponseLazy,
    },
    request_tracker::{RequestTracker, get_request_tracker},
};

pub use sealed::ResponseWaiter;

mod sealed {
    use crate::{
        proto::Response,
        request_tracker::PendingRequestSubscription,
    };

    pub struct ResponseWaiter<Resp> {
        pending_request: PendingRequestSubscription,
        _phantom: core::marker::PhantomData<Resp>,
    }

    impl<Resp: for<'d> mproto::Decode<'d>> ResponseWaiter<Resp> {
        pub async fn wait(self) -> mproto::DecodeResult<Resp> {
            let packet_header_len = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;
            let response_packet = self.pending_request.wait().await;

            let response = mproto::decode_value(&response_packet.as_ref()[packet_header_len..])?;
            let response: Response<Resp> = response;

            Ok(response.payload)
        }
    }

    pub fn wait_response_then_decode<Resp>(pending_request: PendingRequestSubscription)
        -> ResponseWaiter<Resp>
        where Resp: for<'d> mproto::Decode<'d>
    {
        ResponseWaiter {
            pending_request,
            _phantom: core::marker::PhantomData,
        }
    }

    // One day
    /*pub type ResponseWaiter<Resp: for<'d> mproto::Decode<'d>>
        = impl Future<Output = mproto::DecodeResult<Resp>>;

    #[define_opaque(ResponseWaiter)]
    pub fn wait_response_then_decode<Resp>(pending_request: PendingRequestSubscription)
        -> ResponseWaiter<Resp>
        where Resp: for<'d> mproto::Decode<'d>
    {
        async move {
            let packet_header_len = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;
            let response_packet = pending_request.wait().await;

            let response = mproto::decode_value(&response_packet.as_ref()[packet_header_len..])?;
            let response: Response<Resp> = response;

            Ok(response.payload)
        }
    }*/
}

pub struct RequestClient<Req, Resp> {
    name: &'static str,
    rt: modrpc::RuntimeHandle,
    worker_id: u16,
    hooks: crate::RequestClientHooks<Req, Resp>,
    tracker: RequestTracker,
    spawner: modrpc::RoleSpawner,
}

impl<
    Req: mproto::Owned,
    Resp: mproto::Owned,
> RequestClient<Req, Resp> {
    pub async fn call<LikeReq>(&self, payload: LikeReq) -> Resp
        where LikeReq: mproto::Compatible<Req>
    {
        let pending_request = self.tracker.client_start_request();
        self.hooks.request.send(RequestGen::<LikeReq> {
            worker: self.worker_id,
            request_id: pending_request.request_id(),
            payload,
        })
        .await;

        let response_buf = pending_request.await;
        let header_len = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;
        let response: Response<Resp>
            = mproto::decode_value(&response_buf.as_ref()[header_len..]).unwrap();

        response.payload
    }

    pub fn subscribe(
        &self,
        request_subscription:
            impl AsyncFnMut(
                modrpc::EndpointAddr,
                Req::Lazy<'_>,
                ResponseWaiter<Resp>,
            ) + Clone + 'static,
    )
    where
    {
        // Lazily create the inter-worker topic subscription
        let local_worker_context = self.rt.local_worker_context()
            .expect("modrpc::RequestClient::subscribe local worker context");
        modrpc::add_topic_subscription(
            local_worker_context,
            "todo",
            self.hooks.request.plane_id(),
            self.hooks.request.topic(),
        );

        let spawner = self.spawner.clone();
        self.tracker.subscribe(
            self.hooks.request.plane_id(),
            self.hooks.request.topic(),
            Box::new(move |request_packet: modrpc::Packet, pending_request| {
                let mut request_subscription = request_subscription.clone();
                spawner.spawn(async move {
                    let header_len = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;

                    let Ok(header) =
                        mproto::decode_value(&request_packet.as_ref()[..header_len])
                    else { return; };
                    let header: modrpc::TransmitPacket = header;

                    let Ok(request) =
                        mproto::decode_value(&request_packet.as_ref()[header_len..])
                    else { return; };
                    let request: RequestLazy<Req> = request;

                    let Ok(request_payload) = request.payload() else { return; };

                    request_subscription(
                        header.source,
                        request_payload,
                        sealed::wait_response_then_decode(pending_request),
                    )
                    .await;
                });
            }),
        );
    }
}

pub struct RequestClientBuilder<Req, Resp> {
    pub name: &'static str,
    pub hooks: crate::RequestClientHooks<Req, Resp>,
    pub stubs: crate::RequestClientStubs<Req, Resp>,
    pub init: RequestInitState,
}

impl<
    Req: mproto::Owned,
    Resp: mproto::Owned,
> RequestClientBuilder<Req, Resp> {
    pub fn new(
        name: &'static str,
        hooks: crate::RequestClientHooks<Req, Resp>,
        stubs: crate::RequestClientStubs<Req, Resp>,
        _config: &RequestClientConfig,
        init: RequestInitState,
    ) -> Self {
        Self { name, hooks, stubs, init }
    }

    pub fn create_handle(&self, setup: &RoleSetup) -> RequestClient<Req, Resp> {
        let worker_id = setup.worker_id();
        let tracker = get_request_tracker(setup);

        RequestClient {
            name: self.name,
            rt: setup.worker_context().rt().clone(),
            worker_id,
            hooks: self.hooks.clone(),
            tracker,
            spawner: setup.role_spawner().clone(),
        }
    }

    pub fn build(
        self,
        setup: &RoleSetup,
    ) {
        let local_addr = setup.endpoint_addr().endpoint;
        let local_worker_id = setup.worker_id();
        let plane_id = setup.plane_id();

        // The .local() request/response handlers below are actually subscriptions, but we defer
        // creating the inter-worker topic subscription until the user actually creates a
        // subscription on this request because the inter-worker subscription is relatively
        // expensive.

        let tracker = get_request_tracker(setup);
        let request_topic = self.hooks.request.topic();
        self.stubs.request
            .inline_untyped(setup, move |source, packet| {
                tracker.handle_request::<Req>(plane_id, request_topic, source.endpoint, packet.clone());
            })
            .local();

        let tracker = get_request_tracker(setup);
        self.stubs.response.clone()
            .inline_untyped(setup, move |_source, packet| {
                tracker.handle_response::<Resp>(plane_id, local_addr, local_worker_id, packet.clone());
            })
            .local();

        self.stubs.response.clone()
            .route_to_worker(setup, move |_source, packet| {
                let packet_header_len = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;
                let Ok(response) =
                    mproto::decode_value::<ResponseLazy<Resp>>(&packet.as_ref()[packet_header_len..])
                else {
                    return None;
                };
                let Ok(requester) = response.requester() else {
                    return None;
                };
                let Ok(requester_worker) = response.requester_worker() else {
                    return None;
                };

                probius::trace_label("route_to_worker");
                probius::trace_branch(|| {
                    if requester == local_addr && requester_worker != local_worker_id {
                        probius::trace_metric("redirect", 1);
                        // This response is for a locally generated request at a different worker -
                        // route to the correct worker.
                        Some(modrpc::WorkerId(requester_worker))
                    } else {
                        probius::trace_metric("no-redirect", 1);
                        None
                    }
                })
            });
    }
}

impl<Req, Resp> Clone for RequestClient<Req, Resp> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            rt: self.rt.clone(),
            worker_id: self.worker_id,
            hooks: self.hooks.clone(),
            tracker: self.tracker.clone(),
            spawner: self.spawner.clone(),
        }
    }
}


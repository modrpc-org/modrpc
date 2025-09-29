#![allow(unused_variables)]

use crate::interface::P2pBenchmarkInterface;
use crate::proto::{P2pBenchmarkClientConfig, P2pBenchmarkInitState};
use modrpc::{InterfaceRole, RoleSetup};
use std_modrpc::{RequestClient, RequestClientBuilder, RequestClientConfig, RequestClientRole, RequestInitState};

pub struct P2pBenchmarkClientHooks {
    pub test_request: RequestClient<u64, Result<u64, ()>>,
}

pub struct P2pBenchmarkClientStubs {}

pub struct P2pBenchmarkClientRole {}

impl InterfaceRole for P2pBenchmarkClientRole {
    type Interface = P2pBenchmarkInterface;
    type Config = P2pBenchmarkClientConfig;
    type Init = P2pBenchmarkInitState;
    type Stubs = P2pBenchmarkClientStubs;
    type Hooks = P2pBenchmarkClientHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {
        setup.push_object_path("test_request");
        let (test_request_stubs, test_request_hooks) =
            RequestClientRole::setup_worker(
                &i.test_request, setup, &RequestClientConfig { }, &RequestInitState { },
            );
        let test_request_builder = RequestClientBuilder::new(
            "p2p_benchmark_client.test_request",
            test_request_hooks,
            test_request_stubs,
            &RequestClientConfig { },
            RequestInitState { }.clone(),
        );
        let test_request = test_request_builder.create_handle(setup);
        test_request_builder.build(setup);
        setup.pop_object_path();

        (
            Self::Stubs {},
            Self::Hooks {
                test_request,
            },
        )
    }
}

impl Clone for P2pBenchmarkClientHooks {
    fn clone(&self) -> Self {
        Self {
            test_request: self.test_request.clone(),
        }
    }
}

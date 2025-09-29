#![allow(unused_variables)]

use crate::interface::P2pBenchmarkInterface;
use crate::proto::{P2pBenchmarkInitState, P2pBenchmarkServerConfig};
use modrpc::{InterfaceRole, RoleSetup};
use std_modrpc::{RequestInitState, RequestServer, RequestServerBuilder, RequestServerConfig, RequestServerRole};

pub struct P2pBenchmarkServerHooks {
    pub test_request: RequestServer<u64, Result<u64, ()>>,
}

pub struct P2pBenchmarkServerStubs {
    pub test_request: RequestServerBuilder<u64, Result<u64, ()>>,
}

pub struct P2pBenchmarkServerRole {}

impl InterfaceRole for P2pBenchmarkServerRole {
    type Interface = P2pBenchmarkInterface;
    type Config = P2pBenchmarkServerConfig;
    type Init = P2pBenchmarkInitState;
    type Stubs = P2pBenchmarkServerStubs;
    type Hooks = P2pBenchmarkServerHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {
        setup.push_object_path("test_request");
        let (test_request_stubs, test_request_hooks) =
            RequestServerRole::setup_worker(
                &i.test_request, setup, &RequestServerConfig { }, &RequestInitState { },
            );
        let test_request_builder = RequestServerBuilder::new(
            "p2p_benchmark_server.test_request",
            test_request_hooks,
            test_request_stubs,
            &RequestServerConfig { },
            RequestInitState { }.clone(),
        );
        let test_request = test_request_builder.create_handle(setup);
        setup.pop_object_path();

        (
            Self::Stubs {
                test_request: test_request_builder,
            },
            Self::Hooks {
                test_request,
            },
        )
    }
}

impl Clone for P2pBenchmarkServerHooks {
    fn clone(&self) -> Self {
        Self {
            test_request: self.test_request.clone(),
        }
    }
}

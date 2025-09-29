use modrpc::{InterfaceBuilder, InterfaceSchema};
use std_modrpc::RequestInterface;

pub struct P2pBenchmarkInterface {
    pub test_request: RequestInterface<u64, Result<u64, ()>>,
}

impl InterfaceSchema for P2pBenchmarkInterface {
    fn new(ib: &mut InterfaceBuilder) -> Self {
        Self {
            test_request: RequestInterface::new(ib),
        }
    }
}

use modrpc::{InterfaceBuilder, InterfaceSchema};
use std_modrpc::{PropertyInterface, RequestInterface};

pub struct FooInterface {
    pub foo_the_bar: RequestInterface<u32, Result<u64, String>>,
    pub bar_the_foo: RequestInterface<String, Result<String, String>>,
    pub fooness: PropertyInterface<u64>,
}

impl InterfaceSchema for FooInterface {
    fn new(ib: &mut InterfaceBuilder) -> Self {
        Self {
            foo_the_bar: RequestInterface::new(ib),
            bar_the_foo: RequestInterface::new(ib),
            fooness: PropertyInterface::new(ib),
        }
    }
}

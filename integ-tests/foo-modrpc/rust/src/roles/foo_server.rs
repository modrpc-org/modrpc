#![allow(unused_variables)]

use crate::interface::FooInterface;
use crate::proto::{FooInitState, FooServerConfig};
use modrpc::{InterfaceRole, RoleSetup};
use std_modrpc::{PropertyOwner, PropertyOwnerBuilder, PropertyOwnerConfig, PropertyOwnerRole, RequestClient, RequestClientBuilder, RequestClientConfig, RequestClientRole, RequestInitState};

pub struct FooServerHooks {
    pub foo_the_bar: RequestClient<u32, Result<u64, String>>,
    pub bar_the_foo: RequestClient<String, Result<String, String>>,
    pub fooness: PropertyOwner<u64>,
}

pub struct FooServerStubs {
    pub fooness: PropertyOwnerBuilder<u64>,
}

pub struct FooServerRole {}

impl InterfaceRole for FooServerRole {
    type Interface = FooInterface;
    type Config = FooServerConfig;
    type Init = FooInitState;
    type Stubs = FooServerStubs;
    type Hooks = FooServerHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {
        setup.push_object_path("foo_the_bar");
        let (foo_the_bar_stubs, foo_the_bar_hooks) =
            RequestClientRole::setup_worker(
                &i.foo_the_bar, setup, &RequestClientConfig { }, &RequestInitState { },
            );
        let foo_the_bar_builder = RequestClientBuilder::new(
            "foo_server.foo_the_bar",
            foo_the_bar_hooks,
            foo_the_bar_stubs,
            &RequestClientConfig { },
            RequestInitState { }.clone(),
        );
        let foo_the_bar = foo_the_bar_builder.create_handle(setup);
        foo_the_bar_builder.build(setup);
        setup.pop_object_path();
        setup.push_object_path("bar_the_foo");
        let (bar_the_foo_stubs, bar_the_foo_hooks) =
            RequestClientRole::setup_worker(
                &i.bar_the_foo, setup, &RequestClientConfig { }, &RequestInitState { },
            );
        let bar_the_foo_builder = RequestClientBuilder::new(
            "foo_server.bar_the_foo",
            bar_the_foo_hooks,
            bar_the_foo_stubs,
            &RequestClientConfig { },
            RequestInitState { }.clone(),
        );
        let bar_the_foo = bar_the_foo_builder.create_handle(setup);
        bar_the_foo_builder.build(setup);
        setup.pop_object_path();
        setup.push_object_path("fooness");
        let (fooness_stubs, fooness_hooks) =
            PropertyOwnerRole::setup_worker(
                &i.fooness, setup, &PropertyOwnerConfig { }, &init.fooness,
            );
        let fooness_builder = PropertyOwnerBuilder::new(
            "foo_server.fooness",
            fooness_hooks,
            fooness_stubs,
            &PropertyOwnerConfig { },
            init.fooness.clone(),
        );
        let fooness = fooness_builder.create_handle(setup);
        setup.pop_object_path();

        (
            Self::Stubs {
                fooness: fooness_builder,
            },
            Self::Hooks {
                foo_the_bar,
                bar_the_foo,
                fooness,
            },
        )
    }
}

impl Clone for FooServerHooks {
    fn clone(&self) -> Self {
        Self {
            foo_the_bar: self.foo_the_bar.clone(),
            bar_the_foo: self.bar_the_foo.clone(),
            fooness: self.fooness.clone(),
        }
    }
}

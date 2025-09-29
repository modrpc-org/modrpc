#![allow(unused_variables)]

use crate::interface::FooInterface;
use crate::proto::{FooClientConfig, FooInitState};
use modrpc::{InterfaceRole, RoleSetup};
use std_modrpc::{PropertyObserver, PropertyObserverBuilder, PropertyObserverConfig, PropertyObserverRole, RequestInitState, RequestServer, RequestServerBuilder, RequestServerConfig, RequestServerRole};

pub struct FooClientHooks {
    pub foo_the_bar: RequestServer<u32, Result<u64, String>>,
    pub bar_the_foo: RequestServer<String, Result<String, String>>,
    pub fooness: PropertyObserver<u64>,
}

pub struct FooClientStubs {
    pub foo_the_bar: RequestServerBuilder<u32, Result<u64, String>>,
    pub bar_the_foo: RequestServerBuilder<String, Result<String, String>>,
}

pub struct FooClientRole {}

impl InterfaceRole for FooClientRole {
    type Interface = FooInterface;
    type Config = FooClientConfig;
    type Init = FooInitState;
    type Stubs = FooClientStubs;
    type Hooks = FooClientHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        config: &Self::Config,
        init: &Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {
        setup.push_object_path("foo_the_bar");
        let (foo_the_bar_stubs, foo_the_bar_hooks) =
            RequestServerRole::setup_worker(
                &i.foo_the_bar, setup, &RequestServerConfig { }, &RequestInitState { },
            );
        let foo_the_bar_builder = RequestServerBuilder::new(
            "foo_client.foo_the_bar",
            foo_the_bar_hooks,
            foo_the_bar_stubs,
            &RequestServerConfig { },
            RequestInitState { }.clone(),
        );
        let foo_the_bar = foo_the_bar_builder.create_handle(setup);
        setup.pop_object_path();
        setup.push_object_path("bar_the_foo");
        let (bar_the_foo_stubs, bar_the_foo_hooks) =
            RequestServerRole::setup_worker(
                &i.bar_the_foo, setup, &RequestServerConfig { }, &RequestInitState { },
            );
        let bar_the_foo_builder = RequestServerBuilder::new(
            "foo_client.bar_the_foo",
            bar_the_foo_hooks,
            bar_the_foo_stubs,
            &RequestServerConfig { },
            RequestInitState { }.clone(),
        );
        let bar_the_foo = bar_the_foo_builder.create_handle(setup);
        setup.pop_object_path();
        setup.push_object_path("fooness");
        let (fooness_stubs, fooness_hooks) =
            PropertyObserverRole::setup_worker(
                &i.fooness, setup, &PropertyObserverConfig { }, &init.fooness,
            );
        let fooness_builder = PropertyObserverBuilder::new(
            "foo_client.fooness",
            fooness_hooks,
            fooness_stubs,
            &PropertyObserverConfig { },
            init.fooness.clone(),
        );
        let fooness = fooness_builder.create_handle(setup);
        fooness_builder.build(setup);
        setup.pop_object_path();

        (
            Self::Stubs {
                foo_the_bar: foo_the_bar_builder,
                bar_the_foo: bar_the_foo_builder,
            },
            Self::Hooks {
                foo_the_bar,
                bar_the_foo,
                fooness,
            },
        )
    }
}

impl Clone for FooClientHooks {
    fn clone(&self) -> Self {
        Self {
            foo_the_bar: self.foo_the_bar.clone(),
            bar_the_foo: self.bar_the_foo.clone(),
            fooness: self.fooness.clone(),
        }
    }
}

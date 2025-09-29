use crate::interface::FooInterface;
use crate::proto::FooInitState;
use modrpc::{RoleSetup, InterfaceRole};

pub struct FooClientHooks {
    pub foo_the_bar: crate::RequestServer<u32, Result<(), String>>,
    _phantom: std::marker::PhantomData<()>,
}

pub struct FooClientStubs {
    pub foo_the_bar: crate::RequestServerBuilder<u32, Result<(), String>>,
    pub _phantom: std::marker::PhantomData<()>,
}

pub struct FooClientRole {
    _phantom: std::marker::PhantomData<()>,
}

impl InterfaceRole for FooClientRole {
    type Interface = FooInterface;
    type Init = FooInitState;
    type Stubs = FooClientStubs;
    type Hooks = FooClientHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        init: Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {
        let (foo_the_bar_stubs, foo_the_bar_hooks) =
            crate::RequestServerRole::setup_worker(
                &i.foo_the_bar, setup, init.foo_the_bar,
            );
        let foo_the_bar_builder = crate::RequestServerBuilder {
            hooks: foo_the_bar_hooks,
            stubs: foo_the_bar_stubs,
        };
        let foo_the_bar = foo_the_bar_builder.create_handle(setup);

        (
            Self::Stubs {
                foo_the_bar: foo_the_bar_builder,
                _phantom: std::marker::PhantomData,
            },
            Self::Hooks {
                foo_the_bar,
                _phantom: std::marker::PhantomData,
            },
        )
    }

}

impl Clone for FooClientHooks {
    fn clone(&self) -> Self {
        Self {
            foo_the_bar: self.foo_the_bar.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

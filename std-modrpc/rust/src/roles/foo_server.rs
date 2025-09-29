use crate::interface::FooInterface;
use crate::proto::FooInitState;
use modrpc::{RoleSetup, InterfaceRole};

pub struct FooServerHooks {
    pub foo_the_bar: crate::RequestClient<u32, Result<(), String>>,
    _phantom: std::marker::PhantomData<()>,
}

pub struct FooServerStubs {
    pub _phantom: std::marker::PhantomData<()>,
}

pub struct FooServerRole {
    _phantom: std::marker::PhantomData<()>,
}

impl InterfaceRole for FooServerRole {
    type Interface = FooInterface;
    type Init = FooInitState;
    type Stubs = FooServerStubs;
    type Hooks = FooServerHooks;

    fn setup_worker(
        i: &Self::Interface,
        setup: &mut RoleSetup,
        init: Self::Init,
    ) -> (Self::Stubs, Self::Hooks) {
        let (foo_the_bar_stubs, foo_the_bar_hooks) =
            crate::RequestClientRole::setup_worker(
                &i.foo_the_bar, setup, init.foo_the_bar,
            );
        let foo_the_bar_builder = crate::RequestClientBuilder {
            hooks: foo_the_bar_hooks,
            stubs: foo_the_bar_stubs,
        };
        let foo_the_bar = foo_the_bar_builder.create_handle(setup);
        foo_the_bar_builder.build(setup);

        (
            Self::Stubs {
                _phantom: std::marker::PhantomData,
            },
            Self::Hooks {
                foo_the_bar,
                _phantom: std::marker::PhantomData,
            },
        )
    }

}

impl Clone for FooServerHooks {
    fn clone(&self) -> Self {
        Self {
            foo_the_bar: self.foo_the_bar.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

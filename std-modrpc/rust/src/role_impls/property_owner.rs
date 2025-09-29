use std::cell::RefCell;
use std::rc::Rc;

use crate::proto::{PropertyInitState, PropertyOwnerConfig};
use modrpc::RoleSetup;

struct State<T> {
    hooks: crate::PropertyOwnerHooks<T>,
    value: RefCell<T>,
}

#[derive(Clone)]
pub struct PropertyOwner<T> {
    state: Rc<State<T>>,
}

impl<
    T: mproto::Owned,
> PropertyOwner<T> {
    pub async fn update(&mut self, new_value: T) {
        self.state.hooks.update.send(crate::PropertyUpdateGen { new_value: &new_value }).await;
        *self.state.value.borrow_mut() = new_value;
    }

    pub fn with_value<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.state.value.borrow())
    }
}

impl<T: Copy> PropertyOwner<T> {
    pub fn value(&self) -> T {
        *self.state.value.borrow()
    }
}

impl<T: Clone> PropertyOwner<T> {
    pub fn value_cloned(&self) -> T {
        let value_cell: &RefCell<T> = &self.state.value;
        value_cell.clone().into_inner()
    }
}

pub struct PropertyOwnerBuilder<T> {
    stubs: crate::PropertyOwnerStubs<T>,
    state: Rc<State<T>>,
}

impl<
    T: mproto::Owned + Clone,
> PropertyOwnerBuilder<T> {
    pub fn new(
        _name: &'static str,
        hooks: crate::PropertyOwnerHooks<T>,
        stubs: crate::PropertyOwnerStubs<T>,
        _config: &PropertyOwnerConfig,
        init: PropertyInitState<T>,
    ) -> Self {
        let state = Rc::new(State {
            hooks,
            value: RefCell::new(init.value.clone()),
        });
        Self { stubs, state }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> crate::PropertyOwner<T> {
        crate::PropertyOwner { state: self.state.clone() }
    }

    pub fn build(
        self,
        setup: &RoleSetup,
    ) {
        self.stubs.update.inline(setup, |_source, _update| {
            // TODO set value with some policy for conflict resolution when there are multiple
            // owners concurrently setting the value.
        })
        .subscribe();
    }
}

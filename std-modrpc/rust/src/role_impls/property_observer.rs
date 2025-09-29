use std::cell::RefCell;
use std::rc::Rc;

use crate::proto::{PropertyInitState, PropertyObserverConfig};
use modrpc::RoleSetup;

#[derive(Clone)]
pub struct PropertyObserver<T> {
    value: Rc<RefCell<T>>,
}

impl<T: Copy> PropertyObserver<T> {
    pub fn value(&self) -> T {
        *self.value.borrow()
    }
}

impl<T: Clone> PropertyObserver<T> {
    pub fn value_cloned(&self) -> T {
        let value_cell: &RefCell<T> = &*(self.value);
        value_cell.clone().into_inner()
    }
}

pub struct PropertyObserverBuilder<T> {
    stubs: crate::PropertyObserverStubs<T>,
    value: Rc<RefCell<T>>,
}

impl<
    T: mproto::Owned + Clone,
> PropertyObserverBuilder<T> {
    pub fn new(
        _name: &'static str,
        _hooks: crate::PropertyObserverHooks<T>,
        stubs: crate::PropertyObserverStubs<T>,
        _config: &PropertyObserverConfig,
        init: PropertyInitState<T>,
    ) -> Self {
        let value = Rc::new(RefCell::new(init.value.clone()));
        Self {
            stubs,
            value,
        }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> crate::PropertyObserver<T> {
        crate::PropertyObserver {
            value: self.value.clone(),
        }
    }

    pub fn build(
        self,
        setup: &RoleSetup,
    ) {
        let value = self.value.clone();
        self.stubs.update
            .inline(setup, move |_source, update| {
                *value.borrow_mut() = update.new_value;
            })
            .subscribe();
    }
}

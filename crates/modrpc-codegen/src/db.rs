use std::collections::HashMap;

use crate::ast::{Interface, QualifiedIdentifier};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceDefId(usize);

pub struct Database {
    imports: HashMap<String, Module>,
    local: Module,
    mproto_db: mproto_codegen::Database,
}

impl Database {
    pub fn new(mproto_db: mproto_codegen::Database) -> Self {
        Self {
            imports: HashMap::new(),
            local: Module::new(),
            mproto_db,
        }
    }

    pub fn local(&mut self) -> &mut Module { &mut self.local }
    pub fn mproto_db(&self) -> &mproto_codegen::Database { &self.mproto_db }
    pub fn mproto_db_mut(&mut self) -> &mut mproto_codegen::Database { &mut self.mproto_db }

    pub fn add_module(&mut self, name: String, module: Module) {
        self.imports.insert(name, module);
    }

    pub fn lookup_interface<'a>(&'a self, identifier: &QualifiedIdentifier) -> Option<&'a Interface> {
        if let Some(ref module_name) = identifier.module {
            // Importing from another module.
            let module = self.imports.get(module_name)?;
            module.interface_by_name(&identifier.name)
        } else {
            // It's a local definition.
            self.local.interface_by_name(&identifier.name)
        }
    }
}

#[derive(Debug)]
pub struct Module {
    interfaces: Vec<Interface>,
    interfaces_by_name: HashMap<String, InterfaceDefId>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
            interfaces_by_name: HashMap::new(),
        }
    }

    pub fn add_interface(&mut self, interface: Interface) -> InterfaceDefId {
        let id = InterfaceDefId(self.interfaces.len());

        self.interfaces_by_name.insert(interface.name.clone(), id);
        self.interfaces.push(interface);

        id
    }

    pub fn interface(&self, id: InterfaceDefId) -> &Interface {
        &self.interfaces[id.0]
    }

    pub fn interface_by_name<'a>(&'a self, name: &str) -> Option<&'a Interface> {
        let id = self.interfaces_by_name.get(name)?;
        Some(self.interface(*id))
    }
}


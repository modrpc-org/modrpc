use crate::Database;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Schema {
    pub imports: Vec<Import>,
    pub interfaces: Vec<Interface>,
    pub type_defs: Vec<mproto_codegen::ast::TypeDef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Import {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Interface {
    pub name: String,
    pub type_params: Vec<String>,
    pub roles: Vec<String>,

    pub events: Vec<InterfaceEventsList>,
    pub objects: Vec<InterfaceObject>,
    pub config: Vec<InterfaceConfigList>,
    pub required_impls: Vec<InterfaceRequiredImplsList>,
    pub methods: Vec<ObjectMethodsList>,
    pub state: Vec<InterfaceState>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceEventsList {
    pub from_roles: Vec<String>,
    pub to_roles: Vec<String>,
    pub events: Vec<InterfaceEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceEvent {
    pub name: String,
    pub ty: mproto_codegen::ast::Type,
    pub is_private: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceState {
    pub name: String,
    pub ty: mproto_codegen::ast::Type,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceObject {
    pub name: String,
    pub construct: QualifiedIdentifier,
    pub type_args: Vec<mproto_codegen::ast::Type>,
    pub role_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceConfigList {
    pub roles: Vec<String>,
    pub items: Vec<InterfaceConfigItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceConfigItem {
    pub name: String,
    pub ty: mproto_codegen::ast::Type,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceRequiredImplsList {
    pub roles: Vec<String>,
    pub required_impls: Vec<InterfaceRequiredImpl>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct InterfaceRequiredImpl {
    pub name: String,
    pub is_async: bool,
    pub input_ty: mproto_codegen::ast::Type,
    pub output_ty: mproto_codegen::ast::Type,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ObjectMethodsList {
    pub roles: Vec<String>,
    pub methods: Vec<ObjectMethod>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ObjectMethod {
    pub name: String,
    pub is_async: bool,
    pub input_ty: mproto_codegen::ast::Type,
    pub output_ty: mproto_codegen::ast::Type,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct QualifiedIdentifier {
    pub name: String,
    pub module: Option<String>,
}

impl QualifiedIdentifier {
    pub fn local(name: impl Into<String>) -> Self {
        Self { name: name.into(), module: None }
    }
}

impl Interface {
    pub fn requires_impls_for_role(&self, role_name: &str) -> bool {
        if self.has_public_event_impls_for_role(role_name) {
            return true;
        }

        for required_impls in &self.required_impls {
            if required_impls.roles
                .iter()
                .find(|x| x == &role_name)
                .is_some()
            {
                return true;
            }
        }

        false
    }

    pub fn has_public_event_impls_for_role(&self, role_name: &str) -> bool {
        for events_list in &self.events {
            if events_list.events.len() > 0 {
                if events_list.to_roles
                    .iter()
                    .find(|x| x == &role_name)
                    .is_some()
                {
                    for event in &events_list.events {
                        if !event.is_private {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn has_private_event_impls_for_role(&self, role_name: &str) -> bool {
        for events_list in &self.events {
            if events_list.events.len() > 0 {
                if events_list.to_roles
                    .iter()
                    .find(|x| x == &role_name)
                    .is_some()
                {
                    for event in &events_list.events {
                        if event.is_private {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn has_methods_for_role(&self, role_name: &str) -> bool {
        for methods_list in &self.methods {
            if methods_list.methods.len() > 0 {
                if methods_list.roles
                    .iter()
                    .find(|x| x == &role_name)
                    .is_some()
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn has_impls_for_role(&self, role_name: &str) -> bool {
        for required_impls in &self.required_impls {
            if required_impls.roles
                .iter()
                .find(|x| x == &role_name)
                .is_some()
            {
                return true;
            }
        }

        false
    }

    pub fn has_config_for_role(&self, role_name: &str) -> bool {
        for config_list in &self.config {
            if config_list.roles
                .iter()
                .find(|x| x == &role_name)
                .is_some()
            {
                return true;
            }
        }

        false
    }

    pub fn has_state(&self, db: &Database) -> bool {
        if self.state.len() > 0 { return true; }

        let mut has_state = false;

        for object in &self.objects {
            if let Some(object_interface) = db.lookup_interface(&object.construct) {
                has_state |= object_interface.has_state(db);
            } else {
                panic!("Interface::has_state failed to lookup interface {:?}", object.construct);
            }
        }

        has_state
    }
}

impl InterfaceObject {
    /// Return the role in the object's interface used by the specified role in the interface the
    /// object appears in (a.k.a. the parent).
    pub fn get_role_for_parent_role<'a>(
        &self,
        object_interface: &'a Interface,
        parent_role_name: &str,
    ) -> Option<&'a String> {
        let role_arg_index = self.role_args.iter().enumerate()
            .find(|(_, role_arg_name)| &parent_role_name == role_arg_name)
            .map(|(i, _)| i)?;

        Some(&object_interface.roles[role_arg_index])
    }

    /// Return the roles in the object's interface used by the specified role in the interface the
    /// object appears in (a.k.a. the parent).
    pub fn get_roles_for_parent_role<'a>(
        &self,
        object_interface: &'a Interface,
        parent_role_name: &str,
    ) -> Vec<&'a String> {
        self.role_args.iter().enumerate()
            .filter(move |(_, role_arg_name)| &parent_role_name == role_arg_name)
            .map(|(role_arg_index, _)| &object_interface.roles[role_arg_index])
            .collect()
    }
}


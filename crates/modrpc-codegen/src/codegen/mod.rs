use std::collections::HashSet;

use crate::{
    ast::{Interface, Schema},
    Database, Module,
};

pub mod js;
pub mod rust;
pub mod wasm;

pub fn interface_init_state(
    db: &Database,
    interface: &Interface,
) -> mproto_codegen::ast::TypeDef {
    let mut init_fields = Vec::new();

    for state in &interface.state {
        init_fields.push(mproto_codegen::ast::NamedField {
            name: state.name.clone(),
            ty: state.ty.clone(),
        });
    }

    for object in &interface.objects {
        let Some(object_interface) = db.lookup_interface(&object.construct) else {
            panic!("failed to lookup interface '{}'", object.construct.name);
        };

        if !object_interface.has_state(db) { continue; }

        init_fields.push(mproto_codegen::ast::NamedField {
            name: object.name.clone(),
            ty: mproto_codegen::ast::Type::Defined {
                // TODO import InitState from construct's crate
                ident: mproto_codegen::ast::QualifiedIdentifier {
                    name: format!("{}InitState", object.construct.name),
                    module: object.construct.module.clone(),
                },
                args: object.type_args.clone(),
            },
        });
    }

    let params =
        if interface.has_state(db) {
            interface.type_params.clone()
        } else {
            vec![]
        };

    mproto_codegen::ast::TypeDef {
        name: format!("{}InitState", interface.name),
        params,
        body: mproto_codegen::ast::TypeBody::Struct(mproto_codegen::ast::Struct {
            fields: init_fields,
        }),
    }
}

pub fn interface_role_config(
    db: &Database,
    mproto_cx: &mproto_codegen::codegen::CodegenCx<'_>,
    interface: &Interface,
    role_name: &str,
) -> mproto_codegen::ast::TypeDef {
    let mut fields = Vec::new();
    let mut used_type_params = HashSet::new();

    for config_list in &interface.config {
        if !config_list.roles.iter().find(|r| *r == role_name).is_some() {
            continue;
        }

        for config_item in &config_list.items {
            for type_param in &interface.type_params {
                if mproto_codegen::codegen::type_uses_param(mproto_cx, &config_item.ty, type_param) {
                    used_type_params.insert(type_param.clone());
                }
            }

            fields.push(mproto_codegen::ast::NamedField {
                name: config_item.name.clone(),
                ty: config_item.ty.clone(),
            });
        }
    }

    for object in &interface.objects {
        let Some(object_interface) = db.lookup_interface(&object.construct) else {
            panic!("failed to lookup interface '{}'", object.construct.name);
        };

        let object_role_names = object.get_roles_for_parent_role(object_interface, role_name);
        if object_role_names.is_empty() {
            panic!("object '{}' failed to find roles '{}'", object.name, role_name);
        };

        for object_role_name in &object_role_names {
            if !object_interface.has_config_for_role(object_role_name) { continue; }

            let object_interface_role_name = &format!("{}{}", object.construct.name, object_role_name);
            let object_role_config_def = db.mproto_db().lookup_type_def(
                &mproto_codegen::ast::QualifiedIdentifier {
                    name: format!("{object_interface_role_name}Config"),
                    module: object.construct.module.clone(),
                }
            )
            .expect("lookup object role config");

            let mut type_args = vec![];
            for (type_param, type_arg) in object_interface.type_params.iter()
                .zip(object.type_args.iter())
            {
                let object_role_config_uses_type_param =
                    object_role_config_def.params.iter().find(|p| p == &type_param).is_some();
                if object_role_config_uses_type_param {
                    type_args.push(type_arg.clone());
                }
            }

            fields.push(mproto_codegen::ast::NamedField {
                name: object.name.clone(),
                ty: mproto_codegen::ast::Type::Defined {
                    ident: mproto_codegen::ast::QualifiedIdentifier {
                        name: format!("{object_interface_role_name}Config"),
                        module: object.construct.module.clone(),
                    },
                    args: type_args,
                },
            });
        }
    }

    let params = interface.type_params.iter()
        .filter(|p| used_type_params.contains(p.as_str()))
        .cloned()
        .collect();

    mproto_codegen::ast::TypeDef {
        name: format!("{}{}Config", interface.name, role_name),
        params,
        body: mproto_codegen::ast::TypeBody::Struct(mproto_codegen::ast::Struct { fields }),
    }
}

pub fn define_interface_config_and_state_structs(
    db: &mut Database,
    interface: &Interface,
) {
    // Add init state for interface to mproto module.
    let init_state_struct = interface_init_state(db, interface);
    let _ = db.mproto_db_mut().local_mut().new_type_def(init_state_struct);
    // Add interface role config structs to mproto module.
    for role_name in &interface.roles {
        let role_config_struct = interface_role_config(
            db,
            &mproto_codegen::codegen::CodegenCx::new_with_type_params(
                db.mproto_db(), Some("crate::proto"), true, &interface.type_params,
            ),
            &interface,
            role_name,
        );
        let _ = db.mproto_db_mut().local_mut().new_type_def(role_config_struct);
    }
}

pub fn load_imports_recursive(db: &mut Database, schema: &Schema) -> Result<(), ()> {
    for import in &schema.imports {
        if import.path.ends_with(".mproto") {
            let type_defs = mproto_codegen::parse::parse_file(&import.path)
                .expect(&format!("failed to parse import '{}'.", import.path));
            // TODO load mproto imports once mproto supports it.
            //mproto_codegen::load_imports_recursive(db, &import_schema)?;
            // Create mproto module
            let mproto_module = mproto_codegen::Module::from_type_defs(type_defs.clone());
            // Add mproto module to mproto DB
            db.mproto_db_mut().add_module(import.name.clone(), "mproto", mproto_module);
        } else if import.path.ends_with(".modrpc") {
            let mut import_schema = crate::parse::parse_file(&import.path)
                .expect(&format!("failed to parse import '{}'.", import.path));

            load_imports_recursive(db, &import_schema)?;

            // Create mproto module corresponding to modrpc module
            let mproto_module = mproto_codegen::Module::from_type_defs(
                import_schema.type_defs.clone(),
            );
            let lower_cx = LowerCx { module_name: &import.name, module: &mproto_module };
            let lowered_type_defs = import_schema.type_defs.into_iter()
                .map(|type_def| lower_cx.lower_type_def(type_def))
                .collect();
            let lowered_mproto_module = mproto_codegen::Module::from_type_defs(lowered_type_defs);

            // Create mproto module
            for interface in &mut import_schema.interfaces {
                // Modify the imported schema's mproto local type references to be fully qualified.
                for state_field in &mut interface.state {
                    state_field.ty = lower_cx.lower_type(state_field.ty.clone());
                }
                for config_list in &mut interface.config {
                    for config_item in &mut config_list.items {
                        config_item.ty = lower_cx.lower_type(config_item.ty.clone());
                    }
                }
            }
            // Add mproto module to mproto DB
            db.mproto_db_mut().add_module(import.name.clone(), "modrpc", lowered_mproto_module);
            // Add init and config structs after module is installed in the DB since they require
            // that locally referenced types are defined.
            for interface in &mut import_schema.interfaces {
                // TODO re-use define_interface_config_and_state_structs. Requires splitting
                // mproto_codgen::Database out of modrpc_codegen::Database.

                // Add init state for interface to mproto module.
                let init_state_struct = interface_init_state(db, &interface);
                let _ = db.mproto_db_mut().imported_module_mut(&import.name)
                    .expect("lookup mproto module")
                    .new_type_def(init_state_struct);
                // Add interface role config structs to mproto module.
                for role_name in &interface.roles {
                    let role_config_struct = interface_role_config(
                        db,
                        &mproto_codegen::codegen::CodegenCx::new_with_type_params(
                            db.mproto_db(), None, true, &interface.type_params,
                        ),
                        &interface,
                        role_name,
                    );
                    let _ = db.mproto_db_mut().imported_module_mut(&import.name)
                        .expect("lookup mproto module")
                        .new_type_def(role_config_struct);
                }
            }

            // Create modrpc module
            let mut module = Module::new();
            for interface in import_schema.interfaces {
                let _ = module.add_interface(interface.clone());
            }
            // Add modrpc module to modrpc DB
            db.add_module(import.name.clone(), module);
        } else {
            // TODO return a descriptive error
            panic!("imports must end with '.mproto' or '.modrpc' - got '{}'", import.path);
        }
    }

    Ok(())
}

struct LowerCx<'a> {
    module_name: &'a str,
    module: &'a mproto_codegen::Module,
}

impl<'a> LowerCx<'a> {
    /// For now this just ensures all QualifiedIdentifiers for things in the local module have their
    /// module set to `Some(module_name)`. In the future we should actually lower to an IR that has
    /// type references resolved into TypeDefIds.
    fn lower_type_def(
        &self,
        type_def: mproto_codegen::ast::TypeDef,
    ) -> mproto_codegen::ast::TypeDef {
        let body = match type_def.body {
            mproto_codegen::ast::TypeBody::Struct(struct_body) => {
                mproto_codegen::ast::TypeBody::Struct(mproto_codegen::ast::Struct {
                    fields: self.lower_named_fields(struct_body.fields),
                })
            }
            mproto_codegen::ast::TypeBody::Enum(enum_body) => {
                mproto_codegen::ast::TypeBody::Enum(mproto_codegen::ast::Enum {
                    variants: enum_body.variants.into_iter().map(|(name, variant)| {
                        let variant = match variant {
                            mproto_codegen::ast::EnumVariant::Empty => mproto_codegen::ast::EnumVariant::Empty,
                            mproto_codegen::ast::EnumVariant::NamedFields { fields } =>  {
                                mproto_codegen::ast::EnumVariant::NamedFields {
                                    fields: self.lower_named_fields(fields),
                                }
                            }
                        };
                        (name, variant)
                    })
                    .collect()
                })
            }
        };

        mproto_codegen::ast::TypeDef {
            name: type_def.name,
            params: type_def.params,
            body,
        }
    }

    fn lower_named_fields(
        &self,
        fields: Vec<mproto_codegen::ast::NamedField>,
    ) -> Vec<mproto_codegen::ast::NamedField> {
        fields.into_iter().map(|f| {
            mproto_codegen::ast::NamedField {
                name: f.name,
                ty: self.lower_type(f.ty),
            }
        })
        .collect()
    }

    fn lower_type(&self, ty: mproto_codegen::ast::Type) -> mproto_codegen::ast::Type {
        use mproto_codegen::ast::{Type, PrimitiveType, QualifiedIdentifier};
        match ty {
            Type::Primitive(PrimitiveType::Box(inner_ty))  => {
                Type::Primitive(PrimitiveType::Box(Box::new(self.lower_type(*inner_ty))))
            }
            Type::Primitive(PrimitiveType::List(item_ty)) => {
                Type::Primitive(PrimitiveType::List(Box::new(self.lower_type(*item_ty))))
            }
            Type::Primitive(PrimitiveType::Option(item_ty)) => {
                Type::Primitive(PrimitiveType::Option(Box::new(self.lower_type(*item_ty))))
            }
            Type::Primitive(PrimitiveType::Result(ok_ty, err_ty)) => {
                Type::Primitive(PrimitiveType::Result(
                    Box::new(self.lower_type(*ok_ty)),
                    Box::new(self.lower_type(*err_ty)),
                ))
            }
            Type::Defined { ident, args } => {
                let args = args.into_iter().map(|a| self.lower_type(a)).collect();
                if ident.module.is_none() && self.module.type_def_by_name(&ident.name).is_some() {
                    Type::Defined {
                        ident: QualifiedIdentifier {
                            module: Some(self.module_name.into()),
                            name: ident.name,
                        },
                        args,
                    }
                } else {
                    Type::Defined { ident, args }
                }
            }
            ty @ _ => ty,
        }
    }
}

fn write_js_file(
    path: impl AsRef<std::path::Path> + std::fmt::Debug,
    header: &str,
    tokens: &genco::lang::js::Tokens,
) -> std::io::Result<()> {
    use std::io::Write;

    let fmt = genco::fmt::Config::from_lang::<genco::lang::JavaScript>()
        .with_indentation(genco::fmt::Indentation::Space(2));
    let config = genco::lang::js::Config::default();
    let mut file = std::fs::File::create(&path)?;

    file.write_all(header.as_bytes())?;

    let mut w = genco::fmt::IoWriter::new(file);
    tokens
        .format_file(&mut w.as_formatter(&fmt), &config)
        .expect(&format!("format {:?} file", path));

    Ok(())
}

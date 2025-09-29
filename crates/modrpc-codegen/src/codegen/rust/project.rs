use std::path::Path;
use std::fs::File;
use std::io::Write;

use genco::quote;
use mproto_codegen::codegen::name_util::camel_to_snake_case;

use crate::{
    ast::Schema,
    codegen,
    Database,
};

const PROTO_CARGO_TOML: &'static str = include_str!("templates/interface/cargo.toml");
const PROTO_LIB_RS: &'static str = include_str!("templates/interface/lib.rs");

pub fn rust_project_gen(
    root_dir: impl AsRef<Path>,
    project_name: &str,
    schema: &Schema,
) -> std::io::Result<()> {
    let pkg_name = &format!("{}-modrpc", project_name);
    let pkg_root = root_dir.as_ref().join(pkg_name).join("rust");

    std::fs::create_dir_all(&pkg_root)?;

    let _db = rust_proto_package_gen(&pkg_root, pkg_name, schema)?;

    Ok(())
}

pub fn rust_proto_package_gen(
    pkg_root: impl AsRef<Path>,
    pkg_name: &str,
    schema: &Schema,
) -> std::io::Result<Database> {
    let pkg_root = pkg_root.as_ref();
    let local_mproto_module = mproto_codegen::Module::from_type_defs(
        schema.type_defs.clone(),
    );
    let mproto_db = mproto_codegen::Database::new(local_mproto_module);

    let mut db = Database::new(mproto_db);
    codegen::load_imports_recursive(&mut db, schema).unwrap();

    for interface in &schema.interfaces {
        let _ = db.local().add_interface(interface.clone());

        // Define per-interface *InitState and per-role *Config structs
        codegen::define_interface_config_and_state_structs(
            &mut db,
            interface,
        );
    }

    let src_dir = pkg_root.join("src");
    let roles_dir = src_dir.join("roles");
    let role_impls_dir = src_dir.join("role_impls");

    std::fs::create_dir_all(&pkg_root)?;
    std::fs::create_dir_all(&src_dir)?;
    std::fs::create_dir_all(&roles_dir)?;
    std::fs::create_dir_all(&role_impls_dir)?;

    // Write Cargo.toml
    let cargo_toml_path = pkg_root.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        let mut cargo_toml_file = File::create(cargo_toml_path)?;
        cargo_toml_file.write_all(
            PROTO_CARGO_TOML.replace("PKG_NAME", pkg_name).as_bytes()
        )?;
    } else {
        println!(
            "{} already exists - skipping as it might contain hand-written code.",
            cargo_toml_path.display(),
        );
    }

    // Write lib.rs
    let lib_rs_path = src_dir.join("lib.rs");
    if !lib_rs_path.exists() {
        let mut lib_rs_file = File::create(&lib_rs_path)?;
        lib_rs_file.write_all(PROTO_LIB_RS.as_bytes())?;
    } else {
        println!(
            "{} already exists - skipping as it might contain hand-written code.",
            lib_rs_path.display(),
        );
    }

    // Write proto.rs
    let fmt = genco::fmt::Config::from_lang::<genco::lang::Rust>()
        .with_indentation(genco::fmt::Indentation::Space(4));
    let config = genco::lang::rust::Config::default();
    let proto_rust_file = File::create(src_dir.join("proto.rs"))?;
    let mut w = genco::fmt::IoWriter::new(proto_rust_file);
    let mut tokens = genco::lang::rust::Tokens::new();

    for type_def in db.mproto_db().local().type_defs() {
        let type_def_tokens = mproto_codegen::codegen::rust::rust_type_def(
            &mproto_codegen::codegen::CodegenCx::new(db.mproto_db(), None, true),
            type_def,
        );
        tokens = quote! {
            $tokens

            $type_def_tokens
        };
    }

    tokens
        .format_file(&mut w.as_formatter(&fmt), &config)
        .expect("format proto.rs file");

    // Write interface.rs
    let fmt = genco::fmt::Config::from_lang::<genco::lang::Rust>()
        .with_indentation(genco::fmt::Indentation::Space(4));
    let config = genco::lang::rust::Config::default();
    let interface_rust_file = File::create(src_dir.join("interface.rs"))?;
    let mut w = genco::fmt::IoWriter::new(interface_rust_file);
    let mut tokens = genco::lang::rust::Tokens::new();

    for interface in &schema.interfaces {
        let interface_tokens = codegen::rust::rust_interface(&db, interface);
        tokens = quote! {
            $tokens

            $interface_tokens
        };
    }

    tokens
        .format_file(&mut w.as_formatter(&fmt), &config)
        .expect("format interface.rs file");

    // Write role files
    let mut roles_mod_tokens = quote! { };
    for interface in &schema.interfaces {
        for role_name in &interface.roles {
            let snake_interface_role_name = camel_to_snake_case(
                &format!("{}{}", interface.name, role_name)
            );
            write_rust_file(
                &roles_dir.join(format!("{}.rs", snake_interface_role_name)),
                &codegen::rust::rust_interface_role(&db, interface, role_name),
                |f| {
                    f.write_all(b"#![allow(unused_variables)]\n\n")
                },
            )?;

            roles_mod_tokens = quote! {
                mod $(&snake_interface_role_name);
                pub use $(&snake_interface_role_name)::*;
                $roles_mod_tokens
            };
        }
    }
    write_rust_file(&roles_dir.join("mod.rs"), &roles_mod_tokens, |_| Ok(()))?;

    // Write role impls file
    let mut role_impls_mod_tokens = quote! { };
    for interface in &schema.interfaces {
        for role_name in &interface.roles {
            let snake_interface_role_name = camel_to_snake_case(
                &format!("{}{}", interface.name, role_name)
            );
            let path = role_impls_dir.join(&format!("{}.rs", snake_interface_role_name));
            let tokens = &codegen::rust::rust_interface_role_impl(&db, interface, role_name);

            if !path.exists() {
                if !tokens.is_empty() {
                    write_rust_file(&path, tokens, |_| Ok(()))?;
                }
            } else {
                println!(
                    "{} already exists - skipping as it might contain hand-written code.",
                    path.display(),
                );
            }

            if !tokens.is_empty() {
                role_impls_mod_tokens = quote! {
                    mod $(&snake_interface_role_name);
                    pub use $(&snake_interface_role_name)::*;
                    $role_impls_mod_tokens
                };
            }
        }
    }
    write_rust_file(&role_impls_dir.join("mod.rs"), &role_impls_mod_tokens, |_| Ok(()))?;

    Ok(db)
}

fn write_rust_file(
    path: impl AsRef<Path> + std::fmt::Debug,
    tokens: &genco::lang::rust::Tokens,
    prepend: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let fmt = genco::fmt::Config::from_lang::<genco::lang::Rust>()
        .with_indentation(genco::fmt::Indentation::Space(4));
    let config = genco::lang::rust::Config::default();
    let mut file = File::create(&path)?;
    prepend(&mut file)?;
    let mut w = genco::fmt::IoWriter::new(file);

    tokens
        .format_file(&mut w.as_formatter(&fmt), &config)
        .expect(&format!("format {:?} file", path));
    
    Ok(())
}

pub fn rust_role_impl_gen(
    project_name: &str,
    root_dir: impl AsRef<Path>,
    schema: &Schema,
    role_name: &str,
) -> std::io::Result<()> {
    let local_mproto_module = mproto_codegen::Module::from_type_defs(
        schema.type_defs.clone(),
    );
    let mproto_db = mproto_codegen::Database::new(local_mproto_module);

    let pkg_name = format!("{}-{}", project_name, &role_name.to_lowercase());

    let pkg_root = root_dir.as_ref().join(pkg_name);
    let src_dir = pkg_root.join("src");

    std::fs::create_dir_all(&pkg_root)?;
    std::fs::create_dir_all(&src_dir)?;

    // Write Cargo.toml
    let mut cargo_toml_file = File::create(pkg_root.join("Cargo.toml"))?;
    cargo_toml_file.write_all(
        include_str!("templates/server/cargo.toml").replace("PKG_NAME", project_name).as_bytes()
    )?;

    // Write lib.rs
    let fmt = genco::fmt::Config::from_lang::<genco::lang::Rust>()
        .with_indentation(genco::fmt::Indentation::Space(4));
    let config = genco::lang::rust::Config::default();
    let mut main_rs_file = File::create(src_dir.join("main.rs"))?;

    main_rs_file.write("\
        #![feature(type_alias_impl_trait)]\n\
        \n\
        use std::future::Future;\n\
        \n".as_bytes())?;

    let mut w = genco::fmt::IoWriter::new(main_rs_file);

    let interface = &schema.interfaces[0];
    let role_impl = codegen::rust::RustInterfaceImpl::generate(
        &mproto_db, project_name, interface, role_name,
    );

    role_impl.main_rs
        .format_file(&mut w.as_formatter(&fmt), &config)
        .expect("format main.rs file");

    let handlers_dir = src_dir.join("handlers");
    std::fs::create_dir_all(&handlers_dir)?;
    for (handler_name, handler_impl_tokens) in role_impl.handlers {
        write_rust_file(&handlers_dir.join(&format!("{handler_name}.rs")), &handler_impl_tokens, |_| Ok(()))?;
    }

    Ok(())
}

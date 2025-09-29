use std::path::Path;
use std::fs;
use std::io::Write;

use mproto_codegen::codegen::name_util::{
    camel_to_snake_case, camel_to_kebab_case,
};

use crate::{
    ast::Schema,
    codegen,
    Database,
};

const RUST_GLUE_CARGO_TOML: &'static str = include_str!("templates/rust-wasm-glue/cargo.toml");
const RUST_GLUE_BUILD_SH: &'static str = include_str!("templates/rust-wasm-glue/build.sh");

const TS_GLUE_PACKAGE_JSON: &'static str = include_str!("templates/ts-wasm-glue/package.json");
const TS_GLUE_TSCONFIG_JSON: &'static str = include_str!("templates/ts-wasm-glue/tsconfig.json");
const TS_GLUE_BUILD_SH: &'static str = include_str!("templates/ts-wasm-glue/build.sh");

pub fn wasm_project_gen(
    root_dir: impl AsRef<Path>,
    project_name: &str,
    schema: &Schema,
) -> std::io::Result<()> {
    let pkg_name = &format!("{}-modrpc", project_name);
    let wasm_root = root_dir.as_ref().join(pkg_name).join("wasm");

    // Setup db for codegen
    let local_mproto_module = mproto_codegen::Module::from_type_defs(schema.type_defs.clone());
    let mproto_db = mproto_codegen::Database::new(local_mproto_module);

    let mut db = Database::new(mproto_db);
    crate::codegen::load_imports_recursive(&mut db, schema).unwrap();
    for interface in &schema.interfaces {
        let _ = db.local().add_interface(interface.clone());

        // Define per-interface *InitState and per-role *Config structs
        codegen::define_interface_config_and_state_structs(
            &mut db,
            interface,
        );
    }

    for interface in &schema.interfaces {
        let snake_interface_name = &camel_to_snake_case(&interface.name);
        let kebab_interface_name = &camel_to_kebab_case(&interface.name);

        for role_name in &interface.roles {
            let snake_role_name = &camel_to_snake_case(role_name);
            let kebab_role_name = &camel_to_kebab_case(role_name);
            let kebab_interface_role_name = &camel_to_kebab_case(
                &format!("{}{}", interface.name, role_name)
            );

            let role_root = wasm_root.join(kebab_interface_role_name);

            // Write WIT
            let wit_dir = role_root.join("wit");
            fs::create_dir_all(&wit_dir)?;
            fs::write(
                wit_dir.join(format!("{}.wit", kebab_interface_role_name)),
                codegen::wasm::wit_interface(&db, interface, role_name).as_bytes(),
            )?;

            // Role's rust wasm glue

            let role_rust_root = role_root.join("rust");
            fs::create_dir_all(&role_rust_root)?;

            // Write Cargo.toml
            let cargo_toml_path = role_rust_root.join("Cargo.toml");
            if !cargo_toml_path.exists() {
                fs::write(
                    cargo_toml_path,
                    RUST_GLUE_CARGO_TOML
                        .replace("PROJECT_NAME", project_name)
                        .replace("INTERFACE_NAME", kebab_interface_name)
                        .replace("ROLE_NAME", kebab_role_name)
                        .as_bytes(),
                )?;
            } else {
                println!(
                    "{} already exists - skipping as it might contain hand-written code.",
                    cargo_toml_path.display(),
                );
            }

            // Write build.sh
            fs::write(
                role_rust_root.join("build.sh"),
                RUST_GLUE_BUILD_SH
                    .replace("INTERFACE_NAME", snake_interface_name)
                    .replace("ROLE_NAME", snake_role_name)
                    .as_bytes(),
            )?;

            // Write src/lib.rs
            let src_dir = role_rust_root.join("src");
            fs::create_dir_all(&src_dir)?;
            write_rust_file(
                &src_dir.join("lib.rs"),
                &indoc::indoc! {"
                    #![feature(type_alias_impl_trait)]
                    #![feature(thread_local)]
                "},
                &codegen::wasm::wit_impl_glue_rust(
                    &format!("{}_modrpc", project_name.replace("-", "_")),
                    &db, interface, role_name,
                ),
            )?;

            // Role's typescript wasm glue

            let role_typescript_root = role_root.join("typescript");
            fs::create_dir_all(&role_typescript_root)?;

            // Write package.json
            let package_json_path = role_typescript_root.join("package.json");
            if !package_json_path.exists() {
                fs::write(
                    package_json_path,
                    TS_GLUE_PACKAGE_JSON
                        .replace("PROJECT_NAME", project_name)
                        .replace("PKG_NAME", &format!("{}-{}", kebab_interface_name, kebab_role_name))
                        .as_bytes(),
                )?;
            } else {
                println!(
                    "{} already exists - skipping as it might contain hand-written code.",
                    package_json_path.display(),
                );
            }

            // Write tsconfig.json
            fs::write(
                role_typescript_root.join("tsconfig.json"),
                TS_GLUE_TSCONFIG_JSON.as_bytes(),
            )?;

            // Write build.sh
            fs::write(
                role_typescript_root.join("build.sh"),
                TS_GLUE_BUILD_SH
                    .replace("INTERFACE_NAME", snake_interface_name)
                    .replace("ROLE_NAME", snake_role_name)
                    .as_bytes(),
            )?;

            // Write src/index.ts
            let src_dir = role_typescript_root.join("src");
            fs::create_dir_all(&src_dir)?;
            crate::codegen::write_js_file(
                &src_dir.join("index.ts"), "",
                &codegen::wasm::js_wit_glue(
                    &format!("{}-modrpc", project_name),
                    &db, interface, role_name,
                ),
            )?;
        }
    }

    Ok(())
}

fn write_rust_file(
    path: impl AsRef<Path> + std::fmt::Debug,
    header: &str,
    tokens: &genco::lang::rust::Tokens,
) -> std::io::Result<()> {
    let fmt = genco::fmt::Config::from_lang::<genco::lang::Rust>()
        .with_indentation(genco::fmt::Indentation::Space(4));
    let config = genco::lang::rust::Config::default();
    let mut file = fs::File::create(&path)?;

    file.write_all(header.as_bytes())?;

    let mut w = genco::fmt::IoWriter::new(file);
    tokens
        .format_file(&mut w.as_formatter(&fmt), &config)
        .expect(&format!("format {:?} file", path));
    
    Ok(())
}


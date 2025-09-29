use std::path::Path;
use std::fs;
use std::io::Write;

use genco::{self, prelude::*};

use crate::{
    ast::Schema,
    codegen,
    Database,
};

const PROTO_PACKAGE_JSON: &'static str = include_str!("templates/proto/package.json");
const PROTO_TSCONFIG_JSON: &'static str = include_str!("templates/proto/tsconfig.json");
const PROTO_INDEX_TS: &'static str = include_str!("templates/proto/index.ts");

pub fn js_project_gen(
    root_dir: impl AsRef<Path>,
    project_name: &str,
    schema: &Schema,
) -> std::io::Result<()> {
    let pkg_name = &format!("{}-modrpc", project_name);
    let pkg_root = root_dir.as_ref().join(pkg_name).join("typescript");

    fs::create_dir_all(&pkg_root)?;

    js_proto_package_gen(&pkg_root, &format!("{}-modrpc", project_name), schema)?;

    Ok(())
}

pub fn js_proto_package_gen(
    pkg_root: impl AsRef<Path>,
    pkg_name: &str,
    schema: &Schema,
) -> std::io::Result<()> {
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

    fs::create_dir_all(&pkg_root)?;
    fs::create_dir_all(&src_dir)?;

    // Write package.json
    let package_json_path = pkg_root.join("package.json");
    if !package_json_path.exists() {
        fs::write(
            package_json_path,
            PROTO_PACKAGE_JSON.replace("PKG_NAME", pkg_name).as_bytes(),
        )?;
    } else {
        println!(
            "{} already exists - skipping as it might contain hand-written code.",
            package_json_path.display(),
        );
    }

    // Write tsconfig.json
    fs::write(pkg_root.join("tsconfig.json"), PROTO_TSCONFIG_JSON.as_bytes())?;

    // Write index.ts
    fs::write(src_dir.join("index.ts"), PROTO_INDEX_TS.as_bytes())?;

    // Write proto.ts
    let mut proto_tokens = genco::lang::js::Tokens::new();
    /*for interface in &schema.interfaces {
        proto_tokens = quote! {
            $proto_tokens

            $(codegen::js::js_interface_init_state_tokens(&mut db, interface))
        };
    }*/
    for type_def in db.mproto_db().local().type_defs() {
        let mproto_cx = &mproto_codegen::codegen::CodegenCx::new(db.mproto_db(), None, true);
        proto_tokens = quote! {
            $proto_tokens

            $(mproto_codegen::codegen::js::js_type_def(mproto_cx, type_def))
        };
    }
    write_js_file(
        src_dir.join("proto.ts"),
        "",
        &proto_tokens
    )?;

    // Write objects.ts
    let mut object_class_tokens = quote! {
        import * as mproto from "mproto";
    };
    for interface in &schema.interfaces {
        if interface.methods.len() == 0 { continue; }

        for role_name in &interface.roles {
            object_class_tokens = quote! {
                $object_class_tokens

                $(codegen::js::js_wit_object_class(&db, interface, role_name))
            };
        }
    }
    write_js_file(
        src_dir.join("objects.ts"),
        "",
        &object_class_tokens
    )?;

    Ok(())
}

fn write_js_file(
    path: impl AsRef<Path> + std::fmt::Debug,
    header: &str,
    tokens: &genco::lang::js::Tokens,
) -> std::io::Result<()> {
    let fmt = genco::fmt::Config::from_lang::<genco::lang::JavaScript>()
        .with_indentation(genco::fmt::Indentation::Space(2));
    let config = genco::lang::js::Config::default();
    let mut file = fs::File::create(&path)?;

    file.write_all(header.as_bytes())?;

    let mut w = genco::fmt::IoWriter::new(file);
    tokens
        .format_file(&mut w.as_formatter(&fmt), &config)
        .expect(&format!("format {:?} file", path));

    Ok(())
}

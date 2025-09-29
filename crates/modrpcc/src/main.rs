use clap::{App, Arg};

fn main() {
    let matches =
        App::new("modrpc compiler")
            .version("0.1")
            .author("modrpc authors")
            .about("Generate boilerplate code for modrpc interfaces.")
            .arg(Arg::with_name("INPUT")
                .help("Path to modrpc schema definition file")
                .required(true)
                .index(1)
            )
            .args_from_usage("-o, --output-dir [output-dir] 'Path to generate project directory in.'")
            .args_from_usage("-l, --language <language> 'Language to generate project packages for.'")
            .args_from_usage("-n, --name <project_name> 'Name of project to generate.'")
            //.args_from_usage("-r, --role [role_name] 'Name of role to generate if generating a role impl component.'")
            .arg(Arg::with_name("component")
                 .short("c")
                 .long("component")
                 .help("Component to generate - interface or impl.")
                 .required(false)
                 .takes_value(true)
            )
            .get_matches();

    let in_path = matches.value_of("INPUT").unwrap();
    let output_dir = matches.value_of("output-dir").unwrap_or("./");
    let project_name = matches.value_of("name").unwrap();
    let language = matches.value_of("language").unwrap();
    let component = matches.value_of("component").unwrap_or("interface");
    //let role = matches.value_of("role");

    // Open input file
    let schema = match modrpc_codegen::parse::parse_file(in_path) {
        Ok(s) => s,
        Err(e) => {
            println!("ERROR: Failed to load '{}': {}", in_path, e);
            std::process::exit(1);
        }
    };

    // Generate package
    match language {
        "typescript" => {
            modrpc_codegen::codegen::js::js_project_gen(output_dir, project_name, &schema).unwrap();
        }
        "rust" => {
            match component {
                "interface" => {
                    modrpc_codegen::codegen::rust::rust_project_gen(output_dir, project_name, &schema)
                        .unwrap();
                }
                "impl" => {
                    println!("Error: interface impl generation doesn't work yet!");
                    return;
                    /*if let Some(role) = role {
                        modrpc_codegen::codegen::rust::rust_role_impl_gen(
                            project_name, output_dir, &schema, role,
                        )
                        .unwrap();
                    } else {
                        println!("Error: `--role` must be specified when generating a role impl.");
                    }*/
                }
                _ => {
                    println!("Error: Unknown component '{}'. Options: modrpc, impl", component);
                }
            }
        }
        "wasm" => {
            modrpc_codegen::codegen::wasm::wasm_project_gen(output_dir, project_name, &schema).unwrap();
        }
        _ => {
            println!("ERROR: Unsupported language '{}'", language);
        }
    }
}

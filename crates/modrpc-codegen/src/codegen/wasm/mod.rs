use genco::prelude::*;
use mproto_codegen::codegen::name_util::{
    camel_to_kebab_case, camel_to_snake_case, snake_to_camel_case, snake_to_upper_camel_case,
};

use crate::{
    ast::Interface,
    Database,
};

pub use project::wasm_project_gen;

mod project;

pub fn wit_interface(
    db: &Database,
    interface: &Interface,
    role_name: &str,
) -> String {
    // Hook methods

    let mut impl_notify_items = String::new();
    let mut impl_items = String::new();
    let mut hooks_notify_items = String::new();
    let mut hooks_items = String::new();

    for object in &interface.objects {
        let Some(object_interface) = db.lookup_interface(&object.construct) else {
            panic!("failed to lookup interface '{}'", object.construct.name);
        };

        let Some(object_role_name) =
            object.get_role_for_parent_role(object_interface, role_name)
        else {
            panic!("object '{}' failed to find role arg '{}'", object.name, role_name);
        };

        let object_name_wit = witify_snake_name(&object.name);

        for required_impls_list in &object_interface.required_impls {
            if !required_impls_list.roles.contains(object_role_name) { continue; }

            for required_impl in &required_impls_list.required_impls {
                let impl_name_wit = format!(
                    "{}-{}",
                    object_name_wit,
                    witify_snake_name(&required_impl.name),
                );
                if required_impl.is_async {
                    impl_items += &indoc::formatdoc! {"
                        {impl_name_wit}: func(id: u32, input-buf: list<u8>)
                    "};
                    impl_notify_items += &indoc::formatdoc! {"
                        notify-{impl_name_wit}: func(id: u32, output-buf: list<u8>)
                    "};
                } else {
                    impl_items += &indoc::formatdoc! {"
                        {impl_name_wit}: func(input-buf: list<u8>) -> list<u8>
                    "};
                }
            }
        }

        for methods_list in &object_interface.methods {
            if !methods_list.roles.contains(object_role_name) { continue; }

            for method in &methods_list.methods {
                let method_name_wit = format!(
                    "{}-{}",
                    object_name_wit,
                    witify_snake_name(&method.name),
                );
                if method.is_async {
                    hooks_items += &indoc::formatdoc! {"
                        {method_name_wit}: func(input-buf: list<u8>) -> u32
                    "};
                    hooks_notify_items += &indoc::formatdoc! {"
                        notify-{method_name_wit}: func(id: u32, output-buf: list<u8>)
                    "};
                } else {
                    hooks_items += &indoc::formatdoc! {"
                        {method_name_wit}: func(input-buf: list<u8>) -> list<u8>
                    "};
                }
            }
        }
    }

    let interface_role_name_wit = camel_to_kebab_case(&format!("{}{}", interface.name, role_name));

    indoc::formatdoc! {"
        default world {interface_role_name_wit} {{
            import host: interface {{
                modrpc-send-packet-bundle: func(buf: list<u8>)
                {impl_items}
                {hooks_notify_items}
            }}

            export guest: interface {{
                {impl_notify_items}
                {hooks_items}
            }}

            export main: interface {{
                init: func(service-id: u32, endpoint-addr: u64, init-state-buf: list<u8>)
                handle-packet: func(data: list<u8>)
                tick: func()
            }}
        }}
    "}
}

pub fn wit_impl_glue_rust(
    interface_crate_name: &str,
    db: &Database,
    interface: &Interface,
    role_name: &str,
) -> rust::Tokens {
    let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
        db.mproto_db(), Some(interface_crate_name), false, &interface.type_params,
    );

    let mut required_impl_notifies: rust::Tokens = quote! { };
    let mut object_builds: rust::Tokens = quote! { };
    let mut method_calls: rust::Tokens = quote! { };
    for object in &interface.objects {
        let Some(object_interface) = db.lookup_interface(&object.construct) else {
            panic!("failed to lookup interface '{}'", object.construct.name);
        };

        let Some(object_role_name) =
            object.get_role_for_parent_role(object_interface, role_name)
        else {
            panic!("object '{}' failed to find role arg '{}'", object.name, role_name);
        };

        let object_mproto_cx = &mproto_cx.with_type_args(
            &object_interface.type_params,
            &object.type_args,
        );
        let mut required_impl_handlers: rust::Tokens = quote! { };

        for required_impls_list in &object_interface.required_impls {
            if !required_impls_list.roles.contains(object_role_name) { continue; }

            for required_impl in &required_impls_list.required_impls {
                let input_type_tokens =
                    mproto_codegen::codegen::rust::rust_type_tokens(object_mproto_cx, &required_impl.input_ty);
                let output_type_tokens =
                    mproto_codegen::codegen::rust::rust_type_tokens(object_mproto_cx, &required_impl.output_ty);

                required_impl_notifies = quote! {
                    $required_impl_notifies

                    fn notify_$(&object.name)_$(&required_impl.name)(id: u32, result_buf: Vec<u8>) {
                        modrpc_wasm_glue::notify_impl(id, result_buf)
                    }
                };

                required_impl_handlers = quote! {
                    $required_impl_handlers
                    modrpc::async_handler!(
                        { }
                        |cx, requester, input: $input_type_tokens| -> $output_type_tokens {
                            modrpc_wasm_glue::call_impl(host::$(&object.name)_$(&required_impl.name), input).await
                        }
                    ),
                };
            }
        }

        if object_interface.requires_impls_for_role(object_role_name) {
            object_builds = quote! {
                $object_builds

                stubs.$(&object.name).build(
                    s,
                    $required_impl_handlers
                );
            };
        }

        for methods_list in &object_interface.methods {
            if !methods_list.roles.contains(object_role_name) { continue; }

            for method in &methods_list.methods {
                let input_type_tokens =
                    mproto_codegen::codegen::rust::rust_type_tokens(object_mproto_cx, &method.input_ty);
                let output_type_tokens =
                    mproto_codegen::codegen::rust::rust_type_tokens(object_mproto_cx, &method.output_ty);

                method_calls = quote! {
                    $method_calls

                    fn $(&object.name)_$(&method.name)(payload: Vec<u8>) -> u32 {
                        async fn method_call(input: $input_type_tokens) -> $output_type_tokens {
                            STATE.hooks().$(&object.name).clone().$(&method.name)(input).await
                        }

                        modrpc_wasm_glue::method_call(
                            method_call,
                            host::notify_$(&object.name)_$(&method.name),
                            payload,
                        )
                    }
                };
            }
        }
    }

    ////////////////////////////////////

    let interface_prefix = &format!("{}::{}", interface_crate_name, interface.name);
    let interface_role_name = &format!("{}{}", interface.name, role_name);
    let interface_role_prefix = &format!("{}::{}", interface_crate_name, interface_role_name);
    let interface_role_name_wit = camel_to_kebab_case(&format!("{}{}", interface.name, role_name));

    quote! {
        use std::cell::RefCell;

        wit_bindgen::generate!({
            world: $("\"")$(interface_role_name_wit)$("\""),
            path: "../wit/",
        });

        struct State {
            hooks: RefCell<Option<$(interface_role_prefix)Hooks>>,
        }

        impl State {
            fn hooks(&'_ self) -> impl std::ops::Deref<Target = $(interface_role_prefix)Hooks> + '_ {
                std::cell::Ref::map(self.hooks.borrow(), |x| x.as_ref().unwrap())
            }
        }

        #[thread_local]
        static STATE: State = State {
            hooks: RefCell::new(None),
        };

        struct RoleImpl;

        impl main::Main for RoleImpl {
            fn handle_packet(buf: Vec<u8>) {
                modrpc_wasm_glue::receive_packet_bundle(buf);
            }

            fn init(service_id: u32, endpoint_addr: u64, init_state_buf: Vec<u8>) {
                let init_state: $(interface_prefix)InitState =
                    // TODO no unwrap?
                    mproto::decode_value(&init_state_buf).unwrap();

                let hooks = modrpc_wasm_glue::init::<$(interface_role_prefix)Role>(
                    service_id, endpoint_addr,
                    init_state,
                    host::modrpc_send_packet_bundle,
                    start,
                );
                *STATE.hooks.borrow_mut() = Some(hooks);
            }

            fn tick() {
                modrpc_wasm_glue::tick();
            }
        }

        impl guest::Guest for RoleImpl {
            $method_calls

            $required_impl_notifies
        }

        fn start(
            s: &mut modrpc::RoleSetup,
            stubs: $(interface_role_prefix)Stubs,
            hooks: &$(interface_role_prefix)Hooks,
        ) {
            $object_builds
        }

        export_$(camel_to_snake_case(interface_role_name))!(RoleImpl);
    }
}

pub fn js_wit_glue(
    project_js_package: &str,
    db: &Database,
    interface: &Interface,
    role_name: &str,
) -> js::Tokens {
    let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
        db.mproto_db(), Some(project_js_package), false, &interface.type_params,
    );

    ////////////////////////////////////
    // Objects

    let mut object_hooks_interface_fields: js::Tokens = quote! { };
    let mut object_impls_interface_fields: js::Tokens = quote! { };
    let mut required_impls: js::Tokens = quote! { };
    let mut method_resolvers: js::Tokens = quote! { };
    let mut method_notifies: js::Tokens = quote! { };
    let mut object_hooks: js::Tokens = quote! { };

    for object in &interface.objects {
        let Some(object_interface) = db.lookup_interface(&object.construct) else {
            panic!("failed to lookup interface '{}'", object.construct.name);
        };

        let Some(object_role_name) =
            object.get_role_for_parent_role(object_interface, role_name)
        else {
            panic!("object '{}' failed to find role arg '{}'", object.name, role_name);
        };
        let object_interface_role_name = &format!("{}{}", object.construct.name, object_role_name);

        let object_mproto_cx = &mproto_cx.with_type_args(
            &object_interface.type_params,
            &object.type_args,
        );

        for required_impls_list in &object_interface.required_impls {
            if !required_impls_list.roles.contains(object_role_name) { continue; }

            for required_impl in &required_impls_list.required_impls {
                let input_type =
                    mproto_codegen::codegen::js::js_type_tokens(object_mproto_cx, &required_impl.input_ty);
                let output_type =
                    mproto_codegen::codegen::js::js_type_tokens(object_mproto_cx, &required_impl.output_ty);

                let input_encoder =
                    mproto_codegen::codegen::js::js_type_encoder(object_mproto_cx, &required_impl.input_ty);
                let output_encoder =
                    mproto_codegen::codegen::js::js_type_encoder(object_mproto_cx, &required_impl.output_ty);

                let impl_name_camel = &snake_to_camel_case(
                    &format!("{}_{}", object.name, required_impl.name)
                );
                let impl_name_upper_camel = &snake_to_upper_camel_case(
                    &format!("{}_{}", object.name, required_impl.name)
                );

                required_impls = quote! {
                    $required_impls
                    $impl_name_camel(id: number, inputBuf: Uint8Array) {
                      const input = mproto.decodeValue($input_encoder, inputBuf.buffer);
                      const result = roleImpl.$impl_name_camel(input);
                      const outputBuf = new Uint8Array(mproto.encodeValue($output_encoder, result));
                      wasm.guest.notify$(impl_name_upper_camel)(id, outputBuf);
                    },
                };

                object_impls_interface_fields = quote! {
                    $object_impls_interface_fields
                    readonly $impl_name_camel: (input: $input_type) => $output_type;
                };
            }
        }

        let mut object_hook_methods: js::Tokens = quote! { };
        for methods_list in &object_interface.methods {
            if !methods_list.roles.contains(object_role_name) { continue; }

            for method in &methods_list.methods {
                let method_name_camel = &snake_to_camel_case(
                    &format!("{}_{}", object.name, method.name)
                );
                let method_name_upper_camel = &snake_to_upper_camel_case(
                    &format!("{}_{}", object.name, method.name)
                );

                if method.is_async {
                    method_resolvers = quote! {
                        $method_resolvers
                        const $(method_name_camel)Resolvers: { [id: number] : (r: any) => void } = { };
                    };

                    method_notifies = quote! {
                        $method_notifies
                        notify$(method_name_upper_camel)(id: number, resultBuf: Uint8Array) {
                          $(method_name_camel)Resolvers[id](resultBuf);
                          delete $(method_name_camel)Resolvers[id];
                        },
                    };

                    object_hook_methods = quote! {
                        $object_hook_methods
                        wasm.guest.$method_name_camel,
                        $(method_name_camel)Resolvers,
                    };
                }
            }
        }

        let mut object_hook_type_args: js::Tokens = quote! { };
        for type_arg in &object.type_args {
            // Use mproto_cx, not object_mproto_cx since any appearances of type parameters will be
            // from the interface, not from the object.
            let encoder = mproto_codegen::codegen::js::js_type_encoder(mproto_cx, type_arg);
            object_hook_type_args = quote! {
                $object_hook_type_args
                $encoder,
            };
        }

        if object_interface.methods.len() > 0 {
            let object_name_camel = &snake_to_camel_case(&object.name);

            let object_class = &mproto_cx.js_import_qualified(&mproto_codegen::ast::QualifiedIdentifier {
                name: object_interface_role_name.into(),
                module: object.construct.module.clone(),
            });

            object_hooks_interface_fields = quote! {
                $object_hooks_interface_fields
                readonly $object_name_camel: $object_class$(
                    // Use mproto_cx, not object_mproto_cx since any appearances of type parameters will be
                    // from the interface, not from the object.
                    mproto_codegen::codegen::js::js_type_args(
                        mproto_cx,
                        &object.type_args,
                        mproto_codegen::codegen::js::js_type_tokens,
                    )
                );
            };

            object_hooks = quote! {
                $object_hooks
                $object_name_camel: new $object_class(
                  $object_hook_type_args
                  $object_hook_methods
                ),
            };
        }
    }

    ////////////////////////////////////
    // index.ts

    let init_state = &js::import(project_js_package, format!("{}InitState", interface.name));
    let proto_init_state = &js::import(project_js_package, format!("Proto{}InitState", interface.name));

    let interface_role_name = &format!("{}{}", interface.name, role_name);
    let interface_role_name_snake = &camel_to_snake_case(interface_role_name);

    quote! {
        import * as mproto from "mproto";
        import * as modrpc from "modrpc";

        import { instantiate } from $("\"")../dist/wasm/$(interface_role_name_snake)_modrpc.js$("\"");

        export interface $(interface_role_name)Impl {
          $object_impls_interface_fields
        }

        export interface $(interface_role_name)Hooks {
          readonly modrpcReceivePacketBundle: (buf: Uint8Array) => void;
          $object_hooks_interface_fields
        }

        type IRunRole = modrpc.RunInterfaceRole<$init_state, $(interface_role_name)Hooks, $(interface_role_name)Impl>;
        export const Run$(interface_role_name): IRunRole = new class implements IRunRole {
          readonly initStateEncoder = $proto_init_state;

          run(
            serviceId: number,
            endpointAddr: bigint,
            sendPacketBundleFn: (buf: Uint8Array) => void,
            initState: $init_state,
            roleImpl: $(interface_role_name)Impl,
          ): Promise<[$(interface_role_name)Hooks, () => void]> {
            return run$(interface_role_name)(serviceId, endpointAddr, sendPacketBundleFn, initState, roleImpl);
          }
        }

        export async function run$(interface_role_name)(
          serviceId: number,
          endpointAddr: bigint,
          sendPacketBundleFn: (buf: Uint8Array) => void,
          initState: $init_state,
          roleImpl: $(interface_role_name)Impl,
        ): Promise<[$(interface_role_name)Hooks, () => void]> {
          let wasm: any;
          $method_resolvers

          wasm = await instantiate(fetchCompile, {
            "host": {
              modrpcSendPacketBundle: (buf: Uint8Array) => {
                sendPacketBundleFn(buf);
              },
              $required_impls
              $method_notifies
            },
          });

          wasm.main.init(serviceId, endpointAddr, mproto.encodeValue($proto_init_state, initState));

          const hooks = {
            modrpcReceivePacketBundle: (buf: Uint8Array) => {
              wasm.main.handlePacket(buf);
            },
            $object_hooks
          };

          let done = false;
          function stop() {
            done = true;
          }

          async function main() {
            while (!done) {
              await wasm.main.tick();
              await sleep(10);
            }
          }
          main();

          return [hooks, stop];
        }

        const isNode = typeof process !== "undefined" && process.versions && process.versions.node;
        let _fs: any;
        async function fetchCompile(url: string) {
          if (isNode) {
            _fs = _fs || await import("fs/promises");
            const relativeUrl = new URL($("`./wasm/${url}`, import.meta.url"));
            return WebAssembly.compile(await _fs.readFile(relativeUrl));
          } else {
            const relativeUrl = new URL($("`./wasm/${url}`, import.meta.url"));
            return fetch(relativeUrl).then(WebAssembly.compileStreaming);
          }
        }

        function sleep(ms: number) {
          return new Promise(resolve => setTimeout(resolve, ms));
        }
    }
}

fn witify_snake_name(n: &str) -> String {
    n.replace("_", "-")
}

use genco::prelude::*;

use mproto_codegen::codegen::{
    name_util::{camel_to_snake_case, snake_to_upper_camel_case},
    rust::{rust_type_default_value, rust_type_param_list},
};

use crate::{
    ast::{Interface, InterfaceObject},
    Database,
};

pub use project::{rust_project_gen, rust_proto_package_gen, rust_role_impl_gen};

mod project;

fn import_qualified(
    module: Option<impl AsRef<str>>,
    name: impl AsRef<str>,
) -> rust::Tokens {
    if let Some(module) = module {
        quote! { $(rust::import(format!("{}_modrpc", module.as_ref()), name.as_ref())) }
    } else {
        quote! { $(name.as_ref()) }
    }
}

pub fn rust_interface(
    db: &Database,
    interface: &Interface,
) -> rust::Tokens {
    let interface_builder = &rust::import("modrpc", "InterfaceBuilder");
    let interface_event = &rust::import("modrpc", "InterfaceEvent");
    let interface_schema = &rust::import("modrpc", "InterfaceSchema");

    let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
        db.mproto_db(), Some("crate::proto"), true, &interface.type_params,
    );

    let mut events_field_tokens = rust::Tokens::new();
    let mut events_constr_tokens = rust::Tokens::new();
    for events_list in &interface.events {
        for event in &events_list.events {
            let event_type_tokens = mproto_codegen::codegen::rust::rust_type_tokens(mproto_cx, &event.ty);
            events_field_tokens = quote!{
                $events_field_tokens
                pub $(&event.name): $interface_event<$event_type_tokens>,
            };

            let event_name_tokens = quote! { $("\"")$(&event.name)$("\"") };
            events_constr_tokens = quote! {
                $events_constr_tokens
                $(&event.name): ib.event($event_name_tokens),
            };
        }
    }

    let mut objects_field_tokens = rust::Tokens::new();
    let mut objects_constr_tokens = rust::Tokens::new();
    for object in &interface.objects {
        if db.lookup_interface(&object.construct).is_none() {
            panic!("failed to lookup interface '{}'", object.construct.name);
        };

        let object_type_args = &mproto_codegen::codegen::rust::rust_type_arg_list(
            mproto_cx, &object.type_args, None,
        );

        let object_interface_import = import_qualified(
            object.construct.module.as_ref(),
            format!("{}Interface", object.construct.name),
        );
        objects_field_tokens = quote! {
            $objects_field_tokens
            pub $(&object.name): $(object_interface_import)$(object_type_args),
        };

        objects_constr_tokens = quote! {
            $objects_constr_tokens
            $(&object.name): $(&object.construct.name)Interface::new(ib),
        };
    }

    let type_params = &rust_type_param_list(&interface.type_params, None, None);

    let tokens: rust::Tokens = quote! {
        pub struct $(&interface.name)Interface$(type_params) {
            $events_field_tokens
            $objects_field_tokens
        }

        impl$(type_params) $interface_schema for $(&interface.name)Interface$(type_params) {
            fn new(ib: &mut $interface_builder) -> Self {
                Self {
                    $events_constr_tokens
                    $objects_constr_tokens
                }
            }
        }
    };

    tokens
}

pub fn rust_interface_role(
    db: &Database,
    interface: &Interface,
    role_name: &str,
) -> rust::Tokens {
    let role_setup = &rust::import("modrpc", "RoleSetup");
    let event_tx = &rust::import("modrpc", "EventTx");
    let event_rx_builder = &rust::import("modrpc", "EventRxBuilder");
    let interface_role = &rust::import("modrpc", "InterfaceRole");
    let proto_interface = &rust::import("crate::interface", format!("{}Interface", interface.name));
    let init_state = &rust::import("crate::proto", format!("{}InitState", interface.name));
    let role_config = &rust::import("crate::proto", format!("{}{}Config", interface.name, role_name));

    let interface_role_name = &format!("{}{}", interface.name, role_name);
    let interface_role_name_snake = &camel_to_snake_case(interface_role_name);

    let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
        db.mproto_db(), Some("crate::proto"), true, &interface.type_params,
    );

    let mut events_hook_field_tokens = rust::Tokens::new();
    let mut events_hook_constr_field_tokens = rust::Tokens::new();
    let mut events_stubs_field_tokens = rust::Tokens::new();
    let mut events_stubs_constr_field_tokens = rust::Tokens::new();
    let mut events_clone_tokens = rust::Tokens::new();
    for events_list in &interface.events {
        let role_needs_event_tx =
            events_list.from_roles.iter().find(|x| x == &role_name).is_some();
        let role_needs_event_rx =
            events_list.to_roles.iter().find(|x| x == &role_name).is_some();

        for event in &events_list.events {
            let payload_ty = mproto_codegen::codegen::rust::rust_type_tokens(&mproto_cx, &event.ty);

            let hook_field_tokens: rust::Tokens =
                if role_needs_event_tx {
                    quote! {
                        pub $(&event.name): $event_tx<$(&payload_ty)>,
                    }
                } else { quote! { } };
            let hook_constr_tokens: rust::Tokens =
                if role_needs_event_tx {
                    quote! {
                        $(&event.name): setup.event_tx(i.$(&event.name)),
                    }
                } else { quote! { } };
            let stubs_field_tokens: rust::Tokens =
                if role_needs_event_rx {
                    quote! {
                        pub $(&event.name): $event_rx_builder<$(&payload_ty)>,
                    }
                } else { quote! { } };
            let stubs_constr_tokens: rust::Tokens =
                if role_needs_event_rx {
                    quote! {
                        $(&event.name): setup.event_rx(i.$(&event.name)),
                    }
                } else { quote! { } };
            let clone_tokens: rust::Tokens =
                if role_needs_event_tx {
                    quote! {
                        $(&event.name): self.$(&event.name).clone(),
                    }
                } else { quote! { } };

            events_hook_field_tokens = quote! {
                $events_hook_field_tokens
                $hook_field_tokens
            };
            events_hook_constr_field_tokens = quote! {
                $events_hook_constr_field_tokens
                $hook_constr_tokens
            };
            events_stubs_field_tokens = quote! {
                $events_stubs_field_tokens
                $stubs_field_tokens
            };
            events_stubs_constr_field_tokens = quote! {
                $events_stubs_constr_field_tokens
                $stubs_constr_tokens
            };
            events_clone_tokens = quote! {
                $events_clone_tokens
                $clone_tokens
            };
        }
    }

    let mut objects_hook_field_tokens = rust::Tokens::new();
    let mut objects_hook_constr_tokens = rust::Tokens::new();
    let mut objects_hook_constr_stubs_tokens = rust::Tokens::new();
    let mut objects_hook_constr_field_tokens = rust::Tokens::new();
    let mut objects_stubs_field_tokens = rust::Tokens::new();
    let mut objects_clone_tokens = rust::Tokens::new();
    let mut i = 0;
    for object in &interface.objects {
        let Some(object_interface) = db.lookup_interface(&object.construct) else {
            panic!("failed to lookup interface '{}'", object.construct.name);
        };

        let object_role_names = object.get_roles_for_parent_role(object_interface, role_name);
        if object_role_names.is_empty() {
            panic!("object '{}' failed to find roles '{}'", object.name, role_name);
        };

        for object_role_name in &object_role_names {
            let construct_module = object.construct.module.as_ref();
            let object_interface_role_name = &format!("{}{}", object.construct.name, object_role_name);

            let object_field_name =
                if object_role_names.len() > 1 {
                    &format!("{}_{}", object.name, camel_to_snake_case(object_role_name))
                } else {
                    &object.name
                };

            let object_type_args = &mproto_codegen::codegen::rust::rust_type_arg_list(
                mproto_cx, &object.type_args, None,
            );

            let hook_field_tokens = quote! {
                pub $(object_field_name): $(import_qualified(construct_module, object_interface_role_name))$(object_type_args),
            };

            let object_config: &rust::Tokens =
                &if object_interface.has_config_for_role(object_role_name) {
                    quote! { config.$(object_field_name) }
                } else {
                    quote! { $(import_qualified(construct_module, format!("{}Config", object_interface_role_name))) { } }
                };

            let object_init: &rust::Tokens =
                &if object_interface.has_state(db) {
                    quote! { init.$(object_field_name) }
                } else {
                    quote! { $(import_qualified(construct_module, format!("{}InitState", object.construct.name))) { } }
                };

            let mut hook_constr_tokens = quote! {
                let ($(object_field_name)_stubs, $(object_field_name)_hooks) =
                    $(import_qualified(construct_module, format!("{}Role", object_interface_role_name)))::setup_worker(
                        &i.$(&object.name), setup, &$(object_config), &$(object_init),
                    );
                let $(object_field_name)_builder = $(import_qualified(construct_module, format!("{}Builder", object_interface_role_name)))::new(
                    $("\"")$(interface_role_name_snake).$(&object.name)$("\""),
                    $(object_field_name)_hooks,
                    $(object_field_name)_stubs,
                    &$(object_config),
                    $(object_init).clone(),
                );
                let $(object_field_name) = $(object_field_name)_builder.create_handle(setup);
            };
            let hook_constr_field_tokens = quote! {
                $(object_field_name),
            };

            let clone_tokens = quote! {
                $(object_field_name): self.$(object_field_name).clone(),
            };

            // For impl setup, need to determine if the user needs to provide any `impl`s for this
            // role. If the user doesn't need to provide any `impl`s, we auto-implement the stubs for
            // this object.

            let stubs_field_tokens: rust::Tokens;
            let hook_constr_stubs_tokens: rust::Tokens;
            if object_interface.requires_impls_for_role(object_role_name) {
                // User needs to provide impls

                stubs_field_tokens = quote! {
                    pub $(object_field_name): $(import_qualified(construct_module, format!("{}Builder", object_interface_role_name)))$(object_type_args),
                };

                hook_constr_stubs_tokens = quote! {
                    $(object_field_name): $(object_field_name)_builder,
                };
            } else {
                // User does not need to provide impls

                hook_constr_tokens = quote! {
                    $hook_constr_tokens
                    $(object_field_name)_builder.build(setup);
                };

                stubs_field_tokens = quote! { };

                hook_constr_stubs_tokens = quote! { };
            }

            hook_constr_tokens = quote! {
                setup.push_object_path($("\"")$(&object.name)$("\""));
                $hook_constr_tokens
                setup.pop_object_path();
            };

            if i > 0 {
                objects_hook_field_tokens = quote! {
                    $objects_hook_field_tokens
                    $hook_field_tokens
                };
                objects_hook_constr_tokens = quote! {
                    $objects_hook_constr_tokens
                    $hook_constr_tokens
                };
                objects_hook_constr_stubs_tokens = quote! {
                    $objects_hook_constr_stubs_tokens
                    $hook_constr_stubs_tokens
                };
                objects_hook_constr_field_tokens = quote! {
                    $objects_hook_constr_field_tokens
                    $hook_constr_field_tokens
                };
                objects_stubs_field_tokens = quote! {
                    $objects_stubs_field_tokens
                    $stubs_field_tokens
                };
                objects_clone_tokens = quote! {
                    $objects_clone_tokens
                    $clone_tokens
                };
            } else {
                objects_hook_field_tokens = hook_field_tokens;
                objects_stubs_field_tokens = stubs_field_tokens;
                objects_hook_constr_tokens = hook_constr_tokens;
                objects_hook_constr_stubs_tokens = hook_constr_stubs_tokens;
                objects_hook_constr_field_tokens = hook_constr_field_tokens;
                objects_clone_tokens = clone_tokens;
            }

            i += 1;
        }
    }

    ////////////////////////////////////

    let type_params = &rust_type_param_list(&interface.type_params, None, None);
    let type_params_no_lifetime = &rust_type_param_list(&interface.type_params, None, None);
    let type_params_no_lifetime_bounded = &rust_type_param_list(&interface.type_params, None, Some(quote! { mproto::Owned }));

    let mut interface_type_params_tokens = rust::Tokens::new();
    if interface.type_params.len() > 0 {
        interface_type_params_tokens = quote! { $(&interface.type_params[0]) };
        for type_param in &interface.type_params[1..] {
            interface_type_params_tokens = quote! { $interface_type_params_tokens, $(type_param) };
        }
    }
    let interface_type_params_tokens = &interface_type_params_tokens;

    ////////////////////////////////////

    let phantom_data: &rust::Tokens =
        if interface.type_params.len() > 1 {
            &quote! { _phantom: std::marker::PhantomData<($interface_type_params_tokens)>, }
        } else if interface.type_params.len() == 1 {
            &quote! { _phantom: std::marker::PhantomData<$interface_type_params_tokens>, }
        } else {
            &quote! { }
        };

    let role_config_def = db.mproto_db().lookup_type_def(
        &mproto_codegen::ast::QualifiedIdentifier::local(
            format!("{}{}Config", interface.name, role_name)
        )
    )
    .expect("role config struct def");

    let init_state_with_args: rust::Tokens =
        if interface.has_state(db) {
            quote! { $(init_state)$(type_params_no_lifetime) }
        } else {
            quote! { $(init_state) }
        };

    quote! {
        pub struct $(interface_role_name)Hooks$(type_params) {
            $events_hook_field_tokens
            $objects_hook_field_tokens
            $phantom_data
        }

        pub struct $(interface_role_name)Stubs$(type_params) {
            $events_stubs_field_tokens
            $objects_stubs_field_tokens
            $phantom_data
        }

        pub struct $(interface_role_name)Role$(type_params_no_lifetime) {
            $phantom_data
        }

        impl$(type_params_no_lifetime_bounded) $interface_role for $(&interface.name)$(role_name)Role$(type_params_no_lifetime) {
            type Interface = $(proto_interface)$(type_params_no_lifetime);
            type Config = $(role_config)$(rust_type_param_list(&role_config_def.params, None, None));
            type Init = $init_state_with_args;
            type Stubs = $(interface_role_name)Stubs$(type_params);
            type Hooks = $(interface_role_name)Hooks$(type_params);

            fn setup_worker(
                i: &Self::Interface,
                setup: &mut $role_setup,
                config: &Self::Config,
                init: &Self::Init,
            ) -> (Self::Stubs, Self::Hooks) {
                $objects_hook_constr_tokens

                (
                    Self::Stubs {
                        $events_stubs_constr_field_tokens
                        $objects_hook_constr_stubs_tokens
                        $(if !interface.type_params.is_empty() { _phantom: std::marker::PhantomData, })
                    },
                    Self::Hooks {
                        $events_hook_constr_field_tokens
                        $objects_hook_constr_field_tokens
                        $(if !interface.type_params.is_empty() { _phantom: std::marker::PhantomData, })
                    },
                )
            }
        }

        impl$(type_params) Clone for $(interface_role_name)Hooks$(type_params) {
            fn clone(&self) -> Self {
                Self {
                    $events_clone_tokens
                    $objects_clone_tokens
                    $(if !interface.type_params.is_empty() { _phantom: std::marker::PhantomData, })
                }
            }
        }
    }
}

pub fn rust_interface_role_impl(
    db: &Database,
    interface: &Interface,
    role_name: &str,
) -> rust::Tokens {
    if !interface.has_private_event_impls_for_role(role_name)
        && !interface.has_methods_for_role(role_name)
        && !interface.has_config_for_role(role_name)
        && !interface.has_impls_for_role(role_name)
    {
        return quote! { };
    }

    let role_config = &rust::import("crate::proto", format!("{}{}Config", interface.name, role_name));
    let init_state = &rust::import("crate::proto", format!("{}InitState", interface.name));
    let role_setup = &rust::import("modrpc", "RoleSetup");

    let interface_role_name = &format!("{}{}", interface.name, role_name);

    let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
        db.mproto_db(), Some("crate::proto"), true, &interface.type_params,
    );

    let type_params = &rust_type_param_list(&interface.type_params, None, None);
    let type_params_bounded = &rust_type_param_list(&interface.type_params, None, Some(quote! { mproto::Owned }));
    let type_params_no_lifetime = &rust_type_param_list(&interface.type_params, None, None);

    let mut event_handler_stubs = rust::Tokens::new();
    for events_list in &interface.events {
        if !events_list.to_roles.iter().find(|x| x.as_str() == role_name).is_some() { continue; }

        for event in &events_list.events {
            if !event.is_private { continue; }

            let payload_ty = mproto_codegen::codegen::rust::rust_type_tokens(mproto_cx, &event.ty);

            event_handler_stubs = quote! {
                $event_handler_stubs
                let $(&event.name) = self.stubs.$(&event.name)
                    .queued(setup, async move |_source, _event: $(payload_ty)| {
                    })
                    .load_balance();
            };
        }
    }

    let mut methods = rust::Tokens::new();
    for methods_list in &interface.methods {
        if !methods_list.roles.iter().find(|x| x.as_str() == role_name).is_some() { continue; }

        for method in &methods_list.methods {
            let input_ty = &mproto_codegen::codegen::rust::rust_type_tokens(mproto_cx, &method.input_ty);
            let output_ty = &mproto_codegen::codegen::rust::rust_type_tokens(mproto_cx, &method.output_ty);

            methods = quote! {
                $methods
                pub $(
                    if method.is_async { async } else { }
                ) fn $(&method.name)(
                    &self,
                    input: impl mproto::Encode + for<'a> mproto::Decode<'a> + mproto::Compatible<$input_ty>,
                ) -> $output_ty {
                    $(rust_type_default_value(
                        mproto_cx,
                        &method.output_ty,
                    ))
                }
            };
        }
    }

    if !methods.is_empty() {
        methods = quote! {
            impl$(type_params_bounded) $(interface_role_name)$(type_params) {
                $methods
            }
        };
    }

    let role_config_def = db.mproto_db().lookup_type_def(
        &mproto_codegen::ast::QualifiedIdentifier::local(
            format!("{}{}Config", interface.name, role_name)
        )
    )
    .expect("role config struct def");

    let init_state_with_args: &rust::Tokens =
        &if interface.has_state(db) {
            quote! { $(init_state)$(type_params_no_lifetime) }
        } else {
            quote! { $(init_state) }
        };

    quote! {
        #[derive(Clone)]
        pub struct $(interface_role_name)$(type_params) {
            hooks: crate::$(interface_role_name)Hooks$(type_params),
        }

        pub struct $(interface_role_name)Builder$(type_params) {
            pub name: &'static str,
            pub hooks: crate::$(interface_role_name)Hooks$(type_params),
            pub stubs: crate::$(interface_role_name)Stubs$(type_params),
            pub init: $init_state_with_args,
        }

        impl$(type_params_bounded) $(interface_role_name)Builder$(type_params) {
            pub fn new(
                name: &'static str,
                hooks: crate::$(interface_role_name)Hooks$(type_params),
                stubs: crate::$(interface_role_name)Stubs$(type_params),
                config: &$(role_config)$(rust_type_param_list(&role_config_def.params, None, None)),
                init: $init_state_with_args,
            ) -> Self {
                Self { name, hooks, stubs, init }
            }

            pub fn create_handle(
                &self,
                setup: &$role_setup,
            ) -> crate::$(interface_role_name)$(type_params) {
                crate::$(interface_role_name) {
                    hooks: self.hooks.clone(),
                }
            }

            pub fn build(
                self,
                setup: &$role_setup,
            ) {
                $event_handler_stubs
            }
        }

        $methods
    }
}

pub fn request_impl_stub(
    mproto_cx: &mproto_codegen::codegen::CodegenCx,
    object: &InterfaceObject,
) -> rust::Tokens {
    let object_name_upper_camel = &snake_to_upper_camel_case(&object.name);

    let request_ty = mproto_codegen::codegen::rust::rust_type_lazy_tokens(
        mproto_cx, &object.type_args[0],
    );
    let response_ty = mproto_codegen::codegen::rust::rust_type_tokens(
        mproto_cx, &object.type_args[1],
    );

    quote! {
        pub struct $(object_name_upper_camel)Handler {
        }

        impl std_modrpc::RequestHandlerReply for $(object_name_upper_camel)Handler {
            type Input<'a> = $(request_ty);
            type Output = $(response_ty);

            async fn call<'a>(
                &'a mut self,
                mut cx: std_modrpc::RequestContext<'a, Self::Output>,
                request: Self::Input<'a>,
            ) -> Option<()> {
                None
            }
        }
    }
}

pub struct RustInterfaceImpl {
    pub main_rs: rust::Tokens,
    pub handlers: Vec<(String, rust::Tokens)>,
}

impl RustInterfaceImpl {
    pub fn generate(
        mproto_db: &mproto_codegen::Database,
        package_name: &str,
        interface: &Interface,
        role_name: &str,
    ) -> Self {
        let interface_name = &interface.name;
        let interface_role_name = &format!("{}{}", interface.name, role_name);
        let package_name = &package_name.replace("-", "_");

        let def_crate = format!("{}_modrpc", package_name);
        let def_crate = def_crate.as_str();
        let def_source = Some(def_crate);

        let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
            mproto_db, def_source, true, &interface.type_params,
        );

        let rc = &rust::import("std::rc", "Rc");
        let ref_cell = &rust::import("std::cell", "RefCell");
        let block_on = &rust::import("futures_lite::future", "block_on");
        let modmesh_session = &rust::import("modmesh_endpoint", "ModmeshSession");

        let mut register_handlers = rust::Tokens::new();
        let mut handlers = Vec::new();
        for object in &interface.objects {
            if object.construct.name == "Request" && object.construct.module.as_ref().map(|s| s.as_ref()) == Some("std") {
                // TODO also ensure that this interface role acts as the request server role.

                let object_name_upper_camel = &snake_to_upper_camel_case(&object.name);
                register_handlers = quote! {
                    $register_handlers
                    cx.stubs.$(&object.name).build_replier(s, $(object_name_upper_camel)Handler {});
                };

                handlers.push((object.name.clone(), request_impl_stub(mproto_cx, object)));
            }
        }

        let main_rs = quote! {
            struct $(interface_role_name)State {
            }

            fn main() {
                let state = Rc::new(RefCell::new($(interface_role_name)State {
                }));

                loop {
                    $block_on(async move {
                        if let Ok(mut modmesh) = $modmesh_session::connect(("127.0.0.1", 13016)).await {
                            modmesh.host_service::<$def_crate::$interface_role_name>(
                                "untitled-service".into(),
                                $(def_crate)::$(interface_name)InitState { },
                                |cx| start_$(package_name)(cx, state.clone())
                            ).await;

                            modmesh.run().await;
                        }

                        // TODO
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        println!("Retrying modmesh connection...");
                    });
                }
            }

            fn start_$(package_name)(
                cx: modrpc::RoleWorkerContext<$def_crate::$(interface_role_name)Role>,
                state: $rc<$ref_cell<$(interface_role_name)State>>,
            ) {
                $register_handlers
            }
        };

        Self {
            main_rs,
            handlers,
        }
    }
}

pub fn rust_interface_init_state_tokens(
    db: &mut Database,
    interface: &Interface,
) -> rust::Tokens {
    let init_state_struct = crate::codegen::interface_init_state(db, interface);
    let init_state_struct_id = db.mproto_db_mut().local_mut().new_type_def(init_state_struct);

    let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
        db.mproto_db(), Some("crate::proto"), true, &interface.type_params,
    );

    quote! {
        $(mproto_codegen::codegen::rust::rust_type_def(
            &mproto_cx,
            db.mproto_db().local().type_def(init_state_struct_id)
        ))
    }
}

pub fn rust_interface_role_config_tokens(
    db: &mut Database,
    interface: &Interface,
    role_name: &str,
) -> rust::Tokens {
    let config_struct = crate::codegen::interface_role_config(
        db,
        &mproto_codegen::codegen::CodegenCx::new_with_type_params(
            db.mproto_db(), Some("crate::proto"), true, &interface.type_params,
        ),
        interface,
        role_name,
    );
    let config_struct_id = db.mproto_db_mut().local_mut().new_type_def(config_struct);

    quote! {
        $(mproto_codegen::codegen::rust::rust_type_def(
            &mproto_codegen::codegen::CodegenCx::new_with_type_params(
                db.mproto_db(), Some("crate::proto"), true, &interface.type_params,
            ),
            db.mproto_db().local().type_def(config_struct_id)
        ))
    }
}

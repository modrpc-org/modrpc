use genco::prelude::*;
use mproto_codegen::codegen::{
    name_util::snake_to_camel_case,
    js::js_type_param_list,
};

use crate::{
    ast::Interface,
    Database,
};

pub use project::{js_project_gen, js_proto_package_gen};

pub mod project;

pub fn js_wit_object_class(
    db: &Database,
    object_interface: &Interface,
    object_role_name: &String,
) -> js::Tokens {
    let mproto_cx = &mproto_codegen::codegen::CodegenCx::new_with_type_params(
        db.mproto_db(), Some("./proto"), true, &object_interface.type_params,
    );

    let mut method_fields: js::Tokens = quote! { };
    let mut method_args: js::Tokens = quote! { };
    let mut method_set_fields: js::Tokens = quote! { };
    let mut methods: js::Tokens = quote! { };

    for methods_list in &object_interface.methods {
        if !methods_list.roles.contains(object_role_name) { continue; }

        for method in &methods_list.methods {
            let input_type =
                mproto_codegen::codegen::js::js_type_tokens(mproto_cx, &method.input_ty);
            let output_type =
                mproto_codegen::codegen::js::js_type_tokens(mproto_cx, &method.output_ty);

            let input_encoder =
                mproto_codegen::codegen::js::js_type_encoder(mproto_cx, &method.input_ty);
            let output_encoder =
                mproto_codegen::codegen::js::js_type_encoder(mproto_cx, &method.output_ty);

            let method_name_camel = &snake_to_camel_case(&method.name);

            if method.is_async {
                method_fields = quote! {
                    $method_fields
                    private $(method_name_camel)Wasm: (input: Uint8Array) => any;
                    private $(method_name_camel)Resolvers: { [id: number] : (r: any) => void };
                };

                method_args = quote! {
                    $method_args
                    $(method_name_camel)Wasm: (input: Uint8Array) => any,
                    $(method_name_camel)Resolvers: { [id: number] : (r: any) => void },
                };

                method_set_fields = quote! {
                    $method_set_fields
                    this.$(method_name_camel)Wasm = $(method_name_camel)Wasm;
                    this.$(method_name_camel)Resolvers = $(method_name_camel)Resolvers;
                };

                methods = quote! {
                    $methods

                    public async $method_name_camel(input: $input_type): Promise<$output_type> {
                      const inputBuf = new Uint8Array(mproto.encodeValue($input_encoder, input));

                      const id = this.$(method_name_camel)Wasm(inputBuf);
                      const promise: Promise<any> = new Promise(resolve => {
                        this.$(method_name_camel)Resolvers[id] = resolve;
                      });

                      const outputBuf = await promise;
                      return mproto.decodeValue($output_encoder, outputBuf.buffer);
                    }
                };
            }
        }
    }

    let mut type_param_encoder_fields: js::Tokens = quote! { };
    let mut type_param_encoder_args: js::Tokens = quote! { };
    let mut type_param_encoder_set_fields: js::Tokens = quote! { };

    for type_param in &object_interface.type_params {
        let type_param_name = type_param;
        type_param_encoder_fields = quote! {
            $type_param_encoder_fields
            private $(type_param_name)Encoder: mproto.EncoderDecoder<$(type_param_name)>;
        };
        type_param_encoder_args = quote! {
            $type_param_encoder_args
            $(type_param_name)Encoder: mproto.EncoderDecoder<$(type_param_name)>,
        };
        type_param_encoder_set_fields  = quote! {
            $type_param_encoder_set_fields
            this.$(type_param_name)Encoder = $(type_param_name)Encoder;
        };
    }

    let object_interface_role_name = &format!("{}{}", object_interface.name, object_role_name);
    let type_params_list = js_type_param_list(
        &object_interface.type_params,
    );

    quote! {
        class $object_interface_role_name$(type_params_list) {
          $type_param_encoder_fields
          $method_fields

          constructor(
            $type_param_encoder_args
            $method_args
          ) {
            $type_param_encoder_set_fields
            $method_set_fields
          }

          $methods
        }
    }
}

pub fn js_interface_init_state_tokens(
    db: &mut Database,
    interface: &Interface,
) -> js::Tokens {
    let init_state_struct = crate::codegen::interface_init_state(db, interface);
    let init_state_struct_id = db.mproto_db_mut().local_mut().new_type_def(init_state_struct);

    quote! {
        $(mproto_codegen::codegen::js::js_type_def(
            &mproto_codegen::codegen::CodegenCx::new(db.mproto_db(), None, true),
            db.mproto_db().local().type_def(init_state_struct_id)
        ))
    }
}

use nom::{
  branch::alt,
  bytes::complete::{tag, take_while1},
  character::complete::{char, multispace0},
  combinator::{map, opt, cut},
  error::ParseError,
  multi::separated_list0,
  sequence::{preceded, terminated},
  IResult,
};

use mproto_codegen;
use mproto_codegen::ast::TypeDef;

use crate::ast::{
    Import,
    Interface,
    InterfaceEvent,
    InterfaceEventsList,
    InterfaceObject,
    InterfaceConfigItem,
    InterfaceConfigList,
    InterfaceRequiredImpl,
    InterfaceRequiredImplsList,
    InterfaceState,
    ObjectMethodsList,
    ObjectMethod,
    QualifiedIdentifier,
    Schema,
};

pub enum SchemaItem {
    Import(Import),
    Interface(Interface),
    TypeDef(TypeDef),
}

enum InterfaceItem {
    Events(InterfaceEventsList),
    State(Vec<InterfaceState>),
    Objects(Vec<InterfaceObject>),
    ConfigList(InterfaceConfigList),
    RequiredImpls(InterfaceRequiredImplsList),
    ObjectMethods(ObjectMethodsList),
}

fn import(i: &str) -> IResult<&str, Import> {
    let (i, _) = tag("import")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char('"')(i)?;
    let (i, path) = take_while1(|x: char| x != '"')(i)?;
    let (i, _) = char('"')(i)?;

    Ok((i, Import {
        name: name.to_string(),
        path: path.to_string(),
    }))
}

fn interface_def(i: &str) -> IResult<&str, Interface> {
    let (i, _) = tag("interface")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, type_params) = opt(mproto_codegen::parse::type_params_list)(i)?;
    let (i, _) = multispace0(i)?;
    let (i, roles) = roles_list(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char('{')(i)?;
    let (i, _) = multispace0(i)?;

    let (i, items) = separated_list0(multispace0, interface_item)(i)?;

    let (i, _) = multispace0(i)?;
    let (i, _) = char('}')(i)?;

    let mut events = vec![];
    let mut state = vec![];
    let mut objects = vec![];
    let mut config = vec![];
    let mut required_impls = vec![];
    let mut methods = vec![];
    for item in items {
        match item {
            InterfaceItem::Events(item_events) => {
                events.push(item_events);
            }
            InterfaceItem::State(item_state_fields) => {
                state.extend(item_state_fields);
            }
            InterfaceItem::Objects(item_objects) => {
                objects.extend(item_objects);
            }
            InterfaceItem::ConfigList(item_config_list) => {
                config.push(item_config_list);
            }
            InterfaceItem::RequiredImpls(item_required_impls) => {
                required_impls.push(item_required_impls);
            }
            InterfaceItem::ObjectMethods(item_object_methods) => {
                methods.push(item_object_methods);
            }
        }
    }

    let interface = Interface {
        name: name.into(),
        type_params: type_params.unwrap_or_else(|| Vec::new()),
        roles,
        events,
        objects,
        config,
        required_impls,
        methods,
        state,
    };

    Ok((i, interface))
}

fn interface_item<'a>(i: &'a str) -> IResult<&'a str, InterfaceItem> {
    alt((
        map(interface_events_list, |events| InterfaceItem::Events(events)),
        map(interface_state_list, |state_fields| InterfaceItem::State(state_fields)),
        map(interface_objects, |objects| InterfaceItem::Objects(objects)),
        map(interface_config_list, |config_list| InterfaceItem::ConfigList(config_list)),
        map(interface_required_impls_list, |required_impls| InterfaceItem::RequiredImpls(required_impls)),
        map(object_methods_list, |object_methods| InterfaceItem::ObjectMethods(object_methods)),
    ))(i)
}

fn opt_trailing_comma<I, O, E: ParseError<I>>(
    f: impl FnMut(I) -> IResult<I, O, E>,
) -> impl FnMut(I) -> IResult<I, O, E>
    where
        I: nom::Slice<std::ops::RangeFrom<usize>> + nom::InputIter + Clone,
        <I as nom::InputIter>::Item: nom::AsChar,
{
    terminated(f, opt(char(',')))
}

fn interface_events_list<'a>(i: &'a str) -> IResult<&'a str, InterfaceEventsList> {
    let (i, _) = tag("events")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, from_roles) = roles_list(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = tag("->")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, to_roles) = roles_list(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char('{')(i)?;
    let (i, _) = multispace0(i)?;

    let (i, events) = opt_trailing_comma(
        separated_list0(
            preceded(multispace0, char(',')),
            preceded(multispace0, interface_event)
        ),
    )(i)?;

    let (i, _) = multispace0(i)?;
    let (i, _) = char('}')(i)?;

    let interface_events_list = InterfaceEventsList {
        from_roles,
        to_roles,
        events,
    };

    Ok((i, interface_events_list))
}

fn interface_event<'a>(i: &'a str) -> IResult<&'a str, InterfaceEvent> {
    let (i, private) = opt(preceded(tag("private"), multispace0))(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = tag(":")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, ty) = mproto_codegen::parse::ty(i)?;

    let interface_event = InterfaceEvent {
        name: name.to_string(),
        ty,
        is_private: private.is_some(),
    };

    Ok((i, interface_event))
}

fn roles_list(i: &str) -> IResult<&str, Vec<String>> {
    let (i, _) = char('@')(i)?;
    let (i, _) = char('(')(i)?;
    let (i, roles) = separated_list0(
        preceded(multispace0, char(',')),
        preceded(multispace0, identifier),
    )(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = opt(char(','))(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char(')')(i)?;

    let roles = roles.into_iter().map(|p| p.into()).collect();

    Ok((i, roles))
}

fn interface_object(i: &str) -> IResult<&str, InterfaceObject> {
    let (i, name) = identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = tag(":")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, construct) = qualified_identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, type_args) = opt(mproto_codegen::parse::type_args_list)(i)?;
    let (i, _) = multispace0(i)?;
    let (i, role_args) = roles_list(i)?;

    let interface_object = InterfaceObject {
        name: name.to_string(),
        construct,
        type_args: type_args.unwrap_or_else(|| Vec::new()),
        role_args,
    };

    Ok((i, interface_object))
}

fn interface_objects<'a>(i: &'a str) -> IResult<&'a str, Vec<InterfaceObject>> {
    preceded(
        tag("objects"),
        preceded(
            preceded(multispace0, char('{')),
            cut(terminated(
                opt_trailing_comma(separated_list0(
                    preceded(multispace0, char(',')),
                    preceded(multispace0, interface_object),
                )),
                preceded(multispace0, char('}')),
            ))
        ),
    )(i)
}

fn interface_state_list<'a>(i: &'a str) -> IResult<&'a str, Vec<InterfaceState>> {
    let (i, _) = tag("state")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char('{')(i)?;
    let (i, _) = multispace0(i)?;

    let (i, state_fields) = opt_trailing_comma(
        separated_list0(
            preceded(multispace0, char(',')),
            preceded(multispace0, interface_state),
        ),
    )(i)?;

    let (i, _) = multispace0(i)?;
    let (i, _) = char('}')(i)?;

    Ok((i, state_fields))
}

fn interface_state<'a>(i: &'a str) -> IResult<&'a str, InterfaceState> {
    let (i, name) = identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = tag(":")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, ty) = mproto_codegen::parse::ty(i)?;

    Ok((i, InterfaceState { name: name.to_string(), ty }))
}

fn interface_config_list<'a>(i: &'a str) -> IResult<&'a str, InterfaceConfigList> {
    let (i, _) = tag("config")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, roles) = roles_list(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char('{')(i)?;
    let (i, _) = multispace0(i)?;

    let (i, items) = opt_trailing_comma(
        separated_list0(
            preceded(multispace0, char(',')),
            preceded(multispace0, interface_config_item)
        ),
    )(i)?;

    let (i, _) = multispace0(i)?;
    let (i, _) = char('}')(i)?;

    let interface_config_list = InterfaceConfigList {
        roles,
        items,
    };

    Ok((i, interface_config_list))
}

fn interface_config_item<'a>(i: &'a str) -> IResult<&'a str, InterfaceConfigItem> {
    let (i, name) = identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = tag(":")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, ty) = mproto_codegen::parse::ty(i)?;

    Ok((i, InterfaceConfigItem { name: name.to_string(), ty }))
}

fn interface_required_impls_list<'a>(i: &'a str) -> IResult<&'a str, InterfaceRequiredImplsList> {
    let (i, _) = tag("impl")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, roles) = roles_list(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char('{')(i)?;
    let (i, _) = multispace0(i)?;

    let (i, required_impls) = opt_trailing_comma(
        separated_list0(
            preceded(multispace0, char(',')),
            preceded(multispace0, interface_required_impl)
        ),
    )(i)?;

    let (i, _) = multispace0(i)?;
    let (i, _) = char('}')(i)?;

    let interface_required_impls_list = InterfaceRequiredImplsList {
        roles,
        required_impls,
    };

    Ok((i, interface_required_impls_list))
}

fn interface_required_impl<'a>(i: &'a str) -> IResult<&'a str, InterfaceRequiredImpl> {
    let (i, fn_decl) = fn_decl(i)?;

    let interface_required_impl = InterfaceRequiredImpl {
        name: fn_decl.name,
        is_async: fn_decl.is_async,
        input_ty: fn_decl.input_ty,
        output_ty: fn_decl.output_ty,
    };

    Ok((i, interface_required_impl))
}

fn object_methods_list<'a>(i: &'a str) -> IResult<&'a str, ObjectMethodsList> {
    let (i, _) = tag("methods")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, roles) = roles_list(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = char('{')(i)?;
    let (i, _) = multispace0(i)?;

    let (i, methods) = opt_trailing_comma(
        separated_list0(
            preceded(multispace0, char(',')),
            preceded(multispace0, object_method)
        ),
    )(i)?;

    let (i, _) = multispace0(i)?;
    let (i, _) = char('}')(i)?;

    let object_methods_list = ObjectMethodsList {
        roles,
        methods,
    };

    Ok((i, object_methods_list))
}

fn object_method<'a>(i: &'a str) -> IResult<&'a str, ObjectMethod> {
    let (i, fn_decl) = fn_decl(i)?;

    let object_method = ObjectMethod {
        name: fn_decl.name,
        is_async: fn_decl.is_async,
        input_ty: fn_decl.input_ty,
        output_ty: fn_decl.output_ty,
    };

    Ok((i, object_method))
}

struct FnDecl {
    pub name: String,
    pub is_async: bool,
    pub input_ty: mproto_codegen::ast::Type,
    pub output_ty: mproto_codegen::ast::Type,
}

fn fn_decl<'a>(i: &'a str) -> IResult<&'a str, FnDecl> {
    let (i, name) = identifier(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = tag(":")(i)?;
    let (i, _) = multispace0(i)?;

    let (i, maybe_async): (_, Option<&str>) = opt(|i| {
        let (i, async_tag) = tag("async")(i)?;
        let (i, _) = multispace0(i)?;
        Ok((i, async_tag))
    })(i)?;
    let is_async = maybe_async.is_some();

    let (i, input_ty) = mproto_codegen::parse::ty(i)?;
    let (i, _) = multispace0(i)?;
    let (i, _) = tag("->")(i)?;
    let (i, _) = multispace0(i)?;
    let (i, output_ty) = mproto_codegen::parse::ty(i)?;

    let fn_decl = FnDecl {
        name: name.to_string(),
        is_async,
        input_ty,
        output_ty,
    };

    Ok((i, fn_decl))
}

pub fn root<'a>(i: &'a str) -> IResult<&'a str, Vec<SchemaItem>> {
    separated_list0(
        multispace0,
        alt((
            map(import, |i| SchemaItem::Import(i)),
            map(interface_def, |i| SchemaItem::Interface(i)),
            map(mproto_codegen::parse::type_def, |t| SchemaItem::TypeDef(t)),
        )),
    )(i)
}

fn identifier(i: &str) -> IResult<&str, &str> {
    take_while1(
        |x: char| x.is_alphanumeric() || x.is_digit(10) || x == '_'
    )(i)
}

fn qualified_identifier(i: &str) -> IResult<&str, QualifiedIdentifier> {
    let (i, maybe_module): (_, Option<&str>) = opt(|i| {
        let (i, module) = identifier(i)?;
        let (i, _) = char('.')(i)?;
        Ok((i, module))
    })(i)?;

    let (i, name) = identifier(i)?;

    Ok((i, QualifiedIdentifier {
        name: name.to_string(),
        module: maybe_module.map(|x| x.to_string()),
    }))
}

// TODO Proper error type
pub fn parse_schema(i: &str) -> Result<Schema, String> {
    let (_, schema_items) = root(i).map_err(|e| e.to_string())?;

    let mut schema = Schema {
        imports: Vec::new(),
        interfaces: Vec::new(),
        type_defs: Vec::new(),
    };

    for item in schema_items {
        match item {
            SchemaItem::Import(i) => {
                schema.imports.push(i);
            }
            SchemaItem::Interface(i) => {
                schema.interfaces.push(i);
            }
            SchemaItem::TypeDef(t) => {
                schema.type_defs.push(t);
            }
        }
    }

    Ok(schema)
}

pub fn parse_file(path: impl AsRef<std::path::Path>) -> Result<Schema, String> {
    use std::io::Read;

    // Open input file
    let Ok(mut file) = std::fs::File::open(path.as_ref()) else {
        return Err(format!("Failed to open file '{}'", path.as_ref().display()));
    };

    // Load file to string
    let mut file_str = String::new();
    if let Err(e) = file.read_to_string(&mut file_str) {
        return Err(format!("Failed to read file '{}': {e}", path.as_ref().display()));
    }

    // Remove comments
    let mut schema_str = file_str.lines()
        .map(|line| {
            if let Some(index) = line.find("//") {
                // Return the slice from the start of the line up to the comment marker
                &line[..index]
            } else {
                // If no comment is found, return the entire line
                line
            }
        })
        .collect::<Vec<&str>>()
        .join("\n");
    schema_str += "\n";

    parse_schema(&schema_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_events() {
        use mproto_codegen::ast::PrimitiveType::*;
        use mproto_codegen::ast::Type;

        let data = "interface Foo<T, U, V, > @(server, client) { events @(server, foo) -> @(client, bar) { bar : u32, private baz : i8, t: T } }";
        let (_, parsed) = interface_def(data).unwrap();

        assert_eq!(
            parsed,
            Interface {
                name: "Foo".into(),
                type_params: vec!["T".to_string(), "U".to_string(), "V".to_string()],
                roles: vec!["server".to_string(), "client".to_string()],
                events: vec![
                    InterfaceEventsList {
                        from_roles: vec!["server".to_string(), "foo".to_string()],
                        to_roles: vec!["client".to_string(), "bar".to_string()],
                        events: vec![
                            InterfaceEvent { name: "bar".into(), ty: Type::Primitive(U32), is_private: false },
                            InterfaceEvent { name: "baz".into(), ty: Type::Primitive(I8), is_private: true },
                            InterfaceEvent { name: "t".into(), ty: Type::local("T"), is_private: false },
                        ],
                    },
                ],
                objects: vec![],
                config: vec![],
                required_impls: vec![],
                methods: vec![],
                state: vec![],
            }
        );
    }

    #[test]
    fn interface_objects() {
        use mproto_codegen::ast::PrimitiveType;
        use mproto_codegen::ast::Type;

        let data = "interface Foo<T, U, V, > @(Server, Client) { objects { foo_the_bar: Request<T, result<U, V>> @(Server, Client), bar_the_baz: Request<T, result<U, V>> @(Client, Server) } }";
        let (_, parsed) = interface_def(data).unwrap();

        assert_eq!(
            parsed,
            Interface {
                name: "Foo".into(),
                type_params: vec!["T".to_string(), "U".to_string(), "V".to_string()],
                roles: vec!["Server".to_string(), "Client".to_string()],
                events: vec![],
                objects: vec![
                    InterfaceObject {
                        name: "foo_the_bar".to_string(),
                        construct: QualifiedIdentifier::local("Request"),
                        type_args: vec![
                            Type::local("T"),
                            Type::Primitive(PrimitiveType::Result(
                                Box::new(Type::local("U")),
                                Box::new(Type::local("V")),
                            )),
                        ],
                        role_args: vec!["Server".to_string(), "Client".to_string()],
                    },
                    InterfaceObject {
                        name: "bar_the_baz".to_string(),
                        construct: QualifiedIdentifier::local("Request"),
                        type_args: vec![
                            Type::local("T"),
                            Type::Primitive(PrimitiveType::Result(
                                Box::new(Type::local("U")),
                                Box::new(Type::local("V")),
                            )),
                        ],
                        role_args: vec!["Client".to_string(), "Server".to_string()],
                    },
                ],
                config: vec![],
                required_impls: vec![],
                methods: vec![],
                state: vec![],
            }
        );
    }

    #[test]
    fn interface_impls() {
        use mproto_codegen::ast::Type;

        let data = "interface Foo<T, U, V, > @(Server, Client) { impl @(Server) { request_handler: T -> U, foo_bar: async U -> V, } }";
        let (_, parsed) = interface_def(data).unwrap();

        assert_eq!(
            parsed,
            Interface {
                name: "Foo".into(),
                type_params: vec!["T".to_string(), "U".to_string(), "V".to_string()],
                roles: vec!["Server".to_string(), "Client".to_string()],
                events: vec![],
                objects: vec![
                ],
                config: vec![],
                required_impls: vec![
                    InterfaceRequiredImplsList {
                        roles: vec!["Server".to_string()],
                        required_impls: vec![
                            InterfaceRequiredImpl {
                                name: "request_handler".into(),
                                is_async: false,
                                input_ty: Type::local("T"),
                                output_ty: Type::local("U"),
                            },
                            InterfaceRequiredImpl {
                                name: "foo_bar".into(),
                                is_async: true,
                                input_ty: Type::local("U"),
                                output_ty: Type::local("V"),
                            },
                        ],
                    },
                ],
                methods: vec![],
                state: vec![],
            }
        );
    }

    #[test]
    fn interface_methods() {
        use mproto_codegen::ast::Type;

        let data = "interface Foo<T, U, V, > @(Server, Client) { methods @(Server) { get_thing: T -> U, foo_bar: async U -> V, } }";
        let (_, parsed) = interface_def(data).unwrap();

        assert_eq!(
            parsed,
            Interface {
                name: "Foo".into(),
                type_params: vec!["T".to_string(), "U".to_string(), "V".to_string()],
                roles: vec!["Server".to_string(), "Client".to_string()],
                events: vec![],
                objects: vec![
                ],
                config: vec![],
                required_impls: vec![],
                methods: vec![
                    ObjectMethodsList {
                        roles: vec!["Server".to_string()],
                        methods: vec![
                            ObjectMethod {
                                name: "get_thing".into(),
                                is_async: false,
                                input_ty: Type::local("T"),
                                output_ty: Type::local("U"),
                            },
                            ObjectMethod {
                                name: "foo_bar".into(),
                                is_async: true,
                                input_ty: Type::local("U"),
                                output_ty: Type::local("V"),
                            },
                        ],
                    },
                ],
                state: vec![],
            }
        );
    }

    #[test]
    fn interface_config() {
        use mproto_codegen::ast::Type;

        let data = "interface Foo<T, U, V, > @(Server, Client) { config @(Server) { t: T, u: U } }";
        let (_, parsed) = interface_def(data).unwrap();

        assert_eq!(
            parsed,
            Interface {
                name: "Foo".into(),
                type_params: vec!["T".to_string(), "U".to_string(), "V".to_string()],
                roles: vec!["Server".to_string(), "Client".to_string()],
                events: vec![],
                objects: vec![
                ],
                config: vec![
                    InterfaceConfigList {
                        roles: vec!["Server".to_string()],
                        items: vec![
                            InterfaceConfigItem { name: "t".into(), ty: Type::local("T") },
                            InterfaceConfigItem { name: "u".into(), ty: Type::local("U") },
                        ],
                    },
                ],
                required_impls: vec![],
                methods: vec![],
                state: vec![],
            }
        );
    }
}

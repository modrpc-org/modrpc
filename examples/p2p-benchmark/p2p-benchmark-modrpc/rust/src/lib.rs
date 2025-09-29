#![feature(type_alias_impl_trait)]

pub use interface::*;
pub use proto::*;
pub use roles::*;

mod interface;
mod proto;
mod roles;
mod role_impls;

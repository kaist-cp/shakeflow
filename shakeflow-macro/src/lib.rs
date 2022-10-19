//! Implementation of proc macros on expr value and interface type.
//!
//! # Note
//!
//! To use `#[derive(Signal)]` on struct, it is assumed that its `port_decls()` method
//! implementation of `Signal` trait is constructed as struct of its fields.
//!
//! For example, `port_decls()` method implementation of `Protocol` struct of rx_hash module is as follows.
//!
//! ```ignore
//! #[derive(Debug, Clone, Signal)]
//! pub struct Output {
//!     #[member(name = "")]
//!     data: Bits<U<32>>,
//!     #[member(name = "type")]
//!     typ: Bits<U<4>>,
//! }
//!
//! impl Signal for Output {
//!     ...
//!     fn port_decls() -> lir::PortDecls {
//!         lir::PortDecls::Struct(vec![
//!             (None, lir::PortDecls::Bits(32)),
//!             (Some("type".to_string()), lir::PortDecls::Bits(4)),
//!         ])
//!     }
//! }
//! ```

mod expr_project;
mod interface;
mod signal;
mod utils;

use proc_macro::{self, TokenStream};

#[proc_macro_derive(Signal, attributes(member, width, encode))]
pub fn signal(input: TokenStream) -> TokenStream {
    let mut out = signal::derive(input.clone());

    out.extend(expr_project::derive(input.clone()));

    let input_var_arr = utils::get_var_arr_struct(input.clone());
    if !input_var_arr.is_empty() {
        out.extend(input_var_arr.clone());
        out.extend(signal::derive(input_var_arr.clone()));
        out.extend(expr_project::derive(input_var_arr.clone()));
        out.extend(utils::get_var_arr_impls(input, input_var_arr));
    }

    out
}

#[proc_macro_derive(Interface, attributes(member))]
pub fn interface(input: TokenStream) -> TokenStream { interface::derive(input) }

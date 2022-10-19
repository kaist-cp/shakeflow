//! ShakeFlow: functional hardware description with latency-insensitive interface combinators.

// # Tries to deny all lints (`rustc -W help`).
#![deny(absolute_paths_not_starting_with_crate)]
#![deny(anonymous_parameters)]
#![deny(deprecated_in_future)]
#![deny(explicit_outlives_requirements)]
#![deny(keyword_idents)]
#![deny(macro_use_extern_crate)]
#![deny(missing_debug_implementations)]
#![deny(non_ascii_idents)]
#![deny(pointer_structural_match)]
#![deny(rust_2018_idioms)]
#![deny(trivial_numeric_casts)]
#![deny(unaligned_references)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_extern_crates)]
#![deny(unused_import_braces)]
#![deny(unused_qualifications)]
#![deny(variant_size_differences)]
#![deny(warnings)]
//
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::private_doc_tests)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::bare_urls)]
// #![deny(single_use_lifetimes)]
#![deny(unreachable_pub)]
// #![deny(unused_lifetimes)]
//
#![allow(clippy::needless_lifetimes)]
#![allow(elided_lifetimes_in_paths)]
#![allow(incomplete_features)]
// #![allow(rustdoc::missing_doc_code_examples)]
#![allow(type_alias_bounds)]
//
#![feature(generic_const_exprs)]

#[macro_use]
pub mod hir;
pub mod codegen;
pub mod fir;
pub mod firgen;
pub mod lir;
pub mod utils;
pub mod vir;
pub mod virgen;

pub use firgen::Firgen;
pub use hir::*;
#[doc(hidden)]
pub use linked_hash_map;
pub use lir::PrimitiveModule;
pub use shakeflow_macro::{Interface, Signal};
pub use utils::*;
pub use virgen::Virgen;

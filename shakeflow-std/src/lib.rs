//! Standard library.

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
#![allow(clippy::type_complexity)]
#![allow(elided_lifetimes_in_paths)]
#![allow(incomplete_features)]
#![allow(type_alias_bounds)]
// #![allow(rustdoc::missing_doc_code_examples)]
//
#![feature(generic_const_exprs)]
#![feature(adt_const_params)]

use shakeflow::*;

mod arb_mux;
pub mod array_map;
pub mod axis;
mod buffer;
mod buffer_skid;
pub mod command_queue;
pub mod concentrate;
mod counter;
pub mod credit;
pub mod cuckoo_table;
mod demux;
pub mod deque;
pub mod fifo;
mod fsm;
pub mod memory;
mod mux;
mod mux_one_hot;
mod mux_quick;
mod permute;
pub mod priority_mux;
mod register_slice;
pub mod rr_mux;
mod scatter_gather;
pub mod transpose;
pub mod unconcentrate;
pub mod unidir;
pub mod valid_credit;
pub mod valid_ready;

pub use arb_mux::ArbMuxExt;
pub use array_map::*;
pub use command_queue::*;
pub use concentrate::ConcentrateExt;
pub use counter::*;
pub use deque::*;
pub use fsm::FsmExt;
pub use mux::MuxExt;
pub use mux_one_hot::MuxOneHotExt;
pub use mux_quick::MuxQuickExt;
pub use permute::PermuteExt;
pub use priority_mux::PriorityMuxExt;
pub use rr_mux::*;
pub use scatter_gather::*;
pub use transpose::*;
pub use unconcentrate::UnconcentrateExt;
pub use unidir::*;
pub use valid_credit::*;
pub use valid_ready::*;

//! Optimizations.
//!
//! TODO: Move optimizations to LIR.

mod dead_code;
mod wire_cache;

pub use dead_code::*;
pub use wire_cache::*;

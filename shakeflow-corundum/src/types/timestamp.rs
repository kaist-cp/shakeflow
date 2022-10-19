//! Timestamp.

use shakeflow::*;

#[derive(Debug, Clone, Signal)]
pub struct Timestamp {
    #[member(name = "96")]
    ts: Bits<U<96>>,
}

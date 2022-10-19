//! Checksum.

use shakeflow::*;

#[derive(Debug, Default, Clone, Signal)]
pub struct Accumulator {
    sum: Bits<U<16>>,
}

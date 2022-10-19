//! This module works as an extender across the chip.
//!
//! It has valid/ready protocol on both sides.

use shakeflow::*;
use shakeflow_std::*;

pub type C<Width: Num> = VrChannel<Bits<Width>>;

pub fn m<Width: Num>() -> Module<C<Width>, C<Width>> {
    composite::<C<Width>, C<Width>, _>("bsg_relay_fifo", Some("i"), Some("o"), |input, k| input.fifo::<2>(k)).build()
}

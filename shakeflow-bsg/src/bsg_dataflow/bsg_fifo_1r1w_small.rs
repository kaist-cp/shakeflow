//! FIFO with 1 read and 1 write

use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const SLOTS: usize, const HARDEN: bool>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_fifo_1r1w_small", Some("i"), Some("o"), |input, k| {
        // hardened or not
        if HARDEN {
            input.comb_inline(k, super::bsg_fifo_1r1w_small_hardened::m::<V, SLOTS>())
        } else {
            // two fifo or not
            if SLOTS == 2 {
                input.fifo::<2>(k)
            } else {
                input.comb_inline(k, super::bsg_fifo_1r1w_small_unhardened::m::<V, SLOTS>())
            }
        }
    })
    .build()
}

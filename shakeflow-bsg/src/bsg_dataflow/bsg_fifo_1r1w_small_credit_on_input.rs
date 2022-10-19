use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const SLOTS: usize, const HARDEN: bool>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_fifo_1r1w_small_credit_on_input", Some("i"), Some("o"), |input, k| {
        if HARDEN {
            input.into_fifo(k, super::bsg_fifo_1r1w_small_hardened::m::<V, SLOTS>())
        } else {
            input.into_fifo(k, super::bsg_fifo_1r1w_small_unhardened::m::<V, SLOTS>())
        }
    })
    .build()
}

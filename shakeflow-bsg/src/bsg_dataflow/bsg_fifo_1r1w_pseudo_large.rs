use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const ELS: usize>() -> Module<C<V>, C<V>> {
    let big_fifo = composite::<C<V>, C<V>, _>("big_fifo", Some("i"), Some("o"), |input, k| {
        input.fifo_1r1w_from_1rw(k, super::bsg_fifo_1rw_large::m::<V, ELS>())
    })
    .build();

    composite::<C<V>, C<V>, _>("bsg_fifo_1r1w_pseudo_large", Some("i"), Some("o"), |input, k| {
        input.fifo_bypass(k, big_fifo).fifo::<2>(k)
    })
    .build()
}

use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const N: usize>() -> Module<C<V>, C<V>>
where [(); N / 2]: {
    assert!(N % 2 == 0, "Odd number of elements for two port FIFO not handled");

    composite::<C<V>, C<V>, _>("bsg_fifo_1r1w_large_banked", Some("i"), Some("o"), |input, k| {
        let fifo_input = input.comb_inline(k, super::bsg_round_robin_1_to_n::m::<V, 2>());
        let fifo_output = fifo_input
            .array_map(k, "bank", |ch, k| ch.comb_inline(k, super::bsg_fifo_1r1w_pseudo_large::m::<V, { N / 2 }>()));
        let (_, output) = fifo_output.comb_inline(k, super::bsg_round_robin_n_to_1::m::<V, 2, true>());
        output
    })
    .build()
}

use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const ELS: usize>() -> Module<C<V>, C<V>>
where [(); ELS / 2]: {
    assert!(ELS % 2 == 0, "Odd number of elements for two port FIFO not handled");

    let big_fifo = composite::<VrChannel<Array<V, U<2>>>, VrChannel<Array<V, U<2>>>, _>(
        "big_fifo",
        Some("i"),
        Some("o"),
        |input, k| input.fifo_1r1w_from_1rw(k, super::bsg_fifo_1rw_large::m::<Array<V, U<2>>, { ELS / 2 }>()),
    )
    .build();

    composite::<C<V>, C<V>, _>("bsg_fifo_1r1w_large", Some("i"), Some("o"), |input, k| {
        // TODO: Fix this
        let yumi_cnt = UniChannel::source(k, 0.into());

        (input, yumi_cnt)
            .comb_inline(k, super::bsg_serial_in_parallel_out::m::<V, 2>())
            .array_map(k, "into_vr", |ch, k| ch.into_vr(k))
            .gather(k)
            .fifo_bypass(k, big_fifo)
            .scatter(k)
            .comb_inline(k, super::bsg_round_robin_2_to_2::m())
            .array_map(k, "twofer", |ch, k| ch.fifo::<2>(k))
            .comb_inline(k, super::bsg_round_robin_n_to_1::m::<V, 2, true>())
            .1
    })
    .build()
}

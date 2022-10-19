use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal, const NUM_OUT: usize> = (VrChannel<V>, UniChannel<Bits<Log2<U<NUM_OUT>>>>);
pub type EC<V: Signal, const NUM_OUT: usize> = [VrChannel<V>; NUM_OUT];

pub fn m<
    V: Signal,
    const NUM_OUT: usize,
    const ELS: usize,
    const UNBUFFERED_MASK: usize,
    const USE_PSEUDO_LARGE_FIFO: bool,
    const HARDEN: bool,
>() -> Module<IC<V, NUM_OUT>, EC<V, NUM_OUT>> {
    composite::<IC<V, NUM_OUT>, EC<V, NUM_OUT>, _>("bsg_1_to_n_tagged_fifo", Some("i"), Some("o"), |(input, tag), k| {
        let fifo_input = input.zip_uni(k, tag).demux::<NUM_OUT>(k);

        fifo_input.array_map_enumerate(|i, ch| {
            if (UNBUFFERED_MASK >> i) % 2 > 0 {
                ch.into_uni(k, true).into_vr(k)
            } else if USE_PSEUDO_LARGE_FIFO {
                ch.comb_inline(k, super::bsg_fifo_1r1w_pseudo_large::m::<V, ELS>())
            } else if HARDEN {
                ch.comb_inline(k, super::bsg_fifo_1r1w_small_hardened::m::<V, ELS>())
            } else {
                ch.comb_inline(k, super::bsg_fifo_1r1w_small_unhardened::m::<V, ELS>())
            }
        })
    })
    .build()
}

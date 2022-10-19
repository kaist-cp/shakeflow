use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal> = VrChannel<V>;
pub type EC<V: Signal, const ELS: usize> = VrChannel<Array<V, U<ELS>>>;

pub fn m<V: Signal, const ELS: usize, const USE_MINIMAL_BUFFERING: bool, const HI_TO_LO: bool>(
) -> Module<IC<V>, EC<V, ELS>> {
    composite::<IC<V>, EC<V, ELS>, _>("bsg_serial_in_parallel_out_full", Some("i"), Some("o"), |input, k| {
        let fifo_input = input.comb_inline(k, super::bsg_round_robin_1_to_n::m::<V, ELS>());

        let fifo_output = fifo_input.array_map_enumerate(|i, input| {
            if i == 0 && !USE_MINIMAL_BUFFERING {
                input.fifo::<2>(k)
            } else {
                input.buffer(k)
            }
        });

        if HI_TO_LO { fifo_output.reverse(k) } else { fifo_output }.gather(k)
    })
    .build()
}

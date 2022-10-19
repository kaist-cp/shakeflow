//! Takes in a multi-word data and serializes it to a single-word output.
//!
//! There are two options for this module:
//!
//! - Zero bubbles, no dependence, 2 element buffer for the last data word
//! - One bubble, no dependence, 1 element buffer for last data word
//!
//! TODO: Add `hi_to_lo_p`

use shakeflow::*;
use shakeflow_std::*;

/// Ingress channel.
pub type IC<V: Signal, const ELS: usize> = VrChannel<Array<V, U<ELS>>>;
/// Egress channel.
pub type EC<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const ELS: usize, const USE_MINIMAL_BUFFERING: bool>() -> Module<IC<V, ELS>, EC<V>> {
    composite::<IC<V, ELS>, EC<V>, _>("bsg_parallel_in_serial_out", Some("i"), Some("o"), |input, k| {
        // FIFO 0 for the last input data, FIFO 1 for the rest of the data words
        let (fifo0_input, fifo1_input) = input.duplicate::<{ Protocol::Demanding }, { Protocol::Demanding }>(k);
        let fifo0_input = fifo0_input.map(k, |input| input[ELS - 1]);
        let fifo1_input = fifo1_input.map(k, |input| input.clip_const::<Diff<U<ELS>, U<1>>>(0));

        let fifo0_output = fifo0_input.fifo::<2>(k);
        let fifo1_output = if USE_MINIMAL_BUFFERING { fifo1_input.buffer(k) } else { fifo1_input.fifo::<2>(k) };

        let fifo1_output_serial =
            fifo1_output.fsm_egress::<Bits<Log2<Diff<U<ELS>, U<1>>>>, V, _>(k, None, 0.into(), |input, state| {
                let output = input[state];
                let state_next = (state + 1.into()).resize();
                let last = state.is_eq((ELS - 2).into());

                (output, state_next, last)
            });

        let (_, output) = [fifo1_output_serial, fifo0_output].mux(k);
        output
    })
    .build()
}

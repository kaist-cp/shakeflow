//! Creates an array of shift registers, with independently controller three input muxes.
//!
//! - 0: Keep value
//! - 1: Get prev value
//! - 2: Set new value

use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal, const N: usize> = (UniChannel<V>, UniChannel<Array<Bits<U<2>>, U<N>>>);
pub type OC<V: Signal> = UniChannel<V>;

pub fn m<V: Signal, const N: usize>() -> Module<IC<V, N>, OC<V>> {
    composite::<IC<V, N>, OC<V>, _>("bsg_fifo_shift_datapath", Some("i"), Some("o"), |(input, sel), k| {
        input.zip(k, sel).fsm_map::<Array<V, Sum<U<N>, U<1>>>, _, _>(k, None, Expr::x(), |input, state| {
            let (input, sel) = *input;
            let output = state[0];

            let curr = state.clip_const::<U<N>>(0);
            let prev = state.clip_const::<U<N>>(1);

            let state_next = curr
                .zip4(prev, sel, input.repeat())
                .map(|input| {
                    let (curr, prev, sel, input) = *input;

                    select! {
                        sel.is_eq(0b01.into()) => prev,
                        sel.is_eq(0b10.into()) => input,
                        default => curr,
                    }
                })
                .resize();

            (output, state_next)
        })
    })
    .build()
}

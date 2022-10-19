//! Counter for credits. For every decimation credits it would assert token signal once.
//!
//! TODO: Consider ready signal

use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const DECIMATION: usize>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_credit_to_token", Some("i"), Some("o"), |input, k| {
        input.fsm_egress::<Bits<Log2<U<DECIMATION>>>, V, _>(k, Some("from_token"), 0.into(), |input, count| {
            let count_next = count + 1.into();
            let token_ready = count_next.is_gt(DECIMATION.into());
            (input, count_next.resize(), token_ready)
        })
    })
    .build()
}

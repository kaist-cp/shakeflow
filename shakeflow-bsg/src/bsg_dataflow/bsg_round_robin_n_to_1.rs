use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal, const N: usize> = [VrChannel<V>; N];
pub type EC<V: Signal, const N: usize> = (UniChannel<Bits<Log2<U<N>>>>, VrChannel<V>);

pub fn m<V: Signal, const N: usize, const STRICT: bool>() -> Module<IC<V, N>, EC<V, N>> {
    composite::<IC<V, N>, EC<V, N>, _>("bsg_round_robin_n_to_1", Some("i"), Some("o"), |input, k| {
        if STRICT {
            input.rr_mux(k)
        } else {
            todo!()
        }
    })
    .build()
}

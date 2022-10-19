use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal> = VrChannel<V>;
pub type OC<V: Signal, const N: usize> = [VrChannel<V>; N];

pub fn m<V: Signal, const N: usize>() -> Module<IC<V>, OC<V, N>> {
    composite::<IC<V>, OC<V, N>, _>("bsg_round_robin_1_to_n", Some("i"), Some("o"), |input, k| {
        let (input, ptr) = input.counter_transfer::<U<N>>(k);
        input.zip_uni(k, ptr).demux(k)
    })
    .build()
}

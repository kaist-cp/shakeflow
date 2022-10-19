use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal, const NUM_OUT: usize> = (VrChannel<V>, UniChannel<Bits<Log2<U<NUM_OUT>>>>);
pub type EC<V: Signal, const NUM_OUT: usize> = [VrChannel<V>; NUM_OUT];

pub fn m<V: Signal, const NUM_OUT: usize>() -> Module<IC<V, NUM_OUT>, EC<V, NUM_OUT>> {
    composite::<IC<V, NUM_OUT>, EC<V, NUM_OUT>, _>("bsg_1_to_n_tagged", Some("i"), Some("o"), |(input, tag), k| {
        input.zip_uni(k, tag).demux::<NUM_OUT>(k)
    })
    .build()
}

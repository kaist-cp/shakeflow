use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal, const N: usize> = VrChannel<Array<V, U<N>>>;
pub type EC<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const N: usize>() -> Module<IC<V, N>, EC<V>> {
    composite::<IC<V, N>, EC<V>, _>("bsg_parallel_in_serial_out_passthrough", Some("i"), Some("o"), |input, k| {
        input.fsm_egress::<Bits<Log2<U<N>>>, V, _>(k, None, 0.into(), |input, count| {
            let output = input[count];
            let count_next = count + 1.into();
            let last = count_next.is_eq(N.into());

            (output, count_next.resize(), last)
        })
    })
    .build()
}

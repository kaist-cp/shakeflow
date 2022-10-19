use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct I<V: Signal, MaxEls: Num> {
    data: Array<V, MaxEls>,
    len: Bits<Log2<MaxEls>>,
}

#[derive(Debug, Clone, Signal)]
pub struct E<V: Signal> {
    data: V,
    len_valid: bool,
}

pub type IC<V: Signal, MaxEls: Num> = VrChannel<I<V, MaxEls>>;
pub type EC<V: Signal> = VrChannel<E<V>>;

pub fn m<V: Signal, MaxEls: Num>() -> Module<IC<V, MaxEls>, EC<V>> {
    composite::<IC<V, MaxEls>, EC<V>, _>("bsg_parallel_in_serial_out_dynamic", Some("i"), Some("o"), |input, k| {
        let fifo_output = input.fifo::<2>(k);

        fifo_output.fsm_egress::<Bits<Log2<MaxEls>>, E<V>, _>(k, None, 0.into(), |input, count| {
            let output = EProj { data: input.data[count], len_valid: count.is_eq(0.into()) }.into();
            let count_next = count + 1.into();
            let last = count_next.is_eq(input.len.resize());
            (output, count_next.resize(), last)
        })
    })
    .build()
}

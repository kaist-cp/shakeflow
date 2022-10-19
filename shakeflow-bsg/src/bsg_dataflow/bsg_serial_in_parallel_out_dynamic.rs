use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct I<V: Signal, MaxEls: Num> {
    data: V,
    len: Bits<Log2<MaxEls>>,
}

pub type IC<V: Signal, MaxEls: Num> = VrChannel<I<V, MaxEls>>;
pub type EC<V: Signal, MaxEls: Num> = VrChannel<Array<V, MaxEls>>;

#[derive(Debug, Clone, Signal)]
pub struct S<V: Signal, MaxEls: Num> {
    output: Array<V, MaxEls>,
    count: Bits<Log2<MaxEls>>,
    len: Bits<Log2<MaxEls>>,
}

impl<V: Signal, MaxEls: Num> S<V, MaxEls> {
    /// Creates new expr.
    pub fn new_expr() -> Expr<'static, Self> { SProj { output: Expr::x(), count: 0.into(), len: 0.into() }.into() }
}

pub fn m<V: Signal, MaxEls: Num>() -> Module<IC<V, MaxEls>, EC<V, MaxEls>> {
    composite::<IC<V, MaxEls>, EC<V, MaxEls>, _>(
        "bsg_serial_in_parallel_out_dynamic",
        Some("i"),
        Some("o"),
        |input, k| {
            input
                .fsm_ingress::<S<V, MaxEls>, _>(k, None, S::new_expr(), |input, state| {
                    let output_next = state.output.set(state.count, input.data);
                    let count_next = state.count + 1.into();
                    let len_next = state.count.is_eq(0.into()).cond(input.len, state.len);
                    let done = state.count.is_gt(0.into()) & count_next.is_eq(state.len.resize());
                    let state_next = SProj { output: output_next, count: count_next.resize(), len: len_next }.into();
                    (state_next, done)
                })
                .map(k, |input| input.output)
        },
    )
    .build()
}

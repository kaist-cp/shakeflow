use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal> = VrChannel<V>;
pub type EC<V: Signal, const N: usize> = VrChannel<Array<V, U<N>>>;

#[derive(Debug, Clone, Signal)]
pub struct S<V: Signal, const N: usize> {
    output: Array<V, U<N>>,
    count: Bits<Log2<U<N>>>,
}

impl<V: Signal, const N: usize> S<V, N> {
    /// Creates new expr.
    pub fn new_expr() -> Expr<'static, Self> { SProj { output: Expr::x(), count: 0.into() }.into() }
}

pub fn m<V: Signal, const N: usize>() -> Module<IC<V>, EC<V, N>> {
    composite::<IC<V>, EC<V, N>, _>("bsg_serial_in_parallel_out_passthrough", Some("i"), Some("o"), |input, k| {
        input
            .fsm_ingress::<S<V, N>, _>(k, None, S::new_expr(), |input, state| {
                let output_next = state.output.set(state.count, input);
                let count_next = state.count + 1.into();
                let done = count_next.is_eq(N.into());
                let state_next =
                    done.cond(S::new_expr(), SProj { output: output_next, count: count_next.resize() }.into());
                (state_next, done)
            })
            .map(k, |input| input.output)
    })
    .build()
}

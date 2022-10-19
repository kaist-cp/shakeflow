//! A data structure that takes one word per cycle and allows more than one word per cycle to exit.
//!
//! The number of words extracted can vary dynamically.

use shakeflow::*;
use shakeflow_std::*;

// XXX: Is it ok that `yumi_cnt` is in ingress?
pub type IC<V: Signal, const ELS: usize> = (VrChannel<V>, UniChannel<Bits<Log2<U<ELS>>>>);
pub type EC<V: Signal, const ELS: usize> = [UniChannel<Valid<V>>; ELS];

#[derive(Debug, Clone, Signal)]
pub struct S<V: Signal, const ELS: usize> {
    data: Array<Valid<V>, U<ELS>>,
    num_els: Bits<Log2<Sum<U<ELS>, U<1>>>>,
}

impl<V: Signal, const ELS: usize> S<V, ELS> {
    /// Creates new expr.
    pub fn new_expr() -> Expr<'static, Self> { SProj { data: Expr::invalid().repeat(), num_els: 0.into() }.into() }
}

pub fn m<V: Signal, const ELS: usize>() -> Module<IC<V, ELS>, EC<V, ELS>> {
    composite::<IC<V, ELS>, EC<V, ELS>, _>(
        "bsg_serial_in_parallel_out",
        Some("i"),
        Some("o"),
        |(input, yumi_cnt), k| {
            let (input, fire) = input.fire(k);

            input
                .zip_uni(k, yumi_cnt)
                .zip_uni(k, fire)
                // TODO: `yumi_cnt` should also trigger transition of state.
                .fsm_ingress::<S<V, ELS>, _>(k, None, S::new_expr(), |input, state| {
                    let (input, fire) = *input;
                    let (input, yumi_cnt) = *input;

                    let enque = Expr::<Valid<_>>::new(fire, input);
                    let data_next = state
                        .data
                        .resize::<Sum<U<ELS>, U<ELS>>>()
                        .set(state.num_els.resize(), enque)
                        .clip::<U<ELS>>(yumi_cnt.resize());
                    let num_els_next = (state.num_els + fire.repr().resize() - yumi_cnt.resize()).resize();

                    let state_next = SProj { data: data_next, num_els: num_els_next }.into();
                    let last = num_els_next.is_eq(ELS.into());

                    (state_next, last)
                })
                .into_uni(k, true)
                .map(k, |input| input.inner.data)
                .slice(k)
        },
    )
    .build()
}

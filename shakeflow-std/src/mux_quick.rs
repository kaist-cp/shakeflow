//! Mux.

use crate::num::*;
use crate::*;

/// Muxes valid-ready channels.
pub trait MuxQuickExt {
    /// Selector type.
    type Sel: Signal;

    /// Output type.
    type O: Interface;

    /// Muxes valid-ready channels. For the selected valid-ready channel, set the ready expr to true.
    fn mux_quick(self, k: &mut CompositeModuleContext) -> (UniChannel<Self::Sel>, Self::O);
}

fn m<V: Signal, const N: usize>() -> Module<[VrChannel<V>; N], (UniChannel<Bits<U<N>>>, UniChannel<V>)> {
    composite::<[VrChannel<V>; N], (UniChannel<Bits<U<N>>>, UniChannel<V>), _>(
        "mux_quick",
        Some("in"),
        Some("out"),
        |input, k| {
            input.fsm::<(), (UniChannel<Bits<U<N>>>, UniChannel<V>), _>(k, None, ().into(), |ingress_fwd, _, state| {
                let (selector, egress_fwd) =
                    *ingress_fwd.enumerate::<Log2<U<N>>>().fold((0.into(), Expr::x()).into(), |acc, e| {
                        let acc_index: Expr<Bits<U<N>>> = acc.0;
                        let (index, e) = *e;
                        let selected = acc_index.is_eq(0.into()) & e.valid;
                        selected.cond((Expr::<Bits<U<N>>>::from(1) << index, e.inner).into(), acc)
                    });

                let ingress_bwd = Expr::<Ready>::new_arr(selector);

                ((selector, egress_fwd).into(), ingress_bwd, state)
            })
        },
    )
    .build()
}

impl<V: Signal, const N: usize> MuxQuickExt for [VrChannel<V>; N] {
    type O = UniChannel<V>;
    type Sel = Bits<U<N>>;

    fn mux_quick(self, k: &mut CompositeModuleContext) -> (UniChannel<Self::Sel>, Self::O) { self.comb_inline(k, m()) }
}

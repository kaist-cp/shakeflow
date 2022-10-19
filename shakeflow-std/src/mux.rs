//! Mux.

use crate::num::*;
use crate::*;

/// Mux extension.
pub trait MuxExt {
    /// Output interface.
    type Output;

    /// Mux
    fn mux(self, k: &mut CompositeModuleContext) -> Self::Output;
}

/// Mux extension for unichannels
impl<V: Signal, const CL: usize, const N: usize> MuxExt for (UniChannel<Bits<U<CL>>>, [UniChannel<V>; N]) {
    type Output = UniChannel<V>;

    fn mux(self, k: &mut CompositeModuleContext) -> UniChannel<V> {
        self.fsm::<(), UniChannel<V>, _>(k, Some("mux"), ().into(), |fwd_i, bwd_o, s| {
            let (sel, fwd_i) = *fwd_i;

            let fwd_o = fwd_i[sel.resize()];
            let bwd_i = bwd_o.repeat::<U<N>>();

            (fwd_o, (().into(), bwd_i).into(), s)
        })
    }
}

/// Mux extension for valid-ready channels
impl<V: Signal, CL: Num, const N: usize, const P: Protocol> MuxExt for (UniChannel<Bits<CL>>, [VrChannel<V, P>; N]) {
    type Output = VrChannel<V, P>;

    fn mux(self, k: &mut CompositeModuleContext) -> VrChannel<V, P> {
        self.fsm::<(), VrChannel<V, P>, _>(k, Some("mux"), ().into(), |fwd_i, bwd_o, s| {
            let (sel, fwd_i) = *fwd_i;

            let fwd_o = fwd_i[sel.resize()];
            let bwd_i = Expr::<Ready>::new_arr(bwd_o.ready.repr().resize::<U<N>>() << sel.resize());

            (fwd_o, (().into(), bwd_i).into(), s)
        })
    }
}

fn m_vr_array<V: Signal, const N: usize>() -> Module<[VrChannel<V>; N], (UniChannel<Bits<U<N>>>, VrChannel<V>)> {
    composite::<[VrChannel<V>; N], (UniChannel<Bits<U<N>>>, VrChannel<V>), _>(
        "mux",
        Some("in"),
        Some("out"),
        |value, k| {
            value.fsm::<_, (UniChannel<Bits<U<N>>>, VrChannel<V>), _>(
                k,
                None,
                ().into(),
                |ingress_fwd, egress_bwd, state| {
                    let (selector, egress_fwd) =
                        *ingress_fwd.enumerate::<Log2<U<N>>>().fold((0.into(), Expr::x()).into(), |acc, e| {
                            let acc_index: Expr<Bits<U<N>>> = acc.0;
                            let (index, e) = *e;
                            let selected = acc_index.is_eq(0.into()) & e.valid;
                            selected.cond((Expr::<Bits<U<N>>>::from(1) << index, e.inner).into(), acc)
                        });

                    let is_valid_found = selector.is_gt(0.into());
                    let egress_fwd = Expr::<Valid<_>>::new(is_valid_found, egress_fwd);
                    let ingress_bwd = Expr::<Ready>::new_arr(egress_bwd.1.ready.cond(selector, 0.into()));

                    ((egress_bwd.1.ready.cond(selector, 0.into()), egress_fwd).into(), ingress_bwd, state)
                },
            )
        },
    )
    .build()
}

impl<V: Signal, const N: usize> MuxExt for [VrChannel<V>; N] {
    type Output = (UniChannel<Bits<U<N>>>, VrChannel<V>);

    fn mux(self, k: &mut CompositeModuleContext) -> Self::Output { self.comb_inline(k, m_vr_array()) }
}

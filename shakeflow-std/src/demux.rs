//! Demux.

use crate::axis::*;
use crate::num::*;
use crate::*;

#[allow(clippy::type_complexity)]
fn m<V: Signal, CL: Num, const N: usize, const P: Protocol>(
) -> Module<VrChannel<(V, Bits<CL>), P>, [VrChannel<V, P>; N]> {
    composite::<VrChannel<(V, Bits<CL>), P>, [VrChannel<V, P>; N], _>("demux", Some("in"), Some("out"), |value, k| {
        value.fsm(k, None, ().into(), |fwd, bwd, state| {
            let (value, select) = *fwd.inner;

            let select = select.is_gt(0.into()).cond(select.resize(), 0.into());
            let valid = fwd.valid.cond(Expr::from(true), Expr::from(false));

            let fwd = Expr::<Valid<_>>::new_arr(valid.repr().resize::<U<N>>() << select, value.repeat::<U<N>>());

            (fwd, bwd[select], state)
        })
    })
    .build()
}

#[allow(clippy::type_complexity)]
fn m_axi<V: Signal, CL: Num, const N: usize, const P: Protocol>(
) -> Module<AxisVrChannel<(V, Bits<CL>), P>, [AxisVrChannel<V, P>; N]> {
    composite::<AxisVrChannel<(V, Bits<CL>), P>, [AxisVrChannel<V, P>; N], _>(
        "demux",
        Some("in"),
        Some("out"),
        |value, k| value.into_vr(k).demux(k).array_map(k, "into_axis_vr", |p, k| p.into_axis_vr(k)),
    )
    .build()
}

impl<V: Signal, CL: Num, const P: Protocol> VrChannel<(V, Bits<CL>), P> {
    /// Demuxs valid-ready channel.
    pub fn demux<const N: usize>(self, k: &mut CompositeModuleContext) -> [VrChannel<V, P>; N] {
        self.comb_inline(k, m::<V, CL, N, P>())
    }
}

impl<V: Signal, CL: Num, const P: Protocol> AxisVrChannel<(V, Bits<CL>), P> {
    /// Demuxs axis valid-ready channel.
    pub fn demux<const N: usize>(self, k: &mut CompositeModuleContext) -> [AxisVrChannel<V, P>; N] {
        self.comb_inline(k, m_axi::<V, CL, N, P>())
    }
}

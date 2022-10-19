//! Round-robin mux.
//!
//! TODO: generalize to array of channels (like arb_mux).

use shakeflow_macro::Signal;

use super::*;

#[derive(Debug, Default, Clone, Copy, Signal)]
struct State {
    select: bool,
}

#[allow(clippy::type_complexity)]
fn m<D1: Signal, D2: Signal, const P: Protocol>(
) -> Module<(AxisChannel<D1, P>, AxisChannel<D2, P>), AxisChannel<(bool, D1, D2), P>> {
    composite::<(AxisChannel<D1, P>, AxisChannel<D2, P>), AxisChannel<(bool, D1, D2), P>, _>(
        "axis_rr_mux",
        Some("in"),
        Some("out"),
        |input, k| {
            input.fsm(k, None, State::default().into(), move |fwd, bwd, s| {
                // Projections
                let payload0 = fwd.0.inner.payload;
                let tlast0 = fwd.0.inner.tlast;
                let tvalid0 = fwd.0.tvalid;

                let payload1 = fwd.1.inner.payload;
                let tlast1 = fwd.1.inner.tlast;
                let tvalid1 = fwd.1.tvalid;

                let bwd: AxisReadyProj = *bwd;
                let tready = bwd.tready;

                let select = s.select;

                // Calculates exprs.
                let tvalid = (!select).cond(tvalid0, tvalid1);
                let tlast = (!select).cond(tlast0, tlast1);

                let fwd = AxisValueProj { payload: (select, payload0, payload1).into(), tlast };
                let fwd = AxisValidProj { inner: fwd.into(), tvalid };

                let bwd0: Expr<_> = AxisReadyProj { tready: tready & !select }.into();
                let bwd1: Expr<_> = AxisReadyProj { tready: tready & select }.into();
                let bwd = (bwd0, bwd1).into();

                let is_last = tvalid & tready & tlast;
                let s = StateProj { select: select ^ is_last };

                (fwd.into(), bwd, s.into())
            })
        },
    )
    .build()
}

/// Rriter mux that muxes a Interface out of N input Interfaces.
///
/// Output interface is chosen by `select` expr. If the `select` expr is false, then left, and if it is
/// true, then right input interface is selected. The `select` expr has false as the initial value, and it
/// changes when the `last` expr of the selected input interface is true.
pub trait AxisRrMuxExt
where Self: Interface
{
    /// Output type.
    /// Typically, Self = (AxisChannel, AxisChannel), and O = AxisChannel.
    type O: Interface;

    /// Append axis_rr_mux to channel, and return output interface.
    fn axis_rr_mux(self, k: &mut CompositeModuleContext) -> Self::O;
}

impl<D1: Signal, D2: Signal, const P: Protocol> AxisRrMuxExt for (AxisChannel<D1, P>, AxisChannel<D2, P>) {
    type O = AxisChannel<(bool, D1, D2), P>;

    fn axis_rr_mux(self, k: &mut CompositeModuleContext) -> Self::O { self.comb_inline(k, m()) }
}

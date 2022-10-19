//! Mux with the priority for LSB.

use shakeflow_macro::Signal;

use crate::num::*;
use crate::*;

/// Input value type of priority encoder.
#[derive(Debug, Clone, Signal)]
pub struct I<const N: usize> {
    unencoded: Bits<U<N>>,
}

/// Output value type of priority encoder.
#[derive(Debug, Clone, Signal)]
pub struct O<const N: usize> {
    encoded: Bits<Log2<U<N>>>,
}

/// Input interface type of priority encoder.
pub type IC<const N: usize> = UniChannel<I<N>>;

/// Output interface type of priority encoder.
pub type OC<const N: usize> = UniChannel<Valid<O<N>>>;

// TODO: Is this correct?
fn m<V: Signal, const N: usize>() -> Module<[VrChannel<V>; N], VrChannel<(Bits<Log2<U<N>>>, V)>> {
    composite::<([VrChannel<V>; N], IC<N>), (VrChannel<(Bits<Log2<U<N>>>, V)>, IC<N>), _>(
        "priority_mux",
        Some("in"),
        Some("out"),
        |(i_channels, pri_in), k| {
            let pri_out = pri_in.priority_mux(k);

            let (o_channels, pri_in) = (i_channels, pri_out).fsm::<(), (VrChannel<(Bits<Log2<U<N>>>, V)>, IC<N>), _>(
                k,
                None,
                ().into(),
                |fwd, bwd, _| {
                    let (i_channels_fwd, pri_out) = *fwd;
                    let (o_channels_bwd, _) = *bwd;

                    let i_channels_fwd_valid = i_channels_fwd.map(|f| f.valid);
                    let i_channels_fwd_data = i_channels_fwd.map(|f| f.inner);

                    let encoded = pri_out.inner.encoded;

                    (
                        (
                            Expr::<Valid<_>>::new(pri_out.valid, (encoded, i_channels_fwd_data[encoded]).into()),
                            IProj { unencoded: i_channels_fwd_valid }.into(),
                        )
                            .into(),
                        (
                            Expr::<Ready>::new(false.into()).repeat::<U<N>>().set(encoded, o_channels_bwd),
                            Expr::from(()),
                        )
                            .into(),
                        Expr::from(()),
                    )
                },
            );

            (o_channels, pri_in)
        },
    )
    .loop_feedback()
    .build()
}

/// Priority mux.
pub trait PriorityMuxExt
where Self: Interface
{
    /// Output type.
    /// Typically, Self = [B; N], and O = B.
    type O: Interface;

    /// Muxes with the priority for LSB.
    fn priority_mux(self, k: &mut CompositeModuleContext) -> Self::O;
}

impl<const N: usize> PriorityMuxExt for IC<N> {
    type O = OC<N>;

    fn priority_mux(self, k: &mut CompositeModuleContext) -> Self::O {
        self.module_inst::<OC<N>>(
            k,
            "priority_encoder",
            "op_table_start_enc_inst",
            vec![("WIDTH", N), ("LSB_HIGH_PRIORITY", 1)],
            false,
            Some("input"),
            Some("output"),
        )
    }
}

impl<V: Signal, const N: usize> PriorityMuxExt for [VrChannel<V>; N] {
    type O = VrChannel<(Bits<Log2<U<N>>>, V)>;

    fn priority_mux(self, k: &mut CompositeModuleContext) -> Self::O { self.comb_inline(k, m::<_, N>()) }
}

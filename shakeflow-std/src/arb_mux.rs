//! Arbitrated mux.

#![allow(type_alias_bounds)]

use shakeflow_macro::Signal;

use crate::num::*;
use crate::*;

/// Input value of arbiter submodule.
#[derive(Debug, Clone, Signal)]
struct ArbI<const N: usize> {
    request: Bits<U<N>>,
    acknowledge: Bits<U<N>>,
}

/// Output value of arbiter submodule.
#[derive(Debug, Clone, Signal)]
struct ArbO<const N: usize> {
    grant: Bits<U<N>>,
    grant_valid: bool,
    grant_encoded: Bits<Log2<U<N>>>,
}

type ArbIC<const N: usize> = UniChannel<ArbI<N>>;
type ArbOC<const N: usize> = UniChannel<ArbO<N>>;

/// Output value of arbitrated mux. The output inner value is appended by the encoded grant index.
#[derive(Debug, Clone, Signal)]
pub struct O<V: Signal, const N: usize> {
    #[member(name = "")]
    inner: V,
    grant_encoded: Bits<Log2<U<N>>>,
}

type IC<V: Signal, const N: usize> = [VrChannel<V>; N];
type OC<V: Signal, const N: usize> = VrChannel<O<V, N>>;

/// Logic of arbitrated mux.
///
/// Receives `(IC, ArbOC)` and returns `(OC, ArbIC)`.
#[allow(clippy::type_complexity)]
fn logic<'id, V: Signal, const N: usize>(
    fwd_i: Expr<'id, (Array<Valid<V>, U<N>>, ArbO<N>)>, bwd_o: Expr<'id, (Ready, ())>,
) -> (Expr<'id, (Valid<O<V, N>>, ArbI<N>)>, Expr<'id, (Array<Ready, U<N>>, ())>) {
    let (module_input_fwd, arbiter_output) = *fwd_i;
    let m_axis_event_ready_int_reg = bwd_o.0.ready;

    let module_input: Expr<'id, Array<V, U<N>>> = Expr::member(module_input_fwd, 0);
    let s_axis_tvalid: Expr<'id, Bits<U<N>>> = Expr::member(module_input_fwd, 1);

    let ArbOProj { grant, grant_encoded, grant_valid } = *arbiter_output;

    let current = module_input[grant_encoded];
    let current_s_tvalid = s_axis_tvalid[grant_encoded];

    let s_axis_tready = (m_axis_event_ready_int_reg & grant_valid).repr().resize::<U<N>>() << grant_encoded;

    // TODO: The below code assumes LAST_ENABLE=0; Add `& s_axis_tlast` for LAST_ENABLE=1.
    let arbiter_input: Expr<'id, ArbI<N>> =
        ArbIProj { request: s_axis_tvalid & !grant, acknowledge: grant & s_axis_tvalid & s_axis_tready }.into();
    let module_output =
        Expr::<Valid<O<V, N>>>::new(current_s_tvalid & grant_valid, OProj { inner: current, grant_encoded }.into());

    let fwd_o = (module_output, arbiter_input).into();
    let bwd_i = (Expr::<Ready>::new_arr(s_axis_tready), ().into()).into();

    (fwd_o, bwd_i)
}

fn m<V: Signal, const N: usize>(
    arb_type_round_robin: usize, arb_lsb_high_priority: usize,
) -> Module<IC<V, N>, OC<V, N>> {
    composite::<(IC<V, N>, ArbIC<N>), (OC<V, N>, ArbIC<N>), _>(
        "arb_mux",
        Some("in"),
        Some("out"),
        |(input, arb_input), k| {
            let arb_output = arb_input.module_inst::<ArbOC<N>>(
                k,
                "arbiter",
                "arb_inst",
                vec![
                    ("PORTS", N),
                    ("ARB_TYPE_ROUND_ROBIN", arb_type_round_robin),
                    ("ARB_BLOCK", 1),
                    ("ARB_BLOCK_ACK", 1),
                    ("ARB_LSB_HIGH_PRIORITY", arb_lsb_high_priority),
                ],
                true,
                None,
                None,
            );

            (input, arb_output).fsm::<_, _, _>(k, None, ().into(), |fwd_i, bwd_o, s| {
                let (fwd_o, bwd_i) = logic(fwd_i, bwd_o);
                (fwd_o, bwd_i, s)
            })
        },
    )
    .loop_feedback()
    .build()
}

/// Arbiter mux that muxes a Interface out of N input Interfaces.
/// Outputs a tuple of (MuxedInterface, EncodedArbiterGrant).
pub trait ArbMuxExt
where Self: Interface
{
    /// Output type.
    /// Typically, Self = [B; N], and O = B.
    type O: Interface;

    /// Muxes with an arbiter.
    fn arb_mux(
        self, k: &mut CompositeModuleContext, arb_type_round_robin: usize, arb_lsb_high_priority: usize,
    ) -> Self::O;
}

impl<V: Signal, const N: usize> ArbMuxExt for [VrChannel<V>; N] {
    type O = OC<V, N>;

    fn arb_mux(
        self, k: &mut CompositeModuleContext, arb_type_round_robin: usize, arb_lsb_high_priority: usize,
    ) -> Self::O {
        self.comb_inline(k, m::<_, N>(arb_type_round_robin, arb_lsb_high_priority)).buffer_skid(k)
    }
}

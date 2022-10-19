//! Valid-credit protocol.

use std::marker::PhantomData;

use shakeflow_macro::Signal;

use crate::*;

/// Credit signal.
#[derive(Debug, Clone, Signal)]
pub struct Credit {
    /// Credit bit
    pub credit: bool,
}

/// Credit Extension
pub trait CreditExt<'id> {
    /// Creates a new expr.
    fn new(credit: Expr<'id, bool>) -> Self;
}

impl<'id> CreditExt<'id> for Expr<'id, Credit> {
    /// Creates a new expr.
    fn new(credit: Expr<'id, bool>) -> Self { CreditProj { credit }.into() }
}

/// Valid-credit channel.
#[derive(Debug)]
pub struct VcChannel<V: Signal, const P: Protocol = { Protocol::Helpful }> {
    endpoint: lir::Endpoint,
    _marker: PhantomData<V>,
}

impl<V: Signal, const P: Protocol> Interface for VcChannel<V, P> {
    type Bwd = Credit;
    type Fwd = Valid<V>;

    fn interface_typ() -> lir::InterfaceTyp {
        lir::InterfaceTyp::Channel(lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()))
    }

    fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError> {
        assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
        let channel = some_or!(interface.get_channel(), panic!());
        Ok(Self { endpoint: channel.endpoint(), _marker: PhantomData })
    }

    fn try_into_inner(self) -> Result<lir::Interface, InterfaceError> {
        Ok(lir::Interface::Channel(lir::Channel {
            typ: lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()),
            endpoint: self.endpoint,
        }))
    }
}

impl<I: Signal, const P: Protocol> FsmExt<I> for VcChannel<I, P> {
    type Out<O: Signal> = VcChannel<O, P>;

    // TODO: Define more precise semantic of Valid-credit Channel.
    fn fsm_map<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, O>, Expr<'id, S>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> VcChannel<O, P> {
        self.fsm(k, module_name.or(Some("fsm_map")), init, move |input_fwd, output_bwd, state| {
            let input_fwd: ValidProj<I> = *input_fwd;
            let output_bwd: CreditProj = *output_bwd;
            let (output_fwd, state_next) = f(input_fwd.inner, state);

            (
                Expr::<Valid<_>>::new(input_fwd.valid, output_fwd),
                CreditProj { credit: output_bwd.credit }.into(),
                (input_fwd.valid & output_bwd.credit).cond(state_next, state),
            )
        })
    }
}

fn m_into_uni<V: Signal, const P: Protocol>(consuming: bool) -> Module<VcChannel<V, P>, UniChannel<Valid<V>>> {
    composite::<VcChannel<V, P>, UniChannel<Valid<V>>, _>("into_uni", Some("in"), Some("out"), |value, k| {
        value.fsm::<(), _, _>(k, None, ().into(), move |i_fwd, _o_bwd, state| {
            (i_fwd, Expr::<Credit>::new(Expr::from(consuming)), state)
        })
    })
    .build()
}

fn m_into_vr<V: Signal, const P: Protocol>() -> Module<VcChannel<V, P>, VrChannel<V, P>> {
    composite::<VcChannel<V, P>, VrChannel<V, P>, _>("into_vr", Some("in"), Some("out"), |value, k| {
        value.fsm::<(), _, _>(k, None, ().into(), move |i_fwd, o_bwd, state| {
            let o_bwd: ReadyProj = *o_bwd;

            let o_fwd = i_fwd;
            let i_bwd = Expr::<Credit>::new(o_bwd.ready);

            (o_fwd, i_bwd, state)
        })
    })
    .build()
}

impl<I: Signal, const P: Protocol> VcChannel<I, P> {
    /// Transforms into unidirectional channel.
    pub fn into_uni(self, k: &mut CompositeModuleContext) -> UniChannel<Valid<I>> {
        self.comb_inline(k, m_into_uni(true))
    }

    /// Transforms into valid/ready chanel.
    pub fn into_vr(self, k: &mut CompositeModuleContext) -> VrChannel<I, P> { self.comb_inline(k, m_into_vr()) }
}

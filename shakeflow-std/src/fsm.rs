//! Finite state machine (Mealy machine).

use std::ops::*;

use crate::num::*;
use crate::*;

/// Pipeline combinators for `Channel`.
pub trait FsmExt<I: Signal>: Sized {
    /// Output type of `fsm_map`.
    type Out<O: Signal>;

    /// Feeds `self` to a new FSM.
    ///
    /// The FSM is described by `F`, which generates the circuit for (1) the current-cycle output
    /// for all cycles and (2) the next-cycle state. The state is updated only after transfer
    /// cycles.
    fn fsm_map<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, O>, Expr<'id, S>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> Self::Out<O>;

    /// Maps the output.
    fn map<O: Signal, F: 'static + for<'id> Fn(Expr<'id, I>) -> Expr<'id, O>>(
        self, k: &mut CompositeModuleContext, f: F,
    ) -> Self::Out<O> {
        self.fsm_map::<(), O, _>(k, Some("map"), ().into(), move |input, state| (f(input), state))
    }

    /// Feeds `self` to a new `N`-windowing FSM.
    fn window<const N: usize>(
        self, default: Expr<'static, I>, k: &mut CompositeModuleContext,
    ) -> Self::Out<Array<I, U<N>>> {
        self.fsm_map::<Array<I, Diff<U<N>, U<1>>>, Array<I, U<N>>, _>(
            k,
            Some("window"),
            default.repeat::<Diff<U<N>, U<1>>>(),
            |input, state| {
                let output = input.repeat::<U<1>>().append(state);
                let state = output.clone().clip_const::<Diff<U<N>, U<1>>>(0);
                // TODO: resize() here really shouldn't be necessary, but Rust currently cannot reason it.
                (output.resize(), state)
            },
        )
    }
}

impl<I: Signal> FsmExt<I> for UniChannel<I> {
    type Out<O: Signal> = UniChannel<O>;

    fn fsm_map<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, O>, Expr<'id, S>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> UniChannel<O> {
        self.fsm(k, module_name.or(Some("fsm_map")), init, move |input_fwd, output_bwd, state| {
            let (output_fwd, state) = f(input_fwd, state);
            (output_fwd, output_bwd, state)
        })
    }
}

impl<I: Signal, const P: Protocol> FsmExt<I> for VrChannel<I, P> {
    type Out<O: Signal> = VrChannel<O, P>;

    fn fsm_map<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, O>, Expr<'id, S>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> VrChannel<O, P> {
        self.fsm(k, module_name.or(Some("fsm_map")), init, move |input_fwd, output_bwd: Expr<Ready>, state| {
            let (output_fwd, state_next) = f(input_fwd.inner, state);

            (
                Expr::<Valid<_>>::new(input_fwd.valid, output_fwd),
                Expr::<Ready>::new(output_bwd.ready),
                (input_fwd.valid & output_bwd.ready).cond(state_next, state),
            )
        })
    }
}

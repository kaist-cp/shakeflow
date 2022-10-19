//! Finite state machine (Mealy machine).

use std::fmt;
use std::marker::PhantomData;
use std::ops::*;

use crate::hir::*;
use crate::*;

/// Finite state machine (Mealy machine).
#[derive(Clone)]
pub struct Fsm<
    I: Interface,
    O: Interface,
    S: Signal,
    F: for<'id> Fn(
        Expr<'id, I::Fwd>,
        Expr<'id, O::Bwd>,
        Expr<'id, S>,
    ) -> (Expr<'id, O::Fwd>, Expr<'id, I::Bwd>, Expr<'id, S>),
> {
    /// Module name.
    module_name: String,
    /// FSM function.
    pub(crate) f: F,
    /// Initial value of registers in the FSM.
    pub(crate) init: Expr<'static, S>,
    _marker: PhantomData<(I, O)>,
}

impl<
        I: Interface,
        O: Interface,
        S: Signal,
        F: for<'id> Fn(
            Expr<'id, I::Fwd>,
            Expr<'id, O::Bwd>,
            Expr<'id, S>,
        ) -> (Expr<'id, O::Fwd>, Expr<'id, I::Bwd>, Expr<'id, S>),
    > Fsm<I, O, S, F>
{
    /// Creates a new FSM.
    pub fn new(module_name: &str, f: F, init: Expr<'static, S>) -> Self {
        Self { module_name: module_name.to_string(), f, init, _marker: PhantomData }
    }
}

impl<
        I: Interface,
        O: Interface,
        S: Signal,
        F: for<'id> Fn(
            Expr<'id, I::Fwd>,
            Expr<'id, O::Bwd>,
            Expr<'id, S>,
        ) -> (Expr<'id, O::Fwd>, Expr<'id, I::Bwd>, Expr<'id, S>),
    > fmt::Debug for Fsm<I, O, S, F>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fsm").field("init", &self.init).finish()
    }
}

impl<
        I: Interface,
        O: Interface,
        S: Signal,
        F: 'static
            + for<'id> Fn(
                Expr<'id, I::Fwd>,
                Expr<'id, O::Bwd>,
                Expr<'id, S>,
            ) -> (Expr<'id, O::Fwd>, Expr<'id, I::Bwd>, Expr<'id, S>),
    > From<Fsm<I, O, S, F>> for lir::Fsm
{
    fn from(module: Fsm<I, O, S, F>) -> Self {
        let i_fwd = Expr::input(Some("in".to_string()));
        let o_bwd = Expr::input(Some("out".to_string()));
        let s = Expr::input(Some("st".to_string()));
        let (o_fwd, i_bwd, s) = (module.f)(i_fwd, o_bwd, s);

        lir::Fsm {
            input_interface_typ: I::interface_typ(),
            output_interface_typ: O::interface_typ(),
            module_name: module.module_name,
            output_fwd: o_fwd.into_inner(),
            input_bwd: i_bwd.into_inner(),
            state: s.into_inner(),
            init: module.init.into_inner(),
        }
    }
}

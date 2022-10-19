//! Finite state machine (Mealy machine).

use crate::lir::*;

/// Finite state machine (Mealy machine).
#[derive(Debug)]
pub struct Fsm {
    /// Input interface type.
    pub(crate) input_interface_typ: InterfaceTyp,
    /// Output interface type.
    pub(crate) output_interface_typ: InterfaceTyp,
    /// Module name.
    pub(crate) module_name: String,
    /// Output foreward expr.
    pub(crate) output_fwd: ExprId,
    /// Input backward expr.
    pub(crate) input_bwd: ExprId,
    /// State.
    pub(crate) state: ExprId,
    /// Initial value of registers in the FSM.
    pub(crate) init: ExprId,
}

impl PrimitiveModule for Fsm {
    #[inline]
    fn get_module_name(&self) -> String { self.module_name.clone() }

    #[inline]
    fn input_interface_typ(&self) -> InterfaceTyp { self.input_interface_typ.clone() }

    #[inline]
    fn output_interface_typ(&self) -> InterfaceTyp { self.output_interface_typ.clone() }
}

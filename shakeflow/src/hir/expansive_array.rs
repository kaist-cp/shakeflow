//! Expansive array interface.

use arrayvec::ArrayVec;

use crate::hir::*;
use crate::*;

/// Signal type for IO of `ExpansiveArray`.
#[derive(Debug, Clone)]
pub struct ExpansiveArrayValue<V: Signal, const N: usize> {
    #[doc(hidden)]
    inner: Array<V, U<N>>,
}

impl<V: Signal, const N: usize> Signal for ExpansiveArrayValue<V, N> {
    const WIDTH: usize = V::WIDTH * N;

    fn transl(self) -> Vec<bool> { self.inner.transl() }

    fn port_decls() -> lir::PortDecls {
        lir::PortDecls::Struct((0..N).map(|i| (Some(i.to_string()), V::port_decls())).collect())
    }
}

/// Expansive array interface.
#[derive(Debug)]
pub struct ExpansiveArray<I: Interface, const N: usize> {
    #[doc(hidden)]
    inner: [I; N],
}

impl<I: Interface, const N: usize> Interface for ExpansiveArray<I, N> {
    type Bwd = ExpansiveArrayValue<I::Bwd, N>;
    type Fwd = ExpansiveArrayValue<I::Fwd, N>;

    fn interface_typ() -> lir::InterfaceTyp { lir::InterfaceTyp::ExpansiveArray(Box::new(I::interface_typ()), N) }

    fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError> {
        if let lir::Interface::ExpansiveArray(inner) = interface {
            Ok(ExpansiveArray {
                inner: inner
                    .into_iter()
                    .map(|interface_elt| I::try_from_inner(interface_elt).unwrap())
                    .collect::<ArrayVec<I, N>>()
                    .into_inner()
                    .unwrap(),
            })
        } else {
            panic!("internal compiler error")
        }
    }

    fn try_into_inner(self) -> Result<lir::Interface, InterfaceError> {
        Ok(lir::Interface::ExpansiveArray(
            self.inner.into_iter().map(|interface| interface.try_into_inner().unwrap()).collect(),
        ))
    }
}

impl<'id, V: Signal, const N: usize> From<Expr<'id, ExpansiveArrayValue<V, N>>> for Expr<'id, Array<V, U<N>>> {
    fn from(expr: Expr<'id, ExpansiveArrayValue<V, N>>) -> Self {
        let expr_elts =
            (0..N).map(|i| Expr::<V>::member(expr, i)).collect::<ArrayVec<Expr<V>, N>>().into_inner().unwrap();

        expr_elts.into()
    }
}

impl<'id, V: Signal, const N: usize> From<Expr<'id, Array<V, U<N>>>> for Expr<'id, ExpansiveArrayValue<V, N>> {
    fn from(expr: Expr<'id, Array<V, U<N>>>) -> Self {
        lir::Expr::Struct { inner: (0..N).map(|i| (Some(i.to_string()), expr[i].into_inner())).collect() }.into()
    }
}

impl<I: Interface, const N: usize> ExpansiveArray<I, N> {
    /// Transforms into `[I; N]`.
    pub fn into_array(self, k: &mut CompositeModuleContext) -> [I; N] {
        self.fsm(k, None, ().into(), |i_fwd, o_bwd, s| {
            let o_fwd = Expr::<Array<I::Fwd, U<N>>>::from(i_fwd);
            let i_bwd = Expr::<ExpansiveArrayValue<I::Bwd, N>>::from(o_bwd);

            (o_fwd, i_bwd, s)
        })
    }

    /// Transforms from `[I; N]`.
    pub fn from_array(array: [I; N], k: &mut CompositeModuleContext) -> Self {
        array.fsm(k, None, ().into(), |i_fwd, o_bwd, s| {
            let o_fwd = Expr::<ExpansiveArrayValue<I::Fwd, N>>::from(i_fwd);
            let i_bwd = Expr::<Array<I::Bwd, U<N>>>::from(o_bwd);

            (o_fwd, i_bwd, s)
        })
    }
}

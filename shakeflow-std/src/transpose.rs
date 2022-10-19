//! Transpose extension.

use arrayvec::ArrayVec;

use crate::num::*;
use crate::*;

/// Transpose extension.
pub trait TransposeExt {
    /// Output interface.
    type Output;

    /// Transpose
    fn transpose(self, k: &mut CompositeModuleContext) -> Self::Output;
}

impl<V: Signal, const N: usize, const M: usize> TransposeExt for [UniChannel<Array<V, U<M>>>; N] {
    type Output = [UniChannel<Array<V, U<N>>>; M];

    fn transpose(self, k: &mut CompositeModuleContext) -> [UniChannel<Array<V, U<N>>>; M] {
        self.fsm::<(), [UniChannel<Array<V, U<N>>>; M], _>(k, Some("transpose"), ().into(), move |fwd_i, _, s| {
            let mut fwd_o = Expr::<Array<Array<V, U<N>>, U<M>>>::x();
            for y in 0..M {
                let mut row = Expr::<Array<V, U<N>>>::x();
                for x in 0..N {
                    row = row.set(x.into(), fwd_i[x][y]);
                }
                fwd_o = fwd_o.set(y.into(), row);
            }
            let bwd_i = Expr::<Array<(), U<N>>>::x();

            (fwd_o, bwd_i, s)
        })
    }
}

/// Reverse extension.
pub trait ReverseExt {
    /// Reverse
    fn reverse(self, k: &mut CompositeModuleContext) -> Self;
}

impl<V: Signal, const N: usize, const P: Protocol> ReverseExt for [VrChannel<V, P>; N] {
    fn reverse(self, _k: &mut CompositeModuleContext) -> [VrChannel<V, P>; N] {
        self.into_iter().rev().collect::<ArrayVec<_, N>>().into_inner().unwrap()
    }
}

//! Permutation.

use arrayvec::ArrayVec;

use crate::num::*;
use crate::*;

/// Permute extension.
pub trait PermuteExt<const N: usize> {
    /// Selector type.
    type Selector: Interface;

    /// Permute function.
    fn permute(self, k: &mut CompositeModuleContext, selector: Self::Selector) -> Self;
}

impl<V: Signal, const N: usize> PermuteExt<N> for [UniChannel<V>; N] {
    type Selector = UniChannel<Array<Bits<Log2<U<N>>>, U<N>>>;

    fn permute(self, k: &mut CompositeModuleContext, selector: Self::Selector) -> Self {
        (self, selector).fsm::<(), Self, _>(k, Some("permute"), ().into(), |fwd_i, _, s| {
            let (fwd_i, select) = *fwd_i;
            let fwd_o = (0..N).map(|i| fwd_i[select[i]]).collect::<ArrayVec<_, N>>().into_inner().unwrap().into();
            (fwd_o, (Expr::from(()).repeat::<U<N>>(), ().into()).into(), s)
        })
    }
}

impl<V: Signal, const N: usize, const P: Protocol> PermuteExt<N> for [VrChannel<V, P>; N] {
    type Selector = (UniChannel<Array<Bits<Log2<U<N>>>, U<N>>>, UniChannel<Array<Bits<Log2<U<N>>>, U<N>>>);

    fn permute(self, k: &mut CompositeModuleContext, selector: Self::Selector) -> Self {
        // TODO: Calculate `select_inv` from `select`
        let (select, select_inv) = selector;
        (self, select, select_inv).fsm::<(), Self, _>(k, Some("permute"), ().into(), |fwd_i, bwd_o, s| {
            let (fwd_i, select, select_inv) = *fwd_i;
            let fwd_o = (0..N).map(|i| fwd_i[select[i]]).collect::<ArrayVec<_, N>>().into_inner().unwrap().into();
            let bwd_i: Expr<_> =
                (0..N).map(|i| bwd_o[select_inv[i]]).collect::<ArrayVec<_, N>>().into_inner().unwrap().into();
            (fwd_o, (bwd_i, ().into(), ().into()).into(), s)
        })
    }
}

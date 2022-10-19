//! Mux one-hot.

use crate::*;

/// Mux one-hot extension.
pub trait MuxOneHotExt {
    /// Output interface.
    type Output;

    /// Mux one-hot
    fn mux_one_hot(self, k: &mut CompositeModuleContext) -> Self::Output;
}

/// Mux extension for unichannels
impl<N: Num, M: Num> MuxOneHotExt for (UniChannel<Array<Bits<N>, M>>, UniChannel<Bits<M>>) {
    type Output = UniChannel<Bits<N>>;

    fn mux_one_hot(self, k: &mut CompositeModuleContext) -> UniChannel<Bits<N>> {
        self.0.zip(k, self.1).map(k, |i| {
            let (data, sel_one_hot) = *i;
            // TODO: Apply `tree_fold` instead of `fold`
            data.zip(sel_one_hot).map(|e| e.1.cond(e.0, 0.into())).fold(0.into(), |l, r| l | r)
        })
    }
}

/// Mux extension for valid-ready channels
impl<N: Num, M: Num, const P: Protocol> MuxOneHotExt for (VrChannel<Array<Bits<N>, M>, P>, UniChannel<Bits<M>>) {
    type Output = VrChannel<Bits<N>, P>;

    fn mux_one_hot(self, k: &mut CompositeModuleContext) -> VrChannel<Bits<N>, P> {
        self.0.zip_uni(k, self.1).map(k, |i| {
            let (data, sel_one_hot) = *i;
            // TODO: Apply `tree_fold` instead of `fold`
            data.zip(sel_one_hot).map(|e| e.1.cond(e.0, 0.into())).fold(0.into(), |l, r| l | r)
        })
    }
}

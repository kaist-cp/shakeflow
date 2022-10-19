//! Scatter and Gather.

use crate::num::*;
use crate::*;

/// Scatter extension.
pub trait ScatterExt<const N: usize> {
    /// Output type.
    type Out: Interface;

    /// Scatter function.
    fn scatter(self, k: &mut CompositeModuleContext) -> Self::Out;
}

/// Gather extension.
pub trait GatherExt<const N: usize> {
    /// Output type.
    type Out: Interface;

    /// Gather function.
    fn gather(self, k: &mut CompositeModuleContext) -> Self::Out;
}

impl<V: Signal, const N: usize, const P: Protocol> ScatterExt<N> for VrChannel<Array<V, U<N>>, P> {
    type Out = [VrChannel<V, P>; N];

    fn scatter(self, k: &mut CompositeModuleContext) -> Self::Out {
        self.fsm::<(), Self::Out, _>(k, None, ().into(), |fwd_i, bwd_o, s| {
            let fwd_o = Expr::<Valid<_>>::new_arr(fwd_i.valid.repeat::<U<N>>(), fwd_i.inner);
            let bwd_i = Expr::<Ready>::new(bwd_o.map(|e| e.ready).any());

            (fwd_o, bwd_i, s)
        })
    }
}

impl<V: Signal, const N: usize, const P: Protocol> GatherExt<N> for [VrChannel<V, P>; N] {
    type Out = VrChannel<Array<V, U<N>>, P>;

    fn gather(self, k: &mut CompositeModuleContext) -> Self::Out {
        self.fsm::<(), Self::Out, _>(k, None, ().into(), |fwd_i, bwd_o, s| {
            let v_o = fwd_i.map(|fwd| fwd.valid).all();
            let data_o = fwd_i.map(|fwd| fwd.inner);

            let fwd_o = Expr::<Valid<_>>::new(v_o, data_o);
            let bwd_i = bwd_o.repeat::<U<N>>();

            (fwd_o, bwd_i, s)
        })
    }
}

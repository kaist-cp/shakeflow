//! Utilities for unidirectional channels.

use crate::num::*;
use crate::*;

// Unidirectional channel.
channel! {UniChannel<V: Signal>, V, ()}

impl<V: Signal> Clone for UniChannel<V> {
    fn clone(&self) -> Self { Self { endpoint: self.endpoint.clone(), _marker: self._marker } }
}

/// Slice extension.
pub trait SliceExt<V: Signal, const N: usize> {
    /// Output type of slice.
    type Out: Interface;

    /// Slice function.
    fn slice(self, k: &mut CompositeModuleContext) -> Self::Out;
}

impl<V: Signal, const N: usize> SliceExt<V, N> for UniChannel<Array<V, U<N>>> {
    type Out = [UniChannel<V>; N];

    fn slice(self, k: &mut CompositeModuleContext) -> Self::Out {
        self.fsm(k, None, ().into(), |input, _, _| (input, ().into(), ().into()))
    }
}

/// Array of `UniChannel`s trait defined solely for defining `concat`.
pub trait ConcatExt<V: Signal, const N: usize> {
    /// Output type of concatenation.
    type Out: Interface;

    /// Transform an 'array of `UniChannel`s' to an '`UniChannel` of array'.
    fn concat(self, k: &mut CompositeModuleContext) -> Self::Out;
}

impl<V: Signal, const N: usize> ConcatExt<V, N> for [UniChannel<V>; N] {
    type Out = UniChannel<Array<V, U<N>>>;

    fn concat(self, k: &mut CompositeModuleContext) -> Self::Out {
        self.fsm::<(), UniChannel<Array<V, U<N>>>, _>(k, None, ().into(), |i_fwd, _o_bwd, s| {
            (i_fwd, Array::default().into(), s)
        })
    }
}

impl<V: Signal> UniChannel<V> {
    /// Generates the value indefinitely.
    pub fn source(k: &mut CompositeModuleContext, value: Expr<'static, V>) -> Self {
        ().fsm(k, Some("source"), ().into(), move |_fwd, _bwd, state| (value, ().into(), state))
    }

    /// Zips a unidirectional channel with another unidirectional channel.
    pub fn zip<W: Signal>(self, k: &mut CompositeModuleContext, other: UniChannel<W>) -> UniChannel<(V, W)> {
        (self, other).fsm(k, Some("zip"), Expr::x(), |fwd, _, _| (fwd, ((), ()).into(), Expr::from(())))
    }

    /// Zip3
    pub fn zip3<V1: Signal, V2: Signal>(
        self, k: &mut CompositeModuleContext, other1: UniChannel<V1>, other2: UniChannel<V2>,
    ) -> UniChannel<(V, V1, V2)> {
        (self, other1, other2).fsm(k, Some("zip3"), Expr::x(), |fwd, _, _| (fwd, ((), (), ()).into(), ().into()))
    }

    /// Zip4
    pub fn zip4<V1: Signal, V2: Signal, V3: Signal>(
        self, k: &mut CompositeModuleContext, other1: UniChannel<V1>, other2: UniChannel<V2>, other3: UniChannel<V3>,
    ) -> UniChannel<(V, V1, V2, V3)> {
        (self, other1, other2, other3)
            .fsm(k, Some("zip4"), Expr::x(), |fwd, _, _| (fwd, ((), (), (), ()).into(), ().into()))
    }

    /// Zip5
    pub fn zip5<V1: Signal, V2: Signal, V3: Signal, V4: Signal>(
        self, k: &mut CompositeModuleContext, other1: UniChannel<V1>, other2: UniChannel<V2>, other3: UniChannel<V3>,
        other4: UniChannel<V4>,
    ) -> UniChannel<(V, V1, V2, V3, V4)> {
        (self, other1, other2, other3, other4)
            .fsm(k, Some("zip5"), Expr::x(), |fwd, _, _| (fwd, ((), (), (), (), ()).into(), ().into()))
    }

    /// Zip6
    pub fn zip6<V1: Signal, V2: Signal, V3: Signal, V4: Signal, V5: Signal>(
        self, k: &mut CompositeModuleContext, other1: UniChannel<V1>, other2: UniChannel<V2>, other3: UniChannel<V3>,
        other4: UniChannel<V4>, other5: UniChannel<V5>,
    ) -> UniChannel<(V, V1, V2, V3, V4, V5)> {
        (self, other1, other2, other3, other4, other5)
            .fsm(k, Some("zip6"), Expr::x(), |fwd, _, _| (fwd, ((), (), (), (), (), ()).into(), ().into()))
    }
}

impl<V: Signal> UniChannel<Valid<V>> {
    /// TODO: documentation
    ///
    /// Once `cond` is asserted, it must remain so until `self` is transferred. This is necessary to
    /// satisfy `zip_uni`'s requirement.
    #[must_use]
    pub fn filter(self, k: &mut CompositeModuleContext, cond: UniChannel<bool>) -> Self {
        // TODO: maybe we should set ready = false if cond = false?
        self.zip(k, cond).map(k, move |input| {
            let (value, cond) = *input;
            Expr::<Valid<_>>::new(value.valid & cond, value.inner)
        })
    }

    /// Maps the inner value.
    pub fn map_inner<W: Signal, F: 'static + for<'id> Fn(Expr<'id, V>) -> Expr<'id, W>>(
        self, k: &mut CompositeModuleContext, f: F,
    ) -> UniChannel<Valid<W>> {
        self.map(k, move |input| {
            let valid = input.valid;
            let inner = input.inner;

            Expr::<Valid<_>>::new(valid, f(inner))
        })
    }

    /// Zips the inner value with other value.
    pub fn zip_inner<W: Signal>(
        self, k: &mut CompositeModuleContext, other: UniChannel<W>,
    ) -> UniChannel<Valid<(V, W)>> {
        self.zip(k, other).map(k, move |input| {
            let (this, other) = *input;

            let valid = this.valid;
            let inner = this.inner;

            Expr::<Valid<_>>::new(valid, (inner, other).into())
        })
    }
}

impl<V: Signal> UniChannel<Valid<V>> {
    /// Transforms into a valid-ready channel.
    pub fn into_vr(self, k: &mut CompositeModuleContext) -> VrChannel<V> {
        self.fsm::<(), VrChannel<V>, _>(k, Some("into_vr"), Expr::from(()), |fwd, _, s| (fwd, Expr::from(()), s))
    }
}

impl<V: Signal> UniChannel<V> {
    /// Transforms into a deque channel.
    pub fn into_deq(self, k: &mut CompositeModuleContext) -> DeqChannel<V> {
        self.fsm::<(), DeqChannel<V>, _>(k, Some("into_deq"), ().into(), |fwd, _, s| (fwd, ().into(), s))
    }
}

//! Concentrate extension.
//!
//! For given a bunch of signals and a bitvector parameter, gather/concentrate those bits together
//! into a more condensed vector.

use crate::*;

/// Concentrate extension.
pub trait ConcentrateExt {
    /// Output interface.
    type Output<M: Num>;

    /// Concentration
    fn concentrate<M: Num>(self, k: &mut CompositeModuleContext, pattern: Vec<bool>) -> Self::Output<M>;
}

impl<V: Signal, N: Num> ConcentrateExt for UniChannel<Array<V, N>> {
    type Output<M: Num> = UniChannel<Array<V, M>>;

    fn concentrate<M: Num>(self, k: &mut CompositeModuleContext, pattern: Vec<bool>) -> UniChannel<Array<V, M>> {
        // Assertion: Pattern has same length as input width and it is longer than output width.
        assert!(pattern.len() == N::WIDTH && N::WIDTH >= M::WIDTH);

        self.map(k, move |fwd_i| {
            let mut fwd_o = Expr::<Array<V, M>>::x();
            let mut c = 0;

            for i in 0..N::WIDTH {
                if pattern[i] {
                    fwd_o = fwd_o.set(c.into(), fwd_i[i]);
                    c += 1;
                }
            }

            // TODO: Uncomment this
            // assert_eq!(c, M);

            fwd_o
        })
    }
}

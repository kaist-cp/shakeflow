//! Unconcentrate extension.
//!
//! Takes an input vector and spreads the elements according to a bit pattern.

use crate::*;

/// Unconcentrate extension.
pub trait UnconcentrateExt {
    /// Output interface
    type Output<M: Num>;

    /// Unconcentration
    fn unconcentrate<M: Num>(self, k: &mut CompositeModuleContext, pattern: Vec<bool>) -> Self::Output<M>;
}

impl<V: Signal, N: Num> UnconcentrateExt for UniChannel<Array<V, N>> {
    type Output<M: Num> = UniChannel<Array<V, M>>;

    fn unconcentrate<M: Num>(self, k: &mut CompositeModuleContext, pattern: Vec<bool>) -> UniChannel<Array<V, M>> {
        // Assertion: Pattern has same length as output width and it is longer than input width.
        assert!(pattern.len() == M::WIDTH && M::WIDTH >= N::WIDTH);

        self.map(k, move |i| {
            let mut o = Expr::<Array<V, M>>::x();
            let mut c = 0;

            for (idx, p) in pattern.iter().enumerate().take(M::WIDTH) {
                if *p {
                    o = o.set(idx.into(), i[c]);
                    c += 1;
                }
            }

            // TODO: Uncomment this
            // assert_eq!(c, N);

            o
        })
    }
}

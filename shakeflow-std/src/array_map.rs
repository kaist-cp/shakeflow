//! Array map.

use arrayvec::ArrayVec;

use crate::*;

/// Provide ergonomic API for array map.
pub trait ArrayMap<I: Interface, O: Interface> {
    /// Type of the output interface.
    type Target;

    /// Wraps self with given map.
    fn array_map(
        self, k: &mut CompositeModuleContext, name: &str, f: fn(I, &mut CompositeModuleContext) -> O,
    ) -> Self::Target;

    /// TODO: Documentation
    fn array_map_feedback<V: Signal>(
        self, k: &mut CompositeModuleContext, feedback: UniChannel<V>, name: &str,
        f: fn((I, UniChannel<V>), &mut CompositeModuleContext) -> O,
    ) -> Self::Target;

    /// TODO: Documentation and modify to use for-loop in the generated verilog
    fn array_map_enumerate<F: FnMut(usize, I) -> O>(self, f: F) -> Self::Target;
}

impl<const N: usize, I: Interface, O: Interface> ArrayMap<I, O> for [I; N] {
    type Target = [O; N];

    fn array_map(
        self, k: &mut CompositeModuleContext, name: &str, f: fn(I, &mut CompositeModuleContext) -> O,
    ) -> Self::Target {
        let module = composite::<I, O, _>(name, Some("in"), Some("out"), f).build_array();
        self.comb_inline(k, module)
    }

    fn array_map_feedback<V: Signal>(
        self, k: &mut CompositeModuleContext, feedback: UniChannel<V>, name: &str,
        f: fn((I, UniChannel<V>), &mut CompositeModuleContext) -> O,
    ) -> Self::Target {
        let module = composite::<(I, UniChannel<V>), O, _>(name, Some("in"), Some("out"), f).build_array();
        self.into_iter()
            .map(|ch| (ch, feedback.clone()))
            .collect::<ArrayVec<_, N>>()
            .into_inner()
            .unwrap()
            .comb_inline(k, module)
    }

    fn array_map_enumerate<F: FnMut(usize, I) -> O>(self, mut f: F) -> Self::Target {
        self.into_iter().enumerate().map(|(i, ch)| f(i, ch)).collect::<ArrayVec<O, N>>().into_inner().unwrap()
    }
}

/// TODO: Documentation
pub fn range_map<const N: usize, O: Interface, F: FnMut(usize) -> O>(f: F) -> [O; N] {
    (0..N).map(f).collect::<ArrayVec<O, N>>().into_inner().unwrap()
}

/// TODO: Documentation
pub trait UnzipExt<A: Interface, B: Interface> {
    /// Output from A
    type FromA;
    /// Output from B
    type FromB;

    /// Unzip function.
    fn unzip(self) -> (Self::FromA, Self::FromB);
}

impl<const N: usize, A: Interface, B: Interface> UnzipExt<A, B> for [(A, B); N] {
    type FromA = [A; N];
    type FromB = [B; N];

    fn unzip(self) -> (Self::FromA, Self::FromB) {
        let (from_a, from_b): (Vec<_>, Vec<_>) = self.into_iter().unzip();

        let from_a = from_a.into_iter().collect::<ArrayVec<_, N>>().into_inner().unwrap();
        let from_b = from_b.into_iter().collect::<ArrayVec<_, N>>().into_inner().unwrap();

        (from_a, from_b)
    }
}

/// TODO: Documentation
pub trait ZipExt<A: Interface, B: Interface> {
    /// Output from B
    type FromB;
    /// Output
    type Target;

    /// Zip function.
    fn array_zip(self, b: Self::FromB) -> Self::Target;
}

impl<const N: usize, A: Interface, B: Interface> ZipExt<A, B> for [A; N] {
    type FromB = [B; N];
    type Target = [(A, B); N];

    fn array_zip(self, b: [B; N]) -> [(A, B); N] {
        self.into_iter().zip(b.into_iter()).collect::<ArrayVec<(A, B), N>>().into_inner().unwrap()
    }
}

#[derive(Debug)]
/// Array Enumerator
pub struct ArrayEnumerate<const N: usize, I: Interface> {
    inner: [I; N],
}

/// TODO: Documentation
pub trait EnumerateExt<const N: usize, I: Interface> {
    /// enumerate combinator for [I; N]
    fn array_enumerate(self) -> ArrayEnumerate<N, I>;
}

impl<const N: usize, I: Interface> EnumerateExt<N, I> for [I; N] {
    fn array_enumerate(self) -> ArrayEnumerate<N, I> { ArrayEnumerate { inner: self } }
}

impl<const N: usize, I: Interface> ArrayEnumerate<N, I> {
    /// map combinator for array enumerator
    pub fn map<O: Interface, F: FnMut(usize, I) -> O>(self, mut f: F) -> [O; N] {
        self.inner.into_iter().enumerate().map(|(i, ch)| f(i, ch)).collect::<ArrayVec<O, N>>().into_inner().unwrap()
    }
}

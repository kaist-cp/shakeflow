use std::fmt::Debug;
use std::marker::PhantomData;

use tuple_utils::*;

use crate::*;

/// Bit-representable values.
///
/// NOTE: the paper draft says this trait contains `TYPE :: lir::PortDecls`, but it currently has different and yet equivalent components, `WIDTH` and `port_decls()`. We will fix it before author's response.
// HACK: currently, the inner implementation doesn't constrain the length of the boolean array
// because doing so would cause Rust ICE on const generics. Let's fix it later...
pub trait Signal: 'static + Debug + Clone {
    /// Signal's bit width.
    ///
    /// # Note
    ///
    /// `Self::WIDTH` and `Self::port_decls().width()` should be equal.
    const WIDTH: usize;

    #[doc(hidden)]
    fn transl(self) -> Vec<bool>;

    /// Port names and bitwidths.
    ///
    /// `Interface` of shakeflow is actually represented by multiple verilog channels combined.
    /// `port_decls` store the name and width information of these combined verilog channels.
    ///
    /// # Example
    ///
    /// LIR value type of `AxisValue` type used for `Qsfp28Channel` are as follows:
    ///
    /// ```ignore
    /// Struct([
    ///     (None, Struct([(Some("tdata"), Bits(512)), (Some("tkeep"), Bits(64))])),
    ///     (Some("tuser"), Bits(1)),
    ///     (Some("tlast"), Bits(1)),
    /// ])
    /// ```
    fn port_decls() -> lir::PortDecls;

    /// Generates a LIR value.
    fn to_lir(self) -> lir::Expr {
        lir::Expr::Constant { bits: self.transl().into_iter().collect::<Vec<_>>(), typ: Self::port_decls() }
    }
}

impl Signal for () {
    const WIDTH: usize = 0;

    fn transl(self) -> Vec<bool> { vec![] }

    fn port_decls() -> lir::PortDecls { lir::PortDecls::Bits(lir::Shape::new([0])) }
}

impl Signal for bool {
    const WIDTH: usize = 1;

    fn transl(self) -> Vec<bool> { vec![self] }

    fn port_decls() -> lir::PortDecls { lir::PortDecls::Bits(lir::Shape::new([1])) }
}

fn expand_dim_typ<N: Num>(typ: lir::PortDecls) -> lir::PortDecls {
    match typ {
        lir::PortDecls::Struct(inner) => lir::PortDecls::Struct(
            inner.into_iter().map(|(name, m)| (name, expand_dim_typ::<N>(m))).collect::<Vec<_>>(),
        ),
        lir::PortDecls::Bits(shape) => {
            assert_eq!(shape.dim(), 1);
            lir::PortDecls::Bits(lir::Shape::new([N::WIDTH, shape.width()]))
        }
    }
}

/// Array which expands dimension of variable in generated verilog code.
///
/// TODO: Support dimension greater than 2
#[derive(Debug, Clone)]
pub struct VarArray<V: Signal, N: Num> {
    _marker: PhantomData<(V, N)>,
}

impl<V: Signal, N: Num> Copy for VarArray<V, N> {}

impl<V: Signal, N: Num> Signal for VarArray<V, N> {
    const WIDTH: usize = V::WIDTH * N::WIDTH;

    fn transl(self) -> Vec<bool> { todo!() }

    fn port_decls() -> lir::PortDecls { expand_dim_typ::<N>(V::port_decls()) }
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! impl_signal_tuple {
    ($a:ident) => {
        impl<$a: Signal> Signal for ($a,) {
            const WIDTH: usize = <$a as Signal>::WIDTH;

            fn transl(self) -> Vec<bool> {
                ::std::iter::empty()
                    .chain(self.0.transl().into_iter())
                    .collect::<Vec<_>>()
            }

            fn port_decls() -> lir::PortDecls {
                lir::PortDecls::Struct(vec![(Some("0".to_string()), <$a as Signal>::port_decls())])
            }
        }
    };
    ($($a:ident)+) => {
        impl<$($a: Signal,)+> Signal for ($($a,)+) {
            const WIDTH: usize = <<Self as SplitLast>::Left as Signal>::WIDTH + <<Self as SplitLast>::Right as Signal>::WIDTH;

            fn transl(self) -> Vec<bool> {
                let (left, right) = SplitLast::split_last(self);

                left.transl()
                    .into_iter()
                    .chain(right.transl())
                    .collect()
            }

            fn port_decls() -> lir::PortDecls {
                match <<Self as SplitLast>::Left as Signal>::port_decls() {
                    lir::PortDecls::Struct(mut mbrs) => {
                        mbrs.push((
                            Some((Self::arity() - 1).to_string()),
                            <<Self as SplitLast>::Right as Signal>::port_decls(),
                        ));
                        lir::PortDecls::Struct(mbrs)
                    }
                    _ => panic!("internal compiler error"),
                }
            }
        }
    };
}

impl_signal_tuple! { V1 }
impl_signal_tuple! { V1 V2 }
impl_signal_tuple! { V1 V2 V3 }
impl_signal_tuple! { V1 V2 V3 V4 }
impl_signal_tuple! { V1 V2 V3 V4 V5 }
impl_signal_tuple! { V1 V2 V3 V4 V5 V6 }
impl_signal_tuple! { V1 V2 V3 V4 V5 V6 V7 }
impl_signal_tuple! { V1 V2 V3 V4 V5 V6 V7 V8 }
impl_signal_tuple! { V1 V2 V3 V4 V5 V6 V7 V8 V9 }
impl_signal_tuple! { V1 V2 V3 V4 V5 V6 V7 V8 V9 V10 }
impl_signal_tuple! { V1 V2 V3 V4 V5 V6 V7 V8 V9 V10 V11 }
impl_signal_tuple! { V1 V2 V3 V4 V5 V6 V7 V8 V9 V10 V11 V12 }

macro_rules! impl_signal {
    ($typ:ty) => {
        impl Signal for $typ {
            const WIDTH: usize = ::std::mem::size_of::<$typ>() * 8;

            fn transl(self) -> Vec<bool> {
                #[allow(trivial_numeric_casts)]
                (0..(::std::mem::size_of::<$typ>() * 8)).map(|i| (self & ((1 as $typ) << i)) != 0).collect::<Vec<_>>()
            }

            fn port_decls() -> lir::PortDecls { lir::PortDecls::Bits(lir::Shape::new([Self::WIDTH])) }
        }
    };
}

impl_signal!(u8);
impl_signal!(u16);
impl_signal!(u32);
impl_signal!(u64);
impl_signal!(u128);
impl_signal!(usize);

/// Array type
#[derive(Debug, Clone)]
pub struct Array<V: Signal, N: Num> {
    inner: Vec<V>,
    _marker: PhantomData<N>,
}

impl<V: Signal, N: Num> Array<V, N> {
    /// Creates new array.
    pub fn new(inner: Vec<V>) -> Self {
        assert_eq!(inner.len(), N::WIDTH);
        Self { inner, _marker: PhantomData }
    }
}

impl<V: Default + Signal, N: Num> Default for Array<V, N> {
    fn default() -> Self { Self::new(vec![V::default(); N::WIDTH]) }
}

/// Bits type
pub type Bits<N: Num> = Array<bool, N>;

impl<V: Signal, N: Num> Signal for Array<V, N> {
    const WIDTH: usize = V::WIDTH * N::WIDTH;

    fn transl(self) -> Vec<bool> {
        assert_eq!(N::WIDTH, self.inner.len());
        self.inner.into_iter().flat_map(|v| v.transl()).collect()
    }

    fn port_decls() -> lir::PortDecls { V::port_decls().multiple(N::WIDTH) }
}

impl<const N: usize> From<[bool; N]> for Bits<U<N>> {
    fn from(inner: [bool; N]) -> Self { Bits::new(inner.into_iter().collect()) }
}

//! IEEE 754 Single Precision ALU
//!
//! NOTE: In this implementation, we did not consider NaN or inf. In the future we will consider it.

use std::f32;
use std::ops::*;

use shakeflow_macro::Signal;

use crate::*;

/// Signal type representing float32.
#[derive(Debug, Clone, Signal)]
pub struct FP32 {
    #[member(name = "")]
    inner: Bits<U<32>>,
}

impl From<[bool; 32]> for FP32 {
    fn from(inner: [bool; 32]) -> Self { Self { inner: Bits::new(inner.into_iter().collect()) } }
}

impl From<f32> for FP32 {
    fn from(value: f32) -> Self { u32_to_bits(value.to_bits()).into() }
}

impl<'id> Expr<'id, FP32> {
    /// Creates new FP32 expr.
    pub fn new(inner: Expr<'id, Bits<U<32>>>) -> Self { FP32Proj { inner }.into() }
}

impl<'id> From<f32> for Expr<'id, FP32> {
    fn from(value: f32) -> Self { Self::new(Expr::<Bits<U<32>>>::from(value.to_bits() as usize)) }
}

impl<'id> Add<Expr<'id, FP32>> for Expr<'id, FP32> {
    type Output = Expr<'id, FP32>;

    fn add(self, rhs: Expr<'id, FP32>) -> Self::Output {
        let lhs = self.inner;
        let rhs = rhs.inner;

        let out = lir::Expr::Call {
            func_name: "adder".to_string(),
            args: vec![lhs.into_inner(), rhs.into_inner()],
            typ: <Bits<U<32>> as Signal>::port_decls(),
        }
        .into();

        Expr::<FP32>::new(out)
    }
}

impl<'id> Sub<Expr<'id, FP32>> for Expr<'id, FP32> {
    type Output = Expr<'id, FP32>;

    fn sub(self, rhs: Expr<'id, FP32>) -> Self::Output {
        let lhs = self.inner;
        let rhs = rhs.inner;

        let out = lir::Expr::Call {
            func_name: "subtractor".to_string(),
            args: vec![lhs.into_inner(), rhs.into_inner()],
            typ: <Bits<U<32>> as Signal>::port_decls(),
        }
        .into();

        Expr::<FP32>::new(out)
    }
}

impl<'id> Mul<Expr<'id, FP32>> for Expr<'id, FP32> {
    type Output = Expr<'id, FP32>;

    fn mul(self, rhs: Expr<'id, FP32>) -> Self::Output {
        let lhs = self.inner;
        let rhs = rhs.inner;

        let out = lir::Expr::Call {
            func_name: "multiplier".to_string(),
            args: vec![lhs.into_inner(), rhs.into_inner()],
            typ: <Bits<U<32>> as Signal>::port_decls(),
        }
        .into();

        Expr::<FP32>::new(out)
    }
}

impl<'id> Div<Expr<'id, FP32>> for Expr<'id, FP32> {
    type Output = Expr<'id, FP32>;

    fn div(self, rhs: Expr<'id, FP32>) -> Self::Output {
        let lhs = self.inner;
        let rhs = rhs.inner;

        let out = lir::Expr::Call {
            func_name: "divider".to_string(),
            args: vec![lhs.into_inner(), rhs.into_inner()],
            typ: <Bits<U<32>> as Signal>::port_decls(),
        }
        .into();

        Expr::<FP32>::new(out)
    }
}

impl<'id> Expr<'id, FP32> {
    /// Check two exprs are equal.
    pub fn is_eq(&self, other: Expr<'id, FP32>) -> Expr<'id, bool> {
        let rhs = self.inner;
        let lhs = other.inner;
        rhs.is_eq(lhs)
    }

    /// Check `self` is less than `other`.
    pub fn is_lt(&self, other: Expr<'id, FP32>) -> Expr<'id, bool> {
        let diff = *self - other;
        diff.inner[31].repr().is_eq(Expr::from(1))
    }

    /// Check `self` is greater than `other`.
    pub fn is_gt(&self, other: Expr<'id, FP32>) -> Expr<'id, bool> {
        let diff = *self - other;
        diff.inner[31].repr().is_eq(Expr::from(0)) & !diff.inner.is_eq(Expr::from(0))
    }

    /// Check `self` is less or equal than `other`.
    pub fn is_le(&self, other: Expr<'id, FP32>) -> Expr<'id, bool> {
        let diff = *self - other;
        diff.inner[31].repr().is_eq(Expr::from(1)) | diff.inner.is_eq(Expr::from(0))
    }

    /// Check `self` is greater or equal than `other`.
    pub fn is_ge(&self, other: Expr<'id, FP32>) -> Expr<'id, bool> {
        let diff = *self - other;
        diff.inner[31].repr().is_eq(Expr::from(0))
    }
}

//! TODO: Documentation

use std::fmt::Debug;
use std::marker::PhantomData;

use crate::clog2;

/// TODO: Documentation
pub trait Num: Debug + Clone + 'static {
    /// Width.
    const WIDTH: usize;
}

/// Usize number.
#[derive(Debug, Clone)]
pub struct U<const N: usize>;

impl<const N: usize> Num for U<N> {
    const WIDTH: usize = N;
}

/// Sum.
#[derive(Debug, Clone)]
pub struct Sum<L: Num, R: Num>(PhantomData<(L, R)>);

impl<L: Num, R: Num> Num for Sum<L, R> {
    const WIDTH: usize = L::WIDTH + R::WIDTH;
}

/// Diff.
#[derive(Debug, Clone)]
pub struct Diff<L: Num, R: Num>(PhantomData<(L, R)>);

impl<L: Num, R: Num> Num for Diff<L, R> {
    const WIDTH: usize = L::WIDTH - R::WIDTH;
}

/// Product.
#[derive(Debug, Clone)]
pub struct Prod<L: Num, R: Num>(PhantomData<(L, R)>);

impl<L: Num, R: Num> Num for Prod<L, R> {
    const WIDTH: usize = L::WIDTH * R::WIDTH;
}

/// Quotient.
#[derive(Debug, Clone)]
pub struct Quot<L: Num, R: Num>(PhantomData<(L, R)>);

impl<L: Num, R: Num> Num for Quot<L, R> {
    const WIDTH: usize = L::WIDTH / R::WIDTH;
}

/// Modular.
#[derive(Debug, Clone)]
pub struct Mod<L: Num, R: Num>(PhantomData<(L, R)>);

impl<L: Num, R: Num> Num for Mod<L, R> {
    const WIDTH: usize = L::WIDTH % R::WIDTH;
}

/// Log2.
#[derive(Debug, Clone)]
pub struct Log2<N: Num>(PhantomData<N>);

impl<N: Num> Num for Log2<N> {
    const WIDTH: usize = clog2(N::WIDTH);
}

/// Pow2.
#[derive(Debug, Clone)]
pub struct Pow2<N: Num>(PhantomData<N>);

impl<N: Num> Num for Pow2<N> {
    const WIDTH: usize = 2_usize.pow(N::WIDTH as u32);
}

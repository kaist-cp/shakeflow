#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use std::path::Path;

use shakeflow::libstd::{FsmExt, FP32};
use shakeflow::*;
use shakeflow_macro::Signal;

#[derive(Debug, Clone, Signal)]
pub struct I {
    a: Bits<U<32>>,
    b: Bits<U<32>>,
}

#[derive(Debug, Clone, Signal)]
pub struct O {
    add: Bits<U<32>>,
    sub: Bits<U<32>>,
    mul: Bits<U<32>>,
    div: Bits<U<32>>,
}

pub type IC = UniChannel<I>;
pub type OC = UniChannel<O>;

fn fpu() -> Module<IC, OC> {
    composite::<IC, OC, _>("fpu", Some("i"), Some("o"), |input, k| {
        input.map(k, |input| {
            let a = Expr::<FP32>::new(input.a);
            let b = Expr::<FP32>::new(input.b);

            OProj {
                add: (a + b).into_bits(),
                sub: (a - b).into_bits(),
                mul: (a * b).into_bits(),
                div: (a / b).into_bits(),
            }
            .into()
        })
    })
    .build()
}

fn main() -> Result<(), PackageError> {
    let mut package = Package::default();
    package.add(fpu());
    package.gen_vir(Path::new("./build"))
}

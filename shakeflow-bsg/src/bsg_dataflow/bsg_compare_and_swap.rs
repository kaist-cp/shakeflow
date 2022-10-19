//! Compare two values and swap them if they are not in order
//!
//! TODO: Add head and tail pointer

use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct I<Width: Num> {
    data: Array<Bits<Width>, U<2>>,
    swap_on_equal: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct O<Width: Num> {
    data: Array<Bits<Width>, U<2>>,
    swapped: bool,
}

pub type IC<Width: Num> = UniChannel<I<Width>>;
pub type OC<Width: Num> = UniChannel<O<Width>>;

pub fn m<Width: Num, const COND_SWAP_ON_EQUAL: bool>() -> Module<IC<Width>, OC<Width>> {
    composite::<IC<Width>, OC<Width>, _>("bsg_compare_and_swap", Some("i"), Some("o"), |input, k| {
        input.map(k, |input| {
            let data = input.data;
            let swap_on_equal = input.swap_on_equal;

            let gt = data[0].is_gt(data[1]);
            let eq = data[0].is_eq(data[1]);

            let swapped = if COND_SWAP_ON_EQUAL { gt | (eq & swap_on_equal) } else { gt };

            let data = swapped.cond([data[1], data[0]].into(), [data[0], data[1]].into());

            OProj { data, swapped }.into()
        })
    })
    .build()
}

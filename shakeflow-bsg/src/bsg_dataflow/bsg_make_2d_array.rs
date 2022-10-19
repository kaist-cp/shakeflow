use shakeflow::*;
use shakeflow_std::*;

pub type IC<Width: Num, const ITEMS: usize> = UniChannel<Array<Bits<Width>, U<ITEMS>>>;
pub type EC<Width: Num, const ITEMS: usize> = [UniChannel<Bits<Width>>; ITEMS];

pub fn m<Width: Num, const ITEMS: usize>() -> Module<IC<Width, ITEMS>, EC<Width, ITEMS>> {
    composite::<IC<Width, ITEMS>, EC<Width, ITEMS>, _>("bsg_make_2D_array", Some("i"), Some("o"), |input, k| {
        input.slice(k)
    })
    .build()
}

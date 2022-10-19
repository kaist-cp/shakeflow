use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Interface)]
pub struct IC<Width: Num, const ITEMS: usize> {
    data: UniChannel<Array<Bits<Width>, U<ITEMS>>>,
    select: UniChannel<Array<Bits<Log2<U<ITEMS>>>, U<ITEMS>>>,
}

pub type EC<Width: Num, const ITEMS: usize> = UniChannel<Array<Bits<Width>, U<ITEMS>>>;

pub fn m<Width: Num, const ITEMS: usize>() -> Module<IC<Width, ITEMS>, EC<Width, ITEMS>> {
    composite::<IC<Width, ITEMS>, EC<Width, ITEMS>, _>("bsg_permute_box", Some("i"), Some("o"), |input, k| {
        input.data.slice(k).permute(k, input.select).concat(k)
    })
    .build()
}

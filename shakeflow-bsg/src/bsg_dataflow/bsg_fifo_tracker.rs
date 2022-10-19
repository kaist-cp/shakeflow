use shakeflow::*;
use shakeflow_std::*;

pub type IC = fifo::tracker::IC;
pub type EC<PtrWidth: Num> = fifo::tracker::EC<PtrWidth>;

pub fn m<const SLOTS: usize, PtrWidth: Num>() -> Module<IC, EC<PtrWidth>> {
    composite::<IC, EC<PtrWidth>, _>("bsg_fifo_tracker", Some("i"), Some("o"), |input, k| {
        input.comb_inline(k, fifo::tracker::m::<SLOTS, PtrWidth>())
    })
    .build()
}

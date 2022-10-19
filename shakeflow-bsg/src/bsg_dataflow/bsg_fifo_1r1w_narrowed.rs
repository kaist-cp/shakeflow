use shakeflow::*;
use shakeflow_std::*;

pub type IC<Width: Num> = VrChannel<Bits<Width>>;
pub type EC<WidthOut: Num> = VrChannel<Bits<WidthOut>>;

type NumChunks<Width: Num, WidthOut: Num> = Quot<Diff<Sum<Width, WidthOut>, U<1>>, WidthOut>;

pub fn m<Width: Num, WidthOut: Num, const SLOTS: usize, const LSB_TO_MSB: bool>() -> Module<IC<Width>, EC<WidthOut>> {
    composite::<IC<Width>, EC<WidthOut>, _>("bsg_fifo_1r1w_narrowed", Some("i"), Some("o"), |input, k| {
        // TODO: this module assumes the (forward) payload is stable: once valid is asserted, the payload signal doesn't change. Is it true for all examples?
        input
            // FIFO of `els_p` elements of width `width_p`.
            .comb_inline(k, super::bsg_fifo_1r1w_small::m::<Bits<Width>, SLOTS, false>())
            // selecting from two FIFO outputs and sending one out at a time
            .fsm_egress::<Bits<Width>, Bits<WidthOut>, _>(k, None, 0.into(), |input, count| {
                let data = input.resize::<Prod<WidthOut, NumChunks<Width, WidthOut>>>().chunk::<WidthOut>();
                let index = if NumChunks::<Width, WidthOut>::WIDTH == 1 {
                    0.into()
                } else if LSB_TO_MSB {
                    count.resize()
                } else {
                    (Expr::<Bits<Width>>::from(NumChunks::<Width, WidthOut>::WIDTH - 1) - count).resize()
                };
                let data_o = data[index];
                let count_n = (count + 1.into()).resize();
                let last = count.is_eq((NumChunks::<Width, WidthOut>::WIDTH - 1).into());
                (data_o, count_n, last)
            })
    })
    .build()
}

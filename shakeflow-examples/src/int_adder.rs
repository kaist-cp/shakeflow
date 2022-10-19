use shakeflow::*;
use shakeflow_std::*;

pub const DATA_WIDTH: usize = 512;
pub const STRB_WIDTH: usize = DATA_WIDTH / 8;
pub const WORD_WIDTH: usize = 32;
pub const WORD_WIDTH_IN_BYTE: usize = WORD_WIDTH / 8;

// pub const NUM_WORDS: usize = DATA_WIDTH / WORD_WIDTH;

#[derive(Debug, Clone, Signal)]
pub struct IntAdderInput {
    axi_wdata: Bits<U<DATA_WIDTH>>,
    axi_wstrb: Bits<U<STRB_WIDTH>>,
}

pub type IntAdderInputChannel = UniChannel<IntAdderInput>;
pub type IntAdderOutputChannel = UniChannel<Bits<U<WORD_WIDTH>>>;

#[derive(Debug, Clone, Signal)]
struct State {
    axi_wdata_sum_reg: Bits<U<WORD_WIDTH>>,
}

pub fn int_adder() -> Module<IntAdderInputChannel, IntAdderOutputChannel> {
    composite::<IntAdderInputChannel, IntAdderOutputChannel, _>("int_adder", None, Some("axi_wdata_sum"), |input, k| {
        input.fsm_map::<State, _, _>(
            k,
            None,
            StateProj { axi_wdata_sum_reg: [false; WORD_WIDTH].into() }.into(),
            |input, state| {
                let addendum = input
                    .axi_wdata
                    .chunk::<U<WORD_WIDTH>>()
                    .zip(input.axi_wstrb.chunk::<U<WORD_WIDTH_IN_BYTE>>().resize())
                    .map(|e| {
                        let word = e.0;
                        let strb = e.1;
                        strb.all().cond(word, 0.into())
                    })
                    .tree_fold(|l, r| (l + r).resize());
                let sum = (state.axi_wdata_sum_reg + addendum).resize();
                (sum, StateProj { axi_wdata_sum_reg: sum }.into())
            },
        )
    })
    .build()
}

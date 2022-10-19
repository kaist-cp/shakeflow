//! DMA RAM interface.

use shakeflow::*;
use shakeflow_std::*;

/// RAM segment data width
pub const SEG_DATA_WIDTH: usize = 512;

/// RAM segment address width
pub const SEG_ADDR_WIDTH: usize = 12;

/// RAM segment byte enable width
pub const SEG_BE_WIDTH: usize = SEG_DATA_WIDTH / 8;

/// Write cmd.
#[derive(Debug, Clone, Signal)]
pub struct DmaRamWrCmd {
    be: Bits<U<SEG_BE_WIDTH>>,
    addr: Bits<U<SEG_ADDR_WIDTH>>,
    data: Bits<U<SEG_DATA_WIDTH>>,
}

/// Read cmd.
#[derive(Debug, Clone, Signal)]
pub struct DmaRamRdCmd {
    addr: Bits<U<SEG_ADDR_WIDTH>>,
}

/// Read resp.
#[derive(Debug, Clone, Signal)]
pub struct DmaRamRdResp {
    data: Bits<U<SEG_DATA_WIDTH>>,
}

#[derive(Debug, Interface)]
pub struct DmaPsdpramI<const SEG_COUNT: usize> {
    pub wr_cmd: [VrChannel<DmaRamWrCmd>; SEG_COUNT],
    pub rd_cmd: [VrChannel<DmaRamRdCmd>; SEG_COUNT],
}

#[derive(Debug, Interface)]
pub struct DmaPsdpramO<const SEG_COUNT: usize> {
    pub wr_done: [UniChannel<bool>; SEG_COUNT],
    pub rd_resp: [VrChannel<DmaRamRdResp>; SEG_COUNT],
}

//! AXI-Lite interface.
//!
//! TODO: Parametrize constants.

use shakeflow::*;
use static_assertions::*;

/// Width of AXI lite address bus in bits
const AXIL_DATA_WIDTH: usize = 32;

/// Width of AXI lite wstrb (width of data bus in words)
const AXIL_STRB_WIDTH: usize = AXIL_DATA_WIDTH / 8;

// Error: AXI lite interface width must be 32 (instance %m)
const_assert!(AXIL_DATA_WIDTH == 32);

// Error: AXI lite interface requires byte (8-bit) granularity (instance %m)
const_assert!(AXIL_STRB_WIDTH * 8 == AXIL_DATA_WIDTH);

/// s_axil_aw*, s_axil_ar*.
#[derive(Debug, Clone, Signal)]
pub struct Addr<const AXIL_ADDR_WIDTH: usize> {
    addr: Bits<U<AXIL_ADDR_WIDTH>>,
    prot: Bits<U<3>>,
}

/// s_axil_w*.
#[derive(Debug, Clone, Signal)]
pub struct WReq {
    data: Bits<U<AXIL_DATA_WIDTH>>,
    strb: Bits<U<AXIL_STRB_WIDTH>>,
}

impl WReq {
    pub fn new_expr() -> Expr<'static, Self> { WReqProj { data: 0.into(), strb: 0.into() }.into() }
}

/// s_axil_b*.
#[derive(Debug, Clone, Signal)]
pub struct WRes {
    resp: Bits<U<2>>,
}

impl WRes {
    pub fn new_expr() -> Expr<'static, Self> { WResProj { resp: 0.into() }.into() }
}

/// s_axil_r*.
#[derive(Debug, Clone, Signal)]
pub struct RRes {
    data: Bits<U<AXIL_DATA_WIDTH>>,
    resp: Bits<U<2>>,
}

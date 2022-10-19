//! Transmit.

use shakeflow::*;
use shakeflow_std::axis::*;

/// Request for transmit/receive.
#[derive(Debug, Clone, Signal)]
pub struct Req<const QUEUE_INDEX_WIDTH: usize, const REQ_TAG_WIDTH: usize> {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
}

/// Transmit/Receive request status.
#[derive(Debug, Clone, Signal)]
pub struct ReqStatus<const LEN_WIDTH: usize, const REQ_TAG_WIDTH: usize> {
    len: Bits<U<LEN_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
}

/// Descriptor.
#[derive(Debug, Clone, Signal)]
pub struct Desc<const AXIS_DESC_DATA_WIDTH: usize, const AXIS_DESC_KEEP_WIDTH: usize> {
    #[member(name = "")]
    data: Keep<U<AXIS_DESC_DATA_WIDTH>, U<AXIS_DESC_KEEP_WIDTH>>,
    // TODO: Use `DESC_REQ_TAG_WIDTH` instead of 5
    tid: Bits<U<5>>,
    tuser: bool,
}

/// Descriptor request status.
#[derive(Debug, Clone, Signal)]
pub struct DescReqStatus<
    const QUEUE_INDEX_WIDTH: usize,
    const QUEUE_PTR_WIDTH: usize,
    const CPL_QUEUE_INDEX_WIDTH: usize,
    const DESC_REQ_TAG_WIDTH: usize,
> {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    ptr: Bits<U<QUEUE_PTR_WIDTH>>,
    cpl: Bits<U<CPL_QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<DESC_REQ_TAG_WIDTH>>,
    empty: bool,
    error: bool,
}

/// Completion request.
#[derive(Debug, Clone, Signal)]
pub struct CplReq<const QUEUE_INDEX_WIDTH: usize, const DESC_REQ_TAG_WIDTH: usize, const CPL_DATA_SIZE: usize> {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<DESC_REQ_TAG_WIDTH>>,
    data: Bits<U<CPL_DATA_SIZE>>,
}

/// Completion request status.
#[derive(Debug, Clone, Signal)]
pub struct CplReqStatus<DescReqTagWidth: Num> {
    tag: Bits<DescReqTagWidth>,
    full: bool,
    error: bool,
}

// TODO: Use const generic instead of magic numbers (For now, it is only used in engine modules.)
#[derive(Debug, Clone, Signal)]
pub struct DmaDesc {
    dma_addr: Bits<U<64>>,
    ram_addr: Bits<U<19>>,
    len: Bits<U<16>>,
    tag: Bits<U<14>>,
}

// TODO: Use const generic instead of magic numbers (For now, it is only used in engine modules.)
#[derive(Debug, Clone, Signal)]
pub struct DmaDescStatus {
    tag: Bits<U<14>>,
    error: Bits<U<4>>,
}

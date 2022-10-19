use shakeflow::*;

#[derive(Debug, Clone, Signal)]
pub struct WormholeRouterHeader<const CORD_WIDTH_MP: usize, const LEN_WIDTH_MP: usize> {
    len: Bits<U<LEN_WIDTH_MP>>,
    cord: Bits<U<CORD_WIDTH_MP>>,
}

#[derive(Debug, Clone, Signal)]
pub struct WormholeConcentratorHeader<const CORD_WIDTH_MP: usize, const LEN_WIDTH_MP: usize, const CID_WIDTH_MP: usize>
{
    cid: Bits<U<CID_WIDTH_MP>>,
    len: Bits<U<LEN_WIDTH_MP>>,
    cord: Bits<U<CORD_WIDTH_MP>>,
}

#[derive(Debug, Clone, Signal)]
pub struct WormholeRouterPacket<const CORD_WIDTH_MP: usize, const LEN_WIDTH_MP: usize, const PAYLOAD_WIDTH_MP: usize> {
    payload: Bits<U<PAYLOAD_WIDTH_MP>>,
    len: Bits<U<LEN_WIDTH_MP>>,
    cord: Bits<U<CORD_WIDTH_MP>>,
}

#[derive(Debug, Clone, Signal)]
pub struct WormholeConcentratorPacket<
    const CORD_WIDTH_MP: usize,
    const LEN_WIDTH_MP: usize,
    const CID_WIDTH_MP: usize,
    const PAYLOAD_WIDTH_MP: usize,
> {
    payload: Bits<U<PAYLOAD_WIDTH_MP>>,
    cid: Bits<U<CID_WIDTH_MP>>,
    len: Bits<U<LEN_WIDTH_MP>>,
    cord: Bits<U<CORD_WIDTH_MP>>,
}

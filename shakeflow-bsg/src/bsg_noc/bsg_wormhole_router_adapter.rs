use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct WormholeRouterPacket<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize> {
    payload: Bits<U<MAX_PAYLOAD_WIDTH>>,
    len: Bits<U<LEN_WIDTH>>,
    cord: Bits<U<CORD_WIDTH>>,
}

#[derive(Debug, Interface)]
pub struct C<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize, const FLIT_WIDTH: usize> {
    packet: VrChannel<WormholeRouterPacket<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH>>,
    link: VrChannel<Bits<U<FLIT_WIDTH>>>,
}

pub fn m<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize, const FLIT_WIDTH: usize>(
) -> Module<
    C<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>,
    C<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>,
> {
    composite::<
        C<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>,
        C<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>,
        _,
    >("bsg_wormhole_router_adapter", Some("i"), Some("o"), |input, k| C {
        packet: input.link.comb_inline(k, super::bsg_wormhole_router_adapter_out::m()),
        link: input.packet.comb_inline(k, super::bsg_wormhole_router_adapter_in::m()),
    })
    .build()
}

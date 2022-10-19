use shakeflow::*;
use shakeflow_std::*;

use super::bsg_wormhole_router_adapter::*;
use crate::bsg_dataflow::bsg_serial_in_parallel_out_dynamic as sipo;

type MaxNumFlits<
    const MAX_PAYLOAD_WIDTH: usize,
    const LEN_WIDTH: usize,
    const CORD_WIDTH: usize,
    const FLIT_WIDTH: usize,
> = Quot<Sum<Sum<U<MAX_PAYLOAD_WIDTH>, U<LEN_WIDTH>>, U<CORD_WIDTH>>, U<FLIT_WIDTH>>;
type ProtocolLen<
    const MAX_PAYLOAD_WIDTH: usize,
    const LEN_WIDTH: usize,
    const CORD_WIDTH: usize,
    const FLIT_WIDTH: usize,
> = Log2<MaxNumFlits<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>>;

pub type IC<const FLIT_WIDTH: usize> = VrChannel<Bits<U<FLIT_WIDTH>>>;
pub type EC<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize> =
    VrChannel<WormholeRouterPacket<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH>>;

pub fn m<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize, const FLIT_WIDTH: usize>(
) -> Module<IC<FLIT_WIDTH>, EC<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH>> {
    composite::<IC<FLIT_WIDTH>, EC<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH>, _>(
        "bsg_wormhole_router_adapter_out",
        Some("i"),
        Some("link_o"),
        |input, k| {
            input
                .map(k, |input| {
                    sipo::IProj {
                        data: input,
                        len: input.clip_const::<ProtocolLen<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>>(0),
                    }
                    .into()
                })
                .comb_inline(k, sipo::m())
                .map(k, |input| {
                    let input = input.concat();

                    WormholeRouterPacketProj {
                        payload: input.clip_const::<U<MAX_PAYLOAD_WIDTH>>(0),
                        len: input.clip_const::<U<LEN_WIDTH>>(MAX_PAYLOAD_WIDTH),
                        cord: input.clip_const::<U<CORD_WIDTH>>(MAX_PAYLOAD_WIDTH + LEN_WIDTH),
                    }
                    .into()
                })
        },
    )
    .build()
}

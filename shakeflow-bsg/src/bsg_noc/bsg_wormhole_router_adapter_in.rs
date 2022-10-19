use shakeflow::*;
use shakeflow_std::*;

use super::bsg_wormhole_router_adapter::*;
use crate::bsg_dataflow::bsg_parallel_in_serial_out_dynamic as piso;

type MaxNumFlits<
    const MAX_PAYLOAD_WIDTH: usize,
    const LEN_WIDTH: usize,
    const CORD_WIDTH: usize,
    const FLIT_WIDTH: usize,
> = Quot<Sum<Sum<U<MAX_PAYLOAD_WIDTH>, U<LEN_WIDTH>>, U<CORD_WIDTH>>, U<FLIT_WIDTH>>;
type Padded<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize, const FLIT_WIDTH: usize> =
    Prod<MaxNumFlits<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>, U<FLIT_WIDTH>>;

pub type IC<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize> =
    VrChannel<WormholeRouterPacket<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH>>;
pub type EC<const FLIT_WIDTH: usize> = VrChannel<Bits<U<FLIT_WIDTH>>>;

pub fn m<const MAX_PAYLOAD_WIDTH: usize, const LEN_WIDTH: usize, const CORD_WIDTH: usize, const FLIT_WIDTH: usize>(
) -> Module<IC<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH>, EC<FLIT_WIDTH>> {
    composite::<IC<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH>, EC<FLIT_WIDTH>, _>(
        "bsg_wormhole_router_adapter_in",
        Some("i"),
        Some("link_o"),
        |input, k| {
            input
                .map(k, |input| {
                    let padded = input.payload.append(input.len).append(input.cord).resize::<Padded<
                        MAX_PAYLOAD_WIDTH,
                        LEN_WIDTH,
                        CORD_WIDTH,
                        FLIT_WIDTH,
                    >>();

                    piso::IProj { data: padded.chunk::<U<FLIT_WIDTH>>().resize(), len: input.len.resize() }.into()
                })
                .comb_inline(
                    k,
                    piso::m::<Bits<U<FLIT_WIDTH>>, MaxNumFlits<MAX_PAYLOAD_WIDTH, LEN_WIDTH, CORD_WIDTH, FLIT_WIDTH>>(),
                )
                .map(k, |input| input.data)
        },
    )
    .build()
}

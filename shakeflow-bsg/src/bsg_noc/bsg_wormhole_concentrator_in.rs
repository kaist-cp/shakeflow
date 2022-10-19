use shakeflow::*;
use shakeflow_std::*;

use super::{bsg_wormhole_router_input_control as input_control, bsg_wormhole_router_output_control as output_control};

pub type IC<const FLIT_WIDTH: usize, const N: usize> = [VrChannel<Bits<U<FLIT_WIDTH>>>; N];
pub type EC<const FLIT_WIDTH: usize> = VrChannel<Bits<U<FLIT_WIDTH>>>;

pub fn m<
    const FLIT_WIDTH: usize,
    const LEN_WIDTH: usize,
    const CID_WIDTH: usize,
    const CORD_WIDTH: usize,
    const N: usize,
>() -> Module<IC<FLIT_WIDTH, N>, EC<FLIT_WIDTH>> {
    composite::<IC<FLIT_WIDTH, N>, EC<FLIT_WIDTH>, _>(
        "bsg_wormhole_concentrator_in",
        Some("i"),
        Some("o"),
        |input, k| {
            input
                .array_map(k, "in_ch", |ch, k| {
                    let fifo_output = ch.fifo::<2>(k);
                    let (fifo_output, sent) = fifo_output.fire(k);
                    let (fifo_output, data) = fifo_output.clone_uni(k);

                    let wormhole_router_input_control = data
                        .zip(k, sent)
                        .map(k, |input| {
                            let (data, sent) = *input;
                            (
                                Expr::<Valid<_>>::new(
                                    data.valid,
                                    input_control::IProj {
                                        decoded_dest: 1.into(),
                                        payload_len: data.inner.clip_const::<U<LEN_WIDTH>>(CID_WIDTH),
                                    }
                                    .into(),
                                ),
                                sent,
                            )
                                .into()
                        })
                        .comb_inline(k, input_control::m::<1, LEN_WIDTH>());

                    fifo_output.zip_uni(k, wormhole_router_input_control).map(k, |input| {
                        output_control::IProj { data: input.0, reqs: input.1.reqs.into(), release: input.1.release }
                            .into()
                    })
                })
                .comb_inline(k, output_control::m::<FLIT_WIDTH, N>())
                .map(k, |input| input.0)
        },
    )
    .build()
}

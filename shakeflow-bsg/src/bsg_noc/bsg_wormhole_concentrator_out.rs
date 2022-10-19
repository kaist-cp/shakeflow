use shakeflow::*;
use shakeflow_std::*;

use super::{bsg_wormhole_router_input_control as input_control, bsg_wormhole_router_output_control as output_control};

pub type IC<const FLIT_WIDTH: usize> = VrChannel<Bits<U<FLIT_WIDTH>>>;
pub type EC<const FLIT_WIDTH: usize, const N: usize> = [VrChannel<Bits<U<FLIT_WIDTH>>>; N];

pub fn m<
    const FLIT_WIDTH: usize,
    const LEN_WIDTH: usize,
    const CID_WIDTH: usize,
    const CORD_WIDTH: usize,
    const N: usize,
>() -> Module<IC<FLIT_WIDTH>, EC<FLIT_WIDTH, N>> {
    composite::<IC<FLIT_WIDTH>, EC<FLIT_WIDTH, N>, _>(
        "bsg_wormhole_concentrator_out",
        Some("i"),
        Some("o"),
        |input, k| {
            let fifo_output = input.fifo::<2>(k);
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
                                decoded_dest: (Expr::<Bits<U<N>>>::from(1)
                                    << data.inner.clip_const::<U<CID_WIDTH>>(0).resize()),
                                payload_len: data.inner.clip_const::<U<LEN_WIDTH>>(CID_WIDTH),
                            }
                            .into(),
                        ),
                        sent,
                    )
                        .into()
                })
                .comb_inline(k, input_control::m::<N, LEN_WIDTH>())
                .map(k, |input| input.reqs.zip(input.release.repeat()))
                .slice(k);

            fifo_output.duplicate_any::<N>(k).array_zip(wormhole_router_input_control).array_map(
                k,
                "out_ch",
                |(ch, ctl), k| {
                    [ch.zip_uni(k, ctl).map(k, |input| {
                        output_control::IProj { data: input.0, reqs: input.1 .0, release: input.1 .1 }.into()
                    })]
                    .comb_inline(k, output_control::m::<FLIT_WIDTH, 1>())
                    .map(k, |input| input.0)
                },
            )
        },
    )
    .build()
}

use shakeflow::*;
use shakeflow_std::*;

use super::pkg::bsg_wormhole_router::*;
use super::types::*;

const FLIT_WIDTH_P: usize = 10;
const DIMS_P: usize = 2;
const DIRS_LP: usize = (DIMS_P * 2) + 1;

const CORD_DIMS_P: usize = DIMS_P;
const ROUTING_MATRIX_P: [[usize; DIRS_LP]; 2] = STRICT_XY;
const REVERSE_ORDER_P: usize = 0;

const LEN_WIDTH_LP: usize = 5;
const HOLD_ON_VALID_P: usize = 0;

#[derive(Debug, Clone, Signal)]
pub struct V {
    data: Bits<U<FLIT_WIDTH_P>>,
}

#[derive(Debug, Interface)]
pub struct IC {
    link: [VrChannel<V>; DIRS_LP],
    #[member(name = "my_cord")]
    coord: UniChannel<Bits<U<5>>>,
}

#[derive(Debug, Interface)]
pub struct OC {
    link: [VrChannel<V>; DIRS_LP],
}

#[derive(Debug, Clone, Signal)]
pub struct DorI {
    target_cord: Bits<U<5>>,
    my_cord: Bits<U<5>>,
}

pub type DorIC = UniChannel<DorI>;
pub type DorOC = UniChannel<Bits<U<DIRS_LP>>>;

#[derive(Debug, Clone, Signal)]
pub struct WicI {
    yumi: Ready,
    decoded_dest: Bits<U<DIRS_LP>>,
    payload_len: Bits<U<LEN_WIDTH_LP>>,
}

#[derive(Debug, Clone, Signal)]
pub struct WicO {
    reqs: Bits<U<DIRS_LP>>,
    release: bool,
    detected_header: bool,
}

pub type WicIC = UniChannel<Valid<WicI>>;
pub type WicOC = UniChannel<WicO>;

#[derive(Debug, Clone, Signal)]
pub struct WocI {
    reqs: bool,
    release: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct WocO {
    data_sel: Bits<U<DIRS_LP>>,
}

pub type WocIC = [VrChannel<WocI>; DIRS_LP];
pub type WocOC = VrChannel<WocO>;

pub type FeedbackC = UniChannel<Array<Bits<U<DIRS_LP>>, U<DIRS_LP>>>;

impl_custom_inst! {DorIC, DorOC, bsg_wormhole_router_decoder_dor, <dims_p, cord_dims_p, reverse_order_p>, false,}
impl_custom_inst! {WicIC, WicOC, bsg_wormhole_router_input_control, <output_dirs_p, payload_len_bits_p>, true,}
impl_custom_inst! {WocIC, WocOC, bsg_wormhole_router_output_control, <input_dirs_p, hold_on_valid_p>, true,}

pub fn m() -> Module<IC, OC> {
    composite::<(IC, FeedbackC), (OC, FeedbackC), _>(
        "bsg_wormhole_router",
        Some("i"),
        Some("o"),
        |(input, feedback), k| {
            let (in_ch, fifo_data) = input
                .link
                .array_map_enumerate(|i, link| {
                    let fifo = link.fifo::<2>(k);

                    let (fifo, any_yumi) = fifo.fire(k);

                    let filter = feedback.clone().map(k, move |input| input[i].any());

                    let fifo = fifo.filter_bwd(k, filter).into_uni(k, true);

                    let hdr = fifo.clone().map(k, |input| {
                        let data = input.inner.data;
                        WormholeRouterHeaderProj {
                            len: data.clip_const::<U<LEN_WIDTH_LP>>(0),
                            cord: data.clip_const::<U<5>>(LEN_WIDTH_LP),
                        }
                        .into()
                    });

                    let decoded_dest_lo = hdr
                        .clone()
                        .zip(k, input.coord.clone())
                        .map(k, |input| {
                            let (hdr, my_cord) = *input;
                            DorIProj { target_cord: hdr.cord, my_cord }.into()
                        })
                        .bsg_wormhole_router_decoder_dor::<DIMS_P, CORD_DIMS_P, REVERSE_ORDER_P>(
                            k,
                            "dor",
                            Some("i"),
                            Some("req_o"),
                        );

                    let decoded_dest_sparse_lo = decoded_dest_lo
                        .concentrate::<U<DIRS_LP>>(k, usize_to_bits::<DIRS_LP>(ROUTING_MATRIX_P[0][i]).to_vec());

                    let wic_i = fifo.clone().zip4(k, any_yumi, hdr, decoded_dest_sparse_lo).map(k, |input| {
                        let (fifo, any_yumi, hdr, decoded_dest_sparse_lo) = *input;

                        Expr::<Valid<_>>::new(
                            fifo.valid,
                            WicIProj {
                                yumi: Expr::<Ready>::new(any_yumi),
                                decoded_dest: decoded_dest_sparse_lo,
                                payload_len: hdr.len,
                            }
                            .into(),
                        )
                    });

                    let wic_o = wic_i.bsg_wormhole_router_input_control::<DIRS_LP, LEN_WIDTH_LP>(
                        k,
                        "wic",
                        Some("fifo_i"),
                        Some("o"),
                    );

                    let woc_i = fifo.clone().zip(k, wic_o).map(k, move |input| {
                        let (fifo, wic_o) = *input;
                        Expr::<Valid<_>>::new(
                            fifo.valid,
                            WocIProj { reqs: wic_o.reqs[i], release: wic_o.release }.into(),
                        )
                    });

                    let fifo_data_lo = fifo.map(k, |input| input.inner.data);

                    (woc_i, fifo_data_lo)
                })
                .unzip();

            let fifo_data = fifo_data.concat(k);

            let (out_ch, yumis_transpose) = range_map::<DIRS_LP, _, _>(|_| {
                // TODO: concentrate/unconcentrate
                let (in_ch, yumis_lo) = in_ch.clone().array_map(k, "yumis_lo", |ch, k| ch.into_vr(k).fire(k)).unzip();

                let yumis_lo = yumis_lo.concat(k);

                let woc_o = in_ch.bsg_wormhole_router_output_control::<DIRS_LP, HOLD_ON_VALID_P>(
                    k,
                    "woc",
                    Some("i"),
                    Some("o"),
                );

                let (woc_o, woc_o_fwd) = woc_o.clone_uni(k);
                let link_data_o = (fifo_data.clone(), woc_o_fwd.map(k, |input| input.inner.data_sel)).mux_one_hot(k);

                (
                    woc_o.zip_uni(k, link_data_o).map(k, |input| {
                        let (_, link_data_o) = *input;
                        VProj { data: link_data_o }.into()
                    }),
                    yumis_lo,
                )
            })
            .unzip();

            let yumis_transpose = yumis_transpose.transpose(k).concat(k);

            (OC { link: out_ch }, yumis_transpose)
        },
    )
    .loop_feedback()
    .build()
}

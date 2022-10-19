use arrayvec::ArrayVec;
use shakeflow::*;
use shakeflow_std::*;

const WIDTH_P: usize = 10;
// const X_CORD_WIDTH_P: usize = 8;
// const Y_CORD_WIDTH_P: usize = 8;
const LEN_WIDTH_P: usize = 5;
// const RESERVED_WIDTH_P: usize = 2;

const NUM_IN_P: usize = 10;
const REMOTE_CREDITS_P: usize = 6;
const MAX_PAYLOAD_FLITS_P: usize = 20;

const LG_CREDIT_DECIMATION_P: usize = clog2(REMOTE_CREDITS_P + 1);

const USE_PSEUDO_LARGE_FIFO_P: bool = true;

const COUNTER_MIN_VALUE_LP: usize = 0;
const MUX_NUM_IN_LP: usize = NUM_IN_P + 1;
const TAG_WIDTH_LP: usize = clog2(MUX_NUM_IN_LP);
const RAW_WIDTH_LP: usize = WIDTH_P - TAG_WIDTH_LP;

#[derive(Debug, Clone, Signal)]
pub struct V {
    data: Bits<U<WIDTH_P>>,
}

#[derive(Debug, Interface)]
pub struct IC {
    multi_data: VrChannel<V>,
    link: [VrChannel<V>; NUM_IN_P],
}

#[derive(Debug, Interface)]
pub struct OC {
    multi_data: VrChannel<V>,
    link: [VrChannel<V>; NUM_IN_P],
}

#[derive(Debug, Clone, Signal)]
pub struct IStateI {
    set: bool,
    val: Bits<U<LEN_WIDTH_P>>,
    down: bool,
}

pub type IStateOC = UniChannel<Bits<U<LEN_WIDTH_P>>>;

#[derive(Debug, Clone, Signal)]
pub struct OCountI {
    set: bool,
    val: Bits<U<LEN_WIDTH_P>>,
    down: bool,
}

pub type OCountOC = UniChannel<Bits<U<LEN_WIDTH_P>>>;

#[derive(Debug, Interface)]
pub struct ChannelTunnelC {
    multi_data: VrChannel<V>,
    data: [VrChannel<V>; NUM_IN_P],
}

#[derive(Debug, Interface)]
pub struct FeedbackC {
    icount_r: [UniChannel<Bits<U<LEN_WIDTH_P>>>; NUM_IN_P],
    istate_r: UniChannel<Bits<U<LEN_WIDTH_P>>>,
    ocount_r: [UniChannel<Bits<U<LEN_WIDTH_P>>>; NUM_IN_P],
    ostate_r: UniChannel<Bits<U<LEN_WIDTH_P>>>,
    ofifo_sel_r: UniChannel<Bits<U<TAG_WIDTH_LP>>>,
}

impl_custom_inst! {UniChannel<IStateI>, IStateOC, bsg_counter_set_down, <width_p, init_val_p, set_and_down_exclusive_p>, true,}
impl_custom_inst! {ChannelTunnelC, ChannelTunnelC, bsg_channel_tunnel, <width_p, num_in_p, remote_credits_p, use_pseudo_large_fifo_p, lg_credit_decimation_p>, true,}
impl_custom_inst! {VrChannel<V>, VrChannel<V>, bsg_fifo_1r1w_small, <width_p, els_p>, true,}
impl_custom_inst! {VrChannel<V>, VrChannel<V>, bsg_fifo_1r1w_large, <width_p, els_p>, true,}
impl_custom_inst! {VrChannel<V>, VrChannel<V>, bsg_fifo_1r1w_pseudo_large, <width_p, els_p>, true,}

pub fn m() -> Module<IC, OC> {
    composite::<(IC, FeedbackC), (OC, FeedbackC), _>(
        "bsg_channel_tunnel_wormhole",
        Some("i"),
        Some("o"),
        |(input, feedback), k| {
            let IC { multi_data: multi_data_i, link: link_i } = input;
            let FeedbackC { icount_r, istate_r, ocount_r, ostate_r, ofifo_sel_r } = feedback;

            let (multi_data_i, multi_data_i_fire) = multi_data_i.fire(k);
            let (multi_data_i, multi_data_i_fwd) = multi_data_i.clone_uni(k);

            let istate_i = multi_data_i_fwd
                .clone()
                .map(k, |input| input.inner.data)
                .zip3(k, multi_data_i_fire, istate_r)
                .map(k, |input| {
                    let (multi_data_i, fire, istate_r) = *input;
                    let multi_data_i_len = multi_data_i.clip_const::<U<LEN_WIDTH_P>>(0); // TODO: Is this correct?
                    let istate_r_is_min_lo = istate_r.is_eq(COUNTER_MIN_VALUE_LP.into());

                    IStateIProj {
                        set: fire & istate_r_is_min_lo,
                        val: multi_data_i_len,
                        down: fire & !istate_r_is_min_lo,
                    }
                    .into()
                });

            let istate_r = istate_i.clone().bsg_counter_set_down::<LEN_WIDTH_P, COUNTER_MIN_VALUE_LP, 1>(
                k,
                "istate",
                Some("i"),
                Some("o"),
            );

            let multi_data_i_tag =
                multi_data_i_fwd.clone().map(k, |input| input.inner.data.clip_const::<U<TAG_WIDTH_LP>>(RAW_WIDTH_LP));
            let istate_sel_lo = istate_i.map(k, |input| input.set);

            let ififo_sel_r = multi_data_i_tag.buffer_en(k, 0.into(), istate_sel_lo);

            let imux_sel_lo = istate_r.clone().zip(k, ififo_sel_r).map(k, |input| {
                let (istate_r, ififo_sel_r) = *input;
                istate_r.is_eq(COUNTER_MIN_VALUE_LP.into()).cond(NUM_IN_P.into(), ififo_sel_r)
            });

            let multi_data_i_data = multi_data_i_fwd.map(k, |input| input.inner);

            let ififo_vr: [VrChannel<V>; MUX_NUM_IN_LP] = multi_data_i
                .zip_uni(k, imux_sel_lo)
                .map(k, |input| {
                    let (_, imux_sel_lo) = *input;
                    (().into(), imux_sel_lo).into()
                })
                .demux(k)
                .array_map_feedback(k, multi_data_i_data, "zip_data", |(ch, data), k| {
                    ch.zip_uni(k, data).map(k, |input| input.1)
                });

            let mut ififo_vr = ififo_vr.into_iter();

            let outside = ififo_vr.next_back().unwrap().fifo::<2>(k);
            let ififo_o =
                ififo_vr.collect::<ArrayVec<_, NUM_IN_P>>().into_inner().unwrap().array_map(k, "ch_in", |ch, k| {
                    if USE_PSEUDO_LARGE_FIFO_P {
                        ch.bsg_fifo_1r1w_large::<WIDTH_P, { REMOTE_CREDITS_P * MAX_PAYLOAD_FLITS_P }>(
                            k,
                            "ififo",
                            Some("i"),
                            Some("o"),
                        )
                    } else {
                        ch.bsg_fifo_1r1w_pseudo_large::<WIDTH_P, { REMOTE_CREDITS_P * MAX_PAYLOAD_FLITS_P }>(
                            k,
                            "ififo",
                            Some("i"),
                            Some("o"),
                        )
                    }
                });

            let (ofifo, inside_ocount_r) = link_i
                .array_zip(ocount_r)
                .array_map(k, "ch_out", |(link_i, ocount_r), k| {
                    let (link_i, fire) = link_i.fire(k);
                    let (link_i, link_i_fwd) = link_i.clone_uni(k);

                    let ocount_i =
                        link_i_fwd.map(k, |input| input.inner.data).zip3(k, fire, ocount_r).map(k, |input| {
                            let (link_i, fire, ocount_r) = *input;
                            let link_i_len = link_i.clip_const::<U<LEN_WIDTH_P>>(0); // TODO: Is this correct?
                            let ocount_r_is_min_lo = ocount_r.is_eq(COUNTER_MIN_VALUE_LP.into());

                            OCountIProj {
                                set: fire & ocount_r_is_min_lo,
                                val: link_i_len,
                                down: fire & !ocount_r_is_min_lo,
                            }
                            .into()
                        });

                    // FIXME: actually same type, but rust recognize as different type.
                    let ocount_r = ocount_i.module_inst::<OCountOC>(
                        k,
                        "bsg_counter_set_down",
                        "ocount",
                        vec![
                            ("width_p", LEN_WIDTH_P),
                            ("init_val_p", COUNTER_MIN_VALUE_LP),
                            ("set_and_down_exclusive_p", 1),
                        ],
                        true,
                        Some("i"),
                        Some("o"),
                    );

                    let (o_headerin, ofifo): (VrChannel<_>, VrChannel<_>) =
                        link_i.zip_uni(k, ocount_r.clone()).split_map(k, |input| {
                            let (data, ocount_r) = *input;
                            (ocount_r.is_eq(COUNTER_MIN_VALUE_LP.into()), data, data).into()
                        });

                    let inside = o_headerin.fifo::<2>(k);

                    let ofifo = ofifo.bsg_fifo_1r1w_small::<WIDTH_P, 4>(k, "ofifo", Some("i"), Some("o"));

                    (ofifo, (inside, ocount_r))
                })
                .unzip();

            let (inside, ocount_r) = inside_ocount_r.unzip();

            let ChannelTunnelC { multi_data: outside_o, data: inside_o } =
                ChannelTunnelC { multi_data: outside, data: inside }
                    .bsg_channel_tunnel::<RAW_WIDTH_LP, NUM_IN_P, REMOTE_CREDITS_P, {
                        USE_PSEUDO_LARGE_FIFO_P as usize
                    }, LG_CREDIT_DECIMATION_P>(k, "channel_tunnel", Some("i"), Some("o"));

            let ofifo_last = outside_o.filter_bwd_valid(k).fifo::<2>(k);

            let ofifo = ::std::iter::empty()
                .chain(ofifo.into_iter())
                .chain(::std::iter::once(ofifo_last))
                .collect::<ArrayVec<_, MUX_NUM_IN_LP>>()
                .into_inner()
                .unwrap();

            let omux_sel_lo = ostate_r
                .clone()
                .zip(k, ofifo_sel_r)
                .map(k, |input| input.0.is_eq(COUNTER_MIN_VALUE_LP.into()).cond(NUM_IN_P.into(), input.1));

            let multi_data_o = (omux_sel_lo, ofifo).mux(k);

            let (multi_data_o, multi_yumi_i) = multi_data_o.fire(k);
            let (multi_data_o, multi_data_o_fwd) = multi_data_o.clone_uni(k);
            let multi_data_o_tag =
                multi_data_o_fwd.clone().map(k, |input| input.inner.data.clip_const::<U<TAG_WIDTH_LP>>(RAW_WIDTH_LP));

            let ostate_i = multi_data_o_fwd.zip3(k, multi_yumi_i, ostate_r).map(k, |input| {
                let (multi_data_o, yumi, ostate_r) = *input;
                let multi_data_o = multi_data_o.inner.data;
                let ostate_r_is_min_lo = ostate_r.is_eq(COUNTER_MIN_VALUE_LP.into());
                let multi_data_o_is_credit =
                    multi_data_o.clip_const::<U<TAG_WIDTH_LP>>(RAW_WIDTH_LP).is_eq(NUM_IN_P.into());

                IStateIProj {
                    set: yumi & ostate_r_is_min_lo & !multi_data_o_is_credit,
                    val: multi_data_o.clip_const::<U<LEN_WIDTH_P>>(0),
                    down: yumi & !ostate_r_is_min_lo,
                }
                .into()
            });

            let ostate_r = ostate_i.clone().bsg_counter_set_down::<LEN_WIDTH_P, COUNTER_MIN_VALUE_LP, 1>(
                k,
                "ostate",
                Some("i"),
                Some("o"),
            );

            let ostate_set_lo = ostate_i.map(k, |input| input.set);
            let ofifo_sel_r = multi_data_o_tag.buffer_en(k, Expr::x(), ostate_set_lo);

            let (link_o, icount_r) = ififo_o
                .array_zip(inside_o)
                .array_zip(icount_r)
                .array_map(k, "link_o", |((ififo_o, inside_o), icount_r), k| {
                    let icount_is_min_lo =
                        icount_r.clone().map(k, |input| input.is_eq(COUNTER_MIN_VALUE_LP.into()).repr());
                    let link_o = (icount_is_min_lo, [inside_o, ififo_o]).mux(k);
                    (link_o, icount_r)
                })
                .unzip();

            (OC { multi_data: multi_data_o, link: link_o }, FeedbackC {
                icount_r,
                istate_r,
                ocount_r,
                ostate_r,
                ofifo_sel_r,
            })
        },
    )
    .loop_feedback()
    .build()
}

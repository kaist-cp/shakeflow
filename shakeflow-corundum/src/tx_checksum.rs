use shakeflow::num::*;
use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::constants::tx_checksum::*;
use super::types::checksum::*;

/// Control information for packet.
#[derive(Debug, Clone, Signal)]
pub struct CmdControl {
    csum_enable: bool,
    csum_start: Bits<U<8>>,
    csum_offset: Bits<U<8>>,
    csum_init: Bits<U<16>>,
}

#[derive(Debug, Clone, Signal)]
pub struct State<const KEEP_WIDTH: usize> {
    mask: Bits<U<KEEP_WIDTH>>,
    first_cycle: bool,
    input_offset: Bits<U<8>>,
}

/// Checksum information.
#[derive(Debug, Clone, Signal)]
pub struct Csum {
    csum: Bits<U<16>>,
    offset: Bits<U<8>>,
    enable: bool,
}

impl Serialize for Csum {
    fn serialize<'id>(expr: Expr<'id, Self>) -> Expr<'id, Bits<U<{ Csum::WIDTH }>>> {
        let csum = expr.csum;
        let offset = expr.offset;
        let enable = expr.enable;
        enable.repr().append(offset).append(csum).resize()
    }
}

#[derive(Debug, Clone, Signal)]
pub struct Sum<V: Signal> {
    data: V,
    odd: bool,
    enable: bool,
    offset: Bits<U<8>>,
    init: Bits<U<16>>,
    init_valid: bool,
}

pub type CsumPipeline<V> = AxisValue<Sum<V>>;

impl Deserialize for Csum {
    fn deserialize<'id>(expr: Expr<'id, Bits<U<{ Self::WIDTH }>>>) -> Expr<'id, Self> {
        CsumProj { csum: expr.clip_const::<U<16>>(9), offset: expr.clip_const::<U<8>>(1), enable: expr[0] }.into()
    }
}

// TODO(jeehoon.kang): maybe we should add it to the macro as well (like set_data).
impl<'id, V: Signal> SumProj<'id, V> {
    /// Maps the inner data.
    pub fn map_data<W: Signal, F: Fn(Expr<'id, V>) -> Expr<'id, W>>(self, f: F) -> Expr<'id, Sum<W>> {
        SumProj { data: f(self.data), ..self }.into()
    }
}

fn csum_adder<N: Num, W: Num, const P: Protocol>(
    k: &mut CompositeModuleContext, input: VrChannel<CsumPipeline<Array<Bits<N>, W>>, P>,
) -> VrChannel<CsumPipeline<Array<Bits<num::Sum<N, U<1>>>, Quot<W, U<2>>>>> {
    input
        .map(k, |input| {
            let payload = input.payload.map_data(|data| data.chunk::<U<2>>().map(|b| b[0] + b[1]));
            AxisValueProj { payload, tlast: input.tlast }.into()
        })
        .buffer_always(k)
}

impl<const KEEP_WIDTH: usize> State<KEEP_WIDTH> {
    fn new() -> Self { Self { mask: [false; KEEP_WIDTH].into(), first_cycle: false, input_offset: [false; 8].into() } }
}

#[derive(Debug, Clone, Signal)]
pub struct Payload<const DATA_WIDTH: usize, const KEEP_WIDTH: usize, const USER_WIDTH: usize> {
    /// AXI4-Stream TDATA/TKEEP
    #[member(name = "")]
    payload: Keep<U<DATA_WIDTH>, U<KEEP_WIDTH>>,

    /// AXI4-Stream TUSER
    tuser: Bits<U<USER_WIDTH>>,

    /// AXI4-Stream TID
    tid: Bits<U<8>>,

    /// AXI4-Stream TDEST
    tdest: Bits<U<8>>,
}

#[derive(Debug, Interface)]
pub struct I<const DATA_WIDTH: usize, const KEEP_WIDTH: usize, const USER_WIDTH: usize> {
    s_axis: AxisChannel<Payload<DATA_WIDTH, KEEP_WIDTH, USER_WIDTH>>,
    s_axis_cmd: VrChannel<CmdControl>,
}
pub type O<const DATA_WIDTH: usize, const KEEP_WIDTH: usize, const USER_WIDTH: usize> =
    AxisChannel<Payload<DATA_WIDTH, KEEP_WIDTH, USER_WIDTH>>;

pub fn m(module_name: &str) -> Module<I<DATA_WIDTH, KEEP_WIDTH, USER_WIDTH>, O<DATA_WIDTH, KEEP_WIDTH, USER_WIDTH>> {
    composite::<I<DATA_WIDTH, KEEP_WIDTH, USER_WIDTH>, O<DATA_WIDTH, KEEP_WIDTH, USER_WIDTH>, _>(module_name, None, Some("m_axis"), |value, k| {
        // Wrapping command data to axis channel
        let s_axis_cmd = value
            .s_axis_cmd
            .map(k, |v| AxisValueProj { payload: v, tlast: true.into() }.into())
            .into_axis_vr(k);

        // Muxes cmd and data and duplicates it.
        let data_cmd = (s_axis_cmd, value.s_axis).axis_rr_mux(k);
        let (data_in, csum_in) = data_cmd.duplicate::<{ Protocol::Helpful }, { Protocol::Demanding }>(k);

        // Feeds data to a FIFO.
        let data_out: AxisVrChannel<_> = data_in
            .into_vr(k)
            .filter_map(k, |value| {
                let AxisValueProj { payload, tlast } = *value;
                let (index, _, data) = *payload;
                (index, AxisValueProj { payload: data, tlast }.into()).into()
            })
            .into_axis_vr(k)
            .axis_fifo::<U<DATA_FIFO_DEPTH>, DATA_WIDTH, true, KEEP_WIDTH, true, ID_ENABLE, ID_WIDTH, DEST_ENABLE, DEST_WIDTH, USER_ENABLE, USER_WIDTH, 0>(k, "data_fifo");

        // Computes checksum and feeds it in a FIFO. The checksum pipeline requires `fsm_fwd`
        // because it changes the `tvalid` expr (but not `tready`).
        let pipeline_input = csum_in
            .into_vr(k)
            .fsm_and_then::<(CsumPipeline<Array<Bits<U<17>>, U<{ DATA_WIDTH / 32 }>>>, State<KEEP_WIDTH>), CsumPipeline<Array<Bits<U<17>>, U<{ DATA_WIDTH / 32 }>>>, _>(
                k,
                None,
                (Expr::x(), Expr::from(State::new())).into(),
                |input, state| {
                    let (transfer, cmd, data) = *input.payload;

                    let payload = data.payload;
                    let s_axis_tdata = payload.tdata;
                    let s_axis_tkeep = payload.tkeep;
                    let s_axis_tlast = input.tlast;

                    let (sum, state) = *state;

                    // Masks data with keep and the state's mask.
                    let s_axis_tdata_masked = (s_axis_tdata
                        & (s_axis_tkeep & state.mask).map(|b| b.repeat::<U<8>>()).concat().resize::<U<DATA_WIDTH>>())
                    .chunk::<U<16>>();

                    let cmd_transfer = !transfer;
                    let data_transfer = transfer;

                    let sum_next = SumProj {
                        data: data_transfer.cond(
                            s_axis_tdata_masked.chunk::<U<2>>().map(|b| {
                                // Transforms host byte order to network byte order
                                let seg0 = b[0].clip_const::<U<8>>(0);
                                let seg1 = b[0].clip_const::<U<8>>(8);
                                let seg2 = b[1].clip_const::<U<8>>(0);
                                let seg3 = b[1].clip_const::<U<8>>(8);
                                (seg1.append(seg0) + seg3.append(seg2)).repr()
                            }),
                            sum.payload.data.resize(),
                        ).resize(),
                        odd: cmd_transfer.cond(cmd.csum_start[0], sum.payload.odd),
                        enable: cmd_transfer.cond(cmd.csum_enable, sum.payload.enable),
                        offset: cmd_transfer.cond(cmd.csum_offset, sum.payload.offset),
                        init: cmd_transfer.cond(cmd.csum_init, sum.payload.init),
                        init_valid: data_transfer.cond(state.first_cycle, sum.payload.init_valid),
                    }
                    .into();

                    let state_next: Expr<State<KEEP_WIDTH>> = StateProj {
                        mask: data_transfer.cond(
                                (state.input_offset.is_gt(0.into())).cond(
                                    (state.input_offset.is_ge(KEEP_WIDTH.into())).cond(
                                        0.into(),
                                        Expr::from(true).repeat::<U<KEEP_WIDTH>>() << state.input_offset.resize::<Log2<U<KEEP_WIDTH>>>(),
                                    ),
                                    Expr::from(true).repeat::<U<KEEP_WIDTH>>(),
                                ),
                                (cmd.csum_start.is_ge(KEEP_WIDTH.into())).cond(
                                    0.into(),
                                    Expr::from(true).repeat::<U<KEEP_WIDTH>>() << cmd.csum_start.resize::<Log2<U<KEEP_WIDTH>>>(),
                                ),
                            ),
                        first_cycle: cmd_transfer,
                        input_offset: data_transfer.cond(
                                (state.input_offset.is_gt(0.into())).cond(
                                    (state.input_offset.is_ge(KEEP_WIDTH.into()))
                                        .cond(state.input_offset - KEEP_WIDTH.into(), 0.into()),
                                        state.input_offset,
                                ),
                                (cmd.csum_start.is_ge(KEEP_WIDTH.into()))
                                    .cond(cmd.csum_start - KEEP_WIDTH.into(), 0.into()),
                            ),
                    }.into();

                    let output = Expr::<Valid<_>>::new(
                        data_transfer,
                        AxisValueProj {
                            payload: sum_next,
                            tlast: s_axis_tlast,
                        }
                        .into(),
                    );

                    let tlast = data_transfer.cond(s_axis_tlast, output.inner.tlast);
                    let state_inner = AxisValueProj { payload: sum_next, tlast }.into();
                    let state_next = (state_inner, state_next).into();

                    (output, state_next)
                },
            )
            .buffer_always(k)
            .map(k, |value| value.map(|sum| sum.map_data(|data| data.map(|e| e.resize::<U<17>>()).resize::<U<16>>())));

        let pipeline = csum_adder(k, pipeline_input);
        let pipeline = csum_adder(k, pipeline);
        let pipeline = csum_adder(k, pipeline);
        let pipeline = csum_adder(k, pipeline);

        let csum_out: AxisVrChannel<_> = pipeline.map(k, |value| {
                value.map(|sum| {
                    sum.map_data(|data| {
                        let data = data.repr();
                        let data = data.clip_const::<U<16>>(0) + data.clip_const::<U<5>>(16).resize();
                        let data = data.clip_const::<U<16>>(0) + data.clip_const::<U<1>>(16).resize();
                        data.resize::<U<16>>()
                    })
                })
            })
            .fsm_fwd::<(AxisValid<Csum>, Accumulator), Csum, _>(
                k,
                None,
                (Expr::tinvalid(), Expr::x()).into(),
                |input, state| {
                    let sum = input.inner.payload;
                    let sum_last = input.inner.tlast;
                    let sum_valid = input.valid;

                    let (csum, acc) = *state;

                    let acc_temp = sum.init_valid.cond(sum.init, acc.sum) + sum.data;
                    let acc_temp = acc_temp.clip_const::<U<16>>(0) + acc_temp.clip_const::<U<1>>(16).resize::<U<16>>();
                    let acc_temp = acc_temp.resize::<U<16>>();

                    let csum_next = (!sum_last).cond(csum.inner, CsumProj {
                        csum: sum.odd.cond(!acc_temp.clip_const::<U<8>>(8).append(acc_temp.clip_const::<U<8>>(0)), !acc_temp.clip_const::<U<8>>(0).append(acc_temp.clip_const::<U<8>>(8))).resize(),
                        offset: sum.offset,
                        enable: sum.enable,
                    }.into());

                    let acc_next = select! {
                        (sum_valid & sum_last) => Expr::from(0),
                        (sum_valid & !sum_last) => acc_temp,
                        default => acc.sum,
                    };

                    let state_next = (
                        AxisValidProj { inner: csum_next, tvalid: sum_valid & sum_last }.into(),
                        Expr::from(AccumulatorProj { sum: acc_next }),
                    ).into();

                    (Expr::<Valid<_>>::new(csum.tvalid, csum.inner), state_next)
            })
            .map::<AxisValue<Keep<U<{ 16 + 8 + 1 }>, U<1>>>, _>(k, |input| {
                AxisValueProj {
                    payload: KeepProj { tdata: input.serialize().resize(), tkeep: [false].into() }.into(),
                    tlast: false.into(),
                }
                .into()
            })
            .into_axis_vr(k)
            .axis_fifo::<U<CHECKSUM_FIFO_DEPTH>, 25, false, 0, false, false, 0, false, 0, false, 0, 0>(k, "csum_fifo")
            .into_vr(k)
            .map::<AxisValue<Csum>, _>(k, |input| {
                AxisValueProj { payload: Deserialize::deserialize(input.payload.tdata), tlast: true.into() }.into()
            })
            .into_axis_vr(k);

        // Muxes the two FIFOs and fills the calculated checksum in data.
        (csum_out, data_out)
            .axis_rr_mux(k)
            .into_vr(k)
            .fsm_map::<(Csum, bool), AxisValue<_>, _>(
                k,
                None,
                // TODO: Consider `enable` as valid bit
                (Expr::<Csum>::x().set_enable(false.into()), Expr::x()).into(),
                |input, state| {
                    let (frame, csum, data) = *input.payload;

                    let data_out_tdata = data.payload.tdata.chunk::<U<8>>();

                    let (csum_reg, csum_split_reg) = *state;
                    let csum_data_reg = csum_reg.csum;
                    let csum_enable_reg = csum_reg.enable;
                    let csum_offset_reg = csum_reg.offset;

                    let m_axis_tdata = select! {
                        !(frame & csum_enable_reg) | csum_offset_reg.is_ge(KEEP_WIDTH.into()) => data_out_tdata,
                        csum_split_reg => data_out_tdata.set(0.into(), csum_data_reg.clip_const::<U<8>>(0)),
                        csum_offset_reg.is_eq((KEEP_WIDTH - 1).into()) => data_out_tdata
                            .set((KEEP_WIDTH - 1).into(), csum_data_reg.clip_const::<U<8>>(0)),
                        default => data_out_tdata
                            .set(csum_offset_reg.resize(), csum_data_reg.clip_const::<U<8>>(8))
                            .set((csum_offset_reg + 1.into()).resize(), csum_data_reg.clip_const::<U<8>>(0)),
                        }
                        .concat();

                    let (enable, split) = *csum_enable_reg.cond(
                        select! {
                            csum_offset_reg.is_ge(KEEP_WIDTH.into()) => (csum_enable_reg, csum_split_reg).into(),
                            csum_split_reg => (false.into(), csum_split_reg).into(),
                            csum_offset_reg.is_eq((KEEP_WIDTH - 1).into()) => (csum_enable_reg, true.into()).into(),
                            default => (false.into(), csum_split_reg).into(),
                        },
                        (csum_enable_reg, csum_split_reg).into()
                    );

                    let state_next = (!frame).cond(
                        (csum, false.into()).into(),
                        (CsumProj {
                            csum: csum_data_reg,
                            enable,
                            offset: (csum_enable_reg & csum_offset_reg.is_ge(KEEP_WIDTH.into()))
                            .cond(csum_offset_reg - KEEP_WIDTH.into(), csum_offset_reg),
                        }.into(),
                        split,
                    ).into());

                    let output = AxisValueProj {
                        payload: (
                            frame,
                            Expr::<Payload<DATA_WIDTH, KEEP_WIDTH, USER_WIDTH>>::x()
                                .set_payload(KeepProj { tdata: m_axis_tdata.resize(), tkeep: data.payload.tkeep }.into())
                                .set_tuser(data.tuser),
                        ).into(),
                        tlast: input.tlast,
                    }
                    .into();

                    (output, state_next)
                }
            )
            .filter_map(k, |value| {
                let AxisValueProj { payload, tlast } = *value;
                let (index, data) = *payload;
                (index, AxisValueProj { payload: data, tlast }.into()).into()
            })
            .buffer_skid(k)
            .into_axis_vr(k)
    })
    .build()
}

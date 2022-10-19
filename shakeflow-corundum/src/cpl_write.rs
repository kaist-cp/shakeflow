//! Completion write module.

use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::constants::cpl_write::*;
use super::ffis::*;
use super::types::dma_ram::*;
use super::types::request::*;

#[derive(Debug, Clone, Signal)]
pub struct WReq {
    sel: Bits<U<SELECT_WIDTH>>,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
    data: Bits<U<{ CPL_SIZE * 8 }>>,
}

#[derive(Debug, Clone, Signal)]
pub struct WReqStatus {
    tag: Bits<U<REQ_TAG_WIDTH>>,
    full: bool,
    error: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct Cpl<const AXIS_DATA_WIDTH: usize, const AXIS_KEEP_WIDTH: usize> {
    #[member(name = "")]
    data: Keep<U<AXIS_DATA_WIDTH>, U<AXIS_KEEP_WIDTH>>,
    tid: Bits<U<8>>,
    tdest: Bits<U<8>>,
    tuser: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct CplEnqResp {
    addr: Bits<U<DMA_ADDR_WIDTH>>,
    tag: Bits<U<QUEUE_REQ_TAG_WIDTH>>,
    op_tag: Bits<U<QUEUE_OP_TAG_WIDTH>>,
    full: bool,
    error: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct CplEnqCommit {
    op_tag: Bits<U<QUEUE_OP_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct DmaWriteDescReq {
    ram_addr: Bits<U<RAM_ADDR_WIDTH>>,
    len: Bits<U<8>>,
    tag: Bits<Log2<U<DESC_TABLE_SIZE>>>,
}

#[derive(Debug, Clone, Signal)]
pub struct DmaWriteDescResp {
    dma_addr: Bits<U<DMA_ADDR_WIDTH>>,
    ram_addr: Bits<U<RAM_ADDR_WIDTH>>,
    len: Bits<U<DMA_LEN_WIDTH>>,
    tag: Bits<U<DMA_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct DmaWriteDescStatus<const TAG_WIDTH: usize> {
    tag: Bits<U<TAG_WIDTH>>,
    error: Bits<U<4>>,
}

#[derive(Debug, Interface)]
pub struct I {
    /// Completion write request input
    s_axis_req: VrChannel<WReq>,

    /// Completion enqueue response input
    s_axis_cpl_enqueue_resp: [VrChannel<CplEnqResp>; PORTS],

    /// DMA write descriptor status input
    s_axis_dma_write_desc_status: UniChannel<Valid<DmaWriteDescStatus<DMA_TAG_WIDTH>>>,

    /// RAM interface
    dma_ram_rd_cmd: [VrChannel<DmaRamRdCmd>; SEG_COUNT],

    /// Configuration
    enable: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct O {
    /// Completion write request status output
    m_axis_req_status: UniChannel<Valid<WReqStatus>>,

    /// Completion enqueue request output
    m_axis_cpl_enqueue_req: [VrChannel<Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>; PORTS],

    /// Completion enqueue commit output
    m_axis_cpl_enqueue_commit: [VrChannel<CplEnqCommit>; PORTS],

    /// DMA write descriptor output
    m_axis_dma_write_desc: VrChannel<DmaWriteDescResp>,

    /// RAM interface
    dma_ram_rd_resp: [VrChannel<DmaRamRdResp>; SEG_COUNT],
}

#[derive(Debug, Clone, Signal)]
pub struct Feedback {
    queue_query_ready: bool,
    completion_write_ready: bool,
    m_axis_cpl_enqueue_req: Valid<(Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>, Bits<U<CL_PORTS>>)>,
    dma_write_desc: Valid<DmaWriteDescReq>,
    m_axis_req_status: Valid<WReqStatus>,
    m_axis_dma_write_desc: Valid<DmaWriteDescResp>,
    m_axis_cpl_enqueue_commit: Valid<(CplEnqCommit, Bits<U<CL_PORTS>>)>,
}

#[derive(Debug, Interface)]
pub struct DmaClientAxisSinkI {
    s_axis_write_desc: VrChannel<DmaWriteDescReq>,
    s_axis_write_data: AxisChannel<Cpl<AXIS_DATA_WIDTH, AXIS_KEEP_WIDTH>>,
    ram_wr_done: [UniChannel<bool>; SEG_COUNT],
    enable: UniChannel<bool>,
    abort: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct DmaClientAxisSinkO {
    m_axis_write_desc_status: UniChannel<Valid<DmaWriteDescStatus<CL_DESC_TABLE_SIZE>>>,
    ram_wr_cmd: [VrChannel<DmaRamWrCmd>; SEG_COUNT],
}

#[derive(Debug, Clone, Signal)]
struct DescTableEntry {
    active: bool,
    invalid: bool,
    write_done: bool,
    sel: Bits<U<2>>,
    tag: Bits<U<7>>,
    op_tag: Bits<U<6>>,
}

#[derive(Debug, Clone, Signal)]
struct DescTable<const DESC_TABLE_SIZE: usize, const CL_DESC_TABLE_SIZE: usize> {
    entries: DescTableEntryVarArr<DESC_TABLE_SIZE>,
    start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    finish_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
}

impl<const DESC_TABLE_SIZE: usize, const CL_DESC_TABLE_SIZE: usize> DescTable<DESC_TABLE_SIZE, CL_DESC_TABLE_SIZE> {
    fn new_expr() -> Expr<'static, Self> {
        DescTableProj { entries: DescTableEntryVarArr::new_expr(), start_ptr: 0.into(), finish_ptr: 0.into() }.into()
    }
}

pub fn m() -> Module<I, O> {
    composite::<(I, ([VrChannel<DmaRamWrCmd>; SEG_COUNT], UniChannel<Feedback>)), _, _>(
        "cpl_write",
        None,
        None,
        |(input, (dma_ram_wr_cmd, feedback)), k| {
            // Instantiates dma_psdpram.
            let dma_psdpram_output = DmaPsdpramI {
                wr_cmd: dma_ram_wr_cmd,
                rd_cmd: input.dma_ram_rd_cmd,
            }.dma_psdpram::<{DESC_TABLE_SIZE * SEG_COUNT * SEG_BE_WIDTH}, {SEG_COUNT}, {SEG_DATA_WIDTH}, {SEG_ADDR_WIDTH}, {SEG_BE_WIDTH}, {RAM_PIPELINE}>(k, "dma_psdpram_inst", None, None);

            // queue query
            // wait for descriptor request
            let queue_query_ready = feedback.clone().map(k, |input| input.queue_query_ready);
            let s_axis_req = input.s_axis_req.transfer(k, queue_query_ready);

            let m_axis_cpl_enqueue_req = feedback.clone().map(k, |input| input.m_axis_cpl_enqueue_req);
            let (m_axis_cpl_enqueue_req_vr, m_axis_cpl_enqueue_req_remaining) =
                m_axis_cpl_enqueue_req.into_vr(k).register_slice_fwd(k);

            // initiate completion write to DMA RAM
            let cpl_data = s_axis_req.clone().map_inner(k, |inner| inner.data);
            let (cpl_data_vr, cpl_data_remaining) = cpl_data.into_vr(k).register_slice_fwd(k);

            let dma_write_desc = feedback.clone().map(k, |input| input.dma_write_desc);
            let (dma_write_desc_vr, dma_write_desc_remaining) = dma_write_desc.into_vr(k).register_slice_fwd(k);

            // start completion write
            // wait for queue query response
            let completion_write_ready = feedback.clone().map(k, |input| input.completion_write_ready);
            let s_axis_cpl_enqueue_resp = input
                .s_axis_cpl_enqueue_resp
                .array_map(k, "pmux", |input, k| {
                    input.filter_fwd_ready_neg(k).register_slice_bwd(k)
                })
                .priority_mux(k)
                .filter_bwd(k, completion_write_ready)
                .into_uni(k, true);

            // return descriptor request completion
            let m_axis_req_status = feedback.clone().map(k, |input| input.m_axis_req_status).buffer(k, Expr::invalid());

            // initiate completion write
            let m_axis_dma_write_desc = feedback.clone().map(k, |i| i.m_axis_dma_write_desc);
            let (m_axis_dma_write_desc_vr, m_axis_dma_write_desc_remaining) =
                m_axis_dma_write_desc.into_vr(k).register_slice_fwd(k);

            // commit enqueue operation
            let m_axis_cpl_enqueue_commit = feedback.map(k, |input| input.m_axis_cpl_enqueue_commit);
            let (m_axis_cpl_enqueue_commit_vr, m_axis_cpl_enqueue_commit_remaining) =
                m_axis_cpl_enqueue_commit.into_vr(k).register_slice_fwd(k);

            let cpl = cpl_data_vr
                .map(k, |fwd| {
                    let data = KeepProj { tdata: fwd, tkeep: Expr::from(true).repeat::<U<AXIS_KEEP_WIDTH>>() };
                    let payload =
                        CplProj { data: data.into(), tid: 0.into(), tdest: 0.into(), tuser: Expr::from(false) };
                    AxisValueProj { payload: payload.into(), tlast: Expr::from(true) }.into()
                })
                .into_axis_vr(k);

            // Instantiates dma_client_axis_sink.
            let dma_client_axis_sink_output = DmaClientAxisSinkI {
                s_axis_write_desc: dma_write_desc_vr,
                s_axis_write_data: cpl,
                ram_wr_done: dma_psdpram_output.wr_done,
                enable: UniChannel::source(k, true.into()),
                abort: UniChannel::source(k, false.into()),
            }.dma_client_axis_sink::<{SEG_COUNT}, {SEG_DATA_WIDTH}, {SEG_ADDR_WIDTH}, {SEG_BE_WIDTH}, {RAM_ADDR_WIDTH}, {AXIS_DATA_WIDTH}, {(AXIS_KEEP_WIDTH > 1) as usize}, {AXIS_KEEP_WIDTH}, 1, 0, 0, 1, 1, 8, CL_DESC_TABLE_SIZE>(k, "dma_client_axis_sink_inst", None, None);

            let feedback: UniChannel<Feedback> = (input.enable)
                .zip6(k, input.s_axis_dma_write_desc_status, s_axis_req, s_axis_cpl_enqueue_resp, dma_write_desc_remaining, m_axis_dma_write_desc_remaining)
                .zip4(k, m_axis_cpl_enqueue_req_remaining, m_axis_cpl_enqueue_commit_remaining, cpl_data_remaining)
                .fsm_map(
                    k,
                    None,
                    DescTable::<DESC_TABLE_SIZE, CL_DESC_TABLE_SIZE>::new_expr(),
                    |input, state| {
                        // Projections.
                        let (input, m_axis_cpl_enqueue_req_remaining, m_axis_cpl_enqueue_commit_remaining, cpl_data_remaining) = *input;
                        let (enable, s_axis_dma_write_desc_status, s_axis_req, s_axis_cpl_enqueue_resp, dma_write_desc_remaining, m_axis_dma_write_desc_remaining) = *input;
                        let mut state = *state;
                        let entries = state.entries;

                        // Calculates ready exprs.

                        let start_ptr = state.start_ptr;
                        let start_ptr_idx = start_ptr.clip_const::<Log2<U<DESC_TABLE_SIZE>>>(0);
                        let start_entry = DescTableEntryVarArr::get_entry(entries, start_ptr_idx);

                        let finish_ptr = state.finish_ptr;
                        let finish_ptr_idx = finish_ptr.clip_const::<Log2<U<DESC_TABLE_SIZE>>>(0);
                        let finish_entry = DescTableEntryVarArr::get_entry(entries, finish_ptr_idx);

                        let start_entry_ready =
                            !start_entry.active & (start_ptr - finish_ptr).is_lt(DESC_TABLE_SIZE.into());

                        let queue_query_ready = enable
                            & start_entry_ready
                            & !m_axis_cpl_enqueue_req_remaining
                            & !dma_write_desc_remaining
                            & !cpl_data_remaining;

                        let completion_write_ready = s_axis_cpl_enqueue_resp.valid & !m_axis_dma_write_desc_remaining;

                        // Calculates output exprs.

                        // initiate queue query
                        let s_axis_req_valid = s_axis_req.valid;
                        let s_axis_req = s_axis_req.inner;

                        let desc_table_start_en = s_axis_req_valid;

                        let m_axis_cpl_enqueue_req = Expr::<Valid<_>>::new(
                            s_axis_req_valid,
                            (
                                ReqProj { queue: s_axis_req.queue, tag: start_ptr_idx.resize() }.into(),
                                s_axis_req.sel,
                            ).into(),
                        );

                        // initiate completion write to DMA RAM
                        let dma_write_desc = Expr::<Valid<_>>::new(
                            s_axis_req_valid,
                            DmaWriteDescReqProj {
                                ram_addr: start_ptr_idx.resize() << 5,
                                len: Expr::from(CPL_SIZE as u8).repr(),
                                tag: start_ptr_idx,
                            }
                            .into(),
                        );

                        // return descriptor request completion
                        let s_axis_cpl_enqueue_resp = s_axis_cpl_enqueue_resp.inner.1;

                        let m_axis_req_status = Expr::<Valid<_>>::new(
                            completion_write_ready,
                            WReqStatusProj {
                                tag: DescTableEntryVarArr::get_entry(
                                    entries,
                                    s_axis_cpl_enqueue_resp.tag.clip_const::<Log2<U<DESC_TABLE_SIZE>>>(0),
                                )
                                .tag,
                                full: s_axis_cpl_enqueue_resp.full,
                                error: s_axis_cpl_enqueue_resp.error,
                            }
                            .into(),
                        );

                        // initiate completion write
                        let queue_full = s_axis_cpl_enqueue_resp.error | s_axis_cpl_enqueue_resp.full;

                        let m_axis_dma_write_desc = Expr::<Valid<_>>::new(
                            completion_write_ready & !queue_full,
                            DmaWriteDescRespProj {
                                dma_addr: s_axis_cpl_enqueue_resp.addr,
                                ram_addr: s_axis_cpl_enqueue_resp.tag.resize() << 5,
                                len: Expr::from(CPL_SIZE as u16).repr(),
                                tag: s_axis_cpl_enqueue_resp.tag.resize(),
                            }
                            .into(),
                        );

                        let active = finish_entry.active & !start_ptr.is_eq(finish_ptr);
                        let invalid = finish_entry.invalid;
                        let write_done = finish_entry.write_done & !m_axis_cpl_enqueue_commit_remaining;

                        let desc_table_finish_en = active & (invalid | write_done);

                        let desc_table_enqueue = Expr::<Valid<_>>::new(
                            completion_write_ready,
                            s_axis_cpl_enqueue_resp.tag.resize(),
                        );

                        let desc_table_cpl_write_done = s_axis_dma_write_desc_status.map_inner(|inner| inner.tag.resize());

                        // commit enqueue operation
                        let m_axis_cpl_enqueue_commit = Expr::<Valid<_>>::new(
                            active & !invalid & write_done,
                            (
                                CplEnqCommitProj { op_tag: finish_entry.op_tag }.into(),
                                finish_entry.sel,
                            ).into(),
                        );

                        let feedback = FeedbackProj {
                            queue_query_ready,
                            completion_write_ready,
                            m_axis_cpl_enqueue_req,
                            dma_write_desc,
                            m_axis_req_status,
                            m_axis_dma_write_desc,
                            m_axis_cpl_enqueue_commit,
                        };

                        // Calculates the new state.

                        let mut entries = *entries;

                        // desc_table_start_en
                        entries.active = if_then_set! { entries.active, desc_table_start_en, start_ptr_idx, Expr::from(true) };
                        entries.invalid = if_then_set! { entries.invalid, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                        entries.write_done = if_then_set! { entries.write_done, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                        entries.sel = if_then_set_var_arr! { entries.sel, desc_table_start_en, start_ptr_idx, s_axis_req.sel };
                        entries.tag = if_then_set_var_arr! { entries.tag, desc_table_start_en, start_ptr_idx, s_axis_req.tag };
                        entries.op_tag = if_then_set_var_arr! { entries.op_tag, desc_table_start_en, start_ptr_idx, start_entry.op_tag };

                        let new_start_ptr = (start_ptr + 1.into()).resize();
                        state.start_ptr = (!desc_table_start_en).cond(start_ptr, new_start_ptr);

                        // desc_table_enqueue_en
                        entries.op_tag = if_then_set_var_arr! { entries.op_tag, desc_table_enqueue.valid, desc_table_enqueue.inner, s_axis_cpl_enqueue_resp.op_tag };
                        entries.invalid = if_then_set! { entries.invalid, desc_table_enqueue.valid, desc_table_enqueue.inner, completion_write_ready & queue_full };

                        // desc_table_cpl_write_done_en
                        entries.write_done = if_then_set! { entries.write_done, desc_table_cpl_write_done.valid, desc_table_cpl_write_done.inner, Expr::from(true) };

                        // desc_table_finish_en
                        entries.active = if_then_set! { entries.active, desc_table_finish_en, finish_ptr_idx, Expr::from(false) };

                        state.finish_ptr = (!desc_table_finish_en).cond(finish_ptr, (finish_ptr + 1.into()).resize());

                        state.entries = entries.into();
                        (feedback.into(), state.into())
                    },
                );

            (
                O {
                    m_axis_req_status,
                    m_axis_cpl_enqueue_req: m_axis_cpl_enqueue_req_vr.demux(k),
                    m_axis_cpl_enqueue_commit: m_axis_cpl_enqueue_commit_vr.demux(k),
                    m_axis_dma_write_desc: m_axis_dma_write_desc_vr,
                    dma_ram_rd_resp: dma_psdpram_output.rd_resp,
                },
                (dma_client_axis_sink_output.ram_wr_cmd, feedback),
            )
        },
    )
    // Feeds the feedback expr to itself.
    .loop_feedback()
    .build()
}

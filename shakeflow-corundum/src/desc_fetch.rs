//! Descriptor fetch.

// TODO(jeehoon.kang): can we merge desc_fetch and cpl_write?

use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::constants::desc_fetch::*;
use super::ffis::*;
use super::types::dma_ram::*;
use super::types::request::*;

#[derive(Debug, Clone, Signal)]
pub struct RReq {
    sel: Bits<U<SELECT_WIDTH>>,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct RReqStatus {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    ptr: Bits<U<QUEUE_PTR_WIDTH>>,
    cpl: Bits<U<CPL_QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
    empty: bool,
    error: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct Desc<const AXIS_DATA_WIDTH: usize, const AXIS_KEEP_WIDTH: usize> {
    #[member(name = "")]
    data: Keep<U<AXIS_DATA_WIDTH>, U<AXIS_KEEP_WIDTH>>,
    tid: Bits<U<REQ_TAG_WIDTH>>,
    tuser: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct DescDeqResp {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    ptr: Bits<U<QUEUE_PTR_WIDTH>>,
    addr: Bits<U<DMA_ADDR_WIDTH>>,
    block_size: Bits<U<LOG_BLOCK_SIZE_WIDTH>>,
    cpl: Bits<U<CPL_QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<QUEUE_REQ_TAG_WIDTH>>,
    op_tag: Bits<U<QUEUE_OP_TAG_WIDTH>>,
    empty: bool,
    error: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct DescDeqCommit {
    op_tag: Bits<U<QUEUE_OP_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct DmaReadDescReq {
    ram_addr: Bits<U<RAM_ADDR_WIDTH>>,
    len: Bits<U<8>>,
    tag: Bits<Log2<U<DESC_TABLE_SIZE>>>,
    id: Bits<U<REQ_TAG_WIDTH>>,
    user: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct DmaReadDescResp {
    dma_addr: Bits<U<DMA_ADDR_WIDTH>>,
    ram_addr: Bits<U<RAM_ADDR_WIDTH>>,
    len: Bits<U<DMA_LEN_WIDTH>>,
    tag: Bits<U<DMA_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct DmaReadDescStatus<const TAG_WIDTH: usize> {
    tag: Bits<U<TAG_WIDTH>>,
    error: Bits<U<4>>,
}

#[derive(Debug, Interface)]
pub struct I {
    /// Descriptor read request input
    s_axis_req: VrChannel<RReq>,

    /// Descriptor dequeue response input
    s_axis_desc_dequeue_resp: [VrChannel<DescDeqResp>; PORTS],

    /// DMA read descriptor status input
    s_axis_dma_read_desc_status: UniChannel<Valid<DmaReadDescStatus<DMA_TAG_WIDTH>>>,

    /// RAM interface
    dma_ram_wr_cmd: [VrChannel<DmaRamWrCmd>; SEG_COUNT],

    /// Configuration
    enable: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct O {
    /// Descriptor read request status output
    m_axis_req_status: UniChannel<Valid<RReqStatus>>,

    /// Descriptor data output
    m_axis_desc: AxisChannel<Desc<AXIS_DATA_WIDTH, AXIS_KEEP_WIDTH>>,

    /// Descriptor dequeue request output
    m_axis_desc_dequeue_req: [VrChannel<Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>; PORTS],

    /// Descriptor dequeue commit output
    m_axis_desc_dequeue_commit: [VrChannel<DescDeqCommit>; PORTS],

    /// DMA read descriptor output
    m_axis_dma_read_desc: VrChannel<DmaReadDescResp>,

    /// RAM interface
    dma_ram_wr_done: [UniChannel<bool>; SEG_COUNT],
}

#[derive(Debug, Clone, Signal)]
pub struct Feedback {
    queue_query_ready: bool,
    descriptor_fetch_ready: bool,
    dma_read_desc: Valid<DmaReadDescReq>,
    m_axis_dma_read_desc: Valid<DmaReadDescResp>,
    m_axis_desc_dequeue_commit: Valid<(DescDeqCommit, Bits<U<CL_PORTS>>)>,
}

#[derive(Debug, Interface)]
pub struct DmaClientAxisSourceI {
    s_axis_read_desc: VrChannel<DmaReadDescReq>,
    ram_rd_resp: [VrChannel<DmaRamRdResp>; SEG_COUNT],
    enable: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct DmaClientAxisSourceO {
    m_axis_read_desc_status: UniChannel<Valid<DmaReadDescStatus<CL_DESC_TABLE_SIZE>>>,
    m_axis_read_data: AxisChannel<Desc<AXIS_DATA_WIDTH, AXIS_KEEP_WIDTH>>,
    ram_rd_cmd: [VrChannel<DmaRamRdCmd>; SEG_COUNT],
}

#[derive(Debug, Clone, Signal)]
struct DescTableEntry {
    active: bool,
    desc_fetched: bool,
    read_done: bool,
    sel: Bits<U<1>>,
    log_desc_block_size: Bits<U<2>>,
    tag: Bits<U<7>>,
    op_tag: Bits<U<6>>,
}

#[derive(Debug, Clone, Signal)]
struct DescTable<const DESC_TABLE_SIZE: usize, const CL_DESC_TABLE_SIZE: usize> {
    entries: DescTableEntryVarArr<DESC_TABLE_SIZE>,
    start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    desc_read_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    finish_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    active_count: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
}

impl<const DESC_TABLE_SIZE: usize, const CL_DESC_TABLE_SIZE: usize> DescTable<DESC_TABLE_SIZE, CL_DESC_TABLE_SIZE> {
    fn new_expr() -> Expr<'static, Self> {
        DescTableProj {
            entries: DescTableEntryVarArr::new_expr(),
            start_ptr: 0.into(),
            desc_read_ptr: 0.into(),
            finish_ptr: 0.into(),
            active_count: 0.into(),
        }
        .into()
    }
}

pub fn m() -> Module<I, O> {
    composite::<(I, ([VrChannel<DmaRamRdCmd>; SEG_COUNT], UniChannel<Feedback>)), _, _>(
        "desc_fetch",
        None,
        None,
        |input, k| {
            // Projections.
            let (input, (dma_ram_rd_cmd, feedback)) = input;

            // Instantiates dma_psdpram.
            let dma_psdpram_output = DmaPsdpramI {
                wr_cmd: input.dma_ram_wr_cmd,
                rd_cmd: dma_ram_rd_cmd,
            }.dma_psdpram::<{DESC_TABLE_SIZE * DESC_SIZE * (1 << ((1 << LOG_BLOCK_SIZE_WIDTH) - 1))}, {SEG_COUNT}, {SEG_DATA_WIDTH}, {SEG_ADDR_WIDTH}, {SEG_BE_WIDTH}, {RAM_PIPELINE}>(k, "dma_psdram_inst", None, None);

            // queue query
            // wait for descriptor request
            let queue_query_ready = feedback.clone().map(k, |input| input.queue_query_ready);
            let m_axis_desc_dequeue_req = input.s_axis_req.transfer(k, queue_query_ready).map_inner(k, |input| {
                (ReqProj { queue: input.queue, tag: input.tag }.into(), input.sel).into()
            });

            let inc_active = m_axis_desc_dequeue_req.clone().map(k, |input| input.valid);
            let (m_axis_desc_dequeue_req_vr, m_axis_desc_dequeue_req_remaining) =
                m_axis_desc_dequeue_req.into_vr(k).register_slice_fwd(k);

            // descriptor fetch
            // wait for queue query response
            let descriptor_fetch_ready = feedback.clone().map(k, |input| input.descriptor_fetch_ready);
            let s_axis_desc_dequeue_resp = input
                .s_axis_desc_dequeue_resp
                .array_map(k, "pmux", |input, k| {
                    input.filter_fwd_ready_neg(k).register_slice_bwd(k)
                })
                .priority_mux(k)
                .filter_bwd(k, descriptor_fetch_ready.clone())
                .into_uni(k, true);

            // return descriptor request completion
            // Change the valid expr to descriptor_fetch_ready.
            let m_axis_req_status = s_axis_desc_dequeue_resp.clone()
                .zip(k, descriptor_fetch_ready)
                .map(k, |input| {
                    let (value, cond) = *input;
                    Expr::<Valid<_>>::new(cond, value.inner)
                })
                .map_inner(k, |inner| {
                    let (_, inner) = *inner;
                    RReqStatusProj {
                        queue: inner.queue,
                        ptr: inner.ptr,
                        cpl: inner.cpl,
                        tag: inner.tag,
                        empty: inner.empty,
                        error: inner.error,
                    }
                    .into()
                })
                .buffer(k, Expr::invalid());

            // initiate descriptor fetch
            let m_axis_dma_read_desc = feedback.clone().map(k, |input| input.m_axis_dma_read_desc);
            let (m_axis_dma_read_desc_vr, m_axis_dma_read_desc_remaining) =
                m_axis_dma_read_desc.into_vr(k).register_slice_fwd(k);

            // commit dequeue operation
            let m_axis_desc_dequeue_commit = feedback.clone().map(k, |input| input.m_axis_desc_dequeue_commit);
            let (m_axis_desc_dequeue_commit_vr, m_axis_desc_dequeue_commit_remaining) =
                m_axis_desc_dequeue_commit.into_vr(k).register_slice_fwd(k);

            // initiate descriptor read from DMA RAM
            let dma_read_desc = feedback.map(k, |input| input.dma_read_desc);
            let (dma_read_desc_vr, dma_read_desc_remaining) = dma_read_desc.into_vr(k).register_slice_fwd(k);

            // Instantiates dma_client_axis_source.
            let dma_client_axis_source_output = DmaClientAxisSourceI {
                s_axis_read_desc: dma_read_desc_vr,
                ram_rd_resp: dma_psdpram_output.rd_resp,
                enable: UniChannel::source(k, true.into()),
            }.dma_client_axis_source::<{SEG_COUNT}, {SEG_DATA_WIDTH}, {SEG_ADDR_WIDTH}, {SEG_BE_WIDTH}, {RAM_ADDR_WIDTH}, {AXIS_DATA_WIDTH}, {(AXIS_KEEP_WIDTH > 1) as usize}, {AXIS_KEEP_WIDTH}, 1, 1, REQ_TAG_WIDTH, 0, 1, 1, 8, CL_DESC_TABLE_SIZE>(k, "dma_client_axis_source_inst", None, None);

            let feedback: UniChannel<Feedback> = (input.enable)
                .zip6(k, input.s_axis_dma_read_desc_status, s_axis_desc_dequeue_resp, dma_client_axis_source_output.m_axis_read_desc_status, dma_read_desc_remaining, m_axis_dma_read_desc_remaining)
                .zip4(k, m_axis_desc_dequeue_req_remaining, m_axis_desc_dequeue_commit_remaining, inc_active)
                .fsm_map(
                    k,
                    None,
                    DescTable::<DESC_TABLE_SIZE, CL_DESC_TABLE_SIZE>::new_expr(),
                    |input, state| {
                        // Projections.

                        let (input, m_axis_desc_dequeue_req_remaining, m_axis_desc_dequeue_commit_remaining, inc_active) = *input;
                        let (enable, s_axis_dma_read_desc_status, s_axis_desc_dequeue_resp, dma_read_desc_status, dma_read_desc_remaining, m_axis_dma_read_desc_remaining) = *input;
                        let mut state = *state;

                        let entries = state.entries;

                        // Calculates ready exprs.

                        let start_ptr = state.start_ptr;
                        let start_ptr_idx = start_ptr.resize();
                        let start_entry =
                            DescTableEntryVarArr::get_entry(entries, start_ptr_idx);

                        let desc_read_ptr = state.desc_read_ptr;
                        let desc_read_ptr_idx = desc_read_ptr.resize();
                        let desc_read_entry =
                            DescTableEntryVarArr::get_entry(entries, desc_read_ptr_idx);

                        let finish_ptr = state.finish_ptr;
                        let finish_ptr_idx = finish_ptr.resize();
                        let finish_entry =
                            DescTableEntryVarArr::get_entry(entries, finish_ptr_idx);

                        let start_entry_ready = !start_entry.active
                            & (start_ptr - finish_ptr).is_lt(DESC_TABLE_SIZE.into());

                        let queue_query_ready = enable
                            & state.active_count.is_lt(DESC_TABLE_SIZE.into())
                            & start_entry_ready
                            & !m_axis_desc_dequeue_req_remaining;

                        let descriptor_fetch_ready =
                            s_axis_desc_dequeue_resp.valid & start_entry_ready & !m_axis_dma_read_desc_remaining;
                        // TODO: we've relaxed the last condition (L498). Maybe timing violation will happen?

                        let return_descriptor_ready = !desc_read_ptr.is_eq(start_ptr)
                            & desc_read_entry.active
                            & desc_read_entry.desc_fetched
                            & !m_axis_desc_dequeue_commit_remaining
                            & !dma_read_desc_remaining;
                        // TODO: we've relaxed the last two conditions (L549). Maybe timing violation will happen?

                        let finish_ready =
                            !finish_ptr.is_eq(start_ptr) & finish_entry.active & finish_entry.read_done;

                        // Calculates output exprs.

                        // Initiates descriptor fetch.
                        let s_axis_desc_dequeue_resp = s_axis_desc_dequeue_resp.inner;
                        let s_axis_desc_dequeue_resp_sel = s_axis_desc_dequeue_resp.0;
                        let s_axis_desc_dequeue_resp = s_axis_desc_dequeue_resp.1;
                        let queue_empty = s_axis_desc_dequeue_resp.error | s_axis_desc_dequeue_resp.empty;
                        let dec_active = descriptor_fetch_ready & queue_empty;
                        let desc_table_start_en = descriptor_fetch_ready & !queue_empty;

                        let m_axis_dma_read_desc = Expr::<Valid<_>>::new(
                            desc_table_start_en,
                            DmaReadDescRespProj {
                                dma_addr: s_axis_desc_dequeue_resp.addr,
                                ram_addr: start_ptr.resize::<U<RAM_ADDR_WIDTH>>()
                                    << Expr::from((CL_DESC_SIZE + (1 << LOG_BLOCK_SIZE_WIDTH) - 1) as u8)
                                        .repr()
                                        .resize::<Log2<U<RAM_ADDR_WIDTH>>>(),
                                len: Expr::from(DESC_SIZE as u8).repr().resize::<U<DMA_LEN_WIDTH>>()
                                    << s_axis_desc_dequeue_resp.block_size.resize::<Log2<U<DMA_LEN_WIDTH>>>(),
                                tag: start_ptr_idx.resize(),
                            }
                            .into(),
                        );

                        // Commits dequeue operation.
                        let m_axis_desc_dequeue_commit: Expr<Valid<(DescDeqCommit, Bits<U<CL_PORTS>>)>> =
                            Expr::<Valid<_>>::new(
                                return_descriptor_ready,
                                (
                                    DescDeqCommitProj { op_tag: desc_read_entry.op_tag }.into(),
                                    desc_read_entry.sel
                                ).into(),
                            );

                        // Initiates descriptor read from DMA RAM.
                        let dma_read_desc = Expr::<Valid<_>>::new(
                            return_descriptor_ready,
                            DmaReadDescReqProj {
                                ram_addr: desc_read_ptr_idx.resize()
                                    << (Expr::from((CL_DESC_SIZE + (1 << LOG_BLOCK_SIZE_WIDTH) - 1) as u8)
                                        .repr()
                                        .clip_const::<U<5>>(0)),
                                len: Expr::from(DESC_SIZE as u8).repr()
                                    << desc_read_entry.log_desc_block_size.resize::<U<3>>(),
                                tag: desc_read_ptr_idx,
                                id: desc_read_entry.tag,
                                user: false.into(),
                            }
                            .into(),
                        );

                        let feedback = FeedbackProj {
                            queue_query_ready,
                            descriptor_fetch_ready,
                            dma_read_desc,
                            m_axis_dma_read_desc,
                            m_axis_desc_dequeue_commit,
                        };

                        // Calculates the new state.

                        state.active_count = (state.active_count + inc_active.repr().resize()).resize()
                            - dec_active.repr().resize()
                            - finish_ready.repr().resize();

                        // desc_table_start_en: store in descriptor table
                        let mut entries = *entries;

                        entries.active = if_then_set! { entries.active, desc_table_start_en, start_ptr_idx, Expr::from(true) };
                        entries.desc_fetched = if_then_set! { entries.desc_fetched, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                        entries.read_done = if_then_set! { entries.read_done, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                        entries.sel = if_then_set_var_arr! { entries.sel, desc_table_start_en, start_ptr_idx, s_axis_desc_dequeue_resp_sel.resize() };
                        entries.log_desc_block_size = if_then_set_var_arr! { entries.log_desc_block_size, desc_table_start_en, start_ptr_idx, s_axis_desc_dequeue_resp.block_size };
                        entries.tag = if_then_set_var_arr! { entries.tag, desc_table_start_en, start_ptr_idx, s_axis_desc_dequeue_resp.tag };
                        entries.op_tag = if_then_set_var_arr! { entries.op_tag, desc_table_start_en, start_ptr_idx, s_axis_desc_dequeue_resp.op_tag };

                        let new_start_ptr = (start_ptr + 1.into()).resize();
                        state.start_ptr = (!desc_table_start_en).cond(start_ptr, new_start_ptr);

                        // desc_table_desc_read_en: update entry in descriptor table
                        let new_desc_read_ptr = (desc_read_ptr + 1.into()).resize();
                        state.desc_read_ptr = (!return_descriptor_ready).cond(desc_read_ptr, new_desc_read_ptr);

                        // desc_table_desc_fetched_en: update entry in descriptor table
                        let desc_fetched_ptr = s_axis_dma_read_desc_status.inner.tag;
                        entries.desc_fetched = if_then_set! { entries.desc_fetched, s_axis_dma_read_desc_status.valid, desc_fetched_ptr.resize(), Expr::from(true) };

                        // desc_table_desc_read_done_en: update entry in descriptor table
                        let read_done_ptr = dma_read_desc_status.inner.tag;
                        entries.read_done = if_then_set! { entries.read_done, dma_read_desc_status.valid, read_done_ptr.resize(), Expr::from(true) };

                        // desc_table_finish_en: invalidate entry in descriptor table
                        entries.active = if_then_set! { entries.active, finish_ready, finish_ptr_idx.resize(), Expr::from(false) };

                        state.entries = entries.into();

                        state.finish_ptr = (!finish_ready).cond(finish_ptr, (finish_ptr + 1.into()).resize());

                        (feedback.into(), state.into())
                    },
                );

            (
                O {
                    m_axis_req_status,
                    m_axis_desc: dma_client_axis_source_output.m_axis_read_data,
                    m_axis_desc_dequeue_req: m_axis_desc_dequeue_req_vr.demux(k),
                    m_axis_desc_dequeue_commit: m_axis_desc_dequeue_commit_vr.demux(k),
                    m_axis_dma_read_desc: m_axis_dma_read_desc_vr,
                    dma_ram_wr_done: dma_psdpram_output.wr_done,
                },
                (dma_client_axis_source_output.ram_rd_cmd, feedback),
            )
        },
    )
    // Feeds the feedback expr to itself.
    .loop_feedback()
    .build()
}

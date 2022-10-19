//! Transmit engine.
//!
//! TODO: Enable `TX_BUFFER_OFFSET`, `PTP_TS_ENABLE`, `TX_CHECKSUM_ENABLE` parameters

use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::constants::tx_engine::*;
use super::types::request::*;
use super::types::timestamp::*;

#[derive(Debug, Clone, Signal)]
pub struct TDesc {
    addr: Bits<U<RAM_ADDR_WIDTH>>,
    len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    tag: Bits<U<DMA_CLIENT_TAG_WIDTH>>,
    user: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct TDescStatus {
    tag: Bits<U<DMA_CLIENT_TAG_WIDTH>>,
    error: Bits<U<4>>,
}

#[derive(Debug, Clone, Signal)]
pub struct TCsumCmd {
    #[member(name = "csum_enable")]
    enable: bool,
    #[member(name = "csum_start")]
    start: Bits<U<8>>,
    #[member(name = "csum_offset")]
    offset: Bits<U<8>>,
}

#[derive(Debug, Interface)]
pub struct I {
    /// Transmit request input (queue index)
    s_axis_tx_req: VrChannel<Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>,

    /// Descriptor request status input
    s_axis_desc_req_status:
        UniChannel<Valid<DescReqStatus<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH, CPL_QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH>>>,

    /// Descriptor data input
    s_axis_desc: AxisChannel<Desc<AXIS_DESC_DATA_WIDTH, AXIS_DESC_KEEP_WIDTH>>,

    /// Completion request status input
    s_axis_cpl_req_status: UniChannel<Valid<CplReqStatus<U<DESC_REQ_TAG_WIDTH>>>>,

    /// DMA read descriptor status input
    s_axis_dma_read_desc_status: UniChannel<Valid<DmaDescStatus>>,

    /// Transmit descriptor status input
    s_axis_tx_desc_status: UniChannel<Valid<TDescStatus>>,

    /// Transmit timestamp input
    s_axis_tx_ptp_ts: VrChannel<Timestamp>,

    /// Configuration
    enable: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct O {
    /// Transmit request status output
    m_axis_tx_req_status: UniChannel<Valid<ReqStatus<DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>>>,

    /// Descriptor request output
    m_axis_desc_req: VrChannel<Req<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH>>,

    /// Completion request output
    m_axis_cpl_req: VrChannel<CplReq<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH, CPL_DATA_SIZE>>,

    /// DMA read descriptor output
    m_axis_dma_read_desc: VrChannel<DmaDesc>,

    /// Transmit descriptor output
    m_axis_tx_desc: VrChannel<TDesc>,

    /// Transmit checksum command output
    m_axis_tx_csum_cmd: VrChannel<TCsumCmd>,
}

#[derive(Debug, Clone, Signal)]
pub struct Feedback<
    const QUEUE_INDEX_WIDTH: usize,
    const DESC_REQ_TAG_WIDTH: usize,
    const CPL_DATA_SIZE: usize,
    const DMA_CLIENT_LEN_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
> {
    descriptor_fetch_ready: bool,
    descriptor_processing_ready: bool,
    store_ptp_ready: bool,
    m_axis_desc_req: Valid<Req<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH>>,
    m_axis_dma_read_desc: Valid<DmaDesc>,
    m_axis_tx_desc: Valid<TDesc>,
    m_axis_tx_csum_cmd: Valid<TCsumCmd>,
    m_axis_cpl_req: Valid<CplReq<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH, CPL_DATA_SIZE>>,
    m_axis_tx_req_status: Valid<ReqStatus<DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
struct DescTableEntry {
    active: bool,
    invalid: bool,
    desc_fetched: bool,
    data_fetched: bool,
    tx_done: bool,
    cpl_write_done: bool,
    tag: Bits<U<REQ_TAG_WIDTH>>,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    queue_ptr: Bits<U<QUEUE_PTR_WIDTH>>,
    cpl_queue: Bits<U<CPL_QUEUE_INDEX_WIDTH>>,
    csum_start: Bits<U<7>>,
    csum_offset: Bits<U<8>>,
    csum_enable: bool,
    len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    buf_ptr: Bits<Sum<Log2<U<TX_BUFFER_SIZE>>, U<1>>>,
    ptp_ts: Bits<U<96>>,
    read_commit: bool,
    read_count_start: Bits<U<DESC_TABLE_DMA_OP_COUNT_WIDTH>>,
    read_count_finish: Bits<U<DESC_TABLE_DMA_OP_COUNT_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
struct DescTable<const DESC_TABLE_SIZE: usize, const DMA_CLIENT_LEN_WIDTH: usize, const REQ_TAG_WIDTH: usize> {
    entries: DescTableEntryVarArr<DESC_TABLE_SIZE>,
    buf_wr_ptr: Bits<Sum<Log2<U<TX_BUFFER_SIZE>>, U<1>>>,
    buf_rd_ptr: Bits<Sum<Log2<U<TX_BUFFER_SIZE>>, U<1>>>,
    desc_start: bool,
    desc_len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    early_tx_req_status: Valid<ReqStatus<DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>>,
    finish_tx_req_status: Valid<ReqStatus<DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>>,
    active_desc_req_count: Bits<Sum<Log2<U<MAX_DESC_REQ>>, U<1>>>,
    start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    tx_start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    store_ptp_ts_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    cpl_enqueue_start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    finish_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
}

impl<const DESC_TABLE_SIZE: usize, const DMA_CLIENT_LEN_WIDTH: usize, const REQ_TAG_WIDTH: usize>
    DescTable<DESC_TABLE_SIZE, DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>
{
    fn new_expr() -> Expr<'static, Self> {
        DescTableProj {
            entries: DescTableEntryVarArr::new_expr(),
            buf_wr_ptr: 0.into(),
            buf_rd_ptr: 0.into(),
            desc_start: true.into(),
            desc_len: 0.into(),
            early_tx_req_status: Expr::invalid(),
            finish_tx_req_status: Expr::invalid(),
            active_desc_req_count: 0.into(),
            start_ptr: 0.into(),
            tx_start_ptr: 0.into(),
            store_ptp_ts_ptr: 0.into(),
            cpl_enqueue_start_ptr: 0.into(),
            finish_ptr: 0.into(),
        }
        .into()
    }
}

pub fn m() -> Module<I, O> {
    composite::<(I, UniChannel<Feedback< QUEUE_INDEX_WIDTH,  DESC_REQ_TAG_WIDTH,  CPL_DATA_SIZE,  DMA_CLIENT_LEN_WIDTH,  REQ_TAG_WIDTH>>), _, _>("tx_engine", None, None, |(input, feedback), k| {
        // descriptor fetch
        // wait for transmit request
        let descriptor_fetch_ready = feedback.clone().map(k, |input| input.descriptor_fetch_ready);
        let s_axis_tx_req = input.s_axis_tx_req.transfer(k, descriptor_fetch_ready);

        // initiate descriptor fetch
        let m_axis_desc_req = feedback.clone().map(k, |input| input.m_axis_desc_req);
        let (m_axis_desc_req_vr, m_axis_desc_req_remaining) = m_axis_desc_req.into_vr(k).register_slice_fwd(k);

        // descriptor processing and DMA request generation
        let descriptor_processing_ready = feedback.clone().map(k, |input| input.descriptor_processing_ready);
        let s_axis_desc = input.s_axis_desc.into_vr(k).transfer(k, descriptor_processing_ready);

        // initiate data fetch to onboard RAM
        let m_axis_dma_read_desc = feedback.clone().map(k, |input| input.m_axis_dma_read_desc);
        let (m_axis_dma_read_desc_vr, m_axis_dma_read_desc_remaining) =
            m_axis_dma_read_desc.into_vr(k).register_slice_fwd(k);

        // initiate transmit operation
        let m_axis_tx_desc = feedback.clone().map(k, |input| input.m_axis_tx_desc);
        let (m_axis_tx_desc_vr, m_axis_tx_desc_remaining) = m_axis_tx_desc.into_vr(k).register_slice_fwd(k);

        // send TX checksum command
        let m_axis_tx_csum_cmd = feedback.clone().map(k, |input| input.m_axis_tx_csum_cmd);
        let (m_axis_tx_csum_cmd_vr, m_axis_tx_csum_cmd_remaining) = m_axis_tx_csum_cmd.into_vr(k).register_slice_fwd(k);

        // store PTP timestamp
        let store_ptp_ready = feedback.clone().map(k, |input| input.store_ptp_ready);
        let s_axis_tx_ptp_ts = input.s_axis_tx_ptp_ts.transfer(k, store_ptp_ready);

        // initiate queue query
        let m_axis_cpl_req = feedback.clone().map(k, |input| input.m_axis_cpl_req);
        let (m_axis_cpl_req_vr, m_axis_cpl_req_remaining) = m_axis_cpl_req.into_vr(k).register_slice_fwd(k);

        // transmit request completion arbitration
        let m_axis_tx_req_status =feedback.map(k, |input| input.m_axis_tx_req_status).buffer(k, Expr::invalid());

        let feedback: UniChannel<Feedback< QUEUE_INDEX_WIDTH,  DESC_REQ_TAG_WIDTH,  CPL_DATA_SIZE,  DMA_CLIENT_LEN_WIDTH,  REQ_TAG_WIDTH>> = (input.enable)
            .zip6(k, s_axis_tx_req, input.s_axis_desc_req_status, s_axis_desc, input.s_axis_dma_read_desc_status, input.s_axis_tx_desc_status)
            .zip6(k, s_axis_tx_ptp_ts, input.s_axis_cpl_req_status, m_axis_tx_req_status.clone(), m_axis_desc_req_remaining, m_axis_dma_read_desc_remaining)
            .zip4(k, m_axis_tx_desc_remaining, m_axis_tx_csum_cmd_remaining, m_axis_cpl_req_remaining)
            .fsm_map(k, None, DescTable::<DESC_TABLE_SIZE, DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>::new_expr(), |input, state| {
                // Projections.

                let (input, m_axis_tx_desc_remaining, m_axis_tx_csum_cmd_remaining, m_axis_cpl_req_remaining) = *input;
                let (input, s_axis_tx_ptp_ts, s_axis_cpl_req_status, m_axis_tx_req_status, m_axis_desc_req_remaining, m_axis_dma_read_desc_remaining) = *input;
                let (enable, s_axis_tx_req, s_axis_desc_req_status, s_axis_desc, s_axis_dma_read_desc_status, s_axis_tx_desc_status) = *input;
                let mut state = *state;

                let entries = state.entries;

                let start_ptr = state.start_ptr;
                let start_ptr_idx = start_ptr.resize();
                let start_entry = DescTableEntryVarArr::get_entry(entries, start_ptr_idx);

                let tx_start_ptr = state.tx_start_ptr;
                let tx_start_ptr_idx = tx_start_ptr.resize();
                let tx_start_entry = DescTableEntryVarArr::get_entry(entries, tx_start_ptr_idx);

                let store_ptp_ts_ptr = state.store_ptp_ts_ptr;
                let store_ptp_ts_ptr_idx = store_ptp_ts_ptr.resize();
                let store_ptp_ts_entry = DescTableEntryVarArr::get_entry(entries, store_ptp_ts_ptr_idx);

                let cpl_enqueue_start_ptr = state.cpl_enqueue_start_ptr;
                let cpl_enqueue_start_ptr_idx = cpl_enqueue_start_ptr.resize();
                let cpl_enqueue_start_entry = DescTableEntryVarArr::get_entry(entries, cpl_enqueue_start_ptr_idx);

                let finish_ptr = state.finish_ptr;
                let finish_ptr_idx = finish_ptr.resize();
                let finish_entry = DescTableEntryVarArr::get_entry(entries, finish_ptr_idx);

                let buf_wr_ptr = state.buf_wr_ptr;
                let buf_rd_ptr = state.buf_rd_ptr;

                let desc_start = state.desc_start;
                let desc_len = state.desc_len;

                let active_desc_req_count = state.active_desc_req_count;

                // descriptor fetch
                // wait for transmit request
                let s_axis_tx_req_valid = s_axis_tx_req.valid;
                let s_axis_tx_req = s_axis_tx_req.inner;

                let descriptor_fetch_ready = enable
                    & active_desc_req_count.is_lt(MAX_DESC_REQ.into())
                    & !start_entry.active
                    & (start_ptr - finish_ptr).is_lt(DESC_TABLE_SIZE.into())
                    & !m_axis_desc_req_remaining;

                let desc_table_start_en = s_axis_tx_req_valid.cond(Expr::from(true), Expr::from(false));
                let m_axis_desc_req = Expr::<Valid<_>>::new(
                    desc_table_start_en,
                    ReqProj { queue: s_axis_tx_req.queue, tag: start_ptr_idx.resize() }.into(),
                );

                // descriptor fetch
                // wait for queue query response
                let s_axis_desc_req_status_valid = s_axis_desc_req_status.valid;
                let s_axis_desc_req_status = s_axis_desc_req_status.inner;

                let queue_empty = s_axis_desc_req_status.error | s_axis_desc_req_status.empty;

                let desc_table_dequeue_en = s_axis_desc_req_status_valid.cond(Expr::from(true), Expr::from(false));
                let early_tx_req_status_next = (desc_table_dequeue_en & queue_empty).cond(
                    Expr::<Valid<_>>::new(
                        true.into(),
                        ReqStatusProj {
                            len: 0.into(),
                            tag: DescTableEntryVarArr::get_entry(entries, s_axis_desc_req_status.tag.resize()).tag,
                        }
                        .into(),
                    ),
                    state.early_tx_req_status,
                );

                let dec_active_desc_req_1 = (desc_table_dequeue_en & queue_empty).cond(Expr::from(true), Expr::from(false));

                // descriptor processing and DMA request generation
                // TODO descriptor validation?
                let s_axis_desc_valid = s_axis_desc.valid;
                let s_axis_desc = s_axis_desc.inner;
                let s_axis_desc_tid = s_axis_desc.payload.tid.resize();
                let s_axis_desc_tdata = s_axis_desc.payload.data.tdata;
                let s_axis_desc_tlast = s_axis_desc.tlast;
                let desc_entry = DescTableEntryVarArr::get_entry(entries, s_axis_desc_tid.resize());

                let descriptor_processing_ready = !m_axis_dma_read_desc_remaining
                    & (buf_wr_ptr - buf_rd_ptr).is_lt((TX_BUFFER_SIZE - MAX_TX_SIZE).into());

                let desc_start_next = (s_axis_desc_valid & desc_entry.active).cond(false.into(), desc_start);

                let desc_len_next = (desc_len.resize() + s_axis_desc_tdata.clip_const::<U<32>>(32)).clip_const::<U<16>>(0);
                let desc_len_next = desc_len_next.is_gt(MAX_TX_SIZE.into()).cond(MAX_TX_SIZE.into(), desc_len_next);
                let desc_len_next = (s_axis_desc_valid & desc_entry.active).cond(desc_len_next, desc_len);

                let desc_table_read_start_init = s_axis_desc_valid & desc_entry.active & desc_start;
                let desc_table_read_start_init = desc_table_read_start_init.cond(Expr::from(true), Expr::from(false));
                let desc_table_desc_ctrl_en = desc_table_read_start_init.cond(Expr::from(true), Expr::from(false));
                let desc_table_read_start_en = s_axis_desc_valid & desc_entry.active & !(s_axis_desc_tdata.clip_const::<U<32>>(32).is_eq(0.into()));
                let desc_table_read_start_en = desc_table_read_start_en.cond(Expr::from(true), Expr::from(false));
                let desc_table_desc_fetched_en = s_axis_desc_valid & desc_entry.active & s_axis_desc_tlast;
                let desc_table_desc_fetched_en = desc_table_desc_fetched_en.cond(Expr::from(true), Expr::from(false));
                let desc_table_read_start_commit = desc_table_desc_fetched_en.cond(Expr::from(true), Expr::from(false));

                let m_axis_dma_read_desc = Expr::<Valid<_>>::new(
                    desc_table_read_start_en,
                    DmaDescProj {
                        dma_addr: s_axis_desc_tdata.clip_const::<U<64>>(64),
                        ram_addr: (buf_wr_ptr.clip_const::<U<CL_TX_BUFFER_SIZE>>(0) + desc_len.resize()).resize(),
                        len: s_axis_desc_tdata.clip_const::<U<16>>(32),
                        tag: s_axis_desc_tid.resize(),
                    }
                    .into(),
                );

                let desc_table_desc_fetched_len = desc_table_desc_fetched_en.cond(
                    desc_len_next,
                    desc_len,
                );

                let buf_wr_ptr_cond = (buf_wr_ptr.clip_const::<U<CL_TX_BUFFER_SIZE>>(0) + desc_len_next.resize())
                    .is_gt((TX_BUFFER_SIZE - MAX_TX_SIZE).into());
                let buf_wr_ptr_next = select! {
                    desc_table_read_start_commit & buf_wr_ptr_cond => {
                        !buf_wr_ptr & !(Expr::from(TX_BUFFER_PTR_MASK as u16).repr().resize())
                    },
                    desc_table_read_start_commit => {
                        ((buf_wr_ptr + desc_len_next.resize())
                            + Expr::from(TX_BUFFER_PTR_MASK_LOWER as u16).repr().resize()).resize()
                            & !(Expr::from(TX_BUFFER_PTR_MASK_LOWER as u16).repr().resize())
                    },
                    default => buf_wr_ptr,
                };

                let desc_start_next = desc_table_desc_fetched_en.cond(true.into(), desc_start_next);
                let desc_len_next = desc_table_desc_fetched_en.cond(0.into(), desc_len_next);

                // data fetch completion
                // wait for data fetch completion
                let s_axis_dma_read_desc_status_valid = s_axis_dma_read_desc_status.valid;
                let s_axis_dma_read_desc_status = s_axis_dma_read_desc_status.inner;

                let desc_table_data_fetched_en = s_axis_dma_read_desc_status_valid.cond(Expr::from(true), Expr::from(false));
                let desc_table_read_finish_en = s_axis_dma_read_desc_status_valid.cond(Expr::from(true), Expr::from(false));

                // transmit
                // wait for data fetch completion
                let active = tx_start_entry.active & !(tx_start_ptr.is_eq(start_ptr));
                let invalid = tx_start_entry.invalid;
                let update = tx_start_entry.desc_fetched
                    & tx_start_entry.read_commit
                    & tx_start_entry.read_count_start.is_eq(tx_start_entry.read_count_finish)
                    & !m_axis_tx_desc_remaining
                    & !m_axis_tx_csum_cmd_remaining;

                let desc_table_tx_start_en = active & (invalid | update);
                let desc_table_tx_start_en = desc_table_tx_start_en.cond(Expr::from(true), Expr::from(false));
                let transmit = active & !invalid & update;

                let m_axis_tx_desc = Expr::<Valid<_>>::new(
                    transmit,
                    TDescProj {
                        addr: tx_start_entry.buf_ptr.clip_const::<U<CL_TX_BUFFER_SIZE>>(0).resize(),
                        len: tx_start_entry.len,
                        tag: tx_start_ptr_idx.resize(),
                        user: false.into(),
                    }
                    .into(),
                );
                let m_axis_tx_csum_cmd = Expr::<Valid<_>>::new(
                    transmit,
                    TCsumCmdProj {
                        enable: tx_start_entry.csum_enable,
                        start: tx_start_entry.csum_start.resize(),
                        offset: (tx_start_entry.csum_start.resize() + tx_start_entry.csum_offset).clip_const(0),
                    }
                    .into(),
                );

                // transmit done
                // wait for transmit completion
                let s_axis_tx_desc_status_valid = s_axis_tx_desc_status.valid;
                let s_axis_tx_desc_status = s_axis_tx_desc_status.inner;

                let tx_finish_ptr = s_axis_tx_desc_status.tag;
                let tx_finish_ptr_idx = tx_finish_ptr.resize();
                let tx_finish_entry = DescTableEntryVarArr::get_entry(entries, tx_finish_ptr_idx);

                let desc_table_tx_finish_en = s_axis_tx_desc_status_valid.cond(Expr::from(true), Expr::from(false));

                let buf_rd_ptr_cond = (tx_finish_entry.buf_ptr.clip_const::<U<CL_TX_BUFFER_SIZE>>(0) + tx_finish_entry.len.resize())
                    .is_gt((TX_BUFFER_SIZE - MAX_TX_SIZE).into());
                let buf_rd_ptr_next = select! {
                    desc_table_tx_finish_en & buf_rd_ptr_cond => {
                        !tx_finish_entry.buf_ptr & !(Expr::from(TX_BUFFER_PTR_MASK as u16).repr().resize())
                    },
                    // !buf_ptr.clone() & !Expr::from(TX_BUFFER_PTR_MASK as u16).repr().resize(),
                    desc_table_tx_finish_en => {
                        ((tx_finish_entry.buf_ptr + tx_finish_entry.len.resize())
                            + Expr::from(TX_BUFFER_PTR_MASK_LOWER as u16).repr().resize()).resize()
                            & !(Expr::from(TX_BUFFER_PTR_MASK_LOWER as u16).repr().resize())
                    },
                    default => buf_rd_ptr,
                };

                // store PTP timestamp
                let s_axis_tx_ptp_ts_valid = s_axis_tx_ptp_ts.valid;
                let s_axis_tx_ptp_ts = s_axis_tx_ptp_ts.inner;

                let store_ptp_ready = store_ptp_ts_entry.active
                    & !store_ptp_ts_ptr.is_eq(start_ptr)
                    & !store_ptp_ts_ptr.is_eq(tx_start_ptr)
                    & !store_ptp_ts_entry.invalid;

                let desc_table_store_ptp_ts_en = store_ptp_ts_entry.active
                    & !store_ptp_ts_ptr.is_eq(start_ptr)
                    & !store_ptp_ts_ptr.is_eq(tx_start_ptr)
                    & (store_ptp_ts_entry.invalid | s_axis_tx_ptp_ts_valid);
                let desc_table_store_ptp_ts_en = desc_table_store_ptp_ts_en.cond(Expr::from(true), Expr::from(false));

                // finish transmit; start completion queue
                let transmit_ready = cpl_enqueue_start_entry.active
                    & !cpl_enqueue_start_ptr.is_eq(start_ptr)
                    & !cpl_enqueue_start_ptr.is_eq(tx_start_ptr)
                    & !cpl_enqueue_start_ptr.is_eq(store_ptp_ts_ptr);

                let desc_table_cpl_enqueue_start_en = transmit_ready
                    & (cpl_enqueue_start_entry.invalid | (cpl_enqueue_start_entry.tx_done & !m_axis_cpl_req_remaining));
                let desc_table_cpl_enqueue_start_en = desc_table_cpl_enqueue_start_en.cond(Expr::from(true), Expr::from(false));

                // initiate queue query
                let cpl_req_data = cpl_enqueue_start_entry.queue
                    .clip_const::<U<13>>(0)
                    .resize::<U<16>>()
                    .append(cpl_enqueue_start_entry.queue_ptr.clip_const::<U<16>>(0))
                    .append(cpl_enqueue_start_entry.len.clip_const::<U<16>>(0))
                    .resize::<U<64>>()
                    .append(cpl_enqueue_start_entry.ptp_ts.clip_const::<U<48>>(16))
                    .resize();
                let m_axis_cpl_req = Expr::<Valid<_>>::new(
                    transmit_ready
                        & !cpl_enqueue_start_entry.invalid
                        & (cpl_enqueue_start_entry.tx_done & !m_axis_cpl_req_remaining),
                    CplReqProj {
                        queue: cpl_enqueue_start_entry.cpl_queue,
                        tag: cpl_enqueue_start_ptr_idx.resize(),
                        data: cpl_req_data,
                    }
                    .into(),
                );

                // start completion write
                // wait for queue query response
                let s_axis_cpl_req_status_valid = s_axis_cpl_req_status.valid;
                let s_axis_cpl_req_status = s_axis_cpl_req_status.inner;

                let desc_table_cpl_write_done_en = s_axis_cpl_req_status_valid.cond(Expr::from(true), Expr::from(false));

                // operation complete
                let complete_ready = finish_entry.active
                    & !finish_ptr.is_eq(start_ptr)
                    & !finish_ptr.is_eq(cpl_enqueue_start_ptr)
                    & !state.finish_tx_req_status.valid;

                let desc_table_finish_en = complete_ready
                    & (finish_entry.invalid | finish_entry.cpl_write_done);
                let desc_table_finish_en = desc_table_finish_en.cond(Expr::from(true), Expr::from(false));

                // return transmit request completion
                let finish_tx_req_status_next = (complete_ready
                        & !finish_entry.invalid
                        & finish_entry.cpl_write_done).cond(
                    Expr::<Valid<_>>::new(
                        true.into(),
                        ReqStatusProj {
                            len: finish_entry.len,
                            tag: finish_entry.tag,
                        }
                        .into(),
                    ),
                    state.finish_tx_req_status,
                );

                // transmit request completion arbitration
                let finish_completion = finish_tx_req_status_next.valid & !m_axis_tx_req_status.valid;
                let early_completion = early_tx_req_status_next.valid & !m_axis_tx_req_status.valid;

                let m_axis_tx_req_status = select! {
                    finish_completion => Expr::<Valid<_>>::new(
                        true.into(),
                        finish_tx_req_status_next.inner,
                    ),
                    early_completion => Expr::<Valid<_>>::new(
                        true.into(),
                        early_tx_req_status_next.inner,
                    ),
                    default => Expr::invalid(),
                };

                let finish_tx_req_status_next = finish_completion.cond(
                    Expr::invalid(),
                    finish_tx_req_status_next,
                );

                let early_tx_req_status_next = (!finish_completion & early_completion).cond(
                    Expr::invalid(),
                    early_tx_req_status_next,
                );

                let feedback = FeedbackProj {
                    descriptor_fetch_ready,
                    descriptor_processing_ready,
                    store_ptp_ready,
                    m_axis_desc_req,
                    m_axis_dma_read_desc,
                    m_axis_tx_desc,
                    m_axis_tx_csum_cmd,
                    m_axis_cpl_req,
                    m_axis_tx_req_status,
                };

                // Calculates the new state.

                let mut entries = *entries;

                // desc_table_start_en
                // descriptor table operations
                entries.active = if_then_set! { entries.active, desc_table_start_en, start_ptr_idx, Expr::from(true) };
                entries.invalid = if_then_set! { entries.invalid, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.desc_fetched = if_then_set! { entries.desc_fetched, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.data_fetched = if_then_set! { entries.data_fetched, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.tx_done = if_then_set! { entries.tx_done, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.cpl_write_done = if_then_set! { entries.cpl_write_done, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.queue = if_then_set_var_arr! { entries.queue, desc_table_start_en, start_ptr_idx, s_axis_tx_req.queue };
                entries.tag = if_then_set_var_arr! { entries.tag, desc_table_start_en, start_ptr_idx, s_axis_tx_req.tag };

                let new_start_ptr = (start_ptr + 1.into()).resize();
                state.start_ptr = (!desc_table_start_en).cond(start_ptr, new_start_ptr);

                // desc_table_dequeue_en
                let dequeue_ptr = s_axis_desc_req_status.tag.resize();
                entries.queue_ptr = if_then_set_var_arr! { entries.queue_ptr, desc_table_dequeue_en, dequeue_ptr, s_axis_desc_req_status.ptr };
                entries.cpl_queue = if_then_set_var_arr! { entries.cpl_queue, desc_table_dequeue_en, dequeue_ptr, s_axis_desc_req_status.cpl };
                entries.invalid = entries.invalid.set(
                    dequeue_ptr,
                    (!desc_table_dequeue_en).cond(
                        entries.invalid[dequeue_ptr],
                        (s_axis_desc_req_status.error | s_axis_desc_req_status.empty).cond(
                            true.into(),
                            entries.invalid[dequeue_ptr],
                        ),
                    ),
                );

                // desc_table_desc_ctrl_en
                entries.buf_ptr = if_then_set_var_arr! { entries.buf_ptr, desc_table_desc_ctrl_en, s_axis_desc_tid, state.buf_wr_ptr };
                entries.csum_start = if_then_set_var_arr! { entries.csum_start, desc_table_desc_ctrl_en, s_axis_desc_tid, s_axis_desc_tdata.clip_const::<U<7>>(16) };
                entries.csum_offset = if_then_set_var_arr! { entries.csum_offset, desc_table_desc_ctrl_en, s_axis_desc_tid, s_axis_desc_tdata.clip_const::<U<7>>(24).resize() };
                entries.csum_enable = if_then_set! { entries.csum_enable, desc_table_desc_ctrl_en, s_axis_desc_tid, s_axis_desc_tdata[31] };

                // desc_table_desc_fetched_en
                entries.len = if_then_set_var_arr! { entries.len, desc_table_desc_fetched_en, s_axis_desc_tid, desc_table_desc_fetched_len };
                entries.desc_fetched = if_then_set! { entries.desc_fetched, desc_table_desc_fetched_en, s_axis_desc_tid, Expr::from(true) };

                // desc_table_data_fetched_en
                let data_fetched_ptr = s_axis_dma_read_desc_status.tag.resize();
                entries.data_fetched = if_then_set! { entries.data_fetched, desc_table_data_fetched_en, data_fetched_ptr, Expr::from(true) };

                // desc_table_tx_start_en
                entries.data_fetched = if_then_set! { entries.data_fetched, desc_table_tx_start_en, tx_start_ptr_idx, Expr::from(false) };

                let new_tx_start_ptr = (tx_start_ptr + 1.into()).resize();
                state.tx_start_ptr =
                    (!desc_table_tx_start_en).cond(tx_start_ptr, new_tx_start_ptr);

                // desc_table_tx_finish_en
                let tx_finish_ptr = s_axis_tx_desc_status.tag.resize();
                entries.tx_done = if_then_set! { entries.tx_done, desc_table_tx_finish_en, tx_finish_ptr, Expr::from(true) };

                // desc_table_store_ptp_ts_en
                entries.ptp_ts = if_then_set_var_arr! { entries.ptp_ts, desc_table_store_ptp_ts_en, store_ptp_ts_ptr_idx, s_axis_tx_ptp_ts.ts };

                let new_store_ptp_ts_ptr = (store_ptp_ts_ptr + 1.into()).resize();
                state.store_ptp_ts_ptr =
                    (!desc_table_store_ptp_ts_en).cond(store_ptp_ts_ptr, new_store_ptp_ts_ptr);

                // desc_table_cpl_enqueue_start_en
                let new_cpl_enqueue_start_ptr = (cpl_enqueue_start_ptr + 1.into()).resize();
                state.cpl_enqueue_start_ptr = (!desc_table_cpl_enqueue_start_en)
                    .cond(cpl_enqueue_start_ptr, new_cpl_enqueue_start_ptr);

                // desc_table_cpl_write_done_en
                let cpl_write_done_ptr = s_axis_cpl_req_status.tag.resize();
                entries.cpl_write_done = if_then_set! { entries.cpl_write_done, desc_table_cpl_write_done_en, cpl_write_done_ptr, Expr::from(true) };

                // desc_table_finish_en
                entries.active = if_then_set! { entries.active, desc_table_finish_en, finish_ptr_idx, Expr::from(false) };

                let new_finish_ptr = (finish_ptr + 1.into()).resize();
                state.finish_ptr =
                    (!desc_table_finish_en).cond(finish_ptr, new_finish_ptr);

                // desc_table_read_start_en
                entries.read_commit = entries.read_commit.set(
                    s_axis_desc_tid,
                    (!desc_table_read_start_en & !(desc_table_read_start_commit | desc_table_read_start_init)).cond(
                        entries.read_commit[s_axis_desc_tid],
                        desc_table_read_start_commit,
                    ),
                );
                entries.read_count_start = entries.read_count_start.set_var_arr(
                    s_axis_desc_tid,
                    (!desc_table_read_start_en & !(desc_table_read_start_commit | desc_table_read_start_init)).cond(
                        entries.read_count_start[s_axis_desc_tid],
                        select! {
                            desc_table_read_start_en & desc_table_read_start_init =>
                                (entries.read_count_finish[s_axis_desc_tid] + 1.into()).clip_const::<U<4>>(0),
                            desc_table_read_start_en =>
                                (entries.read_count_start[s_axis_desc_tid] + 1.into()).clip_const::<U<4>>(0),
                            desc_table_read_start_init => entries.read_count_finish[s_axis_desc_tid],
                            default => entries.read_count_start[s_axis_desc_tid],
                        },
                    ),
                );

                // desc_table_read_finish_en
                let read_finish_ptr = s_axis_dma_read_desc_status.tag.resize();
                entries.read_count_finish = if_then_set_var_arr! { entries.read_count_finish, desc_table_read_finish_en, read_finish_ptr, (entries.read_count_finish[read_finish_ptr] + 1.into()).clip_const::<U<4>>(0) };

                state.entries = entries.into();

                state.buf_wr_ptr = buf_wr_ptr_next;
                state.buf_rd_ptr = buf_rd_ptr_next;

                state.desc_start = desc_start_next;
                state.desc_len = desc_len_next;

                state.early_tx_req_status = early_tx_req_status_next;

                state.finish_tx_req_status = finish_tx_req_status_next;

                state.active_desc_req_count = (state.active_desc_req_count + desc_table_start_en.repr().resize()).resize()
                    - dec_active_desc_req_1.repr().resize()
                    - desc_table_read_start_commit.repr().resize();

                (feedback.into(), state.into())
            });

        (
            O {
                m_axis_tx_req_status,
                m_axis_desc_req: m_axis_desc_req_vr,
                m_axis_cpl_req: m_axis_cpl_req_vr,
                m_axis_dma_read_desc: m_axis_dma_read_desc_vr,
                m_axis_tx_desc: m_axis_tx_desc_vr,
                m_axis_tx_csum_cmd: m_axis_tx_csum_cmd_vr,
            },
            feedback,
        )
    })
    .loop_feedback()
    .build()
}

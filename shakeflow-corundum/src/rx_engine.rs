//! Receive engine.
//!
//! TODO: Enable `TX_BUFFER_OFFSET`, `PTP_TS_ENABLE`, `TX_CHECKSUM_ENABLE` parameters

use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::constants::rx_engine::*;
use super::types::request::*;
use super::types::timestamp::*;

#[derive(Debug, Clone, Signal)]
pub struct RDesc {
    addr: Bits<U<RAM_ADDR_WIDTH>>,
    len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    tag: Bits<U<DMA_CLIENT_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct RDescStatus {
    len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    tag: Bits<U<DMA_CLIENT_TAG_WIDTH>>,
    user: bool,
    error: Bits<U<4>>,
}

#[derive(Debug, Clone, Signal)]
pub struct RHash {
    #[member(name = "")]
    data: Bits<U<32>>,
    #[member(name = "type")]
    typ: Bits<U<4>>,
}

#[derive(Debug, Interface)]
pub struct I {
    /// Receive request input (queue index)
    s_axis_rx_req: VrChannel<Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>,

    /// Descriptor request status input
    s_axis_desc_req_status:
        UniChannel<Valid<DescReqStatus<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH, CPL_QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH>>>,

    /// Descriptor data input
    s_axis_desc: AxisChannel<Desc<AXIS_DESC_DATA_WIDTH, AXIS_DESC_KEEP_WIDTH>>,

    /// Completion request status input
    s_axis_cpl_req_status: UniChannel<Valid<CplReqStatus<U<DESC_REQ_TAG_WIDTH>>>>,

    /// DMA write descriptor status input
    s_axis_dma_write_desc_status: UniChannel<Valid<DmaDescStatus>>,

    /// Receive descriptor status input
    s_axis_rx_desc_status: UniChannel<Valid<RDescStatus>>,

    /// Receive timestamp input
    s_axis_rx_ptp_ts: VrChannel<Timestamp>,

    /// Receive hash input
    s_axis_rx_hash: VrChannel<RHash>,

    /// Receive checksum input
    s_axis_rx_csum: VrChannel<Bits<U<16>>>,

    /// Configuration
    mtu: UniChannel<Bits<U<DMA_CLIENT_LEN_WIDTH>>>,

    /// Configuration
    enable: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct O {
    /// Receive request status output
    m_axis_rx_req_status: UniChannel<Valid<ReqStatus<DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>>>,

    /// Descriptor request output
    m_axis_desc_req: VrChannel<Req<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH>>,

    /// Completion request output
    m_axis_cpl_req: VrChannel<CplReq<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH, CPL_DATA_SIZE>>,

    /// DMA write descriptor output
    m_axis_dma_write_desc: VrChannel<DmaDesc>,

    /// Receive descriptor output
    m_axis_rx_desc: VrChannel<RDesc>,
}

#[derive(Debug, Clone, Signal)]
pub struct Feedback<
    const QUEUE_INDEX_WIDTH: usize,
    const DESC_REQ_TAG_WIDTH: usize,
    const CPL_DATA_SIZE: usize,
    const DMA_CLIENT_LEN_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
> {
    receive_packet_ready: bool,
    descriptor_processing_ready: bool,
    store_ptp_ready: bool,
    store_rx_hash_ready: bool,
    store_rx_csum_ready: bool,
    m_axis_rx_desc: Valid<RDesc>,
    m_axis_desc_req: Valid<Req<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH>>,
    m_axis_dma_write_desc: Valid<DmaDesc>,
    m_axis_cpl_req: Valid<CplReq<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH, CPL_DATA_SIZE>>,
    m_axis_rx_req_status: Valid<ReqStatus<DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
struct DescTableEntry {
    active: bool,
    rx_done: bool,
    invalid: bool,
    desc_fetched: bool,
    data_written: bool,
    cpl_write_done: bool,
    tag: Bits<U<REQ_TAG_WIDTH>>,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    queue_ptr: Bits<U<QUEUE_PTR_WIDTH>>,
    cpl_queue: Bits<U<CPL_QUEUE_INDEX_WIDTH>>,
    dma_len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    desc_len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    buf_ptr: Bits<Sum<U<CL_RX_BUFFER_SIZE>, U<1>>>,
    ptp_ts: Bits<U<96>>,
    hash: Bits<U<32>>,
    hash_type: Bits<U<4>>,
    csum: Bits<U<16>>,
    read_commit: bool,
    write_count_start: Bits<U<DESC_TABLE_DMA_OP_COUNT_WIDTH>>,
    write_count_finish: Bits<U<DESC_TABLE_DMA_OP_COUNT_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
struct DescTable<const DESC_TABLE_SIZE: usize> {
    entries: DescTableEntryVarArr<DESC_TABLE_SIZE>,
    mtu: Bits<Sum<Log2<U<MAX_RX_SIZE>>, U<1>>>,
    buf_wr_ptr: Bits<Sum<U<CL_RX_BUFFER_SIZE>, U<1>>>,
    buf_rd_ptr: Bits<Sum<U<CL_RX_BUFFER_SIZE>, U<1>>>,
    desc_start: bool,
    desc_done: bool,
    desc_len: Bits<U<DMA_CLIENT_LEN_WIDTH>>,
    active_desc_req_count: Bits<Sum<Log2<U<MAX_DESC_REQ>>, U<1>>>,
    start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    dequeue_start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    store_ptp_ts_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    store_hash_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    store_csum_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    cpl_enqueue_start_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
    finish_ptr: Bits<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>,
}

impl<const DESC_TABLE_SIZE: usize> DescTable<DESC_TABLE_SIZE> {
    fn new_expr() -> Expr<'static, Self> {
        DescTableProj {
            entries: DescTableEntryVarArr::new_expr(),
            mtu: 0.into(),
            buf_wr_ptr: 0.into(),
            buf_rd_ptr: 0.into(),
            desc_start: true.into(),
            desc_done: false.into(),
            desc_len: 0.into(),
            active_desc_req_count: 0.into(),
            start_ptr: 0.into(),
            dequeue_start_ptr: 0.into(),
            store_ptp_ts_ptr: 0.into(),
            store_hash_ptr: 0.into(),
            store_csum_ptr: 0.into(),
            cpl_enqueue_start_ptr: 0.into(),
            finish_ptr: 0.into(),
        }
        .into()
    }
}

pub fn m() -> Module<I, O> {
    composite::<(I, UniChannel<Feedback<QUEUE_INDEX_WIDTH, DESC_REQ_TAG_WIDTH, CPL_DATA_SIZE, DMA_CLIENT_LEN_WIDTH, REQ_TAG_WIDTH>>), _, _>("rx_engine", None, None, |(input, feedback), k| {
        // receive packet
        // wait for receive request
        let receive_packet_ready = feedback.clone().map(k, |input| input.receive_packet_ready);
        let s_axis_rx_req = input.s_axis_rx_req.transfer(k, receive_packet_ready);

        // initiate receive operation
        let m_axis_rx_desc = feedback.clone().map(k, |input| input.m_axis_rx_desc);
        let (m_axis_rx_desc_vr, m_axis_rx_desc_remaining) = m_axis_rx_desc.into_vr(k).register_slice_fwd(k);

        // initiate descriptor fetch
        let m_axis_desc_req = feedback.clone().map(k, |input| input.m_axis_desc_req);
        let (m_axis_desc_req_vr, m_axis_desc_req_remaining) = m_axis_desc_req.into_vr(k).register_slice_fwd(k);

        // descriptor processing and DMA request generation
        let descriptor_processing_ready = feedback.clone().map(k, |input| input.descriptor_processing_ready);
        let s_axis_desc = input.s_axis_desc.into_vr(k).transfer(k, descriptor_processing_ready);

        // initiate data write
        let m_axis_dma_write_desc = feedback.clone().map(k, |input| input.m_axis_dma_write_desc);
        let (m_axis_dma_write_desc_vr, m_axis_dma_write_desc_remaining) =
            m_axis_dma_write_desc.into_vr(k).register_slice_fwd(k);

        // store PTP timestamp
        let store_ptp_ready = feedback.clone().map(k, |input| input.store_ptp_ready);
        let s_axis_rx_ptp_ts = input.s_axis_rx_ptp_ts.transfer(k, store_ptp_ready);

        // store RX hash
        let store_rx_hash_ready = feedback.clone().map(k, |input| input.store_rx_hash_ready);
        let s_axis_rx_hash = input.s_axis_rx_hash.transfer(k, store_rx_hash_ready);

        // store RX checksum
        let store_rx_csum_ready = feedback.clone().map(k, |input| input.store_rx_csum_ready);
        let s_axis_rx_csum = input.s_axis_rx_csum.transfer(k, store_rx_csum_ready);

        // initiate completion write
        let m_axis_cpl_req = feedback.clone().map(k, |input| input.m_axis_cpl_req);
        let (m_axis_cpl_req_vr, m_axis_cpl_req_remaining) = m_axis_cpl_req.into_vr(k).register_slice_fwd(k);

        // operation complete
        let m_axis_rx_req_status = feedback.map(k, |input| input.m_axis_rx_req_status).buffer(k, Expr::invalid());

        let feedback: UniChannel<Feedback< QUEUE_INDEX_WIDTH,  DESC_REQ_TAG_WIDTH,  CPL_DATA_SIZE,  DMA_CLIENT_LEN_WIDTH,  REQ_TAG_WIDTH>> = (input.enable)
            .zip6(k, input.mtu, s_axis_rx_req, input.s_axis_rx_desc_status, input.s_axis_desc_req_status, s_axis_desc)
            .zip6(k, input.s_axis_dma_write_desc_status, s_axis_rx_ptp_ts, s_axis_rx_hash, s_axis_rx_csum, input.s_axis_cpl_req_status)
            .zip5(k, m_axis_rx_desc_remaining, m_axis_desc_req_remaining, m_axis_dma_write_desc_remaining, m_axis_cpl_req_remaining)
            .fsm_map(k, None, DescTable::<DESC_TABLE_SIZE>::new_expr(), |input, state| {
                // Projections.

                let (input, m_axis_rx_desc_remaining, m_axis_desc_req_remaining, m_axis_dma_write_desc_remaining, m_axis_cpl_req_remaining) = *input;
                let (input, s_axis_dma_write_desc_status, s_axis_rx_ptp_ts, s_axis_rx_hash, s_axis_rx_csum, s_axis_cpl_req_status) = *input;
                let (enable, mtu_next, s_axis_rx_req, s_axis_rx_desc_status, s_axis_desc_req_status, s_axis_desc) = *input;
                let mut state = *state;

                let entries = state.entries;

                let start_ptr = state.start_ptr;
                let start_ptr_idx = start_ptr.resize();
                let start_entry = DescTableEntryVarArr::get_entry(entries, start_ptr_idx);

                let dequeue_start_ptr = state.dequeue_start_ptr;
                let dequeue_start_ptr_idx = dequeue_start_ptr.resize();
                let dequeue_start_entry = DescTableEntryVarArr::get_entry(entries, dequeue_start_ptr_idx);

                let store_ptp_ts_ptr = state.store_ptp_ts_ptr;
                let store_ptp_ts_ptr_idx = store_ptp_ts_ptr.resize();
                let store_ptp_ts_entry = DescTableEntryVarArr::get_entry(entries, store_ptp_ts_ptr_idx);

                let store_hash_ptr = state.store_hash_ptr;
                let store_hash_ptr_idx = store_hash_ptr.resize();
                let store_hash_entry = DescTableEntryVarArr::get_entry(entries, store_hash_ptr_idx);

                let store_csum_ptr = state.store_csum_ptr;
                let store_csum_ptr_idx = store_csum_ptr.resize();
                let store_csum_entry = DescTableEntryVarArr::get_entry(entries, store_csum_ptr_idx);

                let cpl_enqueue_start_ptr = state.cpl_enqueue_start_ptr;
                let cpl_enqueue_start_ptr_idx = cpl_enqueue_start_ptr.resize();
                let cpl_enqueue_start_entry = DescTableEntryVarArr::get_entry(entries, cpl_enqueue_start_ptr_idx);

                let finish_ptr = state.finish_ptr;
                let finish_ptr_idx = finish_ptr.resize();
                let finish_entry = DescTableEntryVarArr::get_entry(entries, finish_ptr_idx);

                let mtu = state.mtu;

                let buf_wr_ptr = state.buf_wr_ptr;
                let buf_rd_ptr = state.buf_rd_ptr;

                let desc_start = state.desc_start;
                let desc_done = state.desc_done;
                let desc_len = state.desc_len;

                let active_desc_req_count = state.active_desc_req_count;

                // receive packet
                // wait for receive request
                let s_axis_rx_req_valid = s_axis_rx_req.valid;
                let s_axis_rx_req = s_axis_rx_req.inner;

                let receive_packet_ready = enable
                    & (buf_wr_ptr - buf_rd_ptr).clip_const::<Sum<U<CL_RX_BUFFER_SIZE>, U<1>>>(0).is_lt((RX_BUFFER_SIZE - MAX_RX_SIZE).into())
                    & !start_entry.active
                    & (start_ptr - finish_ptr).clip_const::<Sum<U<CL_DESC_TABLE_SIZE>, U<1>>>(0).is_lt(DESC_TABLE_SIZE.into())
                    & !m_axis_rx_desc_remaining;

                let desc_table_start_en = s_axis_rx_req_valid.cond(Expr::from(true), Expr::from(false));
                let m_axis_rx_desc = Expr::<Valid<_>>::new(
                    desc_table_start_en,
                    RDescProj {
                        addr: buf_wr_ptr.clip_const::<U<CL_RX_BUFFER_SIZE>>(0).resize(),
                        len: mtu.resize(),
                        tag: start_ptr_idx.resize(),
                    }
                    .into(),
                );

                let buf_wr_ptr_cond = (buf_wr_ptr.clip_const::<U<CL_RX_BUFFER_SIZE>>(0) + mtu.resize())
                    .is_gt((RX_BUFFER_SIZE - MAX_RX_SIZE).into());
                let buf_wr_ptr_next = select! {
                    desc_table_start_en & buf_wr_ptr_cond => {
                        !buf_wr_ptr & !(Expr::from(RX_BUFFER_PTR_MASK as u16).repr().resize())
                    },
                    desc_table_start_en => {
                        ((buf_wr_ptr + mtu.resize())
                            + Expr::from(RX_BUFFER_PTR_MASK_LOWER as u16).repr().resize()).resize()
                            & !(Expr::from(RX_BUFFER_PTR_MASK_LOWER as u16).repr().resize())
                    },
                    default => buf_wr_ptr,
                };

                // receive done
                // wait for receive completion
                let s_axis_rx_desc_status_valid = s_axis_rx_desc_status.valid;
                let s_axis_rx_desc_status = s_axis_rx_desc_status.inner;

                let desc_table_rx_finish_en = s_axis_rx_desc_status_valid;

                // descriptor fetch
                let desc_table_dequeue_start_en = dequeue_start_entry.active
                    & !dequeue_start_ptr.is_eq(start_ptr)
                    & dequeue_start_entry.rx_done
                    & !m_axis_desc_req_remaining
                    & active_desc_req_count.is_lt(MAX_DESC_REQ.into());

                let m_axis_desc_req = Expr::<Valid<_>>::new(
                    desc_table_dequeue_start_en,
                    ReqProj { queue: dequeue_start_entry.queue, tag: dequeue_start_ptr_idx.resize() }.into(),
                );

                let inc_active_desc_req = desc_table_dequeue_start_en.cond(Expr::from(true), Expr::from(false));

                // descriptor fetch
                // wait for queue query response
                let s_axis_desc_req_status_valid = s_axis_desc_req_status.valid;
                let s_axis_desc_req_status = s_axis_desc_req_status.inner;

                let queue_empty = s_axis_desc_req_status.error | s_axis_desc_req_status.empty;

                let desc_table_dequeue_en = s_axis_desc_req_status_valid.cond(Expr::from(true), Expr::from(false));

                let dec_active_desc_req_1 = (desc_table_dequeue_en & queue_empty).cond(Expr::from(true), Expr::from(false));

                // descriptor processing and DMA request generation
                // TODO descriptor validation?
                let s_axis_desc_valid = s_axis_desc.valid;
                let s_axis_desc = s_axis_desc.inner;
                let s_axis_desc_tid = s_axis_desc.payload.tid.resize();
                let s_axis_desc_tdata = s_axis_desc.payload.data.tdata;
                let s_axis_desc_tlast = s_axis_desc.tlast;
                let desc_entry = DescTableEntryVarArr::get_entry(entries, s_axis_desc_tid);

                let descriptor_processing_ready = !m_axis_dma_write_desc_remaining;

                let dma_write_desc_len = desc_entry.dma_len - desc_len;

                let desc_start_next = (s_axis_desc_valid & desc_entry.active).cond(false.into(), desc_start);

                let desc_done_next = (s_axis_desc_valid
                    & desc_entry.active
                    & s_axis_desc_tdata.clip_const::<U<32>>(32).is_ge(dma_write_desc_len.resize()))
                .cond(Expr::from(true), desc_done);

                let desc_len_next = (s_axis_desc_valid & desc_entry.active)
                    .cond((desc_len.resize() + s_axis_desc_tdata.clip_const::<U<32>>(32)).clip_const::<U<16>>(0), desc_len);

                let dma_write_desc_len = (s_axis_desc_tdata.clip_const::<U<32>>(32).is_lt(dma_write_desc_len.resize()))
                    .cond(s_axis_desc_tdata.clip_const::<U<16>>(32), dma_write_desc_len);

                let desc_table_write_start_init = s_axis_desc_valid & desc_entry.active & desc_start;
                let desc_table_write_start_en = s_axis_desc_valid & desc_entry.active & !(dma_write_desc_len.is_eq(0.into())) & !desc_done;
                let desc_table_desc_fetched_en = s_axis_desc_valid & desc_entry.active & s_axis_desc_tlast;
                let desc_table_write_start_commit = desc_table_desc_fetched_en;

                let m_axis_dma_write_desc = Expr::<Valid<_>>::new(
                    desc_table_write_start_en,
                    DmaDescProj {
                        dma_addr: s_axis_desc_tdata.clip_const::<U<64>>(64),
                        ram_addr: (desc_entry.buf_ptr.clip_const::<U<CL_RX_BUFFER_SIZE>>(0) + desc_len.resize()).resize(),
                        len: dma_write_desc_len,
                        tag: s_axis_desc_tid.resize(),
                    }
                    .into(),
                );

                let desc_table_desc_fetched_len = desc_table_desc_fetched_en.cond(
                    desc_len_next,
                    desc_len,
                );

                let dec_active_desc_req_2 = desc_table_write_start_commit.cond(Expr::from(true), Expr::from(false));

                let desc_start_next = desc_table_desc_fetched_en.cond(true.into(), desc_start_next);
                let desc_done_next = desc_table_desc_fetched_en.cond(false.into(), desc_done_next);
                let desc_len_next = desc_table_desc_fetched_en.cond(0.into(), desc_len_next);

                // data write completion
                // wait for data write completion
                let s_axis_dma_write_desc_status_valid = s_axis_dma_write_desc_status.valid;
                let s_axis_dma_write_desc_status = s_axis_dma_write_desc_status.inner;

                let desc_table_data_written_en = s_axis_dma_write_desc_status_valid.cond(Expr::from(true), Expr::from(false));
                let desc_table_write_finish_en = s_axis_dma_write_desc_status_valid.cond(Expr::from(true), Expr::from(false));

                // store PTP timestamp
                let s_axis_rx_ptp_ts_valid = s_axis_rx_ptp_ts.valid;
                let s_axis_rx_ptp_ts = s_axis_rx_ptp_ts.inner;

                let store_ptp_ready = store_ptp_ts_entry.active
                    & !store_ptp_ts_ptr.is_eq(start_ptr)
                    & !store_ptp_ts_entry.invalid;

                let desc_table_store_ptp_ts_en = store_ptp_ts_entry.active
                    & !store_ptp_ts_ptr.is_eq(start_ptr)
                    & (store_ptp_ts_entry.invalid | s_axis_rx_ptp_ts_valid);

                // store RX hash
                let s_axis_rx_hash_valid = s_axis_rx_hash.valid;
                let s_axis_rx_hash = s_axis_rx_hash.inner;

                let store_rx_hash_ready = store_hash_entry.active
                    & !store_hash_ptr.is_eq(start_ptr)
                    & !store_hash_entry.invalid;

                let desc_table_store_hash_en = store_hash_entry.active
                    & !store_hash_ptr.is_eq(start_ptr)
                    & (store_hash_entry.invalid | s_axis_rx_hash_valid);

                // store RX checksum
                let s_axis_rx_csum_valid = s_axis_rx_csum.valid;
                let s_axis_rx_csum = s_axis_rx_csum.inner;

                let store_rx_csum_ready = store_csum_entry.active
                    & !store_csum_ptr.is_eq(start_ptr)
                    & !store_csum_entry.invalid;

                let desc_table_store_csum_en = store_csum_entry.active
                    & !store_csum_ptr.is_eq(start_ptr)
                    & (store_csum_entry.invalid | s_axis_rx_csum_valid);

                // finish write data; start completion enqueue
                let completion_enqueue_ready = cpl_enqueue_start_entry.active
                    & !cpl_enqueue_start_ptr.is_eq(start_ptr)
                    & !cpl_enqueue_start_ptr.is_eq(dequeue_start_ptr)
                    & !cpl_enqueue_start_ptr.is_eq(store_ptp_ts_ptr)
                    & !cpl_enqueue_start_ptr.is_eq(store_hash_ptr)
                    & !cpl_enqueue_start_ptr.is_eq(store_csum_ptr);

                let desc_table_cpl_enqueue_start_en = completion_enqueue_ready
                    & (cpl_enqueue_start_entry.invalid | (cpl_enqueue_start_entry.data_written & !m_axis_cpl_req_remaining));

                let buf_rd_ptr_cond = (cpl_enqueue_start_entry.buf_ptr.clip_const::<U<CL_RX_BUFFER_SIZE>>(0) + mtu.resize())
                    .is_gt((RX_BUFFER_SIZE - MAX_RX_SIZE).into());
                let buf_rd_ptr_next = select! {
                    desc_table_cpl_enqueue_start_en & buf_rd_ptr_cond => {
                        !cpl_enqueue_start_entry.buf_ptr & !(Expr::from(RX_BUFFER_PTR_MASK as u16).repr().resize())
                    },
                    desc_table_cpl_enqueue_start_en => {
                        ((cpl_enqueue_start_entry.buf_ptr + mtu.resize())
                            + Expr::from(RX_BUFFER_PTR_MASK_LOWER as u16).repr().resize()).resize()
                            & !(Expr::from(RX_BUFFER_PTR_MASK_LOWER as u16).repr().resize())
                    },
                    default => buf_rd_ptr,
                };

                let cpl_req_data = cpl_enqueue_start_entry.queue
                    .resize::<U<16>>()
                    .append(cpl_enqueue_start_entry.queue_ptr.clip_const::<U<16>>(0))
                    .append(cpl_enqueue_start_entry.dma_len.clip_const::<U<16>>(0))
                    .resize::<U<64>>()
                    .append(cpl_enqueue_start_entry.ptp_ts.clip_const::<U<48>>(16))
                    .append(cpl_enqueue_start_entry.csum.clip_const::<U<16>>(0))
                    .append(cpl_enqueue_start_entry.hash.clip_const::<U<32>>(0))
                    .append(cpl_enqueue_start_entry.hash_type)
                    .resize();
                let m_axis_cpl_req = Expr::<Valid<_>>::new(
                    completion_enqueue_ready
                        & !cpl_enqueue_start_entry.invalid
                        & (cpl_enqueue_start_entry.data_written & !m_axis_cpl_req_remaining),
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

                let desc_table_cpl_write_done_en = s_axis_cpl_req_status_valid;

                // operation complete
                let complete_ready = finish_entry.active
                    & !finish_ptr.is_eq(start_ptr)
                    & !finish_ptr.is_eq(cpl_enqueue_start_ptr);

                let desc_table_finish_en = complete_ready
                    & (finish_entry.invalid | finish_entry.cpl_write_done);

                let m_axis_rx_req_status = Expr::<Valid<_>>::new(
                    desc_table_finish_en,
                    ReqStatusProj {
                        len: finish_entry.invalid.cond(0.into(), finish_entry.dma_len),
                        tag: finish_entry.tag,
                    }
                    .into(),
                );

                let feedback = FeedbackProj {
                    receive_packet_ready,
                    descriptor_processing_ready,
                    store_ptp_ready,
                    store_rx_hash_ready,
                    store_rx_csum_ready,
                    m_axis_rx_desc,
                    m_axis_desc_req,
                    m_axis_dma_write_desc,
                    m_axis_cpl_req,
                    m_axis_rx_req_status,
                };

                // Calculates the new state.

                let mut entries = *entries;

                // desc_table_start_en
                // descriptor table operations
                entries.active = if_then_set! { entries.active, desc_table_start_en, start_ptr_idx, Expr::from(true) };
                entries.rx_done = if_then_set! { entries.rx_done, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.invalid = if_then_set! { entries.invalid, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.desc_fetched = if_then_set! { entries.desc_fetched, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.data_written = if_then_set! { entries.data_written, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.cpl_write_done = if_then_set! { entries.cpl_write_done, desc_table_start_en, start_ptr_idx, Expr::from(false) };
                entries.queue = if_then_set_var_arr! { entries.queue, desc_table_start_en, start_ptr_idx, s_axis_rx_req.queue };
                entries.tag = if_then_set_var_arr! { entries.tag, desc_table_start_en, start_ptr_idx, s_axis_rx_req.tag };
                entries.buf_ptr = if_then_set_var_arr! { entries.buf_ptr, desc_table_start_en, start_ptr_idx, buf_wr_ptr };

                let new_start_ptr = (start_ptr + 1.into()).resize();
                state.start_ptr = (!desc_table_start_en).cond(start_ptr, new_start_ptr);

                // desc_table_rx_finish_en
                let rx_finish_ptr = s_axis_rx_desc_status.tag.resize();
                entries.dma_len = if_then_set_var_arr! { entries.dma_len, desc_table_rx_finish_en, rx_finish_ptr, s_axis_rx_desc_status.len };
                entries.rx_done = if_then_set! { entries.rx_done, desc_table_rx_finish_en, rx_finish_ptr, Expr::from(true) };

                // desc_table_dequeue_start_en
                let new_dequeue_start_ptr = (dequeue_start_ptr + 1.into()).resize();
                state.dequeue_start_ptr = desc_table_dequeue_start_en.cond(new_dequeue_start_ptr, dequeue_start_ptr);

                // desc_table_dequeue_en
                let dequeue_ptr = s_axis_desc_req_status.tag.resize();
                entries.queue_ptr = if_then_set_var_arr! { entries.queue_ptr, desc_table_dequeue_en, dequeue_ptr, s_axis_desc_req_status.ptr };
                entries.cpl_queue = if_then_set_var_arr! { entries.cpl_queue, desc_table_dequeue_en, dequeue_ptr, s_axis_desc_req_status.cpl };
                entries.invalid = entries.invalid.set(dequeue_ptr, (!(desc_table_dequeue_en & (s_axis_desc_req_status.error | s_axis_desc_req_status.empty))).cond(entries.invalid[dequeue_ptr], true.into()));

                // desc_table_desc_fetched_en
                entries.desc_len = if_then_set_var_arr! { entries.desc_len, desc_table_desc_fetched_en, s_axis_desc_tid.resize(), desc_table_desc_fetched_len };
                entries.desc_fetched = if_then_set! { entries.desc_fetched, desc_table_desc_fetched_en, s_axis_desc_tid.resize(), Expr::from(true) };

                // desc_table_data_written_en
                let data_written_ptr = s_axis_dma_write_desc_status.tag.resize();
                entries.data_written = if_then_set! { entries.data_written, desc_table_data_written_en, data_written_ptr, Expr::from(true) };

                // desc_table_store_ptp_ts_en
                entries.ptp_ts = if_then_set_var_arr! { entries.ptp_ts, desc_table_store_ptp_ts_en, store_ptp_ts_ptr_idx, s_axis_rx_ptp_ts.ts };

                let new_store_ptp_ts_ptr = (store_ptp_ts_ptr + 1.into()).resize();
                state.store_ptp_ts_ptr =
                    (!desc_table_store_ptp_ts_en).cond(store_ptp_ts_ptr, new_store_ptp_ts_ptr);

                // desc_table_store_hash_en
                entries.hash = if_then_set_var_arr! { entries.hash, desc_table_store_hash_en, store_hash_ptr_idx, s_axis_rx_hash.data };
                entries.hash_type = if_then_set_var_arr! { entries.hash_type, desc_table_store_hash_en, store_hash_ptr_idx, s_axis_rx_hash.typ };

                let new_store_hash_ptr = (store_hash_ptr + 1.into()).resize();
                state.store_hash_ptr =
                    (!desc_table_store_hash_en).cond(store_hash_ptr, new_store_hash_ptr);

                // desc_table_store_csum_en
                entries.csum = if_then_set_var_arr! { entries.csum, desc_table_store_csum_en, store_csum_ptr_idx, s_axis_rx_csum };

                let new_store_csum_ptr = (store_csum_ptr + 1.into()).resize();
                state.store_csum_ptr =
                    (!desc_table_store_csum_en).cond(store_csum_ptr, new_store_csum_ptr);

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

                // desc_table_write_start_en
                entries.read_commit = entries.read_commit.set(s_axis_desc_tid, (!desc_table_write_start_en & !(desc_table_write_start_commit | desc_table_write_start_init)).cond(entries.read_commit[s_axis_desc_tid], desc_table_write_start_commit));
                entries.write_count_start = entries.write_count_start.set_var_arr(
                    s_axis_desc_tid,
                    (!desc_table_write_start_en & !(desc_table_write_start_commit | desc_table_write_start_init)).cond(
                        entries.write_count_start[s_axis_desc_tid],
                        select! {
                            desc_table_write_start_en & desc_table_write_start_init => (entries.write_count_finish[s_axis_desc_tid] + 1.into()).clip_const::<U<4>>(0),
                            desc_table_write_start_en => (entries.write_count_start[s_axis_desc_tid] + 1.into()).clip_const::<U<4>>(0),
                            desc_table_write_start_init => entries.write_count_finish[s_axis_desc_tid],
                            default => entries.write_count_start[s_axis_desc_tid],
                        },
                    )
                );

                // desc_table_write_finish_en
                let write_finish_ptr = s_axis_dma_write_desc_status.tag.resize();
                entries.write_count_finish = if_then_set_var_arr! { entries.write_count_finish, desc_table_write_finish_en, write_finish_ptr, (entries.write_count_finish[write_finish_ptr] + 1.into()).clip_const::<U<4>>(0) };

                state.entries = entries.into();

                state.mtu = mtu_next.is_gt(MAX_RX_SIZE.into()).cond(MAX_RX_SIZE.into(), mtu_next.resize());

                state.buf_wr_ptr = buf_wr_ptr_next;
                state.buf_rd_ptr = buf_rd_ptr_next;

                state.desc_start = desc_start_next;
                state.desc_done = desc_done_next;
                state.desc_len = desc_len_next;

                state.active_desc_req_count = (state.active_desc_req_count + inc_active_desc_req.repr().resize()).resize()
                    - dec_active_desc_req_1.repr().resize()
                    - dec_active_desc_req_2.repr().resize();

                (feedback.into(), state.into())
            });

        (
            O {
                m_axis_rx_req_status,
                m_axis_desc_req: m_axis_desc_req_vr,
                m_axis_cpl_req: m_axis_cpl_req_vr,
                m_axis_dma_write_desc: m_axis_dma_write_desc_vr,
                m_axis_rx_desc: m_axis_rx_desc_vr,
            },
            feedback,
        )
    })
    .loop_feedback()
    .build()
}

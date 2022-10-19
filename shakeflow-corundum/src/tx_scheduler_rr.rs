//! Transmit scheduler (round-robin).
//!
//! TODO: Parametrize some constants

use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::constants::tx_scheduler_rr::*;
use super::types::axil::*;
use super::types::queue_manager::*;
use super::types::request::*;

#[derive(Debug, Clone, Signal)]
pub struct SchedCtrl {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    enable: bool,
}

#[derive(Debug, Interface)]
pub struct I<const REQ_TAG_WIDTH: usize, const QUEUE_INDEX_WIDTH: usize> {
    /// Transmit request status input
    s_axis_tx_req_status: UniChannel<Valid<ReqStatus<LEN_WIDTH, REQ_TAG_WIDTH>>>,

    /// Doorbell input
    s_axis_doorbell: UniChannel<Valid<Doorbell<QUEUE_INDEX_WIDTH>>>,

    /// Scheduler control input
    s_axis_sched_ctrl: VrChannel<SchedCtrl>,

    /// AXI-Lite slave interface
    #[member(nosep)]
    s_axil_aw: VrChannel<Addr<AXIL_ADDR_WIDTH>>,

    #[member(nosep)]
    s_axil_w: VrChannel<WReq>,

    #[member(nosep)]
    s_axil_ar: VrChannel<Addr<AXIL_ADDR_WIDTH>>,

    /// Control
    enable: UniChannel<bool>,
}

#[derive(Debug, Interface)]
pub struct O<const REQ_TAG_WIDTH: usize, const QUEUE_INDEX_WIDTH: usize> {
    /// Transmit request output (queue index)
    m_axis_tx_req: VrChannel<Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>,

    /// AXI-Lite slave interface
    #[member(nosep)]
    s_axil_b: VrChannel<WRes>,

    #[member(nosep)]
    s_axil_r: VrChannel<RRes>,

    /// Control
    active: UniChannel<bool>,
}

#[derive(Debug, Default, Clone, Copy, Signal)]
struct Selector {
    /// Init queue states
    internal: bool,

    /// AXIL write
    write: bool,

    /// AXIL read
    read: bool,

    /// Handle doorbell
    doorbell: bool,

    /// Transmit complete
    complete: bool,

    /// Transmit request
    req: bool,
}

impl Selector {
    /// Creates new selector expr from bits.
    ///
    /// # Note
    ///
    /// Order of bits should follow the order of selector fields.
    fn from_bits<'id>(selector: Expr<'id, Bits<U<6>>>) -> Expr<'id, Self> {
        SelectorProj {
            internal: selector[0],
            write: selector[1],
            read: selector[2],
            doorbell: selector[3],
            complete: selector[4],
            req: selector[5],
        }
        .into()
    }

    fn is_active<'id>(selector: Expr<'id, Self>) -> Expr<'id, bool> {
        selector.internal | selector.write | selector.read | selector.doorbell | selector.complete | selector.req
    }
}

#[derive(Debug, Clone, Signal)]
struct Feedback<
    const PIPELINE: usize,
    const REQ_TAG_WIDTH: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const QUEUE_RAM_WIDTH: usize,
    const OP_TABLE_SIZE: usize,
> {
    queue_ram_read_data: Bits<U<QUEUE_RAM_WIDTH>>,
    op_table_active: Bits<U<OP_TABLE_SIZE>>,
    finish_entry: OpTableEntry<QUEUE_INDEX_WIDTH, OP_TABLE_SIZE>,
    finish_fifo: Array<FinishFifoEntry<REQ_TAG_WIDTH>, U<OP_TABLE_SIZE>>,
    finish_fifo_wr_ptr: Bits<Sum<Log2<U<OP_TABLE_SIZE>>, U<1>>>,
    finish_fifo_rd_ptr: Bits<Sum<Log2<U<OP_TABLE_SIZE>>, U<1>>>,
    finish: Valid<Finish<OP_TABLE_SIZE>>,
    init: bool,
    init_index: Bits<U<QUEUE_INDEX_WIDTH>>,
    active_queue_count: Bits<U<QUEUE_INDEX_WIDTH>>,
    m_axis_tx_req: Valid<Req<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>,
    s_axil_b: Valid<WRes>,
    s_axil_r: Valid<RRes>,
}

#[derive(Debug, Clone, Signal)]
struct PipelineStage<const QUEUE_RAM_WIDTH: usize, const QUEUE_INDEX_WIDTH: usize, const OP_TABLE_SIZE: usize> {
    selector: Selector,
    command: Cmd<QUEUE_RAM_WIDTH, QUEUE_INDEX_WIDTH, OP_TABLE_SIZE>,
}

#[derive(Debug, Clone, Signal)]
struct OpTableEntry<const QUEUE_INDEX_WIDTH: usize, const OP_TABLE_SIZE: usize> {
    active: bool,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    doorbell: bool,
    is_head: bool,
    next_index: Bits<Log2<U<OP_TABLE_SIZE>>>,
    prev_index: Bits<Log2<U<OP_TABLE_SIZE>>>,
}

#[derive(Debug, Clone, Signal)]
struct FinishFifoEntry<const REQ_TAG_WIDTH: usize> {
    tag: Bits<U<REQ_TAG_WIDTH>>,
    status: bool,
}

#[derive(Debug, Clone, Signal)]
struct Finish<const OP_TABLE_SIZE: usize> {
    ptr: Bits<Log2<U<OP_TABLE_SIZE>>>,
    status: bool,
}

#[derive(Debug, Clone, Signal)]
struct OpState<
    QueueCount: Num,
    const REQ_TAG_WIDTH: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const QUEUE_RAM_WIDTH: usize,
    const OP_TABLE_SIZE: usize,
> {
    queue_ram: VarArray<Bits<U<QUEUE_RAM_WIDTH>>, QueueCount>,
    op_table: OpTableEntryVarArr<QUEUE_INDEX_WIDTH, OP_TABLE_SIZE, OP_TABLE_SIZE>,
    finish_fifo: Array<FinishFifoEntry<REQ_TAG_WIDTH>, U<OP_TABLE_SIZE>>,
    finish_fifo_wr_ptr: Bits<Sum<Log2<U<OP_TABLE_SIZE>>, U<1>>>,
    finish_fifo_rd_ptr: Bits<Sum<Log2<U<OP_TABLE_SIZE>>, U<1>>>,
    finish: Valid<Finish<OP_TABLE_SIZE>>,
    init: bool,
    init_index: Bits<U<QUEUE_INDEX_WIDTH>>,
    active_queue_count: Bits<U<QUEUE_INDEX_WIDTH>>,
}

impl<const OP_TABLE_SIZE: usize> Finish<OP_TABLE_SIZE> {
    fn new_expr() -> Expr<'static, Self> { FinishProj { ptr: 0.into(), status: false.into() }.into() }
}

impl<
        QueueCount: Num,
        const REQ_TAG_WIDTH: usize,
        const QUEUE_INDEX_WIDTH: usize,
        const QUEUE_RAM_WIDTH: usize,
        const OP_TABLE_SIZE: usize,
    > OpState<QueueCount, REQ_TAG_WIDTH, QUEUE_INDEX_WIDTH, QUEUE_RAM_WIDTH, OP_TABLE_SIZE>
{
    fn new_expr() -> Expr<'static, Self> {
        OpStateProj {
            queue_ram: Expr::x(),
            op_table: OpTableEntryVarArr::new_expr(),
            finish_fifo: Expr::x(),
            finish_fifo_wr_ptr: 0.into(),
            finish_fifo_rd_ptr: 0.into(),
            finish: Expr::<Valid<_>>::new(false.into(), Finish::new_expr()),
            init: false.into(),
            init_index: 0.into(),
            active_queue_count: 0.into(),
        }
        .into()
    }
}

#[derive(Debug, Clone, Signal)]
pub struct Cmd<const QUEUE_RAM_WIDTH: usize, const QUEUE_INDEX_WIDTH: usize, const OP_TABLE_SIZE: usize> {
    queue_ram_addr: Bits<U<QUEUE_INDEX_WIDTH>>,
    queue_ram_read_data: Bits<U<QUEUE_RAM_WIDTH>>,
    write_req: WReq,
    op_index: Bits<Log2<U<OP_TABLE_SIZE>>>,
}

impl<const QUEUE_RAM_WIDTH: usize, const QUEUE_INDEX_WIDTH: usize, const OP_TABLE_SIZE: usize> Command
    for Cmd<QUEUE_RAM_WIDTH, QUEUE_INDEX_WIDTH, OP_TABLE_SIZE>
{
    fn collision<'id>(lhs: Expr<'id, Self>, rhs: Expr<'id, Self>) -> Expr<'id, bool> {
        lhs.queue_ram_addr.is_eq(rhs.queue_ram_addr)
    }
}

impl<const QUEUE_RAM_WIDTH: usize, const QUEUE_INDEX_WIDTH: usize, const OP_TABLE_SIZE: usize>
    Cmd<QUEUE_RAM_WIDTH, QUEUE_INDEX_WIDTH, OP_TABLE_SIZE>
{
    /// Creates new expr.
    pub fn new_expr<'id>(
        queue_ram_addr: Expr<'id, Bits<U<QUEUE_INDEX_WIDTH>>>,
        queue_ram_read_data: Expr<'id, Bits<U<QUEUE_RAM_WIDTH>>>, write_req: Expr<'id, WReq>,
        op_index: Expr<'id, Bits<Log2<U<OP_TABLE_SIZE>>>>,
    ) -> Expr<'id, Self> {
        CmdProj { queue_ram_addr, queue_ram_read_data, write_req, op_index }.into()
    }
}

pub fn m<
    const PIPELINE: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const QUEUE_RAM_WIDTH: usize,
    const OP_TABLE_SIZE: usize,
    const QUEUE_COUNT: usize,
>() -> Module<I<REQ_TAG_WIDTH, QUEUE_INDEX_WIDTH>, O<REQ_TAG_WIDTH, QUEUE_INDEX_WIDTH>> {
    composite::<(I<REQ_TAG_WIDTH, QUEUE_INDEX_WIDTH>, (VrChannel<Bits<U<QUEUE_INDEX_WIDTH>>>, UniChannel<Feedback<PIPELINE, REQ_TAG_WIDTH, QUEUE_INDEX_WIDTH, QUEUE_RAM_WIDTH, OP_TABLE_SIZE>>)), _, _>("tx_scheduler_rr", None, None, |input, k| {
        // Projections.
        let (input, (axis_schedule_fifo_in, feedback)) = input;

        // Drop sched ctrl.
        input.s_axis_sched_ctrl.block(k);

        // Calculates the final output.
        let m_axis_tx_req = feedback.clone().map(k, |input| input.m_axis_tx_req);
        let (m_axis_tx_req_vr, m_axis_tx_req_remaining) = m_axis_tx_req.into_vr(k).register_slice_fwd(k);

        let s_axil_b = feedback.clone().map(k, |input| input.s_axil_b);
        let (s_axil_b_vr, s_axil_b_remaining) = s_axil_b.into_vr(k).register_slice_fwd(k);

        let s_axil_r = feedback.clone().map(k, |input| input.s_axil_r);
        let (s_axil_r_vr, s_axil_r_remaining) = s_axil_r.into_vr(k).register_slice_fwd(k);

        let active = feedback.clone().map(k, |f| f.active_queue_count.is_gt(0.into()));

        let output = O {
            m_axis_tx_req: m_axis_tx_req_vr,
            s_axil_b: s_axil_b_vr,
            s_axil_r: s_axil_r_vr,
            active,
        };

        // Instantiates axis_fifo (doorbell).
        let doorbell_fifo_input = input
            .s_axis_doorbell
            .map_inner(k, |input| KeepProj::<U<QUEUE_INDEX_WIDTH>, U<1>> { tdata: input.queue, tkeep: 0.into() }.into())
            .into_vr(k)
            .into_axis_vr(k);

        let axis_doorbell_fifo = doorbell_fifo_input
            .axis_fifo::<U<256>, QUEUE_INDEX_WIDTH, false, 1, false, false, 0, false, 0, false, 0, 0>(k, "doorbell_fifo")
            .into_vr(k)
            .map(k, |input| DoorbellProj { queue: input.tdata }.into());

        // Instantiates axis_fifo (round-robin).
        let rr_fifo_input = axis_schedule_fifo_in
            .map(k, |input| KeepProj { tdata: input, tkeep: [false].into() }.into())
            .into_axis_vr(k);

        let axis_scheduler_fifo_out = rr_fifo_input
            .axis_fifo::<Pow2<U<QUEUE_INDEX_WIDTH>>, QUEUE_INDEX_WIDTH, false, 1, false, false, 0, false, 0, false, 0, 0>(k, "rr_fifo")
            .into_vr(k)
            .map(k, |input| input.tdata);

        // Instantiates priority_encoder.
        let priority_encoder_output = feedback
            .clone()
            .map(k, |f| priority_mux::IProj { unencoded: !f.op_table_active }.into())
            .priority_mux(k);

        // Calculates pipeline input. (op_*_pipe_next[0])

        // init queue states
        let internal = feedback
            .clone()
            .map(k, |feedback| {
                Expr::valid(Cmd::new_expr(feedback.init_index, feedback.queue_ram_read_data, WReq::new_expr(), 0.into()))
            })
            .into_vr(k);

        // AXIL write
        let write =
            input.s_axil_aw.zip_vr(k, input.s_axil_w).zip_uni(k, feedback.clone()).register_slice_bwd(k).map(
                k,
                |input| {
                    let (input, feedback) = *input;
                    let (s_axil_aw, s_axil_w) = *input;
                    let s_axil_awaddr_queue = s_axil_aw.addr.clip_const::<U<QUEUE_INDEX_WIDTH>>(2);
                    Cmd::new_expr(s_axil_awaddr_queue, feedback.queue_ram_read_data, s_axil_w, 0.into())
                },
            );

        // AXIL read
        let read = input.s_axil_ar.zip_uni(k, feedback.clone()).register_slice_bwd(k).map(k, |input| {
            let (s_axil_ar, feedback) = *input;
            let s_axil_araddr_queue = s_axil_ar.addr.clip_const::<U<QUEUE_INDEX_WIDTH>>(2);
            Cmd::new_expr(s_axil_araddr_queue, feedback.queue_ram_read_data, WReq::new_expr(), 0.into())
        });

        // handle doorbell
        let doorbell = axis_doorbell_fifo.zip_uni(k, feedback.clone()).map(k, |input| {
            let (axis_doorbell_fifo, feedback) = *input;
            Cmd::new_expr(axis_doorbell_fifo.queue, feedback.queue_ram_read_data, WReq::new_expr(), 0.into())
        });

        // transmit complete
        let complete = feedback
            .clone()
            .map(k, |f| {
                let output = Cmd::new_expr(
                    f.finish_entry.queue,
                    f.queue_ram_read_data,
                    WReqProj {
                        data: (f.finish.inner.status | f.finish_entry.doorbell).repr().resize(),
                        strb: 0.into(),
                    }
                    .into(),
                    f.finish.inner.ptr,
                );
                Expr::valid(output)
            })
            .into_vr(k);

        // transmit request
        let req = axis_scheduler_fifo_out
            .zip_uni(k, feedback)
            .zip_uni(k, input.enable)
            .zip_uni(k, priority_encoder_output.clone())
            .assert_map(k, |input| {
                let (input, op_table_start_ptr) = *input;
                let (input, enable) = *input;
                let (axis_scheduler_fifo_out, feedback) = *input;

                let output = Cmd::new_expr(
                    axis_scheduler_fifo_out,
                    feedback.queue_ram_read_data,
                    WReq::new_expr(),
                    op_table_start_ptr.inner.encoded.resize(),
                );
                (enable, output).into()
            });

        let ((pipeline_input, pipeline_stage), extra_hazard) = [internal, write, read, doorbell, complete, req].command_queue::<PIPELINE>(k, [false, true, true, false, true, true]);

        let pipeline_input = pipeline_input
            .map(k, |input| PipelineStageProj { selector: Selector::from_bits(input.1), command: input.0 }.into());

        let pipeline_stage = pipeline_stage
            .map(k, |input| PipelineStageProj { selector: Selector::from_bits(input.1), command: input.0 }.into());

        // Calculates axis_scheduler_fifo_in.
        let axis_scheduler_fifo_in = pipeline_stage.clone()
            .map(k, |stage| {
                let selector = stage.selector;
                let stage = stage.command;
                let queue_ram_addr = stage.queue_ram_addr;
                let queue_ram_read_data = stage.queue_ram_read_data;
                let write_req = stage.write_req;

                // queue_ram_read_data_*
                let enabled = queue_ram_read_data[0];
                let active = queue_ram_read_data[6];
                let scheduled = queue_ram_read_data[7];

                let valid = (selector.doorbell & enabled & !scheduled)
                    | (selector.req & enabled & active & scheduled)
                    | (selector.complete & write_req.data[0] & !scheduled)
                    | (selector.write & write_req.data[0] & active & !scheduled);

                Expr::<Valid<_>>::new(valid, queue_ram_addr)
            })
            .into_vr(k);

        // Calculates the feedback.
        let feedback = pipeline_input
        .zip4(k, pipeline_stage, input.s_axis_tx_req_status, priority_encoder_output)
        .fsm_map(k, None, OpState::<U<QUEUE_COUNT>, REQ_TAG_WIDTH, QUEUE_INDEX_WIDTH, QUEUE_RAM_WIDTH, OP_TABLE_SIZE>::new_expr(), |input, state| {
            let (pipeline_input, pipeline_stage, s_axis_tx_req_status, priority_encoder_output) = *input;

            let finish_fifo_we = s_axis_tx_req_status.valid;
            let finish_fifo_entry = FinishFifoEntryProj {
                tag: s_axis_tx_req_status.inner.tag,
                status: s_axis_tx_req_status.inner.len.is_gt(0.into()),
            }
            .into();

            let op_table_start_ptr = priority_encoder_output.inner.encoded;

            let mut state = *state;

            let op_table = state.op_table;

            let finish_fifo_wr_ptr = state.finish_fifo_wr_ptr;
            let finish_fifo_rd_ptr = state.finish_fifo_rd_ptr;

            let finish_fifo = state.finish_fifo;

            let finish = state.finish;

            let init_index = state.init_index;

            let active_queue_count = state.active_queue_count;

            // Updates states from pipeline input.
            let pipeline_input = pipeline_input;

            let selector = pipeline_input.selector;
            let pipeline_input = pipeline_input.command;

            let init_next = state.init | (selector.internal & init_index.is_eq(Expr::from(true).repeat::<U<QUEUE_INDEX_WIDTH>>()));
            let init_index_next = selector.internal.cond((init_index + 1.into()).clip_const(0), init_index);

            let op_table_start_en = selector.req;
            let op_table_start_queue = pipeline_input.queue_ram_addr;

            let finish_next = select! {
                selector.complete => Expr::<Valid<_>>::new(false.into(), finish.inner),
                !finish.valid & !finish_fifo_wr_ptr.is_eq(finish_fifo_rd_ptr) => Expr::valid(
                    FinishProj {
                        ptr: finish_fifo[finish_fifo_rd_ptr.resize()].tag.clip_const(0),
                        status: finish_fifo[finish_fifo_rd_ptr.resize()].status,
                    }
                    .into(),
                ),
                default => state.finish,
            };

            // Update states from pipeline output.
            let selector = pipeline_stage.selector;
            let pipeline_stage = pipeline_stage.command;

            let queue_ram_addr = pipeline_stage.queue_ram_addr;
            let queue_ram_read_data = pipeline_stage.queue_ram_read_data;
            let write_req = pipeline_stage.write_req;
            let op_index = pipeline_stage.op_index;

            let write_data = write_req.data;
            let write_strb = write_req.strb;

            // queue_ram_read_data_*
            let enabled = queue_ram_read_data[0];
            let active = queue_ram_read_data[6];
            let scheduled = queue_ram_read_data[7];
            let op_tail_index = queue_ram_read_data.clip_const(8);

            // queue_*
            let op_tail_index_entry = OpTableEntryVarArr::get_entry(op_table, op_tail_index);
            let tail_active = op_tail_index_entry.active & op_tail_index_entry.queue.is_eq(queue_ram_addr);

            let schedule_queue = enabled & !scheduled;
            let queue_enabled_active_scheduled = enabled & active & scheduled;
            let disabled = write_data[0] & active & !scheduled;

            // queue_ram_be, queue_ram_write_*, queue_ram_wr_en
            let queue_ram_wr_en = selector.read.cond(false.into(), Selector::is_active(selector));

            let queue_ram_write_ptr = queue_ram_addr;

            let queue_ram_write_data = select! {
                selector.internal => queue_ram_read_data.set(0.into(), false.into()).set(6.into(), false.into()).set(7.into(), false.into()),
                (selector.doorbell & schedule_queue) => queue_ram_read_data.set(6.into(), true.into()).set(7.into(), true.into()),
                selector.doorbell => queue_ram_read_data.set(6.into(), true.into()),
                (selector.req & queue_enabled_active_scheduled) => queue_ram_read_data.set_range(7.into(), Expr::from(true).repr()).set_range(8.into(), op_index),
                selector.req => queue_ram_read_data.set_range(7.into(), Expr::from(false).repr()).set_range(8.into(), op_index),
                (selector.complete & write_data[0] & !scheduled) => queue_ram_read_data.set_range(6.into(), Expr::from(true).repeat::<U<2>>()),
                (selector.complete & write_data[0]) => queue_ram_read_data.set(6.into(), true.into()),
                selector.complete => queue_ram_read_data.set(6.into(), false.into()),
                (selector.write & disabled) => queue_ram_read_data.set_range(0.into(), write_data.clip_const::<U<2>>(0)).set(7.into(), true.into()),
                selector.write => queue_ram_read_data.set_range(0.into(), write_data.clip_const::<U<2>>(0)),
                default => queue_ram_read_data,
            };

            let queue_ram_be = select! {
                selector.internal => Expr::from([true, false]),
                selector.doorbell => Expr::from([true, false]),
                (selector.req & queue_enabled_active_scheduled) => Expr::from(true).repeat::<U<2>>(),
                selector.req => Expr::from([true, false]),
                selector.complete => Expr::from([true, false]),
                selector.write => write_strb[0].repr().resize(),
                default => Expr::from(false).repeat::<U<2>>(),
            };

            let active_queue_count_next = select! {
                (selector.doorbell & schedule_queue) => (active_queue_count + 1.into()).clip_const(0),
                (selector.req & !queue_enabled_active_scheduled) => active_queue_count - 1.into(),
                (selector.complete & write_data[0] & !scheduled) => active_queue_count - 1.into(),
                (selector.write & disabled) => (active_queue_count + 1.into()).clip_const(0),
                default => active_queue_count,
            };

            let finish_fifo_rd_ptr_next = (!finish.valid & !finish_fifo_wr_ptr.is_eq(finish_fifo_rd_ptr)).cond(
                (finish_fifo_rd_ptr + 1.into()).resize(),
                finish_fifo_rd_ptr,
            );

            let finish_fifo_wr_ptr_next = s_axis_tx_req_status.valid.cond(
                (finish_fifo_wr_ptr + 1.into()).resize(),
                finish_fifo_wr_ptr,
            );

            let op_index_entry = OpTableEntryVarArr::get_entry(op_table, op_index);

            let op_table_release_en = (selector.req & !queue_enabled_active_scheduled) | selector.complete;

            let op_table_doorbell = select! {
                (selector.doorbell & tail_active) => Expr::<Valid<_>>::valid(op_tail_index),
                selector.complete => Expr::<Valid<_>>::new(!op_index_entry.is_head & op_index_entry.doorbell, op_index_entry.prev_index),
                default => Expr::<Valid<_>>::invalid(),
            };

            let op_table_update_next = select! {
                (selector.req & queue_enabled_active_scheduled) => Expr::<Valid<_>>::valid((op_tail_index, op_index).into()),
                selector.complete => Expr::<Valid<_>>::new(!op_index_entry.is_head, (op_index_entry.prev_index, op_index_entry.next_index).into()),
                default => Expr::<Valid<_>>::invalid(),
            };

            let op_table_update_prev = select! {
                (selector.req & queue_enabled_active_scheduled) => Expr::<Valid<_>>::valid((op_index, op_tail_index, !tail_active | op_index.is_eq(op_tail_index)).into()),
                selector.complete => Expr::<Valid<_>>::new(!op_index.is_eq(op_tail_index), (op_index_entry.next_index, op_index_entry.prev_index, op_index_entry.is_head).into()),
                default => Expr::<Valid<_>>::invalid(),
            };

            let m_axis_tx_req = Expr::<Valid<_>>::new(
                selector.req & enabled & active & scheduled,
                ReqProj { queue: queue_ram_addr, tag: op_index.resize() }.into(),
            );

            let s_axil_b = Expr::<Valid<_>>::new(selector.write, WRes::new_expr());

            let s_axil_r = Expr::<Valid<_>>::new(
                selector.read,
                RResProj {
                    data: Expr::from(false).repeat::<U<32>>()
                        .set(0.into(), enabled)
                        .set(16.into(), active)
                        .set(24.into(), scheduled),
                    resp: 0.into(),
                }
                .into(),
            );

            let finish_entry = OpTableEntryVarArr::get_entry(op_table, finish.inner.ptr);

            let feedback = FeedbackProj {
                queue_ram_read_data: state.queue_ram[pipeline_input.queue_ram_addr.resize()],
                op_table_active: op_table.active,
                finish_entry,
                finish_fifo: state.finish_fifo,
                finish_fifo_wr_ptr: state.finish_fifo_wr_ptr,
                finish_fifo_rd_ptr: state.finish_fifo_rd_ptr,
                finish: state.finish,
                init: state.init,
                init_index: state.init_index,
                active_queue_count: state.active_queue_count,
                m_axis_tx_req,
                s_axil_b,
                s_axil_r,
            };

            // Updates state
            state.finish_fifo_rd_ptr = finish_fifo_rd_ptr_next;
            state.finish_fifo_wr_ptr = finish_fifo_wr_ptr_next;

            state.finish = finish_next;

            state.init = init_next;
            state.init_index = init_index_next;

            state.active_queue_count = active_queue_count_next;

            let queue_ram_entry = state.queue_ram[queue_ram_write_ptr.resize()];
            let new_queue_ram_entry = queue_ram_entry
                .chunk::<U<8>>()
                .zip(queue_ram_write_data.chunk::<U<8>>())
                .zip(queue_ram_be.resize())
                .map(|s| {
                    let (s, be) = *s;
                    let (o, n) = *s;
                    be.cond(n, o)
                })
                .concat()
                .resize();
            state.queue_ram = state.queue_ram.set_var_arr(
                queue_ram_write_ptr.resize(),
                (!queue_ram_wr_en).cond(queue_ram_entry, new_queue_ram_entry),
            );

            let mut op_table = *op_table;
            let op_table_start_ptr = op_table_start_ptr.resize();

            // op_table_start_en
            op_table.active = if_then_set! { op_table.active, op_table_start_en, op_table_start_ptr, Expr::from(true) };
            op_table.queue = if_then_set_var_arr! { op_table.queue, op_table_start_en, op_table_start_ptr, op_table_start_queue };
            op_table.doorbell = if_then_set! { op_table.doorbell, op_table_start_en, op_table_start_ptr, Expr::from(false) };

            // op_table_release_en
            op_table.active = if_then_set! { op_table.active, op_table_release_en, op_index, Expr::from(false) };

            // op_table_doorbell_en
            op_table.doorbell = if_then_set! { op_table.doorbell, op_table_doorbell.valid, op_table_doorbell.inner, Expr::from(false) };

            // op_table_update_next_en
            op_table.next_index = if_then_set_var_arr! { op_table.next_index, op_table_update_next.valid, op_table_update_next.inner.0, op_table_update_next.inner.1 };

            // op_table_update_prev_en
            op_table.prev_index = if_then_set_var_arr! { op_table.prev_index, op_table_update_prev.valid, op_table_update_prev.inner.0, op_table_update_prev.inner.1 };
            op_table.is_head = if_then_set! { op_table.is_head, op_table_update_prev.valid, op_table_update_prev.inner.0, op_table_update_prev.inner.2 };

            state.op_table = op_table.into();

            let new_finish_fifo = finish_fifo_we.cond(
                state.finish_fifo.set(finish_fifo_wr_ptr.resize(), finish_fifo_entry),
                state.finish_fifo,
            );
            state.finish_fifo = new_finish_fifo;

            (feedback.into(), state.into())
        });

        feedback.clone()
            .zip4(k, s_axil_b_remaining, s_axil_r_remaining, m_axis_tx_req_remaining)
            .map(k, |input| {
                let (feedback, s_axil_b_remaining, s_axil_r_remaining, m_axis_tx_req_remaining) = *input;
                [
                    !feedback.init,
                    !s_axil_b_remaining,
                    !s_axil_r_remaining,
                    true.into(),
                    feedback.finish.valid,
                    !m_axis_tx_req_remaining,
                ]
                .into()
            })
            .comb_inline(k, extra_hazard);

        (output, (axis_scheduler_fifo_in, feedback))
    })
    .loop_feedback()
    .build()
}

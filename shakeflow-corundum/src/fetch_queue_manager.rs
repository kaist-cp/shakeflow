//! Issue fetch queue manager.
//!
//! TODO: Parametrize some constants

use shakeflow::*;
use shakeflow_std::{FsmExt, UniChannel, Valid, ValidExt};

use super::queue_manager;
use super::types::axil::*;
use super::types::queue_manager::*;

pub struct FetchQueueManager<
    const PIPELINE: usize,
    const AXIL_ADDR_WIDTH: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
    const OP_TAG_WIDTH: usize,
    const CPL_INDEX_WIDTH: usize,
    const OP_TABLE_SIZE: usize,
    const QUEUE_PTR_WIDTH: usize,
>;

impl<
        const PIPELINE: usize,
        const AXIL_ADDR_WIDTH: usize,
        const QUEUE_INDEX_WIDTH: usize,
        const REQ_TAG_WIDTH: usize,
        const OP_TAG_WIDTH: usize,
        const CPL_INDEX_WIDTH: usize,
        const OP_TABLE_SIZE: usize,
        const QUEUE_PTR_WIDTH: usize,
    >
    queue_manager::QueueManager<
        PIPELINE,
        AXIL_ADDR_WIDTH,
        QUEUE_INDEX_WIDTH,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        QUEUE_PTR_WIDTH,
    >
    for FetchQueueManager<
        PIPELINE,
        AXIL_ADDR_WIDTH,
        QUEUE_INDEX_WIDTH,
        REQ_TAG_WIDTH,
        OP_TAG_WIDTH,
        CPL_INDEX_WIDTH,
        OP_TABLE_SIZE,
        QUEUE_PTR_WIDTH,
    >
{
    type Out = Doorbell<QUEUE_INDEX_WIDTH>;
    type Resp = DeqRes<QUEUE_INDEX_WIDTH, CPL_INDEX_WIDTH, REQ_TAG_WIDTH, OP_TAG_WIDTH, QUEUE_PTR_WIDTH>;

    fn calculate_feedback(
        k: &mut CompositeModuleContext,
        input: UniChannel<PipelineStage<Selector, Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>>,
        stage: UniChannel<PipelineStage<Selector, Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>>,
        op_table_commit: UniChannel<Valid<CommitReq<OP_TAG_WIDTH>>>,
    ) -> UniChannel<Temp<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH, Self::Resp, Self::Out>> {
        input
            .zip3(k, stage, op_table_commit)
            .fsm_map(k, None, OpState::<U<PIPELINE>, Pow2<U<QUEUE_INDEX_WIDTH>>, OP_TABLE_SIZE, QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH>::new_expr(), |input, state| {
                let (input, stage, op_table_commit) = *input;

                let selector = input.selector;
                let input = input.command;

                let op_table_finish = Expr::<Valid<_>>::new(selector.commit, ().into());

                let selector = stage.selector;
                let stage = stage.command;

                let op_table = state.op_table;
                let queue_ram_read_data = state.queue_ram_read_data;

                // queue_ram_read_data_*
                let read_data = queue_ram_read_data[(PIPELINE - 1).into()];
                let head_ptr = read_data.clip_const::<U<16>>(0);
                let tail_ptr = read_data.clip_const::<U<16>>(16);
                let cpl_queue = read_data.clip_const::<U<16>>(32);
                let log_queue_size = read_data.clip_const::<U<4>>(48);
                let log_block_size = read_data.clip_const::<U<2>>(52);
                let active = read_data[55];
                let op_index = read_data.clip_const::<U<8>>(56);
                let base_addr = read_data.clip_const::<U<64>>(64);

                // queue_*
                let queue_op = OpTableEntryVarArr::get_entry(op_table, op_index.resize());

                let queue_active = queue_op.active & queue_op.queue.is_eq(stage.queue_ram_addr);
                let queue_empty_idle = head_ptr.is_eq(tail_ptr);
                // Corundum alternates between const `QUEUE_PTR_WIDTH` and magic number 16; needs manual resize()
                let queue_empty_active = head_ptr.is_eq(queue_op.ptr.resize());
                let queue_empty = queue_active.cond(queue_empty_active, queue_empty_idle);
                // Corundum alternates between const `QUEUE_PTR_WIDTH` and magic number 16; needs manual resize()
                let queue_ram_read_active_tail_ptr = queue_active.cond(queue_op.ptr, tail_ptr.resize());

                // TODO: Use `QUEUE_PTR_WIDTH` instead of 16.
                // Corundum alternates between const `QUEUE_PTR_WIDTH` and magic number 16; needs manual resize()
                let deq_res_addr_index1: Expr<Bits<U<QUEUE_PTR_WIDTH>>> = (Expr::from(true).repeat::<U<16>>() >> (Expr::from(QUEUE_PTR_WIDTH as u64).repr().resize::<U<4>>() - log_queue_size)).resize();
                // TODO: Use `ADDR_WIDTH` instead of 64.
                let deq_res_addr_index2: Expr<Bits<U<ADDR_WIDTH>>> = ((queue_ram_read_active_tail_ptr & deq_res_addr_index1).resize::<U<64>>() << (Expr::from(CL_DESC_SIZE as u8).repr() + log_block_size.resize()).resize::<U<6>>()).resize();
                let m_axis_resp = Expr::<Valid<_>>::new(
                    selector.req,
                    DeqResProj {
                        queue: stage.queue_ram_addr,
                        ptr: queue_ram_read_active_tail_ptr,
                        addr: (base_addr + deq_res_addr_index2).resize(),
                        block_size: log_block_size,
                        cpl: cpl_queue.resize(),
                        tag: stage.req_tag,
                        op_tag: state.op_table_start_ptr.resize(),
                        empty: active & queue_empty,
                        error: !active,
                    }
                    .into(),
                );

                let m_axis_doorbell = Expr::<Valid<_>>::new(
                    selector.write & stage.axil_reg.is_eq(4.into()) & active,
                    DoorbellProj { queue: stage.queue_ram_addr }.into(),
                );

                let s_axil_b = Expr::<Valid<_>>::new(selector.write, WRes::new_expr());

                let s_axil_r = Expr::<Valid<_>>::new(
                    selector.read,
                    RResProj {
                        resp: 0.into(),
                        data: select! {
                            stage.axil_reg.is_eq(0.into()) => base_addr.clip_const::<U<32>>(0), // base address lower 32
                            stage.axil_reg.is_eq(1.into()) => base_addr.clip_const::<U<32>>(32), // base address upper 32
                            stage.axil_reg.is_eq(2.into()) => log_queue_size.resize::<U<8>>().append(log_block_size).resize::<U<31>>().append(active.repeat::<U<1>>()).resize(),
                            stage.axil_reg.is_eq(3.into()) => cpl_queue.resize(),
                            stage.axil_reg.is_eq(4.into()) => head_ptr.resize(),
                            stage.axil_reg.is_eq(6.into()) => tail_ptr.resize(),
                            default => Expr::from(false).repeat(),
                        },
                    }
                    .into(),
                );

                let feedback = TempProj {
                    op_table_start_entry: OpTableEntryVarArr::get_entry(op_table, state.op_table_start_ptr),
                    op_table_finish_entry: OpTableEntryVarArr::get_entry(op_table, state.op_table_finish_ptr),
                    response: m_axis_resp,
                    output: m_axis_doorbell,
                    s_axil_b,
                    s_axil_r,
                };

                // Updates state

                // write_(data|strobe)_next
                let write_req = stage.write_req;

                // queue_ram_wr_en, queue_ram_write_*, queue_ram_be
                let queue_ram_wr_en = selector.read.cond(false.into(), Selector::is_active(selector));

                let queue_ram_write_ptr = stage.queue_ram_addr;

                let w_ram_write = select! {
                    // base address lower 32 (base address is read-only when queue is active)
                    stage.axil_reg.is_eq(0.into()) & !active => (
                        read_data.set_range(64.into(), write_req.data.resize::<U<32>>()).resize(),
                        Expr::from(false).repeat::<U<8>>().append(write_req.strb.clip_const::<U<4>>(0)).resize()
                    ).into(),
                    // base address upper 32 (base address is read-only when queue is active)
                    stage.axil_reg.is_eq(1.into()) & !active => (
                        read_data.clip_const::<U<96>>(0).append(write_req.data.resize::<U<32>>()).resize(),
                        Expr::from(false).repeat::<U<12>>().append(write_req.strb.clip_const::<U<4>>(0)).resize()
                    ).into(),
                    stage.axil_reg.is_eq(2.into()) => {
                        let data = read_data.clip_const::<U<8>>(48);

                        let log_queue_size = !active & write_req.strb[0];
                        let data = log_queue_size.cond(write_req.data.clip_const::<U<4>>(0).append(data.clip_const::<U<4>>(4)).resize(), data);

                        let log_desc_block_size = !active & write_req.strb[1];
                        let data = log_desc_block_size.cond(data.set_range(4.into(), write_req.data.clip_const::<U<2>>(8)), data);

                        let active = write_req.strb[3];
                        let data = active.cond(data.clip_const::<U<7>>(0).append(write_req.data.clip_const::<U<1>>(31)).resize(), data);

                        let be = log_queue_size | log_desc_block_size | active;

                        (
                            read_data.set_range(48.into(), data).resize(),
                            Expr::from(false).repeat::<U<6>>().append(be.repeat::<U<1>>()).resize()
                        ).into()
                    },
                    // completion queue index (completion queue index is read-only when queue is active)
                    stage.axil_reg.is_eq(3.into()) & !active => (
                        read_data.set_range(32.into(), write_req.data.clip_const::<U<16>>(0)).resize(),
                        Expr::from(false).repeat::<U<4>>().append(write_req.strb.clip_const::<U<2>>(0)).resize()
                    ).into(),
                    // head pointer
                    stage.axil_reg.is_eq(4.into()) => (
                        write_req.data.clip_const::<U<16>>(0).append(read_data.clip_const::<U<112>>(16)).resize(),
                        write_req.strb.clip_const::<U<2>>(0).resize()
                    ).into(),
                    // tail pointer (tail pointer is read-only when queue is active)
                    stage.axil_reg.is_eq(6.into()) & !active => (
                        read_data.set_range(16.into(), write_req.data.clip_const::<U<16>>(0)),
                        Expr::from(false).repeat::<U<2>>().append(write_req.strb.clip_const::<U<2>>(0)).resize()
                    ).into(),
                    default => (
                        Expr::x(),
                        Expr::from(false).repeat::<U<QUEUE_RAM_BE_WIDTH>>()
                    ).into(),
                };

                let (queue_ram_write_data, queue_ram_be) = *select! {
                    selector.req => (
                        read_data
                            .set_range(56.into(), state.op_table_start_ptr)
                            .set_range(61.into(), Expr::from(false).repeat::<U<3>>())
                            .resize(),
                        Expr::from(false).repeat::<U<7>>().append((active & !queue_empty).repeat::<U<1>>()).resize(),
                    ).into(),
                    selector.commit => (
                        read_data.set_range(16.into(), write_req.data.clip_const::<U<16>>(0)).resize(),
                        Expr::from([false, false, true, true]).resize(),
                    ).into(),
                    selector.write => (w_ram_write.0.resize(), w_ram_write.1).into(),
                    default => (read_data, Expr::from(false).repeat()).into(),
                };

                let op_table_start = Expr::<Valid<_>>::new(
                    selector.req & active & !queue_empty,
                    OpTableStartProj {
                        queue: stage.queue_ram_addr,
                        // line below is equal to: queue_ptr: (queue_ram_read_active_tail_ptr + 1.into()).resize(),
                        queue_ptr: (queue_ram_read_active_tail_ptr + Expr::from([true]).resize::<U<QUEUE_PTR_WIDTH>>()).resize(),
                    }
                    .into(),
                );

                let state = update_op_table(
                    state,
                    op_table_start,
                    op_table_finish,
                    op_table_commit.map_inner(|inner| inner.op_tag.resize())
                );

                let mut state = *state;

                // Rust doesn't know clog2(1 << QUEUE_INDEX_WIDTH) == QUEUE_INDEX_WIDTH... must manually specify.
                let row = state.queue_ram[queue_ram_write_ptr.resize()];
                // row: [bool; QUEUE_RAM_BE_WIDTH * 8], data: [bool; QUEUE_RAM_BE_WIDTH * 8], be: [bool; QUEUE_RAM_BE_WIDTH]
                // Rust doesn't know (N * 8) / 8 == N, so manually resize().
                let new_row = row.chunk::<U<8>>().zip(queue_ram_write_data.chunk::<U<8>>()).zip(queue_ram_be.resize()).map(|s| {
                    let (s, be) = *s;
                    let (o, n) = *s;
                    be.cond(n, o)
                }).concat();
                // Rust doesn't know clog2(1 << QUEUE_INDEX_WIDTH) == QUEUE_INDEX_WIDTH... must manually specify.
                // `row` is of length `8 * ((QUEUE_RAM_BE_WIDTH * 8) / 8)`, must manually resize() since Rust cannot infer the type
                state.queue_ram = state
                    .queue_ram
                    .set_var_arr(queue_ram_write_ptr.resize(), (!queue_ram_wr_en).cond(row, new_row.resize()));

                let new_queue_ram_read_data = (0..PIPELINE)
                    .fold(queue_ram_read_data, |acc, i| {
                        if i == 0 {
                            acc.set_var_arr(0.into(), state.queue_ram[input.queue_ram_addr.resize()])
                        } else {
                            acc.set_var_arr(i.into(), queue_ram_read_data[(i - 1).into()])
                        }
                    });

                state.queue_ram_read_data = new_queue_ram_read_data;

                (feedback.into(), state.into())
            })
    }
}

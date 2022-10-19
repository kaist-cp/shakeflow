//! Queue mananger.

use shakeflow::*;
use shakeflow_std::*;

use super::types::axil::*;
use super::types::queue_manager::*;

pub trait QueueManager<
    const PIPELINE: usize,
    const AXIL_ADDR_WIDTH: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
    const OP_TAG_WIDTH: usize,
    const QUEUE_PTR_WIDTH: usize,
>
{
    type Resp: Signal;
    type Out: Signal;

    fn calculate_feedback(
        k: &mut CompositeModuleContext,
        input: UniChannel<PipelineStage<Selector, Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>>,
        stage: UniChannel<PipelineStage<Selector, Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>>,
        op_table_commit: UniChannel<Valid<CommitReq<OP_TAG_WIDTH>>>,
    ) -> UniChannel<Temp<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH, Self::Resp, Self::Out>>;
}

pub fn m<
    const PIPELINE: usize,
    const AXIL_ADDR_WIDTH: usize,
    const QUEUE_INDEX_WIDTH: usize,
    const REQ_TAG_WIDTH: usize,
    const OP_TAG_WIDTH: usize,
    const QUEUE_PTR_WIDTH: usize,
    M: QueueManager<PIPELINE, AXIL_ADDR_WIDTH, QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH, OP_TAG_WIDTH, QUEUE_PTR_WIDTH> + 'static,
>(
    module_name: &str,
) -> Module<ManagerInput<AXIL_ADDR_WIDTH, QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH, OP_TAG_WIDTH>, ManagerOutput<M::Resp, M::Out>>
{
    composite::<
        (
            ManagerInput<AXIL_ADDR_WIDTH, QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH, OP_TAG_WIDTH>,
            UniChannel<Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>,
        ),
        (ManagerOutput<M::Resp, M::Out>, UniChannel<Cmd<QUEUE_INDEX_WIDTH, REQ_TAG_WIDTH>>),
        _,
    >(module_name, None, None, |(input, commit), k| {
        // Muxes writes, reads, and dequeue requests considering the feedback.
        //
        // NOTE: the below backward register slices and the pipeline stage 0 may have duplicated
        // information, e.g., if reads's backward register slice's ready buffer is asserted,
        // then the first pipeline item should be a read. Maybe we can later optimize this by
        // reusing backward register slice buffer as hazard expr...

        // AXIL write
        let write = input.s_axil_aw.zip_vr(k, input.s_axil_w).register_slice_bwd(k).map(k, |input| {
            let (s_axil_aw, s_axil_w) = *input;
            let s_axil_awaddr_queue = s_axil_aw.addr.clip_const::<U<QUEUE_INDEX_WIDTH>>(5);
            let s_axil_awaddr_reg = s_axil_aw.addr.clip_const::<U<3>>(2);
            Cmd::new_expr(s_axil_awaddr_queue, s_axil_awaddr_reg, s_axil_w, Expr::x())
        });

        // AXIL read
        let read = input.s_axil_ar.register_slice_bwd(k).map(k, |input| {
            let s_axil_araddr_queue = input.addr.clip_const::<U<QUEUE_INDEX_WIDTH>>(5);
            let s_axil_araddr_reg = input.addr.clip_const::<U<3>>(2);
            Cmd::new_expr(s_axil_araddr_queue, s_axil_araddr_reg, WReq::new_expr(), Expr::x())
        });

        // Dequeue commit finalize (update pointer)
        let commit = commit.map(k, |input| Expr::valid(input)).into_vr(k);

        // Dequeue request
        let request = input.request.zip_uni(k, input.enable.clone()).register_slice_bwd(k).assert_map(k, |input| {
            let (request, enable) = *input;

            let output =
                Cmd::new_expr(request.queue.clip_const::<U<QUEUE_INDEX_WIDTH>>(0), Expr::x(), Expr::x(), request.tag);
            let cond = enable;

            (cond, output).into()
        });

        // Push into command queue.
        let ((pipeline_input, pipeline_stage), extra_hazard) =
            [write, read, commit, request].command_queue::<PIPELINE>(k, [true, true, false, true]);

        let pipeline_input = pipeline_input
            .map(k, |input| PipelineStageProj { selector: Selector::from_bits(input.1), command: input.0 }.into());

        let pipeline_stage = pipeline_stage
            .map(k, |input| PipelineStageProj { selector: Selector::from_bits(input.1), command: input.0 }.into());

        // Calculates op_table_commit.
        let enable_buffer = input.enable.buffer(k, false.into());
        let op_table_commit = input
            .commit
            .zip_uni(k, enable_buffer)
            .assert_map(k, |i| {
                let (inner, ready) = *i;
                (ready, inner).into()
            })
            .into_uni(k, true);

        // Calculates the feedback.
        let feedback = M::calculate_feedback(k, pipeline_input, pipeline_stage, op_table_commit);

        // Calculates the final output.
        let m_axis_resp = feedback.clone().map(k, |input| input.response);
        let (m_axis_resp_vr, m_axis_resp_remaining) = m_axis_resp.into_vr(k).register_slice_fwd(k);

        let output = feedback.clone().map(k, |input| input.output).buffer(k, Expr::invalid());

        let s_axil_b = feedback.clone().map(k, |input| input.s_axil_b);
        let (s_axil_b_vr, s_axil_b_remaining) = s_axil_b.into_vr(k).register_slice_fwd(k);

        let s_axil_r = feedback.clone().map(k, |input| input.s_axil_r);
        let (s_axil_r_vr, s_axil_r_remaining) = s_axil_r.into_vr(k).register_slice_fwd(k);

        let output = ManagerOutput { response: m_axis_resp_vr, output, s_axil_b: s_axil_b_vr, s_axil_r: s_axil_r_vr };

        feedback
            .clone()
            .zip4(k, m_axis_resp_remaining, s_axil_b_remaining, s_axil_r_remaining)
            .map(k, |input| {
                let (feedback, m_axis_resp_remaining, s_axil_b_remaining, s_axil_r_remaining) = *input;
                let finish_entry = feedback.op_table_finish_entry;
                let start_entry = feedback.op_table_start_entry;
                [
                    !s_axil_b_remaining,
                    !s_axil_r_remaining,
                    finish_entry.active & finish_entry.commit,
                    !m_axis_resp_remaining & !start_entry.active,
                ]
                .into()
            })
            .comb_inline(k, extra_hazard);

        let commit = feedback.map(k, |input| {
            let entry = input.op_table_finish_entry;
            Cmd::new_expr(
                entry.queue,
                Expr::x(),
                WReqProj { data: entry.ptr.resize(), strb: Expr::x() }.into(),
                Expr::x(),
            )
        });

        (output, commit)
    })
    .loop_feedback()
    .build()
}

use shakeflow::*;
use shakeflow_std::*;

// TODO: Support `in_channel_count_mask`, `out_channel_count_mask`
const WIDTH_P: usize = 10;
const NUM_IN_P: usize = 5;
const NUM_OUT_P: usize = 5;

const MIDDLE_MEET_LP: usize = if NUM_IN_P > NUM_OUT_P { NUM_OUT_P } else { NUM_IN_P };

pub type IC = [VrChannel<Bits<U<WIDTH_P>>>; NUM_IN_P];
pub type OC = [VrChannel<Bits<U<WIDTH_P>>>; NUM_OUT_P];

#[derive(Debug, Clone, Signal)]
pub struct BrrF2FO {
    go_channels: Bits<U<MIDDLE_MEET_LP>>,
    go_cnt: Bits<U<MIDDLE_MEET_LP>>,
}

pub type BrrF2FOC = UniChannel<BrrF2FO>;

#[derive(Debug, Interface)]
pub struct FeedbackC {
    ready: [UniChannel<bool>; NUM_OUT_P],
    go_channels: UniChannel<Bits<U<MIDDLE_MEET_LP>>>,
    go_cnt: UniChannel<Bits<U<MIDDLE_MEET_LP>>>,
}

impl_custom_inst! {(IC, UniChannel<Bits<U<MIDDLE_MEET_LP>>>, UniChannel<Bits<U<MIDDLE_MEET_LP>>>), [UniChannel<Valid<Bits<U<WIDTH_P>>>>; MIDDLE_MEET_LP], bsg_rr_f2f_input, <width_p, num_in_p, middle_meet_p>, true,}
impl_custom_inst! {([UniChannel<bool>; NUM_OUT_P], UniChannel<Bits<U<MIDDLE_MEET_LP>>>, UniChannel<Bits<U<MIDDLE_MEET_LP>>>), [VrChannel<Bits<U<WIDTH_P>>>; NUM_OUT_P], bsg_rr_f2f_output, <width_p, num_out_p, middle_meet_p>, true,}
impl_custom_inst! {[VrChannel<Bits<U<WIDTH_P>>>;NUM_OUT_P], BrrF2FOC, bsg_rr_f2f_middle, <width_p, middle_meet_p>, false,}

pub fn m() -> Module<IC, OC> {
    composite::<(IC, FeedbackC), (OC, FeedbackC), _>(
        "bsg_round_robin_fifo_to_fifo",
        Some("i"),
        Some("o"),
        |(input, feedback), k| {
            let FeedbackC { ready, go_channels, go_cnt } = feedback;

            let head_fwd = (input, go_channels.clone(), go_cnt.clone())
                .bsg_rr_f2f_input::<WIDTH_P, NUM_IN_P, MIDDLE_MEET_LP>(k, "bsg_rr_ff_in", Some("i"), Some("o"));

            let ready = ready.concat(k);

            let head_bwd = (ready.clone().slice(k), go_channels, go_cnt)
                .bsg_rr_f2f_output::<WIDTH_P, NUM_OUT_P, MIDDLE_MEET_LP>(k, "bsg_rr_ff_out", Some("i"), Some("o"));

            let head_fwd = head_fwd.concat(k);

            let brrf2fm_o = head_fwd
                .clone()
                .slice(k)
                .array_zip(ready.slice(k))
                .array_map(k, "brrf2fm_i", |(fwd, bwd), k| fwd.into_vr(k).filter_bwd(k, bwd))
                .bsg_rr_f2f_middle::<WIDTH_P, MIDDLE_MEET_LP>(k, "brrf2fm", Some("i"), Some("o"));

            let output = head_bwd
                .array_zip(head_fwd.slice(k))
                .array_map(k, "final", |(bwd, fwd), k| bwd.zip_uni(k, fwd).map_fwd(k, None, |input| input.inner.1));

            let (output, ready) = output.array_map(k, "get_ready", |ch, k| ch.fire(k)).unzip();

            let feedback = FeedbackC {
                ready,
                go_channels: brrf2fm_o.clone().map(k, |input| input.go_channels),
                go_cnt: brrf2fm_o.map(k, |input| input.go_cnt),
            };

            (output, feedback)
        },
    )
    .loop_feedback()
    .build()
}

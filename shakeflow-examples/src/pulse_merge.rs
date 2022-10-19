use shakeflow::num::*;
use shakeflow::*;
use shakeflow_std::*;

pub const INPUT_WIDTH: usize = 2;
pub const COUNT_WIDTH: usize = 4;

#[derive(Debug, Clone, Signal)]
pub struct O {
    count: Bits<U<COUNT_WIDTH>>,
    pulse: bool,
}

pub type IC = UniChannel<Bits<U<INPUT_WIDTH>>>;
pub type OC = UniChannel<O>;

pub fn pulse_merge() -> Module<IC, OC> {
    composite::<IC, OC, _>("pulse_merge", Some("pulse_in"), None, |input, k| {
        input.fsm_map::<O, _, _>(k, None, OProj { count: 0.into(), pulse: false.into() }.into(), |input, state| {
            let count = state.count;
            let pulse = state.pulse;
            let pulse_next = count.is_gt(0.into());
            let count_next = (pulse_next.cond(count - 1.into(), count)
                + input.map(|x| x.repr().resize::<U<COUNT_WIDTH>>()).tree_fold(|l, r| (l + r).resize()))
            .resize();

            let output = OProj { count, pulse }.into();
            let state_next = OProj { count: count_next, pulse: pulse_next }.into();
            (output, state_next)
        })
    })
    .build()
}

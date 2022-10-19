use shakeflow::*;
use shakeflow_std::*;

type C<Width: Num> = UniChannel<Valid<Bits<Width>>>;

pub fn m<Width: Num, const STAGES: usize>() -> Module<C<Width>, C<Width>> {
    composite::<C<Width>, C<Width>, _>("bsg_shift_reg", Some("i"), Some("o"), |input, k| {
        input.fsm_map::<Array<Valid<Bits<Width>>, U<STAGES>>, _, _>(k, None, Expr::x(), |input, state| {
            let state_next = input.repeat::<U<1>>().append(state.clip_const::<Diff<U<STAGES>, U<1>>>(0)).resize();
            (state[STAGES - 1], state_next)
        })
    })
    .build()
}

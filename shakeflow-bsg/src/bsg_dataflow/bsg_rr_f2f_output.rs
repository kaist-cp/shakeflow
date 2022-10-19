use shakeflow::*;
use shakeflow_std::*;

const WIDTH_P: usize = 10;
const NUM_OUT_P: usize = 5;
const MIDDLE_MEET_P: usize = 5;
const MIN_OUT_MIDDLE_MEET_P: usize = MIDDLE_MEET_P;

#[derive(Debug, Interface)]
pub struct IC {
    data_head: [VrChannel<Bits<U<WIDTH_P>>>; MIN_OUT_MIDDLE_MEET_P],
    go_channels: UniChannel<Bits<U<MIN_OUT_MIDDLE_MEET_P>>>,
    go_cnt: UniChannel<Bits<Log2<Sum<U<MIN_OUT_MIDDLE_MEET_P>, U<1>>>>>,
}

pub type OC = [VrChannel<Bits<U<WIDTH_P>>>; NUM_OUT_P];

impl_custom_inst! {UniChannel<Bits<Log2<Sum<U<MIN_OUT_MIDDLE_MEET_P>, U<1>>>>>, UniChannel<Bits<Log2<U<NUM_OUT_P>>>>, bsg_circular_ptr, <slots_p, max_add_p>, true,}
impl_custom_inst! {([VrChannel<Bits<U<WIDTH_P>>>; MIN_OUT_MIDDLE_MEET_P], UniChannel<Bits<Log2<U<NUM_OUT_P>>>>), [VrChannel<Bits<U<WIDTH_P>>>; MIDDLE_MEET_P], bsg_rotate_right, <width_p>, false,}

pub fn m() -> Module<IC, OC> {
    composite::<IC, OC, _>("bsg_rr_f2f_output", Some("i"), Some("o"), |input, k| {
        let IC { data_head, go_channels, go_cnt } = input;

        let optr_r = go_cnt.bsg_circular_ptr::<NUM_OUT_P, MIN_OUT_MIDDLE_MEET_P>(k, "c_ptr", Some("add_i"), Some("o"));

        (data_head, optr_r)
            .bsg_rotate_right::<WIDTH_P>(k, "ready_rr", Some("i"), Some("o"))
            .array_zip(go_channels.slice(k))
            .array_map(k, "output", |(ch, go_channel), k| {
                ch.zip_uni(k, go_channel).and_then(k, None, |input| Expr::<Valid<_>>::new(input.1, input.0)).buffer(k)
            })
    })
    .build()
}

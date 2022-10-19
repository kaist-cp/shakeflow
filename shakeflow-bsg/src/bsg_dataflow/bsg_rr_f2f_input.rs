use shakeflow::*;
use shakeflow_std::*;

const WIDTH_P: usize = 10;
const NUM_IN_P: usize = 5;
const MIDDLE_MEET_P: usize = 3;
// const MIDDLE_MEET_DATA_LP: usize = MIDDLE_MEET_P * WIDTH_P;
const MIN_IN_MIDDLE_MEET_P: usize = MIDDLE_MEET_P;

#[derive(Debug, Interface)]
pub struct IC {
    data: [VrChannel<Bits<U<WIDTH_P>>>; NUM_IN_P],
    go_channels: UniChannel<Bits<U<MIN_IN_MIDDLE_MEET_P>>>,
    go_cnt: UniChannel<Bits<Log2<Sum<U<MIN_IN_MIDDLE_MEET_P>, U<1>>>>>,
}

pub type OC = [UniChannel<Valid<Bits<U<WIDTH_P>>>>; MIDDLE_MEET_P];

impl_custom_inst! {UniChannel<Bits<Log2<Sum<U<MIN_IN_MIDDLE_MEET_P>, U<1>>>>>, UniChannel<Bits<Log2<U<NUM_IN_P>>>>, bsg_circular_ptr, <slots_p, max_add_p>, true,}
impl_custom_inst! {([VrChannel<Bits<U<WIDTH_P>>>; NUM_IN_P], UniChannel<Bits<Log2<U<NUM_IN_P>>>>), [VrChannel<Bits<U<WIDTH_P>>>; MIDDLE_MEET_P], bsg_rotate_right, <width_p>, false, }

pub fn m() -> Module<IC, OC> {
    composite::<IC, OC, _>("bsg_rr_f2f_input", Some("i"), Some("o"), |input, k| {
        let IC { data, go_channels, go_cnt } = input;

        let iptr_r = go_cnt.bsg_circular_ptr::<NUM_IN_P, MIN_IN_MIDDLE_MEET_P>(k, "c_ptr", Some("add_i"), Some("o"));

        (data, iptr_r)
            .bsg_rotate_right::<NUM_IN_P>(k, "valid_rr", Some("i"), Some("o"))
            .array_zip(go_channels.slice(k))
            .array_map(k, "output", |(ch, go_channel), k| ch.filter_bwd(k, go_channel).into_uni(k, true))
    })
    .build()
}

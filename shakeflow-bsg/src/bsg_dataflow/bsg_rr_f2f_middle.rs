use shakeflow::*;
use shakeflow_std::*;

// const WIDTH_P: usize = 10;
const MIDDLE_MEET_P: usize = 1;
const USE_POPCOUNT_P: bool = false;

#[derive(Debug, Clone, Signal)]
pub struct I {
    valid_head: Bits<U<MIDDLE_MEET_P>>,
    ready_head: Bits<U<MIDDLE_MEET_P>>,
}

#[derive(Debug, Clone, Signal)]
pub struct O {
    go_channels: Bits<U<MIDDLE_MEET_P>>,
    go_cnt: Bits<Log2<Sum<U<MIDDLE_MEET_P>, U<1>>>>,
}

pub type IC = UniChannel<I>;
pub type OC = UniChannel<O>;

impl_custom_inst! {UniChannel<Bits<U<MIDDLE_MEET_P>>>, UniChannel<Bits<U<MIDDLE_MEET_P>>>, bsg_scan, <width_p, and_p, lo_to_hi_p>, false,}
impl_custom_inst! {UniChannel<Bits<U<MIDDLE_MEET_P>>>, UniChannel<Bits<Log2<Sum<U<MIDDLE_MEET_P>, U<1>>>>>, bsg_popcount, <width_p>, false,}
impl_custom_inst! {UniChannel<Bits<U<MIDDLE_MEET_P>>>, UniChannel<Bits<Log2<Sum<U<MIDDLE_MEET_P>, U<1>>>>>, bsg_thermometer_count, <width_p>, false,}

pub fn m() -> Module<IC, OC> {
    composite::<IC, OC, _>("bsg_rr_f2f_middle", Some("i"), Some("o"), |input, k| {
        let happy_channels = input.map(k, |input| input.valid_head & input.ready_head);

        let go_channels_int = happy_channels.bsg_scan::<MIDDLE_MEET_P, 1, 1>(k, "and_scan", Some("i"), Some("o"));

        let go_cnt_o = if USE_POPCOUNT_P {
            go_channels_int.clone().bsg_popcount::<MIDDLE_MEET_P>(k, "pop", Some("i"), Some("o"))
        } else {
            go_channels_int.clone().bsg_thermometer_count::<MIDDLE_MEET_P>(k, "thermo", Some("i"), Some("o"))
        };

        go_channels_int.zip(k, go_cnt_o).map(k, |input| OProj { go_channels: input.0, go_cnt: input.1 }.into())
    })
    .build()
}

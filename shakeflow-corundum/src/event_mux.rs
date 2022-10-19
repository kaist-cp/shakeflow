use shakeflow::*;
use shakeflow_std::*;

use super::constants::event_mux::*;

#[derive(Debug, Clone, Signal)]
pub struct Event {
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    #[member(name = "type")]
    typ: Bits<U<EVENT_TYPE_WIDTH>>,
    source: Bits<U<EVENT_SOURCE_WIDTH>>,
}

type EventChannel = VrChannel<Event>;

pub fn m() -> Module<[EventChannel; PORTS], EventChannel> {
    composite::<[EventChannel; PORTS], _, _>("event_mux", Some("s_axis_event"), Some("m_axis_event"), |value, k| {
        // Parameters defined by their values used at `test_fpga_core`
        value.arb_mux(k, 1, 1).map(k, |input| input.inner)
    })
    .build()
}

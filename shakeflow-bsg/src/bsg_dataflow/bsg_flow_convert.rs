use shakeflow::*;
use shakeflow_std::*;

// TODO: Use const generic
#[allow(dead_code)]
const SEND_V_AND_READY_P: bool = false;
const SEND_V_THEN_YUMI_P: bool = false;
const SEND_READY_THEN_V_P: bool = false;
const SEND_RETRY_THEN_V_P: bool = false;
const SEND_V_AND_RETRY_P: bool = false;
const RECV_V_AND_READY_P: bool = false;
const RECV_V_THEN_YUMI_P: bool = false;
const RECV_READY_THEN_V_P: bool = false;
const RECV_V_AND_RETRY_P: bool = false;
const RECV_V_THEN_RETRY_P: bool = false;
const WIDTH_P: usize = 1;

#[derive(Debug, Clone, Signal)]
pub struct I {
    v: Bits<U<WIDTH_P>>,
    fc: Bits<U<WIDTH_P>>,
}

pub fn m() -> Module<UniChannel<I>, UniChannel<I>> {
    composite::<UniChannel<I>, UniChannel<I>, _>("bsg_flow_convert", Some("i"), Some("o"), |input, k| {
        input.fsm_map::<(), _, _>(k, None, ().into(), |input, state| {
            let (fc_i, v_i) = (input.fc, input.v);
            let fc_o = if (SEND_V_THEN_YUMI_P & RECV_V_AND_READY_P) | (SEND_V_THEN_YUMI_P & RECV_READY_THEN_V_P) {
                fc_i & v_i
            } else if SEND_V_THEN_YUMI_P & RECV_V_AND_RETRY_P {
                !fc_i & v_i
            } else if SEND_READY_THEN_V_P & RECV_V_THEN_YUMI_P {
                panic!("A unhandled case requires fifo")
            } else if SEND_READY_THEN_V_P & RECV_V_THEN_RETRY_P {
                panic!("unhandled case requires fifo")
            } else if SEND_RETRY_THEN_V_P & RECV_V_THEN_YUMI_P {
                panic!("unhandled case require fifo")
            } else if (SEND_RETRY_THEN_V_P | SEND_V_AND_RETRY_P) ^ (RECV_V_THEN_RETRY_P | RECV_V_AND_RETRY_P) {
                !fc_i
            } else {
                fc_i
            };

            let v_o = if RECV_READY_THEN_V_P & !SEND_READY_THEN_V_P { v_i & fc_i } else { v_i };

            (IProj { fc: fc_o, v: v_o }.into(), state)
        })
    })
    .build()
}

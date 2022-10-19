use shakeflow::*;
use shakeflow_std::*;

use super::types::qsfp28::*;

pub fn m() -> Module<Qsfp28Channel, Qsfp28Channel> {
    const LEN: usize = 60;

    composite::<Qsfp28Channel, Qsfp28Channel, _>("cmac_pad", Some("s_axis"), Some("m_axis"), |input, k| {
        input
            .into_vr(k)
            .fsm_map::<bool, _, _>(k, None, false.into(), |value, is_frame| {
                let data = value.payload.data;
                let payload = value.payload.set_data(
                    data.set_tdata(data.tdata & data.tkeep.map(|b| b.repeat::<U<8>>()).concat().resize())
                        .set_tkeep(data.tkeep | (!is_frame).repeat::<U<LEN>>().resize::<U<QSFP28_KEEP_WIDTH>>()),
                );
                (value.set_payload(payload), !value.tlast)
            })
            .into_axis_vr(k)
    })
    .build()
}

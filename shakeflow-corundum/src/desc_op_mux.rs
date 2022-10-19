use std::fmt::Debug;

use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::*;

use super::constants::desc_op_mux::*;
use super::types::request::*;

#[derive(Debug, Clone, Signal)]
pub struct DescReq<const REQ_TAG_WIDTH: usize> {
    sel: Bits<U<SELECT_WIDTH>>,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    tag: Bits<U<REQ_TAG_WIDTH>>,
}

#[derive(Debug, Clone, Signal)]
pub struct Desc<const REQ_TAG_WIDTH: usize, const AXIS_DATA_WIDTH: usize, const AXIS_KEEP_WIDTH: usize> {
    #[member(name = "")]
    data: Keep<U<AXIS_DATA_WIDTH>, U<AXIS_KEEP_WIDTH>>,
    tid: Bits<U<REQ_TAG_WIDTH>>,
    tuser: bool,
}

#[derive(Debug, Interface)]
pub struct I {
    desc: AxisChannel<Desc<M_REQ_TAG_WIDTH, AXIS_DATA_WIDTH, AXIS_KEEP_WIDTH>>,
    req: [VrChannel<DescReq<S_REQ_TAG_WIDTH>>; PORTS],
    req_status:
        UniChannel<Valid<DescReqStatus<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH, CPL_QUEUE_INDEX_WIDTH, M_REQ_TAG_WIDTH>>>,
}

#[derive(Debug, Interface)]
pub struct O {
    desc: [AxisChannel<Desc<S_REQ_TAG_WIDTH, AXIS_DATA_WIDTH, AXIS_KEEP_WIDTH>>; PORTS],
    req: VrChannel<DescReq<M_REQ_TAG_WIDTH>>,
    req_status: [UniChannel<
        Valid<DescReqStatus<QUEUE_INDEX_WIDTH, QUEUE_PTR_WIDTH, CPL_QUEUE_INDEX_WIDTH, S_REQ_TAG_WIDTH>>,
    >; PORTS],
}

pub fn m() -> Module<I, O> {
    composite::<I, _, _>("desc_op_mux", Some("s_axis"), Some("m_axis"), |value, k| {
        let m_req = value.req.arb_mux(k, ARB_TYPE_ROUND_ROBIN, ARB_LSB_HIGH_PRIORITY).map(k, |m_req| {
            DescReqProj { tag: m_req.inner.tag.append(m_req.grant_encoded).resize(), ..*m_req.inner }.into()
        });

        let m_req_status = value
            .req_status
            .fsm_map(
                k,
                Some("req_status_demux"),
                Expr::<Valid<_>>::invalid().repeat::<U<PORTS>>(),
                |s_req_status, state| {
                    let s_req_status_valid = s_req_status.valid;
                    let s_req_status = s_req_status.inner;

                    let index = if PORTS > 1 { s_req_status.tag >> S_REQ_TAG_WIDTH } else { 0.into() }.resize();
                    let m_req_status = DescReqStatusProj::<
                        QUEUE_INDEX_WIDTH,
                        QUEUE_PTR_WIDTH,
                        CPL_QUEUE_INDEX_WIDTH,
                        S_REQ_TAG_WIDTH,
                    > {
                        tag: s_req_status.tag.resize(),
                        ..*s_req_status
                    };
                    let m_req_status = Expr::<Valid<_>>::invalid()
                        .repeat::<U<PORTS>>()
                        .set(index, Expr::<Valid<_>>::new(s_req_status_valid, m_req_status.into()));
                    (state, m_req_status)
                },
            )
            .slice(k);

        let m_desc = value
            .desc
            .into_vr(k)
            .filter_fwd_ready(k)
            .map(k, |value| {
                let tid = value.payload.tid;
                let index = if PORTS > 1 { tid >> S_REQ_TAG_WIDTH } else { 0.into() };
                let payload = value.payload;
                let new_payload = DescProj { tid: payload.tid.resize::<U<S_REQ_TAG_WIDTH>>(), ..*payload }.into();
                let new_value = AxisValueProj { payload: new_payload, tlast: value.tlast }.into();
                (new_value, index).into()
            })
            .buffer_skid(k)
            .filter_bwd_valid(k)
            .into_axis_vr(k)
            .demux::<PORTS>(k);

        O { desc: m_desc, req: m_req, req_status: m_req_status }
    })
    .build()
}

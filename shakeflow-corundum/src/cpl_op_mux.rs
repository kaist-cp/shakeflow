use std::fmt::Debug;

use shakeflow::*;
use shakeflow_std::*;

use super::constants::cpl_op_mux::*;
use super::types::request::*;

#[derive(Debug, Clone, Signal)]
pub struct CplReq<const SELECT_WIDTH: usize, ReqTagWidth: Num> {
    sel: Bits<U<SELECT_WIDTH>>,
    queue: Bits<U<QUEUE_INDEX_WIDTH>>,
    tag: Bits<ReqTagWidth>,
    data: Bits<U<{ CPL_SIZE * 8 }>>,
}

#[derive(Debug, Interface)]
pub struct I<const PORTS: usize, const SELECT_WIDTH: usize, const S_REQ_TAG_WIDTH: usize> {
    req: [VrChannel<CplReq<SELECT_WIDTH, U<S_REQ_TAG_WIDTH>>>; PORTS],
    req_status: UniChannel<Valid<CplReqStatus<Sum<U<S_REQ_TAG_WIDTH>, Log2<U<PORTS>>>>>>,
}

#[derive(Debug, Interface)]
pub struct O<const PORTS: usize, const SELECT_WIDTH: usize, const S_REQ_TAG_WIDTH: usize> {
    req: VrChannel<CplReq<SELECT_WIDTH, Sum<U<S_REQ_TAG_WIDTH>, Log2<U<PORTS>>>>>,
    req_status: [UniChannel<Valid<CplReqStatus<U<S_REQ_TAG_WIDTH>>>>; PORTS],
}

pub fn m<const PORTS: usize, const SELECT_WIDTH: usize, const S_REQ_TAG_WIDTH: usize>(
    module_name: &str,
) -> Module<I<PORTS, SELECT_WIDTH, S_REQ_TAG_WIDTH>, O<PORTS, SELECT_WIDTH, S_REQ_TAG_WIDTH>> {
    composite::<I<PORTS, SELECT_WIDTH, S_REQ_TAG_WIDTH>, O<PORTS, SELECT_WIDTH, S_REQ_TAG_WIDTH>, _>(
        module_name,
        Some("s_axis"),
        Some("m_axis"),
        |value, k| {
            let m_req = value.req.arb_mux(k, ARB_TYPE_ROUND_ROBIN, ARB_LSB_HIGH_PRIORITY).map(k, |m_req| {
                CplReqProj::<SELECT_WIDTH, Sum<U<S_REQ_TAG_WIDTH>, Log2<U<PORTS>>>> {
                    tag: m_req.inner.tag.append(m_req.grant_encoded),
                    ..*m_req.inner
                }
                .into()
            });

            let m_req_status = value
                .req_status
                .fsm_map(k, Some("shift_valid"), Expr::invalid().repeat::<U<PORTS>>(), |s_req_status, state| {
                    let index = if PORTS > 1 { s_req_status.inner.tag >> S_REQ_TAG_WIDTH } else { 0.into() }
                        .resize::<Log2<U<PORTS>>>();
                    let m_req_status = CplReqStatusProj::<U<S_REQ_TAG_WIDTH>> {
                        tag: s_req_status.inner.tag.resize(),
                        ..*s_req_status.inner
                    };
                    let next_state = Expr::<Valid<_>>::invalid()
                        .repeat::<U<PORTS>>()
                        .set(index, Expr::<Valid<_>>::new(s_req_status.valid, m_req_status.into()));
                    (state, next_state)
                })
                .slice(k);

            O { req: m_req, req_status: m_req_status }
        },
    )
    .build()
}

use shakeflow::*;

use super::types::*;
use crate::*;

// #[derive(Debug, Clone, Signal)]
// enum DeleteState {
//     Free,
//     CalculateHash,
//     MemLookup,
//     MemUpdate,
//     Send,
// }
//
// #[derive(Debug, Clone, Signal)]
// struct State {
//     state: DeleteState,
//     response: HtUpdateResp,
//     hashes: [HashType; NUM_TABLES],
// }
//
// type StateTransitionResult<'id> =
//     (Expr<'id, bool>, Expr<'id, (Valid<MemAccessRequest>, Valid<CalculateHashInput>)>, Expr<'id, State>);
//
// fn free_to_hash_transition<'id>(
//     state: Expr<'id, State>, delete_req: Expr<'id, Valid<HtUpdateReq>>,
// ) -> StateTransitionResult<'id> {
//     let is_req_arrived = state.state.is_eq(DeleteState::Free.into()) & delete_req.valid;
//     let hash_req = CalculateHashInputProj { key: delete_req.inner.key }.into();
//     let resp = HtUpdateResp::new(delete_req.inner);
//     let state_next = state.set_state(DeleteState::CalculateHash.into()).set_response(resp);
//
//     (is_req_arrived, (Expr::invalid(), Expr::valid(hash_req)).into(), state_next)
// }
//
// fn hash_to_lookup_transition<'id>(
//     state: Expr<'id, State>, hash_resp: Expr<'id, Valid<CalculateHashOutput>>,
// ) -> StateTransitionResult<'id> {
//     let is_hash_arrived = state.state.is_eq(DeleteState::CalculateHash.into()) & hash_resp.valid;
//     let mem_req =
//         MemAccessRequestProj { op: MemAccessOp::Lookup.into(), entries: Expr::x(), hashes: hash_resp.inner.hashes }
//             .into();
//     let state_next = state.set_state(DeleteState::MemLookup.into());
//
//     (is_hash_arrived, (Expr::valid(mem_req), Expr::invalid()).into(), state_next)
// }
//
// fn lookup_to_update_transition<'id>(
//     state: Expr<'id, State>, mem_resp: Expr<'id, Valid<MemAccessResult>>,
// ) -> StateTransitionResult<'id> {
//     let is_lookup_arrived = state.state.is_eq(DeleteState::MemLookup.into())
//         & mem_resp.valid
//         & mem_resp.inner.op.is_eq(MemAccessOp::Lookup.into());
//
//     // TODO: Skip update request if success is false
//     let success = {
//         (0..NUM_TABLES).fold(false.into(), |acc, i| {
//             let entry = mem_resp.inner.entries[i];
//             let is_match = entry.valid & entry.key.is_eq(state.response.key);
//             acc | is_match
//         })
//     };
//     let entries = mem_resp.inner.entries.zip(state.response.key.repeat()).map(|values| {
//         let (entry, key) = *values;
//         let is_match = entry.valid & entry.key.is_eq(key);
//         HtEntryProj { key: entry.key, value: entry.value, valid: is_match.cond(false.into(), entry.valid) }.into()
//     });
//     let mem_req = MemAccessRequestProj { op: MemAccessOp::Update.into(), entries, hashes: state.hashes }.into();
//     let state_next = state.set_state(DeleteState::MemUpdate.into()).set_response(state.response.set_success(success));
//
//     (is_lookup_arrived, (Expr::valid(mem_req), Expr::invalid()).into(), state_next)
// }
//
// fn update_to_send_transition<'id>(
//     state: Expr<'id, State>, mem_resp: Expr<'id, Valid<MemAccessResult>>,
// ) -> StateTransitionResult<'id> {
//     let is_update_arrived = state.state.is_eq(DeleteState::MemUpdate.into())
//         & mem_resp.valid
//         & mem_resp.inner.op.is_eq(MemAccessOp::Update.into());
//
//     let state_next = state.set_state(DeleteState::Send.into());
//
//     (is_update_arrived, (Expr::invalid(), Expr::invalid()).into(), state_next)
// }
//
// fn send_to_free_transition<'id>(state: Expr<'id, State>, ready: Expr<'id, Ready>) -> StateTransitionResult<'id> {
//     let is_ready_arrived = state.state.is_eq(DeleteState::Send.into()) & ready.ready;
//     let state_next = state.set_state(DeleteState::Free.into());
//
//     (is_ready_arrived, (Expr::invalid(), Expr::invalid()).into(), state_next)
// }

// pub fn m() -> Module<DeleteIC, DeleteOC> {
//     composite::<DeleteIC, DeleteOC, _>(
//         "delete",
//         Some("in"),
//         Some("out"),
//         |((mem_resp_channel, hash_resp_channel), delete_req_channel), k| {
//             let state_free =
//                 StateProj { hashes: Expr::x(), state: DeleteState::Free.into(), response: Expr::x() }.into();
//
//             (mem_resp_channel, hash_resp_channel, delete_req_channel).fsm::<State, DeleteOC, _>(
//                 k,
//                 Some("delete"),
//                 state_free,
//                 move |input_fwd, output_bwd, state| {
//                     let (mem_resp, hash_resp, delete_req) = *input_fwd;
//
//                     let (is_free2hash, free2hash_req, free2hash_state) = free_to_hash_transition(state, delete_req);
//
//                     let (is_hash2lookup, hash2lookup_req, hash2lookup_state) =
//                         hash_to_lookup_transition(state, hash_resp);
//
//                     let (is_lookup2update, lookup2update_req, lookup2update_state) =
//                         lookup_to_update_transition(state, mem_resp);
//
//                     let (is_update2send, update2send_req, update2send_state) =
//                         update_to_send_transition(state, mem_resp);
//
//                     let (is_send2free, send2free_req, send2free_state) = send_to_free_transition(state, output_bwd.1);
//
//                     let state_next = select! {
//                         is_free2hash => free2hash_state,
//                         is_hash2lookup => hash2lookup_state,
//                         is_lookup2update => lookup2update_state,
//                         is_update2send => update2send_state,
//                         is_send2free => send2free_state,
//                         default => state,
//                     };
//
//                     let c_in = select! {
//                         is_free2hash => free2hash_req,
//                         is_hash2lookup => hash2lookup_req,
//                         is_lookup2update => lookup2update_req,
//                         is_update2send => update2send_req,
//                         is_send2free => send2free_req,
//                         default => (Expr::invalid(), Expr::invalid()).into(),
//                     };
//
//                     let out = Expr::<Valid<_>>::new(state_next.state.is_eq(DeleteState::Send.into()), state.response);
//
//                     let ready = Expr::<Ready>::new(is_send2free);
//                     ((c_in, out).into(), (().into(), ().into(), ready).into(), state_next)
//                 },
//             )
//         },
//     )
//     .build()
// }

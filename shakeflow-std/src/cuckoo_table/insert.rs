use shakeflow::*;

use super::types::*;
use crate::*;

// #[derive(Debug, Clone, Signal)]
// enum InsertState {
//     Free,
//     CalculateHash,
//     MemLookup,
//     MemUpdate,
//     Send,
// }
//
// #[derive(Debug, Clone, Signal)]
// struct State {
//     state: InsertState,
//     hashes: [HashType; NUM_TABLES],
//     response: HtUpdateResp,
//     current_entry: HtEntry,
//     current_trial: [bool; clog2(MAX_TRIALS)],
//     insert_failure_count: Bits<U<16>>,
//     victim_bit: Bits<U<1>>,
//     victim_idx: [bool; clog2(MAX_TRIALS)],
// }
//
// type StateTransitionResult<'id> =
//     (Expr<'id, bool>, Expr<'id, (Valid<MemAccessRequest>, Valid<CalculateHashInput>)>, Expr<'id, State>);
//
// fn free_to_hash_transition<'id>(
//     state: Expr<'id, State>, insert_req: Expr<'id, Valid<HtUpdateReq>>,
// ) -> StateTransitionResult<'id> {
//     let is_requst_arrived = state.state.is_eq(InsertState::Free.into()) & insert_req.valid;
//     let hash_req = CalculateHashInputProj { key: insert_req.inner.key }.into();
//     let current_entry =
//         HtEntryProj { key: insert_req.inner.key, value: insert_req.inner.value, valid: true.into() }.into();
//     let state_next = state
//         .set_state(InsertState::CalculateHash.into())
//         .set_current_entry(current_entry)
//         .set_victim_idx(0.into())
//         .set_current_trial(0.into())
//         .set_response(HtUpdateResp::new(insert_req.inner));
//
//     (is_requst_arrived, (Expr::invalid(), Expr::valid(hash_req)).into(), state_next)
// }
//
// fn hash_to_lookup_transition<'id>(
//     state: Expr<'id, State>, hash_resp: Expr<'id, Valid<CalculateHashOutput>>,
// ) -> StateTransitionResult<'id> {
//     let is_hash_arrived = (state.state.is_eq(InsertState::CalculateHash.into())) & hash_resp.valid;
//     let mem_read_req =
//         MemAccessRequestProj { op: MemAccessOp::Lookup.into(), entries: Expr::x(), hashes: hash_resp.inner.hashes }
//             .into();
//     let state_next = state.set_state(InsertState::MemLookup.into()).set_hashes(hash_resp.inner.hashes);
//
//     (is_hash_arrived, (Expr::valid(mem_read_req), Expr::invalid()).into(), state_next)
// }
//
// fn lookup_to_update_transition<'id>(
//     state: Expr<'id, State>, mem_resp: Expr<'id, Valid<MemAccessResult>>,
// ) -> StateTransitionResult<'id> {
//     let is_lookup_arrived = state.state.is_eq(InsertState::MemLookup.into())
//         & mem_resp.valid
//         & mem_resp.inner.op.is_eq(MemAccessOp::Lookup.into());
//     let entries: Expr<[HtEntry; NUM_TABLES]> = mem_resp.inner.entries;
//     // 1. check if there is free slot
//     let (is_free_slot_found, slot) =
//         (0..NUM_TABLES).fold((Expr::<bool>::from(false), Expr::<Bits<U<4>>>::x()), |(valid, slot), i| {
//             let entry = entries[i];
//             let is_entry_free = !entry.valid;
//             (valid | is_entry_free, is_entry_free.cond(i.into(), slot))
//         });
//     // 2-1. when there is free slot, update write back entries by inserting to the
//     //    slot
//     let entries_success = entries.set(slot, state.current_entry);
//     // 2-2. when there is no free slot, update write back entries by eviction
//     let (victim_entry, entries_evict) = {
//         let victim_pos: Expr<Bits<U<4>>> = ((state.hashes[state.victim_idx] % Expr::<Bits<U<4>>>::from(NUM_TABLES - 1))
//             + state.victim_bit.resize())
//         .resize();
//
//         let victim_entry = entries[victim_pos];
//         (victim_entry, entries.set(slot, state.current_entry))
//     };
//
//     let write_back_entries = is_free_slot_found.cond(entries_success, entries_evict);
//
//     let mem_update_req =
//         MemAccessRequestProj { op: MemAccessOp::Update.into(), entries: write_back_entries, hashes: state.hashes }
//             .into();
//
//     let current_entry_next = is_free_slot_found.cond(Expr::x(), victim_entry);
//     let victim_bit_next = is_free_slot_found.cond(state.victim_bit, !state.victim_bit);
//     let victim_idx = select! {
//         is_free_slot_found => state.victim_idx,
//         !is_free_slot_found & state.victim_idx.is_eq(MAX_TRIALS.into()) => 0.into(),
//         !is_free_slot_found & state.victim_idx.is_lt(MAX_TRIALS.into()) => (state.victim_idx + 1.into()).resize(),
//         default => state.victim_idx,
//     };
//
//     let state_next = state
//         .set_state(InsertState::MemUpdate.into())
//         .set_current_entry(current_entry_next)
//         .set_current_trial((state.current_trial + 1.into()).resize())
//         .set_victim_bit(victim_bit_next)
//         .set_victim_idx(victim_idx)
//         .set_response(state.response.set_success(is_free_slot_found));
//
//     (is_lookup_arrived, (Expr::valid(mem_update_req), Expr::invalid()).into(), state_next)
// }
//
// fn update_to_hash_transition<'id>(
//     state: Expr<'id, State>, mem_resp: Expr<'id, Valid<MemAccessResult>>,
// ) -> StateTransitionResult<'id> {
//     let is_update2hash = state.state.is_eq(InsertState::MemUpdate.into())
//         & mem_resp.valid
//         & mem_resp.inner.op.is_eq(MemAccessOp::Update.into())
//         & !state.response.success
//         & state.current_trial.is_lt(MAX_TRIALS.into());
//
//     let hash_req = CalculateHashInputProj { key: state.current_entry.key }.into();
//
//     let state_next = state.set_state(InsertState::CalculateHash.into());
//
//     (is_update2hash, (Expr::invalid(), Expr::valid(hash_req)).into(), state_next)
// }
// fn update_to_send_transition<'id>(
//     state: Expr<'id, State>, mem_resp: Expr<'id, Valid<MemAccessResult>>,
// ) -> StateTransitionResult<'id> {
//     let is_update2free: Expr<bool> = state.state.is_eq(InsertState::MemUpdate.into())
//         & mem_resp.valid
//         & mem_resp.inner.op.is_eq(MemAccessOp::Update.into())
//         & (state.response.success | !state.current_trial.is_lt(MAX_TRIALS.into()));
//
//     let insert_failure_next =
//         state.response.success.cond(state.insert_failure_count, (state.insert_failure_count + 1.into()).resize());
//
//     let state_next = state.set_state(InsertState::Send.into()).set_insert_failure_count(insert_failure_next);
//
//     (is_update2free, (Expr::invalid(), Expr::invalid()).into(), state_next)
// }
// fn send_to_free_transition<'id>(state: Expr<'id, State>, ready: Expr<'id, Ready>) -> StateTransitionResult<'id> {
//     let is_ready_arrived = state.state.is_eq(InsertState::Send.into()) & ready.ready;
//     let state_next = state.set_state(InsertState::Free.into());
//
//     (is_ready_arrived, (Expr::invalid(), Expr::invalid()).into(), state_next)
// }

// pub fn m() -> Module<InsertIC, InsertOC> {
// composite::<InsertIC, InsertOC, _>(
//     "insert",
//     Some("in"),
//     Some("out"),
//     |((mem_resp_channel, hash_resp_channel), insert_req_channel), k| {
//         let state_free = StateProj {
//             state: InsertState::Free.into(),
//             current_entry: Expr::x(),
//             hashes: Expr::x(),
//             response: Expr::x(),
//             current_trial: 0.into(),
//             insert_failure_count: 0.into(),
//             victim_bit: 0.into(),
//             victim_idx: 0.into(),
//         }
//         .into();
//
//         (mem_resp_channel, hash_resp_channel, insert_req_channel).fsm::<State, InsertOC, _>(
//             k,
//             Some("insert"),
//             state_free,
//             move |input_fwd, output_bwd, state| {
//                 let (mem_resp, hash_resp, insert_req) = *input_fwd;
//                 // when free, send hash request
//                 let (is_free2hash, free2hash_req, free2hash_state) = free_to_hash_transition(state, insert_req);
//
//                 let (is_hash2lookup, hash2lookup_req, hash2lookup_state) =
//                     hash_to_lookup_transition(state, hash_resp);
//
//                 let (is_lookup2update, lookup2update_req, lookup2update_state) =
//                     lookup_to_update_transition(state, mem_resp);
//
//                 let (is_update2hash, update2hash_req, update2hash_state) =
//                     update_to_hash_transition(state, mem_resp);
//
//                 let (is_update2send, update2send_req, update2send_state) =
//                     update_to_send_transition(state, mem_resp);
//
//                 let (is_send2free, send2free_req, send2free_state) =
//                     send_to_free_transition(state, output_bwd.1 .0);
//
//                 let c_in = select! {
//                     is_free2hash => free2hash_req,
//                     is_hash2lookup => hash2lookup_req,
//                     is_lookup2update => lookup2update_req,
//                     is_update2hash => update2hash_req,
//                     is_update2send => update2send_req,
//                     is_send2free => send2free_req,
//                     default => (Expr::invalid(), Expr::invalid()).into(),
//                 };
//
//                 let state_next = select! {
//                     is_free2hash => free2hash_state,
//                     is_hash2lookup => hash2lookup_state,
//                     is_lookup2update => lookup2update_state,
//                     is_update2hash => update2hash_state,
//                     is_update2send => update2send_state,
//                     is_send2free => send2free_state,
//                     default => state,
//                 };
//
//                 let out = (
//                     Expr::<Valid<_>>::new(state_next.state.is_eq(InsertState::Send.into()), state.response),
//                     Expr::valid(state.insert_failure_count),
//                 )
//                     .into();
//
//                 let ready = Expr::<Ready>::new(is_send2free);
//                 ((c_in, out).into(), (().into(), ().into(), ready).into(), state_next)
//             },
//         )
//     },
// )
// .build()
//     todo!()
// }

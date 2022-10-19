//! Cuckoo Hash Table

mod calculate_hashes;
mod types;

use shakeflow::*;
use types::*;

use crate::axis::*;
use crate::*;

/// AXIS basic data type
#[derive(Debug, Clone, Signal)]
pub struct AxisData<V: Signal> {
    /// tdata
    tdata: V,
}

const fn pad_width(width: usize, align_by: usize) -> usize { align_usize(width, align_by) - (width) }

// Types for Basis Data Types
pub(super) type KeyType = Bits<U<KEY_SIZE>>;
pub(super) type ValueType = Bits<U<VALUE_SIZE>>;
pub(super) type HashType = Bits<Log2<U<TABLE_SIZE>>>;

pub(super) type HashEntryPair = (HashType, HtEntry);

/// A entry of the Hash Table
#[derive(Debug, Clone, Signal)]
pub struct HtEntry {
    key: KeyType,
    value: ValueType,
    valid: bool,
}

impl HtEntry {
    /// Create an initial entry
    pub(super) fn init_expr() -> Expr<'static, Self> {
        HtEntryProj { key: 0.into(), value: 0.into(), valid: false.into() }.into()
    }
}

/// From <https://github.com/fpgasystems/Vitis_with_100Gbps_TCP-IP/blob/vitis_2022_1/fpga-network-stack/hls/hash_table/hash_table.hpp#:~:text=enum%20lookupSource%20%7BRX%2C%20TX_APP%7D%3B>
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Signal)]
pub enum LookupSource {
    /// Rx
    Rx,
    /// Tx
    Tx,
}

/// Indicated the operation of update request
/// From <https://github.com/fpgasystems/Vitis_with_100Gbps_TCP-IP/blob/vitis_2022_1/fpga-network-stack/hls/hash_table/hash_table.hpp#:~:text=typedef%20enum%20%7BKV_INSERT%2C%20KV_DELETE%7D%20kvOperation%3B>
#[derive(Debug, Clone, Copy, Signal)]
pub enum KvOperation {
    /// Insert
    KvInsert,
    /// Delete
    KvDelete,
}

/// Request Operation
#[derive(Debug, Clone, Copy, Signal)]
pub enum RequestOp {
    /// Lookup
    Lookup,
    /// Insert
    Insert,
    /// Delete
    Delete,
}

/// Request Supertype for simplified logic
///
/// This is the supertype of `HtLookupReq` and `HtUpdateReq` in Easynet implementation.
#[derive(Debug, Clone, Signal)]
pub struct Request {
    op: RequestOp,
    key: KeyType,
    value: ValueType,
    lookup_source: LookupSource,
}

/// Lookup Response
#[derive(Debug, Clone, Signal)]
pub struct LookupResp {
    key: KeyType,
    value: ValueType,
    hit: bool,
    lookup_source: LookupSource,
    byte_align_padding: Bits<U<{ pad_width(KEY_SIZE + VALUE_SIZE + 1 + 1, BITS_OF_BYTES) }>>,
}

/// Update Response
#[derive(Debug, Clone, Signal)]
pub struct UpdateResp {
    kv_operation: KvOperation,
    key: KeyType,
    value: ValueType,
    success: bool,
    lookup_source: LookupSource,
    byte_align_padding: Bits<U<{ pad_width(KEY_SIZE + VALUE_SIZE + 1 + 1 + 1, BITS_OF_BYTES) }>>,
}

const BITS_OF_BYTES: usize = 8;

/// Lookup Request
#[derive(Debug, Clone, Signal)]
pub struct LookupReq {
    key: KeyType,
    lookup_source: LookupSource,
    byte_align_padding: Bits<U<{ pad_width(KEY_SIZE + 1, BITS_OF_BYTES) }>>,
}

/// Update Request
#[derive(Debug, Clone, Signal)]
pub struct UpdateReq {
    kv_operation: KvOperation,
    key: KeyType,
    value: ValueType,
    lookup_source: LookupSource,
    byte_align_padding: Bits<U<{ pad_width(KEY_SIZE + VALUE_SIZE + 1 + 1, BITS_OF_BYTES) }>>,
}

/// Hash Table Ingress
#[derive(Debug, Interface)]
pub struct HI {
    /// Lookup Request Channel
    ///
    /// Request size is 72 bits. It is originally 65 bits in HLS, but `#pragma aggregate`
    /// bype-alignes the size of bitvector while bit-packing the struct
    s_axis_lup_req: AxisVrChannel<AxisData<Bits<U<{ LookupReq::WIDTH }>>>>,

    /// Update Request Channel
    ///
    /// Request size is 88 bits. It is originally 81 bits in HLS, but `#pragma aggregate`
    /// bype-alignes the size of bitvector while bit-packing the struct
    s_axis_upd_req: AxisVrChannel<AxisData<Bits<U<{ UpdateReq::WIDTH }>>>>,
}

/// Hash Table Egress
#[derive(Debug, Interface)]
pub struct HO {
    /// Lookup Response Channel
    m_axis_lup_rsp: AxisVrChannel<AxisData<Bits<U<{ LookupResp::WIDTH }>>>>,

    /// Update Response Channel
    m_axis_upd_rsp: AxisVrChannel<AxisData<Bits<U<{ UpdateResp::WIDTH }>>>>,

    /// Insert failure count
    register_failure_count: UniChannel<Valid<Bits<U<16>>>>,
}

/// Hash Table Feedback
#[derive(Debug, Interface)]
struct Feedback {
    /// Insert Retry Channel
    /// Retry the insert operation if this channel is valid
    insert_retry: VrChannel<Request>,
}

/// Cuckoo Hash Table
///
/// Ported from: <https://github.com/fpgasystems/Vitis_with_100Gbps_TCP-IP/tree/vitis_2020_1/fpga-network-stack/hls/hash_table>
///
/// TODO: Make Hash Table to be configurable using constant generic (to what extent?)
pub fn m() -> Module<HI, HO> {
    composite::<HI, HO, _>("hash_table", Some("in"), Some("out"), |input, k| {
        let (feedback_source, feedback_sink) = k.feedback::<Feedback>();

        let tables = (0..NUM_TABLES)
            .map(|i| k.register(Some(&i.to_string()), memory::m::<_, 0, TABLE_SIZE>(HtEntry::init_expr())).split())
            .collect::<Vec<_>>();

        let lookup_req = input.s_axis_lup_req.into_vr(k).map(k, |req| {
            let lookup_req: Expr<_> = LookupReqProj {
                key: req.tdata.clip_const::<U<KEY_SIZE>>(0),
                lookup_source: req.tdata.clip_const::<U<{ LookupSource::WIDTH }>>(KEY_SIZE).into(),
                byte_align_padding: 0.into(),
            }
            .into();
            // tdata field of incoming request is byte-aligned HtLookupReq, so we parse the
            // bitvector as follows
            //
            // ref: https://github.com/fpgasystems/Vitis_with_100Gbps_TCP-IP/blob/38481cc163f3f4de24b90eeda3319d58ebbeefb6/fpga-network-stack/hls/hash_table/hash_table.hpp#L55
            RequestProj {
                op: RequestOp::Lookup.into(),
                key: lookup_req.key,
                lookup_source: lookup_req.lookup_source,
                value: Expr::x(),
            }
            .into()
        });
        let update_req = input.s_axis_upd_req.into_vr(k).map(k, |req| {
            let update_req: Expr<_> = UpdateReqProj {
                kv_operation: req.tdata.clip_const::<U<{ KvOperation::WIDTH }>>(0).into(),
                key: req.tdata.clip_const::<U<KEY_SIZE>>(KvOperation::WIDTH),
                value: req.tdata.clip_const::<U<VALUE_SIZE>>(KvOperation::WIDTH + KEY_SIZE),
                lookup_source: req
                    .tdata
                    .clip_const::<U<{ LookupSource::WIDTH }>>(KvOperation::WIDTH + KEY_SIZE + VALUE_SIZE)
                    .into(),
                byte_align_padding: 0.into(),
            }
            .into();
            // tdata field of incoming request is byte-aligned HtUpdateReq, so we parse the
            // bitvector as follows
            //
            // ref: https://github.com/fpgasystems/Vitis_with_100Gbps_TCP-IP/blob/38481cc163f3f4de24b90eeda3319d58ebbeefb6/fpga-network-stack/hls/hash_table/hash_table.hpp#L75
            RequestProj {
                op: update_req
                    .kv_operation
                    .is_eq(KvOperation::KvInsert.into())
                    .cond(RequestOp::Insert.into(), RequestOp::Delete.into()),
                key: update_req.key,
                value: update_req.value,
                lookup_source: update_req.lookup_source,
            }
            .into()
        });

        // NOTE:
        // Lookup should be prioritized than update in according to easynet implementation
        let (_, request_muxed) = [feedback_source.insert_retry, lookup_req, update_req].mux(k);

        let (request_muxed, request_muxed_uni) = request_muxed.clone_uni(k);

        let hashes: [VrChannel<HashType, { Protocol::Demanding }>; NUM_TABLES] =
            request_muxed.map(k, |req| req.key).comb(k, None, calculate_hashes::m());

        let mem_resp: VrChannel<Array<(_, _), U<NUM_TABLES>>, { Protocol::Helpful }> =
            hashes.array_enumerate().map(|i, ch| ch.buffer(k).comb_inline(k, tables[i].0.clone())).gather(k);

        let request_buffered = request_muxed_uni.map(k, |v| v.inner).buffer(
            k,
            RequestProj { key: 0.into(), lookup_source: LookupSource::Tx.into(), op: Expr::x(), value: 0.into() }
                .into(),
        );

        let [lookup, insert, delete] = mem_resp
            .zip_uni(k, request_buffered)
            .map(k, |input| {
                let (entries, request) = *input;
                let op_input =
                    OpModuleInputProj { request, entries: entries.map(|v| (v.0.resize(), v.1).into()) }.into();
                let selector: Expr<Bits<U<2>>> = select! {
                    request.op.is_eq(RequestOp::Lookup.into()) => 0.into(),
                    request.op.is_eq(RequestOp::Insert.into()) => 1.into(),
                    request.op.is_eq(RequestOp::Delete.into()) => 2.into(),
                    default => 0.into(),
                };

                (op_input, selector).into()
            })
            .demux(k);

        let lookup_resp = lookup.comb(k, None, m_lookup());
        let (insert_resp, insert_write_req, insert_retry, register_failure_count) = insert.comb(k, None, m_insert());
        let (delete_resp, delete_write_req) = delete.comb(k, None, m_delete());

        Feedback { insert_retry }.comb_inline(k, feedback_sink);

        let (_, upd_resp_muxed) = [insert_resp, delete_resp].mux(k);
        let (_, write_req_muxed) = [insert_write_req, delete_write_req].mux(k);
        write_req_muxed.scatter(k).array_enumerate().map(|i, ch| ch.comb_inline(k, tables[i].1.clone()).sink(k));

        HO {
            m_axis_lup_rsp: lookup_resp
                .map(k, |resp| {
                    AxisDataProj {
                        tdata: resp
                            .key
                            .append(resp.value)
                            .append(resp.hit.repeat::<U<1>>())
                            .append(resp.lookup_source.into())
                            .append(resp.byte_align_padding)
                            .resize(),
                    }
                    .into()
                })
                .into_axis_vr(k),
            m_axis_upd_rsp: upd_resp_muxed
                .map(k, |resp| {
                    {
                        AxisDataProj {
                            tdata: Expr::<Bits<_>>::from(resp.kv_operation)
                                .append(resp.key)
                                .append(resp.value)
                                .append(resp.success.repeat::<U<1>>().append(resp.lookup_source.into()))
                                .append(resp.byte_align_padding)
                                .resize(),
                        }
                    }
                    .into()
                })
                .into_axis_vr(k),
            register_failure_count,
        }
    })
    .build()
}

#[derive(Debug, Clone, Signal)]
struct OpModuleInput {
    request: Request,
    entries: Array<HashEntryPair, U<NUM_TABLES>>,
}

type OpModuleIC = VrChannel<OpModuleInput>;

type LookupOC = VrChannel<LookupResp>;

/// Lookup Module
fn m_lookup() -> Module<OpModuleIC, LookupOC> {
    composite::<OpModuleIC, LookupOC, _>("lookup", Some("i"), Some("o"), |input, k| {
        input.map(k, |inner| {
            let key = inner.request.key;
            let (value, hit) = *inner.entries.zip(key.repeat()).fold((Expr::x(), false.into()).into(), |acc, x| {
                let (hash_entry_pair, key) = *x;
                let (value_acc, hit_acc) = *acc;
                let hit_current = hash_entry_pair.1.valid & hash_entry_pair.1.key.is_eq(key);
                let entry_new = hit_current.cond(hash_entry_pair.1.value, value_acc);

                (entry_new, hit_current | hit_acc).into()
            });

            LookupRespProj { key, value, hit, lookup_source: inner.request.lookup_source, byte_align_padding: 0.into() }
                .into()
        })
    })
    .build()
}

type DeleteOC = (VrChannel<UpdateResp>, VrChannel<Array<HashEntryPair, U<NUM_TABLES>>>);

/// Delete Module
fn m_delete() -> Module<OpModuleIC, DeleteOC> {
    composite::<OpModuleIC, DeleteOC, _>("delete", Some("i"), Some("o"), |input, k| {
        let (input, input_dup) = input.duplicate::<{ Protocol::Demanding }, { Protocol::Demanding }>(k);
        let response = input.map(k, |inner| {
            let RequestProj { key, value, lookup_source, .. } = *inner.request;
            let success = (0..NUM_TABLES).fold(false.into(), |acc, i| {
                let entry = inner.entries[i].1;
                let is_match = entry.valid & entry.key.is_eq(key);
                acc | is_match
            });

            UpdateRespProj {
                key,
                kv_operation: KvOperation::KvDelete.into(),
                value,
                success,
                lookup_source,
                byte_align_padding: 0.into(),
            }
            .into()
        });

        let writes = input_dup.map(k, |inner| {
            inner.entries.zip(inner.request.key.repeat()).map(|x| {
                let (x, key) = *x;
                let (hash, entry) = *x;
                let is_match = entry.valid & entry.key.is_eq(key);

                (hash, entry.set_valid(is_match.cond(false.into(), entry.valid))).into()
            })
        });

        (response.buffer(k), writes.buffer(k))
    })
    .build()
}

type InsertOC = (
    VrChannel<UpdateResp>,
    VrChannel<Array<HashEntryPair, U<NUM_TABLES>>>,
    VrChannel<Request>,
    UniChannel<Valid<Bits<U<16>>>>,
);

#[derive(Debug, Clone, Signal)]
struct InsertState {
    victim_bit: Bits<U<1>>,
    victim_idx: Bits<Log2<U<NUM_TABLES>>>,
    request: Request,
    insert_failure_count: Bits<U<16>>,
    trial_count: Bits<Log2<U<MAX_TRIALS>>>,
}

/// Insert Module
fn m_insert() -> Module<OpModuleIC, InsertOC> {
    composite::<OpModuleIC, InsertOC, _>("insert", Some("i"), Some("o"), |input, k| {
        let [dup1, dup2, dup3]: [VrChannel<
            (Valid<UpdateResp>, Array<HashEntryPair, U<NUM_TABLES>>, Valid<Request>, Bits<U<16>>),
            { Protocol::Demanding },
        >; 3] = input
            .fsm_map::<InsertState, _, _>(
                k,
                Some("insert_logic"),
                InsertStateProj {
                    victim_bit: 0.into(),
                    victim_idx: 0.into(),
                    request: Expr::x(),
                    insert_failure_count: 0.into(),
                    trial_count: 0.into(),
                }
                .into(),
                |input, state| {
                    let OpModuleInputProj { request, entries } = *input;

                    let (is_free_slot_found, slot_idx) = *entries.enumerate().fold(
                        (false.into(), Expr::<Bits<Log2<U<NUM_TABLES>>>>::from(0)).into(),
                        |acc, x| {
                            let (success_acc, acc_idx) = *acc;
                            let (current_idx, entry) = *x;
                            let is_entry_free = !entry.1.valid;

                            (success_acc | is_entry_free, is_entry_free.cond(current_idx, acc_idx)).into()
                        },
                    );

                    let req_entry = HtEntryProj { key: request.key, value: request.value, valid: true.into() }.into();

                    let entries_success = entries.set(slot_idx, (entries[slot_idx].0, req_entry).into());

                    let (victim_entry, entries_evict) = {
                        let victim_pos = ((entries[state.victim_idx].0
                            % Expr::<Bits<Log2<U<NUM_TABLES>>>>::from(NUM_TABLES - 1))
                            + state.victim_bit.resize())
                        .resize();
                        let victim_entry = entries[victim_pos];
                        (victim_entry, entries.set(victim_pos, (victim_entry.0, req_entry).into()))
                    };

                    let write_req_out = is_free_slot_found.cond(entries_success, entries_evict);

                    let retry_req_out = Expr::<Valid<_>>::new(
                        !is_free_slot_found & state.trial_count.is_lt(MAX_TRIALS.into()),
                        RequestProj {
                            op: RequestOp::Insert.into(),
                            key: victim_entry.1.key,
                            value: victim_entry.1.value,
                            lookup_source: request.lookup_source,
                        }
                        .into(),
                    );

                    let insert_failure_count_out = state.insert_failure_count;

                    let update_out = Expr::<Valid<_>>::new(
                        is_free_slot_found | (!is_free_slot_found & !state.trial_count.is_lt(MAX_TRIALS.into())),
                        state.trial_count.is_eq(0.into()).cond(
                            UpdateRespProj {
                                kv_operation: KvOperation::KvInsert.into(),
                                key: request.key,
                                value: request.value,
                                success: is_free_slot_found,
                                lookup_source: request.lookup_source,
                                byte_align_padding: 0.into(),
                            }
                            .into(),
                            UpdateRespProj {
                                kv_operation: KvOperation::KvInsert.into(),
                                key: state.request.key,
                                value: state.request.value,
                                success: is_free_slot_found,
                                lookup_source: state.request.lookup_source,
                                byte_align_padding: 0.into(),
                            }
                            .into(),
                        ),
                    );

                    let victim_bit_next = !state.victim_bit;
                    let victim_idx_add_one = state.victim_idx + 1.into();
                    let victim_idx_next = select! {
                        !is_free_slot_found =>
                            victim_idx_add_one.is_eq(NUM_TABLES.into()).cond(
                                0.into(),
                                victim_idx_add_one.resize()
                            ),
                        (is_free_slot_found | (!is_free_slot_found & !state.trial_count.is_lt(MAX_TRIALS.into()))) =>
                            0.into(),
                        default => state.victim_idx,
                    };
                    let trial_count_next = select! {
                        !is_free_slot_found => (state.trial_count + 1.into()).resize(),
                        (is_free_slot_found | (!is_free_slot_found & !state.trial_count.is_lt(MAX_TRIALS.into()))) =>
                            0.into(),
                        default => state.trial_count,
                    };
                    let request_next = state.trial_count.is_eq(0.into()).cond(request, state.request);
                    let insert_failure_count_next = !is_free_slot_found
                        .cond((state.insert_failure_count + 1.into()).resize(), state.insert_failure_count);
                    let state_next = InsertStateProj {
                        victim_bit: victim_bit_next,
                        victim_idx: victim_idx_next,
                        request: request_next,
                        insert_failure_count: insert_failure_count_next,
                        trial_count: trial_count_next,
                    }
                    .into();

                    ((update_out, write_req_out, retry_req_out, insert_failure_count_out).into(), state_next)
                },
            )
            .duplicate_n(k);

        let (dup1, dup1_uni) = dup1.clone_uni(k);

        (
            dup1.and_then(k, None, |v| v.0).buffer(k),
            dup2.map(k, |v| v.1).buffer(k),
            dup3.and_then(k, None, |v| v.2).buffer(k),
            dup1_uni.map(k, |v| Expr::valid(v.inner.3)),
        )
    })
    .build()
}

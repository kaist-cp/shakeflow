use shakeflow::*;
use shakeflow_std::*;

const WIDTH_P: usize = 10;
const ELS_P: usize = 5;
const LG_ELS_LP: usize = clog2(ELS_P);

#[derive(Debug, Clone, Signal)]
pub struct Write {
    id: Bits<U<LG_ELS_LP>>,
    data: Bits<U<WIDTH_P>>,
}

#[derive(Debug, Clone, Signal)]
pub struct AllocId {
    id: Bits<U<LG_ELS_LP>>,
}

#[derive(Debug, Interface)]
pub struct IC {
    write: UniChannel<Valid<Write>>,
}

#[derive(Debug, Interface)]
pub struct OC {
    fifo_alloc: VrChannel<AllocId>,
    fifo_deq: VrChannel<Write>,
    empty: UniChannel<bool>,
}

#[derive(Debug, Clone, Signal)]
pub struct TrackerI {
    enq: bool,
    deq: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct TrackerO {
    wptr_r: Bits<U<LG_ELS_LP>>,
    rptr_r: Bits<U<LG_ELS_LP>>,
    full: bool,
    empty: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct MI {
    w: Valid<Write>,
    r: Valid<AllocId>,
}

impl_custom_inst! {UniChannel<TrackerI>, UniChannel<TrackerO>, bsg_fifo_tracker, <els_p>, true,}
impl_custom_inst! {UniChannel<Valid<Bits<U<LG_ELS_LP>>>>, UniChannel<Bits<U<ELS_P>>>, bsg_decode_with_v, <num_out_p>, false,}
impl_custom_inst! {UniChannel<MI>, UniChannel<Bits<U<WIDTH_P>>>, bsg_mem_1r1w, <width_p, els_p>, true,}

pub fn m() -> Module<IC, OC> {
    composite::<(IC, UniChannel<TrackerI>), (OC, UniChannel<TrackerI>), _>(
        "bsg_fifo_reorder",
        Some("i"),
        Some("o"),
        |(input, tracker_i), k| {
            let input = input.write;

            let tracker_o = tracker_i.clone().bsg_fifo_tracker::<ELS_P>(k, "tracker0", Some("i"), Some("o"));

            let empty = tracker_o.clone().map(k, |input| input.empty);

            let set_valid = input.clone().map_inner(k, |input| input.id).bsg_decode_with_v::<ELS_P>(
                k,
                "set_demux0",
                Some("i"),
                Some("o"),
            );

            let clear_valid = tracker_o
                .clone()
                .zip(k, tracker_i)
                .map(k, |input| {
                    let (tracker_o, tracker_i) = *input;
                    Expr::<Valid<_>>::new(tracker_i.deq, tracker_o.rptr_r)
                })
                .bsg_decode_with_v::<ELS_P>(k, "clear_demux0", Some("i"), Some("o"));

            let valid = UniChannel::buffer_set_clear(k, set_valid, clear_valid);

            let fifo_alloc = tracker_o
                .clone()
                .map(k, |input| {
                    let full = input.full;
                    let wptr_r = input.wptr_r;
                    Expr::<Valid<_>>::new(!full, AllocIdProj { id: wptr_r }.into())
                })
                .into_vr(k);

            let (fifo_alloc, fifo_alloc_yumi) = fifo_alloc.fire(k);
            let enq = tracker_o.clone().zip(k, fifo_alloc_yumi).map(k, |input| {
                let (tracker_o, yumi) = *input;
                !tracker_o.full & yumi
            });

            let fifo_deq = tracker_o.zip(k, valid).map(k, |input| {
                let (input, valid) = *input;
                let rptr_r = input.rptr_r;
                let empty = input.empty;
                Expr::<Valid<_>>::new(valid[rptr_r.resize()] & !empty, rptr_r)
            });

            let fifo_deq_data = input
                .zip(k, fifo_deq.clone())
                .map(k, |input| {
                    let (input, fifo_deq) = *input;

                    MIProj {
                        w: input,
                        r: Expr::<Valid<_>>::new(fifo_deq.valid, AllocIdProj { id: fifo_deq.inner }.into()),
                    }
                    .into()
                })
                .bsg_mem_1r1w::<WIDTH_P, ELS_P>(k, "mem0", Some("i"), Some("r_data_o"));

            let fifo_deq = fifo_deq
                .zip(k, fifo_deq_data)
                .map(k, |input| {
                    let (fifo_deq, fifo_deq_data) = *input;
                    Expr::<Valid<_>>::new(fifo_deq.valid, WriteProj { id: fifo_deq.inner, data: fifo_deq_data }.into())
                })
                .into_vr(k);

            let (fifo_deq, deq) = fifo_deq.fire(k);

            let feedback = enq.zip(k, deq).map(k, |input| {
                let (enq, deq) = *input;
                TrackerIProj { enq, deq }.into()
            });

            let output = OC { fifo_alloc, fifo_deq, empty };

            (output, feedback)
        },
    )
    .loop_feedback()
    .build()
}

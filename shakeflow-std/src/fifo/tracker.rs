//! FIFO Tracker.

use shakeflow::*;

use crate::*;

/// Ingress signal of FIFO tracker.
#[derive(Debug, Clone, Signal)]
pub struct I {
    enq: bool,
    deq: bool,
}

/// Egress signal of FIFO tracker.
#[derive(Debug, Clone, Signal)]
pub struct E<PtrWidth: Num> {
    wptr_r: Bits<PtrWidth>,
    rptr_r: Bits<PtrWidth>,
    rptr_n: Bits<PtrWidth>,
    full: bool,
    empty: bool,
}

/// Ingress channel of FIFO tracker.
pub type IC = UniChannel<I>;
/// Egress channel of FIFO tracker.
pub type EC<PtrWidth: Num> = UniChannel<E<PtrWidth>>;

/// Inner logic of FIFO tracker.
pub fn m<const SLOTS: usize, PtrWidth: Num>() -> Module<IC, EC<PtrWidth>> {
    composite::<IC, EC<PtrWidth>, _>("bsg_fifo_tracker", Some("i"), Some("o"), |input, k| {
        let wptr = input.clone().map(k, |input| input.enq).counter::<U<SLOTS>>(k);
        let rptr = input.clone().map(k, |input| input.deq).counter::<U<SLOTS>>(k);

        input.zip3(k, wptr, rptr).fsm_map(k, None, I { enq: false, deq: true }.into(), |input, last_input| {
            let (input, wptr, rptr) = *input;
            let (wptr_r, _) = *wptr;
            let (rptr_r, rptr_n) = *rptr;

            let equal_ptrs = wptr_r.is_eq(rptr_r);
            let full = equal_ptrs & last_input.enq;
            let empty = equal_ptrs & last_input.deq;

            let output =
                EProj { wptr_r: wptr_r.resize(), rptr_r: rptr_r.resize(), rptr_n: rptr_n.resize(), full, empty }.into();
            let last_input = (input.enq | input.deq).cond(input, last_input);
            (output, last_input)
        })
    })
    .build()
}

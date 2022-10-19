//! FIFO with 1 read or write.

use shakeflow::*;

use crate::*;

/// Ingress signal.
#[derive(Debug, Clone, Signal)]
pub struct I<V: Signal> {
    data: Valid<V>,
    enq_not_deq: bool,
}

/// Egress signal.
#[derive(Debug, Clone, Signal)]
pub struct E<V: Signal> {
    full: bool,
    empty: bool,
    data: V,
}

#[derive(Debug, Clone, Signal)]
struct S<V: Signal, const ELS: usize> {
    mem: Array<V, U<ELS>>,
    last_op_is_read: bool,
    output: V,
}

impl<V: Signal, const ELS: usize> S<V, ELS> {
    fn new_expr() -> Expr<'static, Self> {
        SProj { mem: Expr::x(), last_op_is_read: false.into(), output: Expr::x() }.into()
    }
}

fn m_fifo_1rw<V: Signal, const ELS: usize>() -> Module<UniChannel<I<V>>, UniChannel<E<V>>> {
    composite::<UniChannel<I<V>>, UniChannel<E<V>>, _>("fifo_1rw_large", Some("i"), Some("o"), |input, k| {
        let enque = input.clone().map(k, |input| input.data.valid & input.enq_not_deq);
        let deque = input.clone().map(k, |input| input.data.valid & !input.enq_not_deq);

        let tracker = enque
            .clone()
            .zip(k, deque.clone())
            .map(k, |input| {
                let (enq, deq) = *input;
                fifo::tracker::IProj { enq, deq }.into()
            })
            .comb_inline(k, fifo::tracker::m::<ELS, Log2<U<ELS>>>());

        input.zip4(k, enque, deque, tracker).fsm_map::<S<V, ELS>, E<V>, _>(k, None, S::new_expr(), |input, state| {
            let (input, enque, deque, tracker) = *input;

            let fifo_empty = tracker.wptr_r.is_eq(tracker.rptr_r) & state.last_op_is_read;
            let fifo_full = tracker.wptr_r.is_eq(tracker.rptr_r) & !state.last_op_is_read;

            let output = EProj { full: fifo_full, empty: fifo_empty, data: state.output }.into();
            let state_next = SProj {
                mem: state.mem.set(tracker.wptr_r, enque.cond(input.data.inner, state.mem[tracker.wptr_r])),
                last_op_is_read: deque,
                output: state.mem[tracker.rptr_r],
            }
            .into();

            (output, state_next)
        })
    })
    .build()
}

impl<V: Signal> UniChannel<I<V>> {
    /// FIFO with 1 read or write.
    pub fn fifo_1rw<const ELS: usize>(self, k: &mut CompositeModuleContext) -> UniChannel<E<V>> {
        self.comb_inline(k, m_fifo_1rw::<V, ELS>())
    }
}

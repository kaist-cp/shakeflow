//! FIFO with 1 read and 1 write

use shakeflow::*;

use super::one_read_write;
use crate::*;

/// Ingress signal of FIFO.
#[derive(Debug, Clone, Signal)]
pub struct FifoI<V: Signal, const SLOTS: usize> {
    write: Valid<(Bits<Log2<U<SLOTS>>>, V)>,
    read: Bits<Log2<U<SLOTS>>>,
    read_n: Bits<Log2<U<SLOTS>>>,
}

fn m_fifo_1r1w<V: Signal, const SLOTS: usize>(
    logic: fn(UniChannel<FifoI<V, SLOTS>>, &mut CompositeModuleContext) -> UniChannel<V>,
) -> Module<VrChannel<V>, VrChannel<V>> {
    composite::<(VrChannel<V>, UniChannel<bool>), (VrChannel<V>, UniChannel<bool>), _>(
        "fifo",
        Some("i"),
        Some("o"),
        |(input, deque), k| {
            let (input, enque) = input.fire(k);

            let tracker = enque
                .clone()
                .zip(k, deque)
                .map(k, |input| {
                    let (enq, deq) = *input;
                    fifo::tracker::IProj { enq, deq }.into()
                })
                .comb_inline(k, fifo::tracker::m::<SLOTS, Log2<U<SLOTS>>>());

            let (input, input_data) = input.clone_uni(k);

            let write = input_data.zip3(k, enque, tracker.clone()).map(k, |input| {
                let (input, enque, tracker) = *input;
                Expr::<Valid<_>>::new(enque, (tracker.wptr_r, input.inner).into())
            });

            let fifo_input = write.zip(k, tracker.clone()).map(k, |input| {
                let (write, tracker) = *input;
                FifoIProj { write, read: tracker.rptr_r, read_n: tracker.rptr_n }.into()
            });

            let output_data = logic(fifo_input, k);

            let output = (input, output_data, tracker).fsm::<(), VrChannel<V>, _>(
                k,
                None,
                ().into(),
                |ingress_fwd, _, state| {
                    let (_, output_data, tracker) = *ingress_fwd;
                    let egress_fwd = Expr::<Valid<_>>::new(!tracker.empty, output_data);
                    let ingress_bwd = (Expr::<Ready>::new(!tracker.full), ().into(), ().into()).into();
                    (egress_fwd, ingress_bwd, state)
                },
            );

            output.fire(k)
        },
    )
    .loop_feedback()
    .build()
}

fn m_fifo_1r1w_from_1rw<V: Signal>(
    fifo_1rw: Module<UniChannel<one_read_write::I<V>>, UniChannel<one_read_write::E<V>>>,
) -> Module<VrChannel<V>, VrChannel<V>> {
    composite::<
        (VrChannel<V>, (UniChannel<bool>, UniChannel<bool>)),
        (VrChannel<V>, (UniChannel<bool>, UniChannel<bool>)),
        _,
    >("fifo_1r1w_from_1rw", Some("i"), Some("o"), |(input, (fire, full)), k| {
        let fifo_1rw_input = (input, fire, full).fsm::<(), UniChannel<one_read_write::I<V>>, _>(
            k,
            None,
            ().into(),
            |ingress_fwd, _, state| {
                let (ingress_fwd, fire, full) = *ingress_fwd;
                let egress_fwd = one_read_write::IProj {
                    data: Expr::<Valid<_>>::new(ingress_fwd.valid | fire, ingress_fwd.inner),
                    enq_not_deq: ingress_fwd.valid,
                }
                .into();
                let ingress_bwd = (Expr::<Ready>::new(!full), ().into(), ().into()).into();
                (egress_fwd, ingress_bwd, state)
            },
        );

        let fifo_1rw_output = fifo_1rw_input.comb_inline(k, fifo_1rw);

        let full = fifo_1rw_output.clone().map(k, |input| input.full);
        let output = fifo_1rw_output.map(k, |input| Expr::<Valid<_>>::new(!input.empty, input.data)).into_vr(k);
        let (output, fire) = output.fire(k);

        (output, (fire, full))
    })
    .loop_feedback()
    .build()
}

impl<V: Signal> VrChannel<V> {
    /// FIFO with 1 read and 1 write.
    pub fn fifo_1r1w<const SLOTS: usize>(
        self, k: &mut CompositeModuleContext,
        logic: fn(UniChannel<FifoI<V, SLOTS>>, &mut CompositeModuleContext) -> UniChannel<V>,
    ) -> Self {
        self.comb_inline(k, m_fifo_1r1w(logic))
    }

    /// Constructs FIFO 1R1W from FIFO 1RW.
    pub fn fifo_1r1w_from_1rw(
        self, k: &mut CompositeModuleContext,
        fifo_1rw: Module<UniChannel<one_read_write::I<V>>, UniChannel<one_read_write::E<V>>>,
    ) -> Self {
        self.comb_inline(k, m_fifo_1r1w_from_1rw(fifo_1rw))
    }
}

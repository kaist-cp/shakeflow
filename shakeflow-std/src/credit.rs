//! Credit-based Control Flow.

use shakeflow::*;

use crate::*;

fn m_into_fifo<V: Signal>(fifo: Module<VrChannel<V>, VrChannel<V>>) -> Module<VrChannel<V>, VrChannel<V>> {
    composite::<(VrChannel<V>, UniChannel<bool>), (VrChannel<V>, UniChannel<bool>), _>(
        "into_fifo",
        Some("i"),
        Some("o"),
        |(input, deque), k| {
            // We do not use `egress_bwd` since it is guaranteed by credit system not to send extra inputs and every
            // input must be stored FIFO used to keep the data values.
            let fifo_input =
                input.zip_uni(k, deque).fsm::<bool, VrChannel<V>, _>(k, None, false.into(), |ingress_fwd, _, state| {
                    let ingress_bwd = Expr::<Ready>::new(state);
                    let egress_fwd = Expr::<Valid<_>>::new(ingress_fwd.valid, ingress_fwd.inner.0);
                    let state_next = ingress_fwd.inner.1;

                    (egress_fwd, ingress_bwd, state_next)
                });

            fifo_input.comb_inline(k, fifo).fire(k)
        },
    )
    .loop_feedback()
    .build()
}

impl<V: Signal> VrChannel<V> {
    /// Converts the valid-ready protocol into valid-credit protocol, by keeping the count of available credits.
    pub fn into_credit_flow<const CREDIT_MAX_VAL: usize, const DECIMATION: usize>(
        self, k: &mut CompositeModuleContext,
    ) -> VrChannel<V> {
        self.fsm::<Bits<Log2<U<CREDIT_MAX_VAL>>>, VrChannel<V>, _>(
            k,
            Some("into_credit_flow"),
            0.into(),
            |ingress_fwd, egress_bwd, credit_cnt| {
                let ingress_bwd = Expr::<Ready>::new(credit_cnt.is_gt(0.into()));
                let ingress_transfer = ingress_fwd.valid & ingress_bwd.ready;
                let egress_fwd = Expr::<Valid<_>>::new(ingress_transfer, ingress_fwd.inner);

                let up = egress_bwd.ready.cond(DECIMATION.into(), 0.into());
                let down = ingress_transfer.repr().resize();

                let credit_cnt_next = (credit_cnt + up - down).resize();

                (egress_fwd, ingress_bwd, credit_cnt_next)
            },
        )
    }

    /// Converts valid-credit protocol into valid-ready protocol, by using a FIFO to keep the data.
    pub fn into_fifo(self, k: &mut CompositeModuleContext, fifo: Module<VrChannel<V>, VrChannel<V>>) -> VrChannel<V> {
        self.comb_inline(k, m_into_fifo(fifo))
    }
}

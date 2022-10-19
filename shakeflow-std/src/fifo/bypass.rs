//! FIFO bypass.

use shakeflow::*;

use crate::*;

fn m_fifo_bypass<V: Signal>(fifo: Module<VrChannel<V>, VrChannel<V>>) -> Module<VrChannel<V>, VrChannel<V>> {
    composite::<(VrChannel<V>, VrChannel<V>), (VrChannel<V>, VrChannel<V>), _>(
        "fifo_bypass",
        Some("i"),
        Some("o"),
        |(input, fifo_output), k| {
            let (output, fifo_input) = (input, fifo_output).fsm::<(), (VrChannel<V>, VrChannel<V>), _>(
                k,
                None,
                ().into(),
                |ingress_fwd, egress_bwd, state| {
                    let enque = ingress_fwd.0.valid & egress_bwd.0.ready;

                    let egress_fwd = (
                        ingress_fwd.1.valid.cond(ingress_fwd.1, Expr::<Valid<_>>::new(enque, ingress_fwd.0.inner)),
                        Expr::<Valid<_>>::new(enque & (ingress_fwd.1.valid | !egress_bwd.0.ready), ingress_fwd.0.inner),
                    )
                        .into();
                    let ingress_bwd = (egress_bwd.1, egress_bwd.0).into();

                    (egress_fwd, ingress_bwd, state)
                },
            );

            let fifo_output = fifo_input.comb_inline(k, fifo);

            (output, fifo_output)
        },
    )
    .loop_feedback()
    .build()
}

impl<V: Signal> VrChannel<V> {
    /// FIFO bypass.
    pub fn fifo_bypass(self, k: &mut CompositeModuleContext, fifo: Module<VrChannel<V>, VrChannel<V>>) -> VrChannel<V> {
        self.comb_inline(k, m_fifo_bypass(fifo))
    }
}

//! Skid buffer for valid-ready channels.
//!
//! Reference: <https://chipressco.wpcomstaging.com/2019/03/16/valid-ready-protocol-and-register-slice/>

use crate::*;

/// Creates a forward register slice module.
fn m_fwd<V: Signal, const P: Protocol>() -> Module<VrChannel<V, P>, (VrChannel<V>, UniChannel<bool>)> {
    composite::<VrChannel<V, P>, (VrChannel<V>, UniChannel<bool>), _>(
        "register_slice_fwd",
        Some("in"),
        Some("out"),
        |value, k| {
            value.fsm::<_, (VrChannel<V>, UniChannel<bool>), _>(k, None, Expr::invalid(), |fwd, bwd, s| {
                let fwd_valid = fwd.valid;
                let is_occupied = s.valid;
                let bwd_ready = bwd.0.ready;

                let remaining = is_occupied & !bwd_ready;

                let s_next = select! {
                    fwd_valid & !remaining => fwd,
                    bwd_ready => Expr::invalid(),
                    default => s,
                };

                ((s, remaining).into(), Expr::<Ready>::new(!remaining), s_next)
            })
        },
    )
    .build()
}

/// Creates a backward register slice module.
fn m_bwd<V: Signal, const P: Protocol>() -> Module<VrChannel<V, P>, VrChannel<V>> {
    composite::<VrChannel<V, P>, VrChannel<V>, _>("register_slice_bwd", Some("in"), Some("out"), |value, k| {
        value.fsm::<Ready, VrChannel<_>, _>(k, None, Expr::<Ready>::new(false.into()), |fwd, bwd, s| (fwd, s, bwd))
    })
    .build()
}

impl<I: Signal, const P: Protocol> VrChannel<I, P> {
    /// Adds a forward register slice.
    ///
    /// # Note
    ///
    /// The second `UniChannel<bool>` indicates whether the registered value is occupied and remaining in
    /// the next cycle.
    pub fn register_slice_fwd(self, k: &mut CompositeModuleContext) -> (VrChannel<I>, UniChannel<bool>) {
        self.comb_inline(k, m_fwd())
    }

    /// Adds a backward register slice.
    ///
    /// Once the slave's tready is asserted, it should remain so until transfer; otherwise, the
    /// register slice will malfunction. Reason: we didn't install data/valid registers for the case
    /// that (1) ready_reg is true; and (2) ready_in is false.
    #[must_use]
    pub fn register_slice_bwd(self, k: &mut CompositeModuleContext) -> VrChannel<I> { self.comb_inline(k, m_bwd()) }
}

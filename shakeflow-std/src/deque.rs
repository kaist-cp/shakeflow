//! Data-deque channel.
//!
//! When it is sending the last piece it would assert the deque to previous module.

use shakeflow_macro::Signal;

use crate::*;

/// Deque signal.
#[derive(Debug, Clone, Signal)]
pub struct Deque {
    /// Deque bit
    pub deque: bool,
}

/// Deque Extension
pub trait DequeExt<'id> {
    /// Creates a new expr
    fn new(deque: Expr<'id, bool>) -> Self;
}

impl<'id> DequeExt<'id> for Expr<'id, Deque> {
    /// Creates a new expr.
    fn new(deque: Expr<'id, bool>) -> Self { DequeProj { deque }.into() }
}

// Data-deque channel.
channel! {DeqChannel<V: Signal>, V, Deque}

impl<V: Signal> DeqChannel<V> {
    /// Transforms into valid-ready channel.
    ///
    /// Since all packets are valid in deque channel, the valid signal of valid-ready signal is always high.
    pub fn into_vr(self, k: &mut CompositeModuleContext) -> VrChannel<V> {
        self.fsm::<(), VrChannel<V>, _>(k, Some("into_vr"), ().into(), |data, bwd_o, s| {
            let fwd_o = Expr::valid(data);
            let bwd_i = Expr::<Deque>::new(bwd_o.ready);

            (fwd_o, bwd_i, s)
        })
    }

    /// Returns deque signal.
    pub fn deque(self, k: &mut CompositeModuleContext) -> (Self, UniChannel<bool>) {
        let (fire, this) = self.fsm::<_, _, _>(k, Some("fire"), ().into(), move |fwd, bwd: Expr<((), Deque)>, s| {
            let bwd = bwd.1;

            let bwd_ready = bwd.deque;

            let fire = bwd_ready;

            ((fire, fwd).into(), bwd, s)
        });

        (this, fire)
    }
}

impl<I: Signal> DeqChannel<I> {
    /// Fsm for deque channel.
    pub fn fsm_map<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, O>, Expr<'id, S>, Expr<'id, bool>),
    >(
        self, k: &mut CompositeModuleContext, init: Expr<'static, S>, f: F,
    ) -> DeqChannel<O> {
        self.fsm::<S, DeqChannel<O>, _>(k, Some("fsm_map"), init, move |fwd_i, bwd_o, s| {
            let (fwd_o, s_next, last) = f(fwd_i, s);
            let deque = bwd_o.deque;

            (fwd_o, Expr::<Deque>::new(deque & last), deque.cond(s_next, s))
        })
    }
}

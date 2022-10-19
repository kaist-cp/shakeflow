//! Buffer for unidirectional channels.

use shakeflow_macro::Signal;

use crate::num::*;
use crate::*;

/// Buffer's state.
#[derive(Debug, Clone, Signal)]
struct State<V: Signal> {
    /// Buffered value
    #[member(name = "")]
    inner: V,
}

/// Creates a buffer module.
fn m<V: Signal>(init: Expr<'static, V>) -> Module<UniChannel<V>, UniChannel<V>> {
    composite::<UniChannel<V>, UniChannel<V>, _>("buffer", Some("in"), Some("out"), |value, k| {
        value.fsm_map(k, None, StateProj { inner: init }.into(), |value, state| {
            (state.inner, StateProj { inner: value }.into())
        })
    })
    .build()
}

/// Creates a buffer module with enable signal.
fn m_en<V: Signal>(init: Expr<'static, V>) -> Module<UniChannel<(V, bool)>, UniChannel<V>> {
    composite::<UniChannel<(V, bool)>, UniChannel<V>, _>("buffer_en", Some("in"), Some("out"), |value, k| {
        value.fsm_map(k, None, StateProj { inner: init }.into(), move |value, state| {
            let (value, en) = *value;
            (state.inner, en.cond(StateProj { inner: value }.into(), state))
        })
    })
    .build()
}

/// Creates a buffer module with delay
fn m_buffer_cycles<V: Signal, const STAGES: usize>(
    init: Expr<'static, V>, delay: usize,
) -> Module<UniChannel<V>, UniChannel<V>> {
    assert!(STAGES >= delay);

    composite::<UniChannel<V>, UniChannel<V>, _>("n_cycle_buffer", Some("inp"), Some("out"), |input, k| {
        input.fsm_map::<Array<V, U<STAGES>>, V, _>(k, None, init.repeat::<U<STAGES>>(), move |input, state| {
            let state_next =
                input.repeat::<U<1>>().append(state.clip_const::<Diff<U<STAGES>, U<1>>>(0)).resize::<U<STAGES>>();
            let output = state[delay];
            (output, state_next)
        })
    })
    .build()
}

/// Creates a buffer module with set and clear signals.
///
/// Note: In this implementation, set signal overrides clear signal.
#[allow(clippy::type_complexity)]
fn m_set_clear<const N: usize>() -> Module<UniChannel<(Bits<U<N>>, Bits<U<N>>)>, UniChannel<Bits<U<N>>>> {
    composite::<UniChannel<(Bits<U<N>>, Bits<U<N>>)>, UniChannel<Bits<U<N>>>, _>(
        "buffer_set_clear",
        Some("in"),
        Some("out"),
        |value, k| {
            value.fsm_map(k, None, StateProj { inner: 0.into() }.into(), move |value, state| {
                let (set, clear) = *value;
                (state.inner, StateProj { inner: (state.inner & !clear) | set }.into())
            })
        },
    )
    .build()
}

/// Creates a buffer module with valid-ready signals.
fn m_vr<V: Signal, const P: Protocol>() -> Module<VrChannel<V, P>, VrChannel<V>> {
    composite::<VrChannel<V, P>, VrChannel<V>, _>("buffer_vr", Some("in"), Some("out"), |value, k| {
        value.fsm::<Valid<V>, VrChannel<V>, _>(k, None, Expr::invalid(), move |ingress_fwd, egress_bwd, state| {
            let full = state.valid;

            let egress_fwd = state;
            let ingress_bwd = Expr::<Ready>::new(!full);

            // Incoming and outgoing cannot be both `true`.
            let incoming = ingress_fwd.valid & ingress_bwd.ready;
            let outgoing = egress_fwd.valid & egress_bwd.ready;

            let s_next = select! {
                incoming => Expr::<Valid<_>>::valid(ingress_fwd.inner),
                outgoing => Expr::invalid(),
                default => state,
            };

            (egress_fwd, ingress_bwd, s_next)
        })
    })
    .build()
}

/// Creates a buffer module with valid-ready signals.
fn m_vr_always<V: Signal, const P: Protocol>() -> Module<VrChannel<V, P>, VrChannel<V>> {
    composite::<VrChannel<V, P>, VrChannel<V>, _>("buffer_vr_always", Some("in"), Some("out"), |value, k| {
        value.fsm::<Valid<V>, VrChannel<V>, _>(k, None, Expr::invalid(), move |ingress_fwd, egress_bwd, state| {
            let egress_fwd = state;
            let ingress_bwd = egress_bwd;
            let state_next = egress_bwd.ready.cond(ingress_fwd, state);
            (egress_fwd, ingress_bwd, state_next)
        })
    })
    .build()
}

/// Creates a FIFO module with N-elements queue.
///
/// TODO: Current implementation is not actually producing helpful protocol. We need to fix it
fn m_fifo<V: Signal, const N: usize, const P: Protocol>() -> Module<VrChannel<V, P>, VrChannel<V>> {
    composite::<VrChannel<V, P>, VrChannel<V>, _>("fifo_vr", Some("in"), Some("out"), |value, k| {
        value.fsm::<(Array<V, U<N>>, Bits<U<N>>, Bits<U<N>>, Bits<U<N>>), VrChannel<V>, _>(
            k,
            None,
            (Expr::<V>::x().repeat(), 0.into(), 0.into(), 0.into()).into(),
            move |fwd_i, bwd_o, s| {
                let (mem, count, read, write) = *s;

                let empty = count.is_eq(0.into());
                let full = count.is_eq(N.into());
                let q = mem[read.resize::<Log2<U<N>>>()];

                let enq = fwd_i.valid & !full;
                let deq = bwd_o.ready & !empty;

                let fwd_o = Expr::<Valid<_>>::new(!empty, q);
                let bwd_i = Expr::<Ready>::new(!full);

                let mem_next = mem.set(write.resize(), enq.cond(fwd_i.inner, mem[write.resize()]));
                let count_next = (count + enq.repr().resize() - deq.repr().resize()).resize();
                let read_next = deq.cond((read + 1.into()).resize(), read);
                let write_next = enq.cond((write + 1.into()).resize(), write);

                let read_next = read_next.is_ge(N.into()).cond(0.into(), read_next);
                let write_next = write_next.is_ge(N.into()).cond(0.into(), write_next);

                let s_next = (mem_next, count_next, read_next, write_next).into();

                (fwd_o, bwd_i, s_next)
            },
        )
    })
    .build()
}

impl<I: Signal> UniChannel<Valid<I>> {
    /// Creates a buffer module with valid/ready channel as an input, which updates when valid.
    pub fn buffer_valid(self, k: &mut CompositeModuleContext, init: Expr<'static, I>) -> UniChannel<I> {
        let valid = self.clone().map(k, |input| input.valid);
        self.map(k, |input| input.inner).buffer_en(k, init, valid)
    }
}

impl<I: Signal> UniChannel<I> {
    /// Adds a buffer.
    #[must_use]
    pub fn buffer(self, k: &mut CompositeModuleContext, init: Expr<'static, I>) -> Self { self.comb_inline(k, m(init)) }

    /// Adds a buffer with enable signal.
    pub fn buffer_en(self, k: &mut CompositeModuleContext, init: Expr<'static, I>, en: UniChannel<bool>) -> Self {
        self.zip(k, en).comb_inline(k, m_en(init))
    }

    /// Adds a buffer with n-cycle delays.
    pub fn buffer_with_delay<const STAGES: usize>(
        self, k: &mut CompositeModuleContext, init: Expr<'static, I>, delay: usize,
    ) -> Self {
        self.comb_inline(k, m_buffer_cycles::<I, STAGES>(init, delay))
    }
}

impl<const N: usize> UniChannel<Bits<U<N>>> {
    /// Adds a buffer with set and clear signals.
    pub fn buffer_set_clear(
        k: &mut CompositeModuleContext, set: UniChannel<Bits<U<N>>>, clear: UniChannel<Bits<U<N>>>,
    ) -> Self {
        set.zip(k, clear).comb_inline(k, m_set_clear())
    }
}

impl<I: Signal, const P: Protocol> VrChannel<I, P> {
    /// Adds a buffer.
    ///
    /// It can receive data when the internal buffer is empty.
    pub fn buffer(self, k: &mut CompositeModuleContext) -> VrChannel<I> { self.comb_inline(k, m_vr()) }

    /// Adds a buffer.
    ///
    /// If the data can be received from the egress side, the data can also be received from the ingress side.
    pub fn buffer_always(self, k: &mut CompositeModuleContext) -> VrChannel<I> { self.comb_inline(k, m_vr_always()) }

    /// Adds a fifo.
    pub fn fifo<const N: usize>(self, k: &mut CompositeModuleContext) -> VrChannel<I> {
        self.comb_inline(k, m_fifo::<I, N, P>())
    }
}

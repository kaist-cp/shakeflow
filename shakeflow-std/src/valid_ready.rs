//! Utilities for valid-ready channels.

use std::marker::PhantomData;

use shakeflow_macro::Signal;

use crate::*;

/// Indicates the valid-ready channel's producer side is helpful or demanding.
///
/// ### Explanation
/// The meaning of Helpful/Demanding in shakeflow has subtle difference compared
/// to the original definition in Basejump paper.
///
/// In Basejump, Helpful/Demanding indicates whether there is dependency between
/// ready/valid signal calculation.
/// For example, if valid signal depends on the ready signal, then the producer channel is
/// Demanding according to Basejump's definition.
///
/// In shakeflow, protocol indicates the constraint for consumer module's ready calculation.
/// For example, if a channel's type is demanding, the consumer of this channel must be Helpful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Helpful. It offers the information up-front.
    ///
    /// If `Helpful`, ready signal of consumer can depend on the valid signal.
    Helpful,
    /// Demanding. It requires up-front information before asserting their output.
    ///
    /// If `Demanding`, ready signal of consumer should not depend on the valid signal. If so,
    /// there will be a combinational loop.
    Demanding,
}

/// Valid-ready channel.
#[derive(Debug)]
pub struct VrChannel<V: Signal, const P: Protocol = { Protocol::Helpful }> {
    endpoint: lir::Endpoint,
    _marker: PhantomData<V>,
}

impl<V: Signal, const P: Protocol> Interface for VrChannel<V, P> {
    type Bwd = Ready;
    type Fwd = Valid<V>;

    fn interface_typ() -> lir::InterfaceTyp {
        lir::InterfaceTyp::Channel(lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()))
    }

    fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError> {
        assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
        let channel = some_or!(interface.get_channel(), panic!());
        Ok(Self { endpoint: channel.endpoint(), _marker: PhantomData })
    }

    fn try_into_inner(self) -> Result<lir::Interface, InterfaceError> {
        Ok(lir::Interface::Channel(lir::Channel {
            typ: lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()),
            endpoint: self.endpoint,
        }))
    }
}

/// Valid/ready channel's forward exprs.
#[derive(Debug, Clone, Signal)]
pub struct Valid<V: Signal> {
    /// Inner data
    #[member(name = "")]
    pub inner: V,

    /// Valid bit
    pub valid: bool,
}

/// Ready signal.
#[derive(Debug, Clone, Signal)]
pub struct Ready {
    /// Ready bit
    pub ready: bool,
}

/// ValidExt
pub trait ValidExt<'id, V: Signal> {
    /// Creates a new expr.
    fn new(valid: Expr<'id, bool>, inner: Expr<'id, V>) -> Self;

    /// Creates a new array of exprs.
    fn new_arr<N: Num>(valid: Expr<'id, Bits<N>>, inner: Expr<'id, Array<V, N>>) -> Expr<'id, Array<Valid<V>, N>>;

    /// Creates an invalid expr.
    fn invalid() -> Expr<'id, Valid<V>>;

    /// Creates an valid expr.
    fn valid(inner: Expr<'id, V>) -> Expr<'id, Valid<V>>;

    /// Maps the inner value.
    fn map_inner<W: Signal>(self, f: fn(Expr<'id, V>) -> Expr<'id, W>) -> Expr<'id, Valid<W>>;

    /// Zips the inner value with other value.
    fn zip_inner<W: Signal>(self, other: Expr<'id, W>) -> Expr<'id, Valid<(V, W)>>;
}

impl<'id, V: Signal> ValidExt<'id, V> for Expr<'id, Valid<V>> {
    /// Creates a new expr.
    fn new(valid: Expr<'id, bool>, inner: Expr<'id, V>) -> Self { ValidProj { inner, valid }.into() }

    /// Creates a new array of exprs.
    fn new_arr<N: Num>(valid: Expr<'id, Bits<N>>, inner: Expr<'id, Array<V, N>>) -> Expr<'id, Array<Valid<V>, N>> {
        lir::Expr::Struct { inner: vec![(None, inner.into_inner()), (Some("valid".to_string()), valid.into_inner())] }
            .into()
    }

    /// Creates an invalid expr.
    fn invalid() -> Self { Self::new(false.into(), Expr::x()) }

    /// Creates an valid expr.
    fn valid(inner: Expr<'id, V>) -> Self { Self::new(true.into(), inner) }

    /// Maps the inner value.
    fn map_inner<W: Signal>(self, f: fn(Expr<'id, V>) -> Expr<'id, W>) -> Expr<'id, Valid<W>> {
        ValidProj { inner: f(self.inner), valid: self.valid }.into()
    }

    /// Zips the inner value with other value.
    fn zip_inner<W: Signal>(self, other: Expr<'id, W>) -> Expr<'id, Valid<(V, W)>> {
        ValidProj { inner: (self.inner, other).into(), valid: self.valid }.into()
    }
}

/// Ready Extension
pub trait ReadyExt<'id> {
    /// Creates a new expr.
    fn new(ready: Expr<'id, bool>) -> Self;

    /// Creates a new array of exprs.
    fn new_arr<N: Num>(ready: Expr<'id, Bits<N>>) -> Expr<'id, Array<Ready, N>>;
}

impl<'id> ReadyExt<'id> for Expr<'id, Ready> {
    /// Creates a new expr.
    fn new(ready: Expr<'id, bool>) -> Expr<'id, Ready> { ReadyProj { ready }.into() }

    /// Creates a new array of exprs.
    fn new_arr<N: Num>(ready: Expr<'id, Bits<N>>) -> Expr<'id, Array<Ready, N>> {
        lir::Expr::Struct { inner: vec![(Some("ready".to_string()), ready.into_inner())] }.into()
    }
}

fn m_split_map<
    I: Signal,
    O1: Signal,
    O2: Signal,
    F: 'static + for<'id> Fn(Expr<'id, I>) -> Expr<'id, (bool, O1, O2)>,
>(
    f: F,
) -> Module<VrChannel<I>, (VrChannel<O1>, VrChannel<O2>)> {
    composite::<VrChannel<I>, (VrChannel<O1>, VrChannel<O2>), _>("split_map", Some("in"), Some("out"), |input, k| {
        input.fsm::<(), (VrChannel<_>, VrChannel<_>), _>(k, None, ().into(), move |fwd, bwd, s| {
            let (branch, output1, output2) = *f(fwd.inner);

            let fwd1 = Expr::<Valid<_>>::new(fwd.valid & branch, output1);
            let fwd2 = Expr::<Valid<_>>::new(fwd.valid & !branch, output2);
            let bwd = Expr::<Ready>::new(branch.cond(bwd.0.ready, bwd.1.ready));

            ((fwd1, fwd2).into(), bwd, s)
        })
    })
    .build()
}

fn m_duplicate<I: Signal, const P: Protocol, const P1: Protocol, const P2: Protocol>(
) -> Module<VrChannel<I, P>, (VrChannel<I, P1>, VrChannel<I, P2>)> {
    // TODO: Check this in compile time
    assert!(P1 == Protocol::Demanding || P2 == Protocol::Demanding);

    composite::<VrChannel<I, P>, (VrChannel<I, P1>, VrChannel<I, P2>), _>(
        "duplicate",
        Some("in"),
        Some("out"),
        |input, k| {
            input.fsm::<(), (VrChannel<I, P1>, VrChannel<I, P2>), _>(k, None, ().into(), move |fwd, bwd, s| {
                // Projections.
                let (bwd0, bwd1) = *bwd;

                let fwd0: Expr<_> = ValidProj { inner: fwd.inner, valid: fwd.valid & bwd1.ready }.into();
                let fwd1: Expr<_> = ValidProj { inner: fwd.inner, valid: fwd.valid & bwd0.ready }.into();
                let bwd = ReadyProj { ready: bwd0.ready & bwd1.ready };

                ((fwd0, fwd1).into(), bwd.into(), s)
            })
        },
    )
    .build()
}

fn m_duplicate_any<I: Signal, const N: usize, const P: Protocol>() -> Module<VrChannel<I, P>, [VrChannel<I, P>; N]> {
    composite::<VrChannel<I, P>, [VrChannel<I, P>; N], _>("duplicate_any", Some("in"), Some("out"), |input, k| {
        input.fsm::<(), [VrChannel<I, P>; N], _>(k, None, ().into(), |ingress_fwd, egress_bwd, state| {
            let ingress_bwd = Expr::<Ready>::new(egress_bwd.map(|e| e.ready).any());
            let egress_fwd = ingress_fwd.repeat();
            (egress_fwd, ingress_bwd, state)
        })
    })
    .build()
}

fn m_duplicate_n<I: Signal, const N: usize, const P: Protocol>(
) -> Module<VrChannel<I, P>, [VrChannel<I, { Protocol::Demanding }>; N]> {
    composite::<VrChannel<I, P>, [VrChannel<I, { Protocol::Demanding }>; N], _>(
        "duplicate_n",
        Some("in"),
        Some("out"),
        |input, k| {
            input.fsm::<(), [VrChannel<I, { Protocol::Demanding }>; N], _>(k, None, ().into(), move |fwd, bwd, s| {
                let bwd_all_ready = bwd.map(|ready| ready.ready).all();

                let fwd_array =
                    fwd.repeat::<U<N>>().zip(bwd_all_ready.repeat()).map(|v| v.0.set_valid(v.0.valid & v.1));
                let bwd = Expr::<Ready>::new(bwd_all_ready);

                (fwd_array, bwd, s)
            })
        },
    )
    .build()
}

fn m_into_uni<V: Signal, const P: Protocol>(consuming: bool) -> Module<VrChannel<V, P>, UniChannel<Valid<V>>> {
    composite::<VrChannel<V, P>, UniChannel<Valid<V>>, _>("into_uni", Some("s_axis"), Some("m_axi"), |value, k| {
        value.fsm::<(), _, _>(k, None, ().into(), move |i_fwd, _o_bwd, state| {
            (i_fwd, Expr::<Ready>::new(consuming.into()), state)
        })
    })
    .build()
}

#[allow(clippy::type_complexity)]
fn m_clone_uni<V: Signal, const P: Protocol>() -> Module<VrChannel<V, P>, (VrChannel<V, P>, UniChannel<Valid<V>>)> {
    composite::<VrChannel<V, P>, (VrChannel<V, P>, UniChannel<Valid<V>>), _>(
        "clone_uni",
        Some("s_axis"),
        Some("m_axis"),
        |value, k| {
            value.fsm::<(), (VrChannel<V, P>, UniChannel<Valid<V>>), _>(
                k,
                None,
                ().into(),
                move |i_fwd, o_bwd, state| {
                    let o_fwd: Expr<_> = ValidProj { inner: i_fwd.inner, valid: i_fwd.valid }.into();
                    let i_bwd = o_bwd.0;

                    ((o_fwd, o_fwd).into(), i_bwd, state)
                },
            )
        },
    )
    .build()
}

impl<I: Signal, const P: Protocol> VrChannel<I, P> {
    /// Duplicates the inner signal.
    ///
    /// The ready signal for input channel is high when both ready signals for egress channels are high.
    ///
    /// NOTE: The protocol of returning VrChannel can have at most 1 Helpful type.
    /// The semantic of duplicate actually produces two Helpful channels according to Basejump's
    /// definition. But if we allow both egress channels to be Helpful, combinational loop will
    /// appear if both consumers of the egress has Demanding ingress channel. To prevent such case,
    /// we assert that there can be at most 1 Helpful channel as egress.
    pub fn duplicate<const P1: Protocol, const P2: Protocol>(
        self, k: &mut CompositeModuleContext,
    ) -> (VrChannel<I, P1>, VrChannel<I, P2>) {
        // TODO: Check this in compile time
        assert!(P1 == Protocol::Demanding || P2 == Protocol::Demanding);

        let (this, this_cloned) = self.comb_inline(k, m_duplicate());
        (this, this_cloned)
    }

    /// Duplicates the inner signal.
    ///
    /// TODO: Documentation
    pub fn duplicate_any<const N: usize>(self, k: &mut CompositeModuleContext) -> [VrChannel<I, P>; N] {
        self.comb_inline(k, m_duplicate_any())
    }

    /// Duplicates the inner signal.
    ///
    /// The ready signal for input channel is high when both ready signals for egress channels are high.
    ///
    /// TODO:
    /// Currently, `duplicate` & `duplicate_n` cannot be merged due to Helpful/Demanding Protocol.
    /// - `duplicate_n` looks at all the ready signals of N egress channels, so every egress channel is Demanding.
    /// - In contrast, `duplicate` can return at most 1 Helpful channel.
    /// - If we change the implementation of `duplicate_n` module to calculate valid signal of each
    ///   egress channel without looking at the valid signal of itself, we can allow at most one
    ///   Helpful channel. But to do this, the return type would become complicated.
    pub fn duplicate_n<const N: usize>(
        self, k: &mut CompositeModuleContext,
    ) -> [VrChannel<I, { Protocol::Demanding }>; N] {
        self.comb_inline(k, m_duplicate_n())
    }

    /// Returns fire signal and itself. (fire signal: valid & ready)
    ///
    /// TODO: Consider helpful/demanding for Unichannel.
    pub fn fire(self, k: &mut CompositeModuleContext) -> (Self, UniChannel<bool>) {
        let (fire, this) = self.fsm::<_, _, _>(k, Some("fire"), ().into(), move |fwd, bwd: Expr<((), Ready)>, s| {
            let bwd = bwd.1;

            let fwd_valid = fwd.valid;
            let bwd_ready = bwd.ready;

            let fire = fwd_valid & bwd_ready;

            ((fire, fwd).into(), bwd, s)
        });
        (this, fire)
    }

    /// TODO: Documentation
    pub fn transfer(self, k: &mut CompositeModuleContext, ready_next: UniChannel<bool>) -> UniChannel<Valid<I>> {
        self.zip_uni(k, ready_next).fsm::<_, UniChannel<Valid<I>>, _>(
            k,
            Some("transfer"),
            Expr::from(false),
            move |fwd, _, s| {
                let fwd_valid = fwd.valid;
                let (fwd_inner, ready_next) = *fwd.inner;

                let fwd_ready = s;

                let is_transfer = fwd_valid & fwd_ready;

                let s_next = is_transfer.cond(false.into(), ready_next);

                (Expr::<Valid<_>>::new(is_transfer, fwd_inner), Expr::<Ready>::new(fwd_ready), s_next)
            },
        )
    }

    /// Runs an FSM from the ingress valid/ready channel.
    ///
    /// This method lets you 'accumulate' info from successive input signals to the internal state
    /// until it is ready to be transmitted, in which case `f()`'s `done` boolean output should return true.
    ///
    /// ### Detailed explanation about `done`
    ///
    /// If `done` is true,
    /// 1. egress_fwd's `valid` signal is asserted, transmitting the output.
    /// 2. ingress_bwd's `ready` signal is deasserted, blocking the input.
    /// 3. `done` is deasserted in the next cycle, unblocking the input.
    ///
    /// This implementation has the advantage of:
    /// - Being able to transmit info accumulated from a stream of input signals
    ///   as soon as you have acquired the required info, even if the input transmission is not finished.
    /// - Being able to block ingress signals if the fsm is ready to transmit to output but the egress's ready is deasserted.
    ///
    /// ### Example:
    ///
    /// Suppose having to parse the header from a frame consisting of 5 consecutive packets, P1-P5.
    ///
    /// Only the packets P1-P2 contain info about the header, and the remaining three packets can be dropped.
    ///
    /// The following is the timeline upon receiving each of the packets:
    ///
    /// (We assume the ready signal of the egress is always true.)
    ///
    /// - Initially: `done` is false.
    /// - P1: Since `done` is false, the fsm takes the path `ingress_transfer`,
    ///       i.e. the fsm reads the ingress and computes the next cycle's states.
    ///       `f()` is implemented so that `done` is false.
    /// - P2: Since `done` is false, the fsm takes the path `ingress_transfer`.
    ///       `f()` is implemented so that `done` is TRUE.
    /// - P3: Since `done` is TRUE, the fsm takes the path `egress_transfer`,
    ///       i.e. the fsm spends the cycle outputting the current state to egress.
    ///       The ingress packet P3 is not read because the `ready` is deasserted.
    ///       The path `egress_transfer` sets `done` to false.
    /// - P3 again, and onwards: `done` is false, and the fsm keeps receiving packets
    ///                          until `f()` computes `done` to be TRUE again.
    ///
    pub fn fsm_ingress<
        S: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, S>, Expr<'id, bool>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> VrChannel<S, P> {
        self.fsm::<(S, bool), VrChannel<S, P>, _>(
            k,
            module_name.or(Some("fsm_ingress")),
            (init, false.into()).into(),
            move |ingress_fwd, egress_bwd, state| {
                let (state, done) = *state;
                let ingress_bwd = Expr::<Ready>::new(!done);
                let ingress_transfer = ingress_fwd.valid & !done;
                let egress_transfer = done & egress_bwd.ready;
                let state_next = select! {
                    ingress_transfer => f(ingress_fwd.inner, state).into(),
                    egress_transfer => (state, false.into()).into(),
                    default => (state, done).into(),
                };
                (Expr::<Valid<_>>::new(done, state), ingress_bwd, state_next)
            },
        )
    }

    /// TODO: Remove this
    pub fn fsm_fwd<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, Valid<I>>, Expr<'id, S>) -> (Expr<'id, Valid<O>>, Expr<'id, S>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> VrChannel<O, P> {
        self.fsm::<_, VrChannel<_, P>, _>(k, module_name.or(Some("fsm_fwd")), init, move |fwd, bwd, s| {
            let (fwd_next, s_next) = f(fwd, s);
            let ready = bwd.ready;
            (fwd_next, bwd, ready.cond(s_next, s))
        })
    }

    /// TODO: documentation
    ///
    /// The state is updated only when `ready` is asserted.
    ///
    /// Once the resulting channel's output becomes valid, it must remain the same until it's
    /// transferred.
    pub fn map_fwd<O: Signal, F: 'static + for<'id> Fn(Expr<'id, Valid<I>>) -> Expr<'id, Valid<O>>>(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, f: F,
    ) -> VrChannel<O, P> {
        self.fsm_fwd(k, module_name.or(Some("map_fwd")), ().into(), move |i, s| (f(i), s))
    }

    /// TODO: documentation
    pub fn fsm_and_then<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, Valid<O>>, Expr<'id, S>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> VrChannel<O, { Protocol::Demanding }> {
        self.fsm::<_, VrChannel<_, { Protocol::Demanding }>, _>(
            k,
            module_name.or(Some("fsm_and_then")),
            init,
            move |ingress_fwd, egress_bwd, state| {
                let ingress_bwd = egress_bwd;
                let ingress_transfer = ingress_fwd.valid & ingress_bwd.ready;
                // XXX: `egress_fwd` should be updated even if `ready` is not high?
                let (egress_fwd, state_next) = *select! {
                    ingress_transfer => f(ingress_fwd.inner, state).into(),
                    default => (Expr::invalid(), state).into(),
                };
                (egress_fwd, ingress_bwd, state_next)
            },
        )
    }

    /// TODO: Documentation
    pub fn and_then<O: Signal, F: 'static + for<'id> Fn(Expr<'id, I>) -> Expr<'id, Valid<O>>>(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, f: F,
    ) -> VrChannel<O, { Protocol::Demanding }> {
        self.fsm_and_then(k, module_name.or(Some("and_then")), ().into(), move |i, s| (f(i), s))
    }

    /// TODO: documentation
    ///
    /// Once `cond` is asserted, it must remain so until `self` is transferred. This is necessary to
    /// satisfy `zip_uni`'s requirement.
    #[must_use]
    pub fn filter_bwd(self, k: &mut CompositeModuleContext, cond: UniChannel<bool>) -> VrChannel<I, P> {
        self.zip_uni(k, cond).fsm::<(), Self, _>(k, Some("filter_bwd"), Expr::from(()), move |i_fwd, o_bwd, s| {
            let i_valid = i_fwd.valid;
            let (inner, cond) = *i_fwd.inner;
            let o_ready = o_bwd.ready;

            (Expr::<Valid<_>>::new(i_valid, inner), Expr::<Ready>::new(o_ready & cond), s)
        })
    }

    /// TODO: Documentation
    #[must_use]
    pub fn filter_fwd_ready(self, k: &mut CompositeModuleContext) -> VrChannel<I, { Protocol::Demanding }> {
        self.fsm::<(), VrChannel<I, { Protocol::Demanding }>, _>(
            k,
            Some("filter_fwd_ready"),
            Expr::from(()),
            |fwd, bwd, s| {
                let mut fwd_next = *fwd;
                fwd_next.valid = fwd_next.valid & bwd.ready;
                (fwd_next.into(), bwd, s)
            },
        )
    }

    /// TODO: Documentation
    #[must_use]
    pub fn filter_fwd_ready_neg(self, k: &mut CompositeModuleContext) -> VrChannel<I, { Protocol::Demanding }> {
        self.fsm::<(), VrChannel<I, { Protocol::Demanding }>, _>(
            k,
            Some("filter_fwd_ready_neg"),
            Expr::from(()),
            |fwd, bwd, s| {
                let mut fwd_next = *fwd;
                fwd_next.valid = fwd_next.valid & !bwd.ready;
                (fwd_next.into(), bwd, s)
            },
        )
    }

    /// Zips a valid-ready channel with an unidirectional channel.
    ///
    /// Once the given channel's output becomes valid, the `other` channel must remain the same until it's
    /// transferred. It's required by the valid-ready protocol.
    pub fn zip_uni<J: Signal>(self, k: &mut CompositeModuleContext, other: UniChannel<J>) -> VrChannel<(I, J), P> {
        (self, other).fsm::<(), VrChannel<(I, J), P>, _>(k, Some("zip_uni"), ().into(), |fwd, bwd, state| {
            let (fwd_i, fwd_j) = *fwd;
            let fwd_i = fwd_i;
            let fwd = ValidProj { valid: fwd_i.valid, inner: (fwd_i.inner, fwd_j).into() };
            (fwd.into(), (bwd, ().into()).into(), state)
        })
    }

    /// Transforms into a unidirectional channel.
    pub fn into_uni(self, k: &mut CompositeModuleContext, consuming: bool) -> UniChannel<Valid<I>> {
        self.comb_inline(k, m_into_uni(consuming))
    }

    /// Transforms into data-deque channel.
    ///
    /// TODO: Consider helpful/demanding for deque channel.
    pub fn into_deq(self, k: &mut CompositeModuleContext) -> DeqChannel<I> {
        self.fsm::<(), DeqChannel<I>, _>(k, Some("into_deq"), ().into(), |fwd_i, bwd_o, s| {
            let fwd_o = fwd_i.inner;
            let bwd_i = Expr::<Ready>::new(bwd_o.deque);

            (fwd_o, bwd_i, s)
        })
    }

    /// Clones into a unidirectional channel with no `Bwd` expr.
    pub fn clone_uni(self, k: &mut CompositeModuleContext) -> (Self, UniChannel<Valid<I>>) {
        let (this, this_cloned) = self.comb_inline(k, m_clone_uni());
        (this, this_cloned)
    }

    /// Sinks the valid-ready channel by consuming.
    pub fn sink(self, k: &mut CompositeModuleContext) { let _ = self.into_uni(k, true); }

    /// Blocks the valid-ready channel without consuming.
    pub fn block(self, k: &mut CompositeModuleContext) { let _ = self.comb_inline(k, m_into_uni(false)); }

    /// Receives until `cond` is low. If `cond` becomes high, then turn on valid signal.
    pub fn receive_until(self, k: &mut CompositeModuleContext, cond: UniChannel<bool>) -> VrChannel<I> {
        (self, cond).fsm::<(), VrChannel<I>, _>(k, None, ().into(), |fwd_i, bwd_o, s| {
            let (fwd_i, cond) = *fwd_i;

            let fwd_o = fwd_i.set_valid(fwd_i.valid & cond);
            let bwd_i = (Expr::<Ready>::new(bwd_o.ready | !cond), ().into()).into();

            (fwd_o, bwd_i, s)
        })
    }
}

impl<I: Signal> VrChannel<I> {
    /// Generates unit values indefinitely.
    pub fn source(k: &mut CompositeModuleContext, value: Expr<'static, I>) -> Self {
        ().fsm(k, Some("source"), ().into(), move |_fwd, _bwd, state| {
            (ValidProj { inner: value, valid: true.into() }.into(), ().into(), state)
        })
    }

    /// TODO: documentation
    pub fn split_map<O1: Signal, O2: Signal, F: 'static + for<'id> Fn(Expr<'id, I>) -> Expr<'id, (bool, O1, O2)>>(
        self, k: &mut CompositeModuleContext, f: F,
    ) -> (VrChannel<O1>, VrChannel<O2>) {
        self.comb_inline(k, m_split_map(f))
    }

    /// TODO: documentation
    pub fn filter_map<O: Signal, F: 'static + for<'id> Fn(Expr<'id, I>) -> Expr<'id, (bool, O)>>(
        self, k: &mut CompositeModuleContext, f: F,
    ) -> VrChannel<O> {
        let (take, drop) = self.split_map(k, move |i| {
            let (is_valid, output) = *f(i);
            (is_valid, output, ().into()).into()
        });
        drop.sink(k);
        take
    }

    /// TODO: documentation
    pub fn assert_map<O: Signal, F: 'static + for<'id> Fn(Expr<'id, I>) -> Expr<'id, (bool, O)>>(
        self, k: &mut CompositeModuleContext, f: F,
    ) -> VrChannel<O> {
        let (take, drop) = self.split_map(k, move |i| {
            let (is_valid, output) = *f(i);
            (is_valid, output, ().into()).into()
        });
        drop.block(k);
        take
    }

    /// TODO: documentation
    #[must_use]
    pub fn filter<F: 'static + for<'id> Fn(Expr<'id, I>) -> Expr<'id, bool>>(
        self, k: &mut CompositeModuleContext, f: F,
    ) -> VrChannel<I> {
        self.filter_map(k, move |i| (f(i), i).into())
    }

    /// TODO: Documentation
    #[must_use]
    pub fn filter_bwd_valid(self, k: &mut CompositeModuleContext) -> VrChannel<I> {
        self.fsm::<(), Self, _>(k, Some("filter_bwd_valid"), Expr::from(()), |ingress_fwd, egress_bwd, s| {
            let ingress_bwd = ReadyProj { ready: (ingress_fwd.valid & egress_bwd.ready) }.into();
            (ingress_fwd, ingress_bwd, s)
        })
    }

    /// Zips a valid-ready channel with another valid-ready channel.
    pub fn zip_vr<J: Signal, const P: Protocol>(
        self, k: &mut CompositeModuleContext, other: VrChannel<J, P>,
    ) -> VrChannel<(I, J), P> {
        (self, other).fsm::<(), VrChannel<(I, J), P>, _>(k, Some("zip_vr"), ().into(), |fwd, bwd, state| {
            // Projections.
            let (fwd_i, fwd_j) = *fwd;

            let fwd = Expr::<Valid<_>>::new(fwd_i.valid & fwd_j.valid, (fwd_i.inner, fwd_j.inner).into());
            let bwd = Expr::<Ready>::new(bwd.ready);

            (fwd, (bwd, bwd).into(), state)
        })
    }

    /// Runs an FSM from the egress valid/ready channel.
    pub fn fsm_egress<
        S: Signal,
        O: Signal,
        F: 'static + for<'id> Fn(Expr<'id, I>, Expr<'id, S>) -> (Expr<'id, O>, Expr<'id, S>, Expr<'id, bool>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> VrChannel<O> {
        self.fsm::<S, VrChannel<O>, _>(
            k,
            module_name.or(Some("fsm_egress")),
            init,
            move |ingress_fwd, egress_bwd, state| {
                let (egress_fwd, state_next, last) = f(ingress_fwd.inner, state);
                let egress_transfer = ingress_fwd.valid & egress_bwd.ready;
                let ingress_transfer = egress_transfer & last;
                let egress_fwd = Expr::<Valid<_>>::new(ingress_fwd.valid, egress_fwd);
                let ingress_bwd = Expr::<Ready>::new(egress_bwd.ready & last);
                let state_next = select! {
                    ingress_transfer => init,
                    egress_transfer => state_next,
                    default => state,
                };
                (egress_fwd, ingress_bwd, state_next)
            },
        )
    }
}

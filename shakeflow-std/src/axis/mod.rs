//! AXI4 interface.

use std::marker::PhantomData;

use shakeflow_macro::Signal;

use crate::*;

mod chunk;
mod fifo;
mod rr_mux;
mod split;

pub use fifo::AxisFifoExt;
pub use rr_mux::AxisRrMuxExt;

/// Buffer with TKEEP expr.
#[derive(Debug, Clone, Signal)]
pub struct Keep<WIDTH: Num, KWIDTH: Num> {
    /// AXI4-Stream TDATA
    pub tdata: Bits<WIDTH>,

    /// AXI4-Stream TKEEP
    pub tkeep: Bits<KWIDTH>,
}

/// AXI4-Stream inner data.
///
/// See [Xilinx PG085](https://www.xilinx.com/support/documentation/ip_documentation/axis_infrastructure_ip_suite/v1_1/pg085-axi4stream-infrastructure.pdf) for more details.
#[derive(Debug, Clone, Signal)]
pub struct AxisValue<Payload: Signal> {
    /// AXI4-Stream TDATA/TKEEP
    #[member(name = "")]
    pub payload: Payload,

    /// AXI4-Stream TLAST
    pub tlast: bool,
}

/// Valid/ready channel's forward exprs.
///
/// # Note
///
/// Try to use `Valid<V>` when the valid bit is not used as TVALID expr of AXI4-Stream protocol.
#[derive(Debug, Clone, Signal)]
pub struct AxisValid<V: Signal> {
    /// Inner data
    #[member(name = "")]
    pub inner: V,

    /// AXI4-Stream TVALID
    pub tvalid: bool,
}

/// Valid/ready channel's backward channels.
///
/// # Note
///
/// Try to use `Ready<V>` when the ready bit is not used as TREADY expr of AXI4-Stream protocol.
#[derive(Debug, Clone, Signal)]
pub struct AxisReady {
    /// AXI4-Stream TREADY
    pub tready: bool,
}

/// AxisValid Extension
pub trait AxisValidExt<'id, V: Signal> {
    /// Creates an invalid expr.
    fn tinvalid() -> Expr<'id, AxisValid<V>>;
}

impl<'id, V: Signal> AxisValidExt<'id, V> for Expr<'id, AxisValid<V>> {
    /// Creates an invalid expr.
    fn tinvalid() -> Expr<'id, AxisValid<V>> { AxisValidProj { inner: Expr::x(), tvalid: false.into() }.into() }
}

/// AXIs valid-ready channel.
// TODO: why not macro?
#[derive(Debug)]
pub struct AxisVrChannel<V: Signal, const P: Protocol = { Protocol::Helpful }> {
    endpoint: lir::Endpoint,
    _marker: PhantomData<V>,
}

impl<V: Signal, const P: Protocol> Interface for AxisVrChannel<V, P> {
    type Bwd = AxisReady;
    type Fwd = AxisValid<V>;

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

/// AXI4-Stream channel.
pub type AxisChannel<Data, const P: Protocol = { Protocol::Helpful }> = AxisVrChannel<AxisValue<Data>, P>;

/// AXI Value Extention
pub trait AxisValueExt<'id, V: Signal> {
    /// Maps the inner value.
    fn map<W: Signal, F: Fn(Expr<'id, V>) -> Expr<'id, W>>(self, f: F) -> Expr<'id, AxisValue<W>>;
}

impl<'id, V: Signal> AxisValueExt<'id, V> for Expr<'id, AxisValue<V>> {
    /// Maps the inner value.
    fn map<W: Signal, F: Fn(Expr<'id, V>) -> Expr<'id, W>>(self, f: F) -> Expr<'id, AxisValue<W>> {
        AxisValueProj { payload: f(self.payload), tlast: self.tlast }.into()
    }
}

impl<V: Signal, const P: Protocol> VrChannel<V, P> {
    /// Converts into axis valid/ready channel.
    pub fn into_axis_vr(self, k: &mut CompositeModuleContext) -> AxisVrChannel<V, P> {
        self.fsm::<(), AxisVrChannel<V, P>, _>(k, Some("into_axis_vr"), Expr::x(), move |fwd, bwd, s| {
            let ValidProj { inner, valid } = *fwd;
            let AxisReadyProj { tready } = *bwd;

            (AxisValidProj { inner, tvalid: valid }.into(), ReadyProj { ready: tready }.into(), s)
        })
    }
}

impl<V: Signal, const P: Protocol> AxisVrChannel<V, P> {
    /// Converts into valid/ready channel.
    pub fn into_vr(self, k: &mut CompositeModuleContext) -> VrChannel<V, P> {
        self.fsm::<(), VrChannel<V, P>, _>(k, Some("into_vr"), Expr::x(), move |fwd, bwd, s| {
            let AxisValidProj { inner, tvalid } = *fwd;
            let ReadyProj { ready } = *bwd;

            (Expr::<Valid<_>>::new(tvalid, inner), AxisReadyProj { tready: ready }.into(), s)
        })
    }

    /// Duplicates the inner signal.
    ///
    /// The ready signal for input channel is high when both ready signals for output channels are high.
    // TODO: support duplicate to N channels
    pub fn duplicate<const P1: Protocol, const P2: Protocol>(
        self, k: &mut CompositeModuleContext,
    ) -> (AxisVrChannel<V, P1>, AxisVrChannel<V, P2>) {
        // TODO: Check this in compile time
        assert!(P1 == Protocol::Demanding || P2 == Protocol::Demanding);

        let this = self.into_vr(k);
        let (this, this_cloned) = this.duplicate(k);
        (this.into_axis_vr(k), this_cloned.into_axis_vr(k))
    }
}

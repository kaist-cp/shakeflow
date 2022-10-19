use std::fmt::Debug;
use std::ops::*;

use arrayvec::ArrayVec;
use linked_hash_map::LinkedHashMap;
use thiserror::Error;
use tuple_utils::*;

use crate::hir::*;
use crate::lir;

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum InterfaceError {
    #[error("more channels are required")]
    NoChannels,
    #[error("there are too many channels")]
    TooManyChannels,
}

/// Interface of channels.
pub trait Interface: 'static + Sized + Debug {
    /// Forward exprs.
    type Fwd: Signal;

    /// Backward exprs.
    type Bwd: Signal;

    /// Returns the interface type.
    fn interface_typ() -> lir::InterfaceTyp;

    /// Tries to convert `lir::Interface` to `Self`.
    fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError>;

    /// Tries to convert `self` to `lir::Interface`.
    fn try_into_inner(self) -> Result<lir::Interface, InterfaceError>;

    /// Chains `self` as input to a new module, and returns the module's output.
    fn comb_inline<O: Interface>(self, k: &mut CompositeModuleContext, module: Module<Self, O>) -> O {
        // Adds submodule.
        let input_interface = self.try_into_inner().expect("internal compiler error");
        let output_interface = k.inner.add_submodule(module.inner, input_interface);

        // Converts output interface.
        O::try_from_inner(output_interface).expect("internal compiler error")
    }

    /// Feeds `self` to a new FSM.
    ///
    /// The FSM is described by `F`, which generates the circuit for (1) the current-cycle output; and (2) the next-cycle state.
    fn fsm<
        S: Signal,
        O: 'static + Interface,
        F: 'static
            + for<'id> Fn(
                Expr<'id, Self::Fwd>,
                Expr<'id, O::Bwd>,
                Expr<'id, S>,
            ) -> (Expr<'id, O::Fwd>, Expr<'id, Self::Bwd>, Expr<'id, S>),
    >(
        self, k: &mut CompositeModuleContext, module_name: Option<&str>, init: Expr<'static, S>, f: F,
    ) -> O {
        self.comb_inline(k, Fsm::new(module_name.unwrap_or("fsm"), f, init).into())
    }

    /// Feeds `self` to a new Module instantiation.
    // TODO: Remove this
    #[allow(clippy::too_many_arguments)]
    fn module_inst<O: Interface>(
        self, k: &mut CompositeModuleContext, module_name: &str, inst_name: &str, params: Vec<(&str, usize)>,
        has_clk: bool, input_prefix: Option<&str>, output_prefix: Option<&str>,
    ) -> O {
        let module_inst = ModuleInst::new(
            module_name.to_string(),
            inst_name.to_string(),
            params.into_iter().map(|(s, w)| (s.to_string(), w)).collect(),
            has_clk,
            input_prefix.map(String::from),
            output_prefix.map(String::from),
            None,
        );
        self.comb_inline(k, module_inst.into())
    }

    /// Module instantiation api for shakeflow modules
    fn comb<O: Interface>(
        self, k: &mut CompositeModuleContext, inst_postfix: Option<&str>, shakeflow_module: Module<Self, O>,
    ) -> O {
        assert!(
            !matches!(&*shakeflow_module.inner.inner, lir::ModuleInner::VirtualModule(_)),
            "Use comb_inline to comb Virtual Module"
        );
        self.comb_inline(k, ModuleInst::from_module(inst_postfix, shakeflow_module).into())
    }
}

impl Interface for () {
    type Bwd = ();
    type Fwd = ();

    fn interface_typ() -> lir::InterfaceTyp { lir::InterfaceTyp::Unit }

    fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError> {
        assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
        Ok(())
    }

    fn try_into_inner(self) -> Result<lir::Interface, InterfaceError> { Ok(lir::Interface::Unit) }
}

/// Macro for decraring custom channel
///
/// TODO: auto document generation using channel name?
#[macro_export]
macro_rules! channel {
    (
        $channel_name: ident,
        $fwd: tt,
        $bwd: tt
    ) => {
        #[derive(Debug)]
        pub struct $channel_name {
            endpoint: ::shakeflow::lir::Endpoint,
        }
        impl ::shakeflow::Interface for $channel_name {
            type Bwd = $bwd;
            type Fwd = $fwd;

            fn interface_typ() -> ::shakeflow::lir::InterfaceTyp {
                lir::InterfaceTyp::Channel(lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()))
            }

            fn try_from_inner(interface: lir::Interface) -> Result<Self, ::shakeflow::InterfaceError> {
                assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
                let channel = some_or!(interface.get_channel(), panic!());
                Ok(Self { endpoint: channel.endpoint() })
            }

            fn try_into_inner(self) -> Result<::shakeflow::lir::Interface, ::shakeflow::InterfaceError> {
                Ok(::shakeflow::lir::Interface::Channel(::shakeflow::lir::Channel {
                    typ: ::shakeflow::lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()),
                    endpoint: self.endpoint,
                }))
            }
        }
    };
    (
        $channel_name: ident <$value_generic:ident: Signal>,
        $fwd: tt,
        $bwd: tt
    ) => {
        #[derive(Debug)]
        #[allow(missing_docs)]
        pub struct $channel_name<$value_generic: Signal> {
            endpoint: ::shakeflow::lir::Endpoint,
            _marker: ::std::marker::PhantomData<$value_generic>,
        }
        impl<$value_generic: ::shakeflow::Signal> ::shakeflow::Interface for $channel_name<$value_generic> {
            type Bwd = $bwd;
            type Fwd = $fwd;

            fn interface_typ() -> ::shakeflow::lir::InterfaceTyp {
                lir::InterfaceTyp::Channel(lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()))
            }

            fn try_from_inner(interface: lir::Interface) -> Result<Self, ::shakeflow::InterfaceError> {
                assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
                let channel = some_or!(interface.get_channel(), panic!());
                Ok(Self { endpoint: channel.endpoint(), _marker: ::std::marker::PhantomData })
            }

            fn try_into_inner(self) -> Result<::shakeflow::lir::Interface, ::shakeflow::InterfaceError> {
                Ok(::shakeflow::lir::Interface::Channel(::shakeflow::lir::Channel {
                    typ: ::shakeflow::lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()),
                    endpoint: self.endpoint,
                }))
            }
        }
    };
    (
        $channel_name: ident <$value_generic:ident: Signal>,
        $fwd: tt,
        $bwd: ident<$bwd_generic: ident>
    ) => {
        #[derive(Debug)]
        pub struct $channel_name<$value_generic: Signal> {
            endpoint: ::shakeflow::lir::Endpoint,
            _marker: ::std::marker::PhantomData<$value_generic>,
        }
        impl<$value_generic: ::shakeflow::Signal> ::shakeflow::Interface for $channel_name<$value_generic> {
            type Bwd = $bwd<$bwd_generic>;
            type Fwd = $fwd;

            fn interface_typ() -> ::shakeflow::lir::InterfaceTyp {
                lir::InterfaceTyp::Channel(lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()))
            }

            fn try_from_inner(interface: lir::Interface) -> Result<Self, ::shakeflow::InterfaceError> {
                assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
                let channel = some_or!(interface.get_channel(), panic!());
                Ok(Self { endpoint: channel.endpoint(), _marker: ::std::marker::PhantomData })
            }

            fn try_into_inner(self) -> Result<::shakeflow::lir::Interface, ::shakeflow::InterfaceError> {
                Ok(::shakeflow::lir::Interface::Channel(::shakeflow::lir::Channel {
                    typ: ::shakeflow::lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()),
                    endpoint: self.endpoint,
                }))
            }
        }
    };
    (
        $channel_name: ident <$value_generic:ident: Signal>,
        $fwd: ident<$fwd_generic: ident>,
        $bwd: tt
    ) => {
        #[derive(Debug)]
        pub struct $channel_name<$value_generic: Signal> {
            endpoint: ::shakeflow::lir::Endpoint,
            _marker: ::std::marker::PhantomData<$value_generic>,
        }
        impl<$value_generic: ::shakeflow::Signal> ::shakeflow::Interface for $channel_name<$value_generic> {
            type Bwd = $bwd;
            type Fwd = $fwd<$fwd_generic>;

            fn interface_typ() -> ::shakeflow::lir::InterfaceTyp {
                lir::InterfaceTyp::Channel(lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()))
            }

            fn try_from_inner(interface: lir::Interface) -> Result<Self, ::shakeflow::InterfaceError> {
                assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
                let channel = some_or!(interface.get_channel(), panic!());
                Ok(Self { endpoint: channel.endpoint(), _marker: ::std::marker::PhantomData })
            }

            fn try_into_inner(self) -> Result<::shakeflow::lir::Interface, ::shakeflow::InterfaceError> {
                Ok(::shakeflow::lir::Interface::Channel(::shakeflow::lir::Channel {
                    typ: ::shakeflow::lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()),
                    endpoint: self.endpoint,
                }))
            }
        }
    };
    (
        $channel_name: ident <$value_generic:ident: Signal>,
        $fwd: tt <$fwd_generic: ident>,
        $bwd: tt <$bwd_generic: ident>
    ) => {
        #[derive(Debug)]
        pub struct $channel_name<$value_generic: Signal> {
            endpoint: ::shakeflow::lir::Endpoint,
            _marker: ::std::marker::PhantomData<$value_generic>,
        }
        impl<$value_generic: ::shakeflow::Signal> ::shakeflow::Interface for $channel_name<$value_generic> {
            type Bwd = $bwd<$bwd_generic>;
            type Fwd = $fwd<$fwd_generic>;

            fn interface_typ() -> ::shakeflow::lir::InterfaceTyp {
                lir::InterfaceTyp::Channel(lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()))
            }

            fn try_from_inner(interface: lir::Interface) -> Result<Self, ::shakeflow::InterfaceError> {
                assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
                let channel = some_or!(interface.get_channel(), panic!());
                Ok(Self { endpoint: channel.endpoint(), _marker: ::std::marker::PhantomData })
            }

            fn try_into_inner(self) -> Result<::shakeflow::lir::Interface, ::shakeflow::InterfaceError> {
                Ok(::shakeflow::lir::Interface::Channel(::shakeflow::lir::Channel {
                    typ: ::shakeflow::lir::ChannelTyp::new(Self::Fwd::port_decls(), Self::Bwd::port_decls()),
                    endpoint: self.endpoint,
                }))
            }
        }
    };
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! impl_interface_tuple {
    ($a:ident) => {
        impl<$a: Interface> Interface for ($a,) {
            type Fwd = ($a::Fwd,);
            type Bwd = ($a::Bwd,);

            fn interface_typ() -> lir::InterfaceTyp {
                let mut inner = LinkedHashMap::new();
                inner.insert("0".to_string(), (None, $a::interface_typ()));
                lir::InterfaceTyp::Struct(inner)
            }

            fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError> {
                match interface {
                    lir::Interface::Struct(mut inner) => {
                        let b = inner.remove("0").unwrap().1;
                        assert!(inner.is_empty(), "internal compiler error");
                        Ok(($a::try_from_inner(b)?,))
                    }
                    _ => panic!("internal compiler error"),
                }
            }

            fn try_into_inner(self) -> Result<lir::Interface, InterfaceError> {
                let mut inner = LinkedHashMap::new();
                inner.insert("0".to_string(), (None, self.0.try_into_inner()?));
                Ok(lir::Interface::Struct(inner))
            }
        }
    };
    ($($a:ident)+) => {
        impl<$($a: Interface,)+> Interface for ($($a,)+) {
            type Fwd = ($($a::Fwd,)+);
            type Bwd = ($($a::Bwd,)+);

            fn interface_typ() -> lir::InterfaceTyp {
                match <<Self as SplitLast>::Left as Interface>::interface_typ() {
                    lir::InterfaceTyp::Struct(mut inner) => {
                        inner.insert(
                            (Self::arity() - 1).to_string(),
                            (None, <<Self as SplitLast>::Right as Interface>::interface_typ()),
                        );
                        lir::InterfaceTyp::Struct(inner)
                    }
                    _ => panic!("internal compiler error"),
                }
            }

            fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError> {
                match interface {
                    lir::Interface::Struct(mut inner) => {
                        let right = inner.remove(&(Self::arity() - 1).to_string()).unwrap().1;

                        Ok(
                            <<Self as SplitLast>::Left as Interface>::try_from_inner(lir::Interface::Struct(inner))?
                                .push_back(<<Self as SplitLast>::Right as Interface>::try_from_inner(right)?),
                        )
                    }
                    _ => panic!("internal compiler error"),
                }
            }

            fn try_into_inner(self) -> Result<lir::Interface, InterfaceError> {
                let (left, right) = SplitLast::split_last(self);

                match left.try_into_inner()? {
                    lir::Interface::Struct(mut inner) => {
                        inner.insert((Self::arity() - 1).to_string(), (None, right.try_into_inner()?));
                        Ok(lir::Interface::Struct(inner))
                    }
                    _ => panic!("internal compiler error"),
                }
            }
        }
    }
}

impl_interface_tuple! { B1 }
impl_interface_tuple! { B1 B2 }
impl_interface_tuple! { B1 B2 B3 }
impl_interface_tuple! { B1 B2 B3 B4 }
impl_interface_tuple! { B1 B2 B3 B4 B5 }
impl_interface_tuple! { B1 B2 B3 B4 B5 B6 }
impl_interface_tuple! { B1 B2 B3 B4 B5 B6 B7 }
impl_interface_tuple! { B1 B2 B3 B4 B5 B6 B7 B8 }
impl_interface_tuple! { B1 B2 B3 B4 B5 B6 B7 B8 B9 }
impl_interface_tuple! { B1 B2 B3 B4 B5 B6 B7 B8 B9 B10 }
impl_interface_tuple! { B1 B2 B3 B4 B5 B6 B7 B8 B9 B10 B11 }
impl_interface_tuple! { B1 B2 B3 B4 B5 B6 B7 B8 B9 B10 B11 B12 }

impl<B: Interface, const N: usize> Interface for [B; N] {
    type Bwd = Array<B::Bwd, U<N>>;
    type Fwd = Array<B::Fwd, U<N>>;

    fn interface_typ() -> lir::InterfaceTyp { lir::InterfaceTyp::Array(Box::new(B::interface_typ()), N) }

    fn try_from_inner(interface: lir::Interface) -> Result<Self, InterfaceError> {
        assert_eq!(interface.typ(), Self::interface_typ(), "internal compiler error");
        if let lir::Interface::Array(interfaces) = interface {
            Ok(interfaces
                .into_iter()
                .map(|interface| B::try_from_inner(interface).unwrap())
                .collect::<ArrayVec<B, N>>()
                .into_inner()
                .unwrap()) // this should be successful because there should be `N` elements.
        } else {
            panic!("internal compiler error");
        }
    }

    fn try_into_inner(self) -> Result<lir::Interface, InterfaceError> {
        Ok(lir::Interface::Array(self.into_iter().map(|interface| interface.try_into_inner().unwrap()).collect()))
    }
}

//! Composite module.
//!
//! For more details, see "Section 2.3 Composite Module" in the paper.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;

use crate::*;

/// Composite module.
#[derive(Debug, Clone)]
pub struct CompositeModule<I: Interface, O: Interface> {
    /// Inner of composite module.
    pub inner: lir::CompositeModule,
    _marker: PhantomData<(I, O)>,
}

/// Composite module context used in map functions.
#[derive(Debug)]
pub struct CompositeModuleContext {
    pub(crate) inner: lir::CompositeModule,
}

impl CompositeModuleContext {
    /// Register given module to the context, and return virtual module with same I/O interfaces
    pub fn register<I: Interface, O: Interface>(
        &mut self, inst_postfix: Option<&str>, module: Module<I, O>,
    ) -> Module<I, O> {
        assert!(
            matches!(&*module.inner.inner, lir::ModuleInner::Composite(..)),
            "Only `CompositeModule` can be registered to the context"
        );
        let module_name = format!("{}_inner", module.inner.get_module_name());
        let module_inst: Module<_, _> = ModuleInst::from_module(inst_postfix, module.clone()).into();
        let registered_index = self.inner.registered_modules.len();
        self.inner.registered_modules.push(module_inst.inner);
        let virtual_module = lir::VirtualModule {
            module_name,
            registered_index,
            input_prefix: "in".to_string(),
            output_prefix: "out".to_string(),
            input_interface_typ: module.inner.inner.input_interface_typ(),
            input_endpoint_path: lir::EndpointPath::default(),
            output_interface_typ: module.inner.inner.output_interface_typ(),
            output_endpoint_path: lir::EndpointPath::default(),
        };
        Module::new(virtual_module.into())
    }

    /// inline version
    pub fn register_inline<I: Interface, O: Interface>(&mut self, module: Module<I, O>) -> Module<I, O> {
        assert!(
            matches!(&*module.inner.inner, lir::ModuleInner::Composite(..)),
            "Only `CompositeModule` can be registered to the context"
        );
        let registered_index = self.inner.registered_modules.len();
        self.inner.registered_modules.push(module.inner.clone());
        let virtual_module = lir::VirtualModule {
            module_name: module.inner.get_module_name(),
            registered_index,
            input_prefix: module.inner.inner.input_prefix().unwrap_or_else(|| "in".to_string()),
            output_prefix: module.inner.inner.output_prefix().unwrap_or_else(|| "out".to_string()),
            input_interface_typ: module.inner.inner.input_interface_typ(),
            input_endpoint_path: lir::EndpointPath::default(),
            output_interface_typ: module.inner.inner.output_interface_typ(),
            output_endpoint_path: lir::EndpointPath::default(),
        };
        Module::new(virtual_module.into())
    }

    /// register inline
    pub fn feedback<F: Interface>(&mut self) -> (F, Module<F, ()>) {
        let (source, sink) = self
            .register_inline(
                composite::<((), F), (F, ()), _>("feedback", Some("in"), Some("o"), |i, _| (i.1, i.0)).build(),
            )
            .split();
        (().comb_inline(self, source), sink)
    }
}

impl<B: Interface> CompositeModule<B, B> {
    fn new(name: String, input_prefix: Option<String>, output_prefix: Option<String>) -> Self {
        let result = CompositeModule::<(), ()> {
            inner: lir::CompositeModule::new(name, input_prefix, output_prefix),
            _marker: PhantomData,
        };

        result.wrap::<B, B, _>(|_, iw, o| (o, iw))
    }
}

impl<B: Interface> Default for CompositeModule<B, B> {
    fn default() -> Self {
        let result = CompositeModule::<(), ()> { inner: lir::CompositeModule::default(), _marker: PhantomData };

        result.wrap::<B, B, _>(|_, iw, o| (o, iw))
    }
}

/// Creates a new composite module with given prefix for input and output channels.
pub fn composite<I: Interface, O: Interface, F: FnOnce(I, &mut CompositeModuleContext) -> O>(
    name: &str, input_prefix: Option<&str>, output_prefix: Option<&str>, f: F,
) -> CompositeModule<I, O> {
    CompositeModule::<I, I>::new(name.to_string(), input_prefix.map(String::from), output_prefix.map(String::from))
        .and_then(f)
}

/// Creates new input interface from given interface type.
pub fn input_interface<I: Interface>() -> lir::Interface {
    I::interface_typ()
        .into_primitives()
        .into_iter()
        .map(|(typ, path)| {
            (
                match typ {
                    lir::InterfaceTyp::Unit => lir::Interface::Unit,
                    lir::InterfaceTyp::Channel(channel_typ) => lir::Interface::Channel(lir::Channel {
                        typ: channel_typ,
                        endpoint: lir::Endpoint::input(path.clone()),
                    }),
                    _ => panic!("not primitive type"),
                },
                path,
            )
        })
        .collect()
}

/// Creates new temporary interface from given interface type.
pub fn temp_interface<O: Interface>() -> lir::Interface {
    O::interface_typ()
        .into_primitives()
        .into_iter()
        .map(|(typ, path)| {
            (
                match typ {
                    lir::InterfaceTyp::Unit => lir::Interface::Unit,
                    lir::InterfaceTyp::Channel(channel_typ) => lir::Interface::Channel(lir::Channel {
                        typ: channel_typ,
                        endpoint: lir::Endpoint::temp(path.clone()),
                    }),
                    _ => panic!("not primitive type"),
                },
                path,
            )
        })
        .collect()
}

impl<I: Interface, O: Interface> CompositeModule<I, O> {
    /// Wraps the module with new input/output interface.
    // TODO: add branded id to `F` to mimic pure functions.
    pub fn wrap<Iw: Interface, Ow: Interface, F: FnOnce(&mut CompositeModuleContext, Iw, O) -> (I, Ow)>(
        self, f: F,
    ) -> CompositeModule<Iw, Ow> {
        let mut ctx = CompositeModuleContext { inner: self.inner };

        // Takes old input/output interface.
        let old_output_interface = mem::take(&mut ctx.inner.output_interface);
        let old_submodules_len = ctx.inner.submodules.len();

        // Creates old input interface and new output interface.
        let (old_input_interface, new_output_interface) = {
            let new_input_interface = Iw::try_from_inner(input_interface::<Iw>()).expect("internal compiler error");
            let output_interface = O::try_from_inner(temp_interface::<O>()).expect("internal compiler error");
            f(&mut ctx, new_input_interface, output_interface)
        };

        let old_input_interface = old_input_interface.try_into_inner().expect("internal compiler error");
        let update_input = {
            let primitives = old_input_interface.into_primitives();
            primitives
                .into_iter()
                .filter_map(|(interface, path)| interface.get_channel().map(|channel| (path, channel)))
                .collect::<HashMap<_, _>>()
        };
        let update_output = {
            let primitives = old_output_interface.into_primitives();
            primitives
                .into_iter()
                .filter_map(|(interface, path)| {
                    interface.get_channel().map(|channel| {
                        (path, match channel.endpoint() {
                            lir::Endpoint::Input { path } => update_input.get(&path).unwrap().clone(),
                            _ => channel,
                        })
                    })
                })
                .collect::<HashMap<_, _>>()
        };

        // Updates old submodules' interfaces.
        for (_, ref mut interface) in ctx.inner.submodules.iter_mut().take(old_submodules_len) {
            *interface = interface
                .clone()
                .into_primitives()
                .into_iter()
                .map(|(interface, path)| {
                    (
                        match interface {
                            lir::Interface::Unit => lir::Interface::Unit,
                            lir::Interface::Channel(channel) => lir::Interface::Channel(match channel.endpoint() {
                                lir::Endpoint::Input { path } => update_input.get(&path).unwrap().clone(),
                                _ => channel,
                            }),
                            _ => panic!("internal compiler error"),
                        },
                        path,
                    )
                })
                .collect();
        }
        for (_, ref mut interface) in ctx.inner.submodules.iter_mut() {
            *interface = interface
                .clone()
                .into_primitives()
                .into_iter()
                .map(|(interface, path)| {
                    (
                        match interface {
                            lir::Interface::Unit => lir::Interface::Unit,
                            lir::Interface::Channel(channel) => lir::Interface::Channel(match channel.endpoint() {
                                lir::Endpoint::Temp { path } => {
                                    let channel = update_output.get(&path).unwrap().clone();
                                    if matches!(channel.endpoint(), lir::Endpoint::Temp { .. }) {
                                        // TODO: Analyze cyclic assignment and handle it
                                        todo!()
                                    }
                                    channel
                                }
                                _ => channel,
                            }),
                            _ => panic!("internal compiler error"),
                        },
                        path,
                    )
                })
                .collect();
        }

        // Updates new output channels.
        let new_output_interface = new_output_interface.try_into_inner().expect("internal compiler error");
        let new_output_interface = new_output_interface
            .into_primitives()
            .into_iter()
            .map(|(interface, path)| {
                (
                    match interface {
                        lir::Interface::Unit => lir::Interface::Unit,
                        lir::Interface::Channel(channel) => lir::Interface::Channel(match channel.endpoint() {
                            lir::Endpoint::Temp { path } => {
                                let channel = update_output.get(&path).unwrap().clone();
                                if matches!(channel.endpoint(), lir::Endpoint::Temp { .. }) {
                                    // TODO: Analyze cyclic assignment and handle it
                                    todo!()
                                }
                                channel
                            }
                            _ => channel,
                        }),
                        _ => panic!("internal compiler error"),
                    },
                    path,
                )
            })
            .collect();

        // Wires new input/output channels.
        ctx.inner.input_interface = input_interface::<Iw>();
        ctx.inner.output_interface = new_output_interface;

        CompositeModule { inner: ctx.inner, _marker: PhantomData }
    }

    /// Transforms the output interface.
    pub fn and_then<Ow: Interface, F: FnOnce(O, &mut CompositeModuleContext) -> Ow>(
        self, f: F,
    ) -> CompositeModule<I, Ow> {
        self.wrap(|k, i, o| (i, f(o, k)))
    }

    /// Builds a new module.
    pub fn build(self) -> Module<I, O> {
        let name = self.inner.name.clone();

        Module::new(self.inner.build(&name))
    }

    /// Builds a new module for array interface.
    pub fn build_array<const N: usize>(self) -> Module<[I; N], [O; N]> {
        let name = self.inner.name.clone();

        Module::new(self.inner.build_array(&name, N))
    }
}

impl<I: Interface, O: Interface, F: Interface> CompositeModule<(I, F), (O, F)> {
    /// Make a loop with feedback from output side to input side
    pub fn loop_feedback(self) -> CompositeModule<I, O> {
        self.wrap(|_, input, (output, feedback)| ((input, feedback), output))
    }
}

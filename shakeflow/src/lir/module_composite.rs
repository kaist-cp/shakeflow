//! Composite module.

use std::rc::Rc;

use thiserror::Error;

use super::*;

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum CompositeModuleError {
    #[error("there are no submodules at the specified index")]
    NoSubmodules,
    #[error("there are no wires at the specified index")]
    NoWires,
    #[error("there are no channels at the specified index")]
    NoChannels,
    #[error("the specified endpoint is already occupied")]
    EndpointOccupied,
    #[error("the types are mismatched")]
    TypMismatch,
}

/// Composite module type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeModuleTyp {
    /// `I` -> `O`
    OneToOne,

    /// `[I; N]` -> `[O; N]`
    NToN(usize),
}

impl Default for CompositeModuleTyp {
    fn default() -> Self { Self::OneToOne }
}

/// Composite module.
#[derive(Debug, Default, Clone)]
pub struct CompositeModule {
    /// Name of module.
    pub name: String,

    /// Type of module.
    pub module_typ: CompositeModuleTyp,

    /// Inner submodules.
    pub submodules: Vec<(Module, Interface)>,

    /// Registered modules
    pub registered_modules: Vec<Module>,

    /// Input interface.
    pub input_interface: Interface,

    /// Input interface's prefix. For example, 's_axis' is input prefix for cmac_pad.
    pub input_prefix: Option<String>,

    /// Output interface.
    pub output_interface: Interface,

    /// Output interface's prefix. For example, 'm_axis' is output prefix for cmac_pad.
    pub output_prefix: Option<String>,
}

impl CompositeModule {
    /// Creates a new composite module with given prefix for input and output channels.
    pub fn new(name: String, input_prefix: Option<String>, output_prefix: Option<String>) -> Self {
        Self {
            name,
            module_typ: CompositeModuleTyp::default(),
            submodules: Vec::default(),
            registered_modules: Vec::default(),
            input_interface: Interface::default(),
            input_prefix,
            output_interface: Interface::default(),
            output_prefix,
        }
    }

    /// Returns input interface type of the module.
    pub fn input_interface_typ(&self) -> InterfaceTyp {
        match self.module_typ {
            CompositeModuleTyp::OneToOne => self.input_interface.typ(),
            CompositeModuleTyp::NToN(n) => InterfaceTyp::Array(Box::new(self.input_interface.typ()), n),
        }
    }

    /// Returns output interface type of the module.
    pub fn output_interface_typ(&self) -> InterfaceTyp {
        match self.module_typ {
            CompositeModuleTyp::OneToOne => self.output_interface.typ(),
            CompositeModuleTyp::NToN(n) => InterfaceTyp::Array(Box::new(self.output_interface.typ()), n),
        }
    }

    /// Adds a submodule.
    pub fn add_submodule(&mut self, module: Module, input_interface: Interface) -> Interface {
        // Inserts the given module.
        let index = self.submodules.len();
        self.submodules.push((module.clone(), input_interface));

        // Calculates the output interface.
        module
            .inner
            .output_interface_typ()
            .into_primitives()
            .into_iter()
            .map(|(primitive_typ, path)| {
                (
                    match primitive_typ {
                        InterfaceTyp::Unit => Interface::Unit,
                        InterfaceTyp::Channel(channel_typ) => Interface::Channel(Channel {
                            typ: channel_typ,
                            endpoint: Endpoint::submodule(index, path.clone()),
                        }),
                        _ => panic!("not primitive type"),
                    },
                    path,
                )
            })
            .collect()
    }

    /// Builds a new module.
    pub fn build(self, name: &str) -> Module {
        Module { inner: Rc::new(ModuleInner::Composite(String::from(name), self)) }
    }

    /// Builds a new module for array interface.
    pub fn build_array(mut self, name: &str, n: usize) -> Module {
        self.module_typ = CompositeModuleTyp::NToN(n);

        Module { inner: Rc::new(ModuleInner::Composite(String::from(name), self)) }
    }

    /// Scan submodule instantiation of composite module
    pub fn scan_submodule_inst(&self) -> Vec<Module> {
        ::std::iter::empty()
            .chain(self.submodules.iter().map(|(module, _)| module))
            .chain(self.registered_modules.iter())
            .flat_map(|module| module.scan_submodule_inst())
            .collect()
    }

    /// Walk the module structure and return a vec of mutable refs to names of all inner `ModuleInst`s.
    pub fn scan_module_inst(&mut self) -> Vec<&mut ModuleInst> {
        ::std::iter::empty()
            .chain(self.submodules.iter_mut().map(|(module, _)| module))
            .chain(self.registered_modules.iter_mut())
            .flat_map(|module| module.scan_module_inst())
            .collect()
    }
}

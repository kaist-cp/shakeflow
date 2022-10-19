//! Virtual Module

use super::{EndpointPath, InterfaceTyp};
use crate::PrimitiveModule;

/// Virtual Module.
#[derive(Debug)]
pub struct VirtualModule {
    /// Module name
    pub(crate) module_name: String,
    /// Registered module's index
    pub(crate) registered_index: usize,
    /// input prefix
    pub(crate) input_prefix: String,
    /// output prefix
    pub(crate) output_prefix: String,
    /// Input interface type of registered module
    pub(crate) input_interface_typ: InterfaceTyp,
    /// endpoint path of input interface
    pub(crate) input_endpoint_path: EndpointPath,
    /// Output interface type of registered module
    pub(crate) output_interface_typ: InterfaceTyp,
    /// endpoint path of output interface
    pub(crate) output_endpoint_path: EndpointPath,
}

impl VirtualModule {
    pub(crate) fn input_endpoint(&self) -> EndpointPath { self.input_endpoint_path.clone() }

    pub(crate) fn output_endpoint(&self) -> EndpointPath { self.output_endpoint_path.clone() }
}

impl PrimitiveModule for VirtualModule {
    fn get_module_name(&self) -> String { self.module_name.clone() }

    fn input_interface_typ(&self) -> InterfaceTyp {
        self.input_interface_typ.get_subinterface(self.input_endpoint_path.clone())
    }

    fn output_interface_typ(&self) -> InterfaceTyp {
        self.output_interface_typ.get_subinterface(self.output_endpoint_path.clone())
    }
}

//! Module.

use std::iter;
use std::marker::PhantomData;
use std::ops::*;

use crate::hir::*;
use crate::*;

/// Module.
#[derive(Debug)]
pub struct Module<I: Interface, O: Interface> {
    pub(crate) inner: lir::Module,
    _marker: PhantomData<(I, O)>,
}

impl<I: Interface, O: Interface> Module<I, O> {
    /// Creates new module.
    pub fn new(inner: lir::Module) -> Self { Module { inner, _marker: PhantomData } }
}

impl<I1: Interface, I2: Interface, O1: Interface, O2: Interface> Module<(I1, I2), (O1, O2)> {
    /// Split a module
    ///
    /// TODO: generalized to N-M modules?
    pub fn split(self) -> (Module<I1, O1>, Module<I2, O2>) {
        match self.inner.inner.deref() {
            lir::ModuleInner::VirtualModule(virtual_module) => {
                let (in1, in2) = match virtual_module.input_interface_typ() {
                    lir::InterfaceTyp::Struct(fields) => {
                        assert_eq!(fields.len(), 2);
                        let mut iter = fields.into_iter();
                        (iter.next().unwrap(), iter.next().unwrap())
                    }
                    _ => todo!(),
                };
                let (out1, out2) = match virtual_module.output_interface_typ() {
                    lir::InterfaceTyp::Struct(fields) => {
                        assert_eq!(fields.len(), 2);
                        let mut iter = fields.into_iter();
                        (iter.next().unwrap(), iter.next().unwrap())
                    }
                    _ => todo!(),
                };
                let vm1 = lir::VirtualModule {
                    registered_index: virtual_module.registered_index,
                    module_name: virtual_module.module_name.clone(),
                    input_prefix: virtual_module.input_prefix.clone(),
                    output_prefix: virtual_module.output_prefix.clone(),
                    input_interface_typ: virtual_module.input_interface_typ(),
                    output_interface_typ: virtual_module.output_interface_typ(),
                    input_endpoint_path: iter::once(lir::EndpointNode::Field(in1.0, in1.1 .0))
                        .chain(virtual_module.input_endpoint().inner.into_iter())
                        .collect(),
                    output_endpoint_path: iter::once(lir::EndpointNode::Field(out1.0, out1.1 .0))
                        .chain(virtual_module.output_endpoint().inner.into_iter())
                        .collect(),
                };
                let vm2 = lir::VirtualModule {
                    registered_index: virtual_module.registered_index,
                    module_name: virtual_module.module_name.clone(),
                    input_prefix: virtual_module.input_prefix.clone(),
                    output_prefix: virtual_module.output_prefix.clone(),
                    input_interface_typ: virtual_module.input_interface_typ(),
                    output_interface_typ: virtual_module.output_interface_typ(),
                    input_endpoint_path: iter::once(lir::EndpointNode::Field(in2.0, in2.1 .0))
                        .chain(virtual_module.input_endpoint().inner.into_iter())
                        .collect(),
                    output_endpoint_path: iter::once(lir::EndpointNode::Field(out2.0, out2.1 .0))
                        .chain(virtual_module.output_endpoint().inner.into_iter())
                        .collect(),
                };
                (hir::Module::new(vm1.into()), hir::Module::new(vm2.into()))
            }
            _ => panic!("internal compiler error: split api can only be used for Virtual Modules"),
        }
    }
}

impl<
        I: Interface,
        O: Interface,
        S: Signal,
        F: 'static
            + for<'id> Fn(
                Expr<'id, I::Fwd>,
                Expr<'id, O::Bwd>,
                Expr<'id, S>,
            ) -> (Expr<'id, O::Fwd>, Expr<'id, I::Bwd>, Expr<'id, S>),
    > From<Fsm<I, O, S, F>> for Module<I, O>
{
    fn from(module: Fsm<I, O, S, F>) -> Self { Self { inner: lir::Fsm::from(module).into(), _marker: PhantomData } }
}

impl<I: Interface, O: Interface> From<ModuleInst<I, O>> for Module<I, O> {
    fn from(module: ModuleInst<I, O>) -> Self {
        Self { inner: lir::ModuleInst::from(module).into(), _marker: PhantomData }
    }
}

impl<I: Interface, O: Interface> Clone for Module<I, O> {
    fn clone(&self) -> Self { Self { inner: self.inner.clone(), _marker: PhantomData } }
}

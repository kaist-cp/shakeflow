//! Module instantiation.

use std::fmt;
use std::marker::PhantomData;

use crate::hir::*;
use crate::*;

/// Module instantiation.
#[derive(Clone)]
pub struct ModuleInst<I: Interface, O: Interface> {
    /// Module name.
    module_name: String,
    /// Instance name.
    pub(crate) inst_name: String,
    /// Parameters.
    pub(crate) params: Vec<(String, usize)>,
    /// Indicates that the module has the clock and reset signal.
    pub(crate) has_clkrst: bool,
    /// Input prefix.
    pub(crate) input_prefix: Option<String>,
    /// Output prefix.
    pub(crate) output_prefix: Option<String>,
    /// Shakeflow Module
    pub(crate) shakeflow_module: Option<Module<I, O>>,
    _marker: PhantomData<(I, O)>,
}

impl<I: Interface, O: Interface> ModuleInst<I, O> {
    /// Creates a new Module instantiation.
    pub fn new(
        module_name: String, inst_name: String, params: Vec<(String, usize)>, has_clkrst: bool,
        input_prefix: Option<String>, output_prefix: Option<String>, shakeflow_module: Option<Module<I, O>>,
    ) -> Self {
        Self {
            module_name,
            inst_name,
            params,
            has_clkrst,
            input_prefix,
            output_prefix,
            shakeflow_module,
            _marker: PhantomData,
        }
    }
}

impl<I: Interface, O: Interface> fmt::Debug for ModuleInst<I, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModuleInst")
            .field("module_name", &self.module_name)
            .field("inst_name", &self.inst_name)
            .field("params", &self.params)
            .field("has_clkrst", &self.has_clkrst)
            .field("input_prefix", &self.input_prefix)
            .field("output_prefix", &self.output_prefix)
            .finish()
    }
}

impl<I: Interface, O: Interface> ModuleInst<I, O> {
    /// generates module_inst fron shakeflow module
    pub(crate) fn from_module(inst_postfix: Option<&str>, module: Module<I, O>) -> Self {
        let module_name = format!("{}_inner", module.inner.get_module_name());
        let inst_name = join_options("_", [
            Some(module_name.clone()),
            Some("inst".to_string()),
            inst_postfix.map(|s| s.to_string()),
        ])
        .unwrap();
        ModuleInst::new(
            module_name,
            inst_name,
            vec![],
            true,
            module.inner.inner.input_prefix(),
            module.inner.inner.output_prefix(),
            Some(module.clone()),
        )
    }
}

impl<I: Interface, O: Interface> From<ModuleInst<I, O>> for lir::ModuleInst {
    fn from(module: ModuleInst<I, O>) -> Self {
        lir::ModuleInst {
            input_interface_typ: I::interface_typ(),
            output_interface_typ: O::interface_typ(),
            module_name: module.module_name,
            inst_name: module.inst_name,
            params: module.params,
            has_clkrst: module.has_clkrst,
            input_prefix: module.input_prefix,
            output_prefix: module.output_prefix,
            module: module.shakeflow_module.map(|module| module.inner),
        }
    }
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! impl_custom_inst {
    (
        $in: ty, $out: ty,
        $module_name: ident,
        <$($params:ident),*>,
        $use_clk: expr,
    ) => {
        ::paste::paste! {
            pub trait [<$module_name:camel Ext>]: Interface {
                type Out: Interface;
                fn $module_name<$(const [<$params:snake:upper>]: usize,)*>(self, k: &mut CompositeModuleContext, inst_name: &str, input_prefix: Option<&str>, output_prefix: Option<&str>) -> Self::Out;
            }
            impl [<$module_name:camel Ext>]
            for $in {
                type Out = $out;

                fn $module_name<$(const [<$params:snake:upper>]: usize,)*>(self, k: &mut CompositeModuleContext, inst_name: &str, input_prefix: Option<&str>, output_prefix: Option<&str>) -> Self::Out {
                    let params = vec![$((stringify!($params), [<$params:snake:upper>]),)*];
                    self.module_inst::<Self::Out>(k, stringify!($module_name), inst_name, params, $use_clk, input_prefix, output_prefix)
                }
            }
        }
    };
    (
      $in: ident <$($in_generics:ident: Signal),*> & <$(const $in_const_generics: ident: usize),*>,
      $out: ident <$($out_generics:ident: Signal),*> & <$(const $out_const_generics: ident: usize),*>,
      $module_name: ident,
      <$($params:ident),*>,
      $use_clk: expr,
    ) => {
        ::paste::paste! {
            pub trait [<$module_name:camel Ext>]<$($out_generics: Signal,)*$(const $out_const_generics: usize,)*>: Interface {
                type Out: Interface;
                fn $module_name<$(const [<$params:snake:upper>]: usize,)*>(self, k: &mut CompositeModuleContext, inst_name: &str, input_prefix: Option<&str>, output_prefix: Option<&str>) -> Self::Out;
            }
            impl<
              $($in_generics: Signal,)*
              $($out_generics: Signal,)*
              $(const $in_const_generics: usize,)*
              $(const $out_const_generics: usize,)*
            > [<$module_name:camel Ext>]<$($out_generics,)*$($out_const_generics,)*>
            for $in<$($in_generics,)*$($in_const_generics,)*> {
                type Out = $out<$($out_generics,)*$($out_const_generics,)*>;
                fn $module_name<$(const [<$params:snake:upper>]: usize,)*>(self, k: &mut CompositeModuleContext, inst_name: &str, input_prefix: Option<&str>, output_prefix: Option<&str>) -> Self::Out {
                    let params = vec![$((stringify!($params), [<$params:snake:upper>]),)*];
                    self.module_inst::<Self::Out>(k, stringify!($module_name), inst_name, params, $use_clk, input_prefix, output_prefix)
                }
            }
        }
    };
}

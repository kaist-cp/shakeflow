use std::io;

use thiserror::Error;

use crate::hir::*;
use crate::lir;

#[allow(missing_docs)]
#[allow(variant_size_differences)]
#[derive(Debug, Error)]
pub enum PackageError {
    #[error("file system error: {error:?}")]
    Fs { error: io::Error },

    #[error("module error: {error:?}")]
    Module { error: lir::ModuleError },
}

/// Package.
#[derive(Debug, Default)]
pub struct Package {
    /// Modules.
    pub modules: Vec<lir::Module>,
}

impl Package {
    /// Adds the given module to package.
    pub fn add<I: Interface, O: Interface>(&mut self, module: Module<I, O>) { self.modules.push(module.inner); }

    /// Scan modules to see if there is submodule instatiation in the
    pub fn scan_submodule_inst(&self) -> Vec<lir::Module> {
        self.modules.iter().flat_map(|module| module.scan_submodule_inst()).collect()
    }

    /// Walk the module structure and return a vec of mutable refs to names of all inner `ModuleInst`s.
    pub fn scan_module_inst(&mut self) -> Vec<&mut lir::ModuleInst> {
        self.modules.iter_mut().flat_map(|module| module.scan_module_inst()).collect()
    }
}

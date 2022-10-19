//! Generates Verilog code.

#![allow(clippy::type_complexity)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use hashcons::merkle::Merkle;

use crate::codegen::*;
use crate::vir::*;
use crate::*;

impl Package {
    fn gen_vir_module<P: AsRef<Path>>(&self, module: &lir::Module, path_dir: P) -> Result<(), PackageError> {
        let path = path_dir.as_ref().join(format!("{}_inner.v", module.get_module_name()));
        let mut file = File::create(path).map_err(|error| PackageError::Fs { error })?;

        let name = format!("{}_inner", module.get_module_name());
        let module: vir::Module =
            gen_module::<Virgen>(name, module).map_err(|error| PackageError::Module { error })?.into();
        let module = vir::opt::wire_cache_opt(module);
        let module = vir::opt::dead_code_opt(module);

        writeln!(file, "{}", module.to_string()).map_err(|error| PackageError::Fs { error })?;

        Ok(())
    }

    /// Generates Verilog code at the given directory path.
    pub fn gen_vir<P: AsRef<Path>>(mut self, path_dir: P) -> Result<(), PackageError> {
        fs::create_dir_all(path_dir.as_ref()).map_err(|error| PackageError::Fs { error })?;

        let mut submodule_map = HashMap::<String, lir::Module>::new();

        // If the module contains multiple `module_inst`s with the same `inst_name`,
        // append index until name collision no longer occurs.
        let mut module_inst_names = HashSet::new();
        for module_inst in self.scan_module_inst() {
            let name = &mut module_inst.inst_name;
            let final_name = if module_inst_names.contains(name) {
                let mut idx = 1;
                while module_inst_names.contains(&format!("{}_{}", name, idx)) {
                    idx += 1;
                }
                let final_name = format!("{}_{}", name, idx);
                *name = final_name.clone();
                final_name
            } else {
                name.to_string()
            };
            module_inst_names.insert(final_name);
        }

        // submodules aggregation, to prevent duplicate compilation for same module
        for submodule in self.scan_submodule_inst().into_iter() {
            let name = submodule.get_module_name();
            if let Some(module) = submodule_map.get(&name) {
                // TODO: Currently we only check the interface type for modules with same name.
                // But there can be modules with same name, same interface, but different logic.
                // We should check this.
                if !module.is_interface_eq(submodule) {
                    Err(PackageError::Module {
                        error: lir::ModuleError::Misc(
                                   format!(
                            "Module {}, which is instantiated as a submodule, is instantiated multiple times with different interfaces"
                                , name)
                        ),
                    })?
                }
            } else {
                let _ = submodule_map.insert(name, submodule);
            }
        }

        // Check if top level modules are instantiated as a submodule
        for module in self.modules.iter() {
            let name = module.get_module_name();
            if submodule_map.get(&name).is_some() {
                Err(PackageError::Module {
                    error: lir::ModuleError::Misc(
                        format!("Module {}, which is contained in the package as a top level module, is also instantiated as a submodule", name),
                    ),
                })?
            }
        }

        for submodule in submodule_map.values() {
            self.gen_vir_module(submodule, &path_dir)?;
        }

        for module in self.modules.iter() {
            self.gen_vir_module(module, &path_dir)?;
        }

        Ok(())
    }
}

impl From<codegen::Module<Virgen>> for vir::Module {
    fn from(module: codegen::Module<Virgen>) -> Self {
        vir::Module { name: module.name, port_decls: module.ports, module_items: module.body }
    }
}

/// Verilog IR Generator
#[derive(Default, Debug)]
pub struct Virgen;

impl Codegen for Virgen {
    type Body = Vec<ModuleItem>;
    type Ports = Vec<PortDeclaration>;

    fn gen_port_decls(&self, module: &lir::Module) -> Result<Vec<PortDeclaration>, lir::ModuleError> {
        Ok(gen_port_decls(module)?
            .into_iter()
            .map(|(dir, width, name)| match dir {
                Direction::Input => PortDeclaration::input(width, name),
                Direction::Output => PortDeclaration::output(width, name),
            })
            .collect())
    }

    fn gen_module_composite(
        &self, module: &lir::CompositeModule, ctx: &mut Context,
    ) -> Result<Vec<ModuleItem>, lir::ModuleError> {
        let composite_module = match module.module_typ {
            lir::CompositeModuleTyp::OneToOne => {
                let mut module_items = vec![];

                let mut decls = Vec::new();

                gen_submodule_wires(module, ctx)?.into_iter().for_each(|(name, shape)| {
                    decls.push(Declaration::net(shape, name));
                });

                if !decls.is_empty() {
                    module_items.push(ModuleItem::Commented(
                        format!("Wires declared by {}", &module.name),
                        Some(format!("End wires declared by {}", &module.name)),
                        vec![ModuleItem::Declarations(decls)],
                    ));
                }

                let conts = self.gen_module_wiring(module, ctx.get_prefix())?;

                if !conts.is_empty() {
                    module_items.push(ModuleItem::Commented(
                        format!("Wiring by {}", &module.name),
                        Some(format!("End wiring by {}", &module.name)),
                        vec![ModuleItem::ContinuousAssigns(conts)],
                    ));
                }

                let mut registered_module_items = vec![];
                for (index, submodule) in module.registered_modules.iter().enumerate() {
                    let comp_name = submodule.get_module_name();
                    ctx.enter_scope(format!("registered_{}_{}", comp_name, index));
                    match &*submodule.inner {
                        lir::ModuleInner::ModuleInst(module) => {
                            registered_module_items.append(&mut self.gen_module_inst(module, ctx)?);
                        }
                        lir::ModuleInner::Composite(_, module) => {
                            module_items.append(&mut self.gen_module_composite(module, ctx)?);
                        }
                        _ => panic!("internal compiler error"),
                    }
                    ctx.leave_scope();
                }
                if !registered_module_items.is_empty() {
                    module_items.push(ModuleItem::Commented(
                        format!("Registered Modules of {}", &module.name),
                        Some(format!("End registered Modules of {}", &module.name)),
                        registered_module_items,
                    ));
                }

                let mut submodule_items = vec![];

                let composite_context_prefix = ctx.get_prefix();

                // Add inner submodule's logic.
                for (index, (submodule, _)) in module.submodules.iter().enumerate() {
                    let comp_name = submodule.get_module_name();
                    ctx.enter_scope(format!("{}_{}", comp_name, index));
                    match &*submodule.inner {
                        lir::ModuleInner::Composite(_, module) => {
                            submodule_items.append(&mut self.gen_module_composite(module, ctx)?);
                        }
                        lir::ModuleInner::Fsm(module) => {
                            submodule_items.append(&mut self.gen_module_fsm(module, ctx)?);
                        }
                        lir::ModuleInner::ModuleInst(module) => {
                            submodule_items.append(&mut self.gen_module_inst(module, ctx)?);
                        }
                        lir::ModuleInner::VirtualModule(module) => {
                            submodule_items.append(&mut self.gen_module_virtual(
                                module,
                                composite_context_prefix.clone(),
                                ctx,
                            )?);
                        }
                    }
                    ctx.leave_scope();
                }
                if !submodule_items.is_empty() {
                    module_items.push(ModuleItem::Commented(
                        format!("Submodules of {}", module.name.clone()),
                        Some(format!("End submodules of {}", module.name.clone())),
                        submodule_items,
                    ));
                }

                module_items
            }
            lir::CompositeModuleTyp::NToN(n) => {
                let genvar_id = ctx.alloc_genvar_id();

                let module_items = {
                    let mut module = module.clone();
                    module.module_typ = lir::CompositeModuleTyp::OneToOne;

                    let mut module_items = vec![];

                    let mut decls = Vec::new();

                    gen_submodule_wires(&module, ctx)?.into_iter().for_each(|(name, shape)| {
                        decls.push(Declaration::net(shape, name));
                    });

                    if !decls.is_empty() {
                        module_items.push(ModuleItem::Declarations(decls));
                    }

                    let conts = self.gen_module_wiring_array(&module, ctx.get_prefix(), &genvar_id)?;

                    if !conts.is_empty() {
                        module_items.push(ModuleItem::ContinuousAssigns(conts));
                    }

                    for (index, submodule) in module.registered_modules.iter().enumerate() {
                        let comp_name = submodule.get_module_name();
                        ctx.enter_scope(format!("registered_{}_{}", comp_name, index));
                        match &*submodule.inner {
                            lir::ModuleInner::ModuleInst(module) => {
                                module_items.append(&mut self.gen_module_inst(module, ctx)?);
                            }
                            _ => panic!("internal compiler error"),
                        }
                        ctx.leave_scope();
                    }

                    let composite_context_prefix = ctx.get_prefix();

                    // Add inner submodule's logic.
                    for (index, (submodule, _)) in module.submodules.iter().enumerate() {
                        let comp_name = submodule.get_module_name();
                        ctx.enter_scope(format!("{}_{}", comp_name, index));
                        match &*submodule.inner {
                            lir::ModuleInner::Composite(_, module) => {
                                module_items.append(&mut self.gen_module_composite(module, ctx)?);
                            }
                            lir::ModuleInner::Fsm(module) => {
                                module_items.append(&mut self.gen_module_fsm(module, ctx)?);
                            }
                            lir::ModuleInner::ModuleInst(module) => {
                                module_items.append(&mut self.gen_module_inst(module, ctx)?);
                            }
                            lir::ModuleInner::VirtualModule(module) => {
                                module_items.append(&mut self.gen_module_virtual(
                                    module,
                                    composite_context_prefix.clone(),
                                    ctx,
                                )?);
                            }
                        }
                        ctx.leave_scope();
                    }

                    module_items
                };

                let module_item = ModuleItem::GeneratedInstantiation(GeneratedInstantiation {
                    genvar_identifier: genvar_id,
                    loop_count: n,
                    loop_body: module_items,
                });

                vec![module_item]
            }
        };

        Ok(vec![ModuleItem::Commented(
            format!("Begin module {}", module.name),
            Some(format!("End module {}", module.name)),
            composite_module,
        )])
    }

    /// Generates target code for FSM.
    ///
    /// # Note
    ///
    /// Layout of generated Verilog code:
    ///
    /// ```verilog
    /// reg ... // (1) state reg, wire (decls)
    ///
    /// assign ... = ... // (1) state reg, wire (conts)
    ///
    /// initial begin
    ///     for (...) begin
    ///         ... // (2) state initialization with dimension > 1
    ///     end
    /// end
    ///
    /// reg ... // (3) input, output logic (decls)
    ///
    /// initial begin
    ///     ... // (3) input, output logic (stmts)
    /// end
    ///
    /// always @* begin
    ///     ... // (3) input, output logic (stmts)
    /// end
    ///
    /// always @(posedge clk) begin
    ///     ... // (4) state update logic
    ///
    ///     if (rst) begin
    ///         ... // (4) state update logic (reset)
    ///     end
    /// end
    /// ```
    fn gen_module_fsm(&self, module: &lir::Fsm, ctx: &mut Context) -> Result<Vec<ModuleItem>, lir::ModuleError> {
        let mut module_items = Vec::new();

        let state_init = gen_module_fsm_state_init(&module.state.into_expr(), &module.init.into_expr(), ctx)?
            .into_iter()
            .map(|(shape, name, init_value)| {
                let init_value = if !init_value.is_empty() && !(init_value[0] == LogicValue::X) {
                    if init_value.iter().all(|x| *x == LogicValue::False) {
                        Some(vir::Expression::number(format!("{}'b0", init_value.len())))
                    } else if init_value.iter().all(|x| *x == LogicValue::X) {
                        Some(vir::Expression::number(format!("{}'bx", init_value.len())))
                    } else {
                        Some(vir::Expression::number(format!("{}'b{}", init_value.len(), init_value.to_string())))
                    }
                } else {
                    None
                };

                (shape, name, init_value)
            })
            .collect::<Vec<_>>();

        // (1) state reg, wire (decls, conts)
        {
            let (mut decls, mut conts) = (Vec::new(), Vec::new());

            state_init.iter().for_each(|(shape, net_name, init_expr)| {
                let reg_name = format!("{}_reg", net_name);

                // Add decls
                decls.push(Declaration::net(shape.clone(), net_name.clone()));
                match init_expr {
                    None => decls.push(Declaration::reg(shape.clone(), reg_name.clone())),
                    Some(expr) => decls.push(Declaration::reg(shape.clone(), reg_name.clone()).with_init(expr.clone())),
                }

                // Add conts
                conts.push(vir::ContinuousAssign::new(
                    vir::Expression::ident(net_name.clone()),
                    vir::Expression::ident(reg_name),
                ));
            });

            if !decls.is_empty() {
                module_items.push(vir::ModuleItem::Declarations(decls));
            }

            if !conts.is_empty() {
                module_items.push(vir::ModuleItem::ContinuousAssigns(conts));
            }
        }

        // (2) state initialization with dimension > 1
        {
            let (mut decls, mut stmts) = (Vec::new(), Vec::new());
            let mut int_name = None;

            state_init.iter().filter(|(shape, ..)| shape.dim() > 1).for_each(|(shape, net_name, _)| {
                let reg_name = format!("{}_reg", net_name.clone());

                let int_name = int_name.get_or_insert(ctx.alloc_int_id());
                let body = vec![Statement::blocking_assignment(
                    vir::Expression::ident(reg_name)
                        .with_range(vir::Range::new_index(vir::Expression::ident(int_name.clone()))),
                    vir::Expression::number("0".to_string()),
                )];

                stmts.push(Statement::Loop(int_name.clone(), vir::Expression::number(shape.get(0).to_string()), body));
            });

            if let Some(int_name) = int_name {
                decls.push(Declaration::integer(int_name));
            }

            if !decls.is_empty() {
                module_items.push(vir::ModuleItem::Declarations(decls));
            }

            if !stmts.is_empty() {
                module_items.push(vir::ModuleItem::AlwaysConstruct("initial".to_string(), stmts));
            }
        }

        // TODO: Give proper names
        let output_fwd = &module.output_fwd;
        let input_bwd = &module.input_bwd;
        let state = &module.state;

        // (3) input, output logic (decls, stmts)
        {
            assert_eq!(output_fwd.into_expr().port_decls().max_dim(), 1);
            assert_eq!(input_bwd.into_expr().port_decls().max_dim(), 1);

            // input, output logic for output forward exprs
            module_items.append(&mut self.gen_module_fsm_output(
                "out".to_string(),
                output_fwd.into_expr(),
                ctx,
                &mut HashMap::new(),
            )?);

            // input, output logic for input backward exprs.
            module_items.append(&mut self.gen_module_fsm_output(
                "in".to_string(),
                input_bwd.into_expr(),
                ctx,
                &mut HashMap::new(),
            )?);
        }

        // (4) state update logic
        {
            // state update logic
            let (decls, mut stmts) =
                self.gen_module_fsm_state("st".to_string(), state.into_expr(), ctx, &mut HashMap::new())?;

            // state reset
            let mut stmts_rst = Vec::new();

            state_init
                .iter()
                .filter_map(|(_, net_name, expr)| {
                    if let Some(expr) = expr {
                        let reg_name = format!("{}_reg", net_name);
                        Some((reg_name, expr))
                    } else {
                        None
                    }
                })
                .for_each(|(reg_name, expr)| {
                    stmts_rst.push(Statement::nonblocking_assignment(vir::Expression::ident(reg_name), expr.clone()));
                });

            if !stmts_rst.is_empty() {
                stmts.push(Statement::Conditional(vir::Expression::ident("rst".to_string()), stmts_rst, Vec::new()));
            }

            if !decls.is_empty() {
                module_items.push(vir::ModuleItem::Declarations(decls));
            }

            if !stmts.is_empty() {
                module_items.push(vir::ModuleItem::AlwaysConstruct("always @(posedge clk)".to_string(), stmts));
            }
        }

        Ok(module_items)
    }

    /// Generates module instantiation.
    fn gen_module_inst(
        &self, module: &lir::ModuleInst, ctx: &mut Context,
    ) -> Result<Vec<ModuleItem>, lir::ModuleError> {
        let connections = gen_connections(module, ctx)?
            .into_iter()
            .map(|(_, port, expr)| (port, vir::Expression::ident(expr)))
            .collect();

        let module_inst = vir::ModuleInstantiation::new(
            module.get_module_name(),
            module.inst_name.clone(),
            module.params.clone(),
            connections,
        );

        Ok(vec![vir::ModuleItem::ModuleInstantiation(module_inst)])
    }

    /// Generated connections for virtual module
    fn gen_module_virtual(
        &self, virtual_module: &lir::VirtualModule, composite_context_prefix: Option<String>, ctx: &mut Context,
    ) -> Result<Vec<ModuleItem>, lir::ModuleError> {
        let assignments = gen_virtual_wirings(virtual_module, composite_context_prefix, ctx.get_prefix())?
            .into_iter()
            .map(|(lvalue, lvalue_range, rvalue, rvalue_range)| {
                let lvalue_expr = match lvalue_range {
                    Some((index, elt_size)) => vir::Expression::ident(lvalue).with_range(vir::Range::new_range(
                        vir::Expression::binary(
                            lir::BinaryOp::Mul,
                            vir::Expression::number(index.to_string()),
                            vir::Expression::number(elt_size.to_string()),
                        ),
                        vir::Expression::number(elt_size.to_string()),
                    )),
                    None => vir::Expression::ident(lvalue),
                };
                let rvalue_expr = match rvalue_range {
                    Some((index, elt_size)) => vir::Expression::ident(rvalue).with_range(vir::Range::new_range(
                        vir::Expression::binary(
                            lir::BinaryOp::Mul,
                            vir::Expression::number(index.to_string()),
                            vir::Expression::number(elt_size.to_string()),
                        ),
                        vir::Expression::number(elt_size.to_string()),
                    )),
                    None => vir::Expression::ident(rvalue),
                };
                ContinuousAssign::new(lvalue_expr, rvalue_expr)
            })
            .collect();

        Ok(vec![ModuleItem::ContinuousAssigns(assignments)])
    }
}

impl Virgen {
    /// Generates wirings in the module.
    fn gen_module_wiring(
        &self, module: &lir::CompositeModule, prefix: Option<String>,
    ) -> Result<Vec<ContinuousAssign>, lir::ModuleError> {
        Ok(gen_wiring(module, prefix)?
            .into_iter()
            .map(|(lvalue, lvalue_range, rvalue, rvalue_range)| {
                let lvalue_expr = match lvalue_range {
                    Some((index, elt_size)) => vir::Expression::ident(lvalue).with_range(vir::Range::new_range(
                        vir::Expression::binary(
                            lir::BinaryOp::Mul,
                            vir::Expression::number(index.to_string()),
                            vir::Expression::number(elt_size.to_string()),
                        ),
                        vir::Expression::number(elt_size.to_string()),
                    )),
                    None => vir::Expression::ident(lvalue),
                };
                let rvalue_expr = match rvalue_range {
                    Some((index, elt_size)) => vir::Expression::ident(rvalue).with_range(vir::Range::new_range(
                        vir::Expression::binary(
                            lir::BinaryOp::Mul,
                            vir::Expression::number(index.to_string()),
                            vir::Expression::number(elt_size.to_string()),
                        ),
                        vir::Expression::number(elt_size.to_string()),
                    )),
                    None => vir::Expression::ident(rvalue),
                };
                ContinuousAssign::new(lvalue_expr, rvalue_expr)
            })
            .collect())
    }

    /// Generates wirings in the array module.
    fn gen_module_wiring_array(
        &self, module: &lir::CompositeModule, prefix: Option<String>, genvar_id: &str,
    ) -> Result<Vec<ContinuousAssign>, lir::ModuleError> {
        Ok(gen_wiring_array(module, prefix)?
            .into_iter()
            .map(|(lvalue, lvalue_generate, lvalue_range, rvalue, rvalue_generate, rvalue_range)| {
                let lvalue_expr = match (lvalue_generate, lvalue_range) {
                    (Some(gen_size), Some((index, elt_size))) => {
                        vir::Expression::ident(lvalue).with_range(vir::Range::new_range(
                            vir::Expression::binary(
                                lir::BinaryOp::Add,
                                vir::Expression::binary(
                                    lir::BinaryOp::Mul,
                                    vir::Expression::ident(genvar_id.to_string()),
                                    vir::Expression::number(gen_size.to_string()),
                                ),
                                vir::Expression::binary(
                                    lir::BinaryOp::Mul,
                                    vir::Expression::number(index.to_string()),
                                    vir::Expression::number(elt_size.to_string()),
                                ),
                            ),
                            vir::Expression::number(elt_size.to_string()),
                        ))
                    }
                    (Some(gen_size), None) => vir::Expression::ident(lvalue).with_range(vir::Range::new_range(
                        vir::Expression::binary(
                            lir::BinaryOp::Mul,
                            vir::Expression::ident(genvar_id.to_string()),
                            vir::Expression::number(gen_size.to_string()),
                        ),
                        vir::Expression::number(gen_size.to_string()),
                    )),
                    (None, Some((index, elt_size))) => {
                        vir::Expression::ident(lvalue).with_range(vir::Range::new_range(
                            vir::Expression::binary(
                                lir::BinaryOp::Mul,
                                vir::Expression::number(index.to_string()),
                                vir::Expression::number(elt_size.to_string()),
                            ),
                            vir::Expression::number(elt_size.to_string()),
                        ))
                    }
                    (None, None) => vir::Expression::ident(lvalue),
                };
                let rvalue_expr = match (rvalue_generate, rvalue_range) {
                    (Some(gen_size), Some((index, elt_size))) => {
                        vir::Expression::ident(rvalue).with_range(vir::Range::new_range(
                            vir::Expression::binary(
                                lir::BinaryOp::Add,
                                vir::Expression::binary(
                                    lir::BinaryOp::Mul,
                                    vir::Expression::ident(genvar_id.to_string()),
                                    vir::Expression::number(gen_size.to_string()),
                                ),
                                vir::Expression::binary(
                                    lir::BinaryOp::Mul,
                                    vir::Expression::number(index.to_string()),
                                    vir::Expression::number(elt_size.to_string()),
                                ),
                            ),
                            vir::Expression::number(elt_size.to_string()),
                        ))
                    }
                    (Some(gen_size), None) => vir::Expression::ident(rvalue).with_range(vir::Range::new_range(
                        vir::Expression::binary(
                            lir::BinaryOp::Mul,
                            vir::Expression::ident(genvar_id.to_string()),
                            vir::Expression::number(gen_size.to_string()),
                        ),
                        vir::Expression::number(gen_size.to_string()),
                    )),
                    (None, Some((index, elt_size))) => {
                        vir::Expression::ident(rvalue).with_range(vir::Range::new_range(
                            vir::Expression::binary(
                                lir::BinaryOp::Mul,
                                vir::Expression::number(index.to_string()),
                                vir::Expression::number(elt_size.to_string()),
                            ),
                            vir::Expression::number(elt_size.to_string()),
                        ))
                    }
                    (None, None) => vir::Expression::ident(rvalue),
                };
                ContinuousAssign::new(lvalue_expr, rvalue_expr)
            })
            .collect())
    }

    /// Generates FSM output.
    ///
    /// Returns `Err` if types of `typ` and `output` are mismatched.
    fn gen_module_fsm_output(
        &self, target: String, output: Merkle<lir::Expr>, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<Vec<ModuleItem>, lir::ModuleError> {
        let (decls, stmts, expr) = self.gen_expr(&output, ctx, cache)?;

        let assignments =
            match_value_typ_exprs(Some(format!("{}_{}", ctx.get_prefix().unwrap(), target)), output.port_decls(), expr);

        let mut conts = vec![];

        for (_, var_name, expr) in assignments {
            conts.push(vir::ContinuousAssign::new(vir::Expression::ident(var_name), expr));
        }

        let mut module_items = vec![];

        if !decls.is_empty() {
            module_items.push(vir::ModuleItem::Declarations(decls));
        }

        if !conts.is_empty() {
            module_items.push(vir::ModuleItem::ContinuousAssigns(conts));
        }

        if !stmts.is_empty() {
            // TODO: Make internal logic a function
            module_items.push(vir::ModuleItem::AlwaysConstruct("initial".to_string(), stmts.clone()));
            module_items.push(vir::ModuleItem::AlwaysConstruct("always @*".to_string(), stmts));
        }

        Ok(module_items)
    }

    /// Generates FSM state.
    ///
    /// Returns `Err` if types of `typ` and `state` are mismatched.
    fn gen_module_fsm_state(
        &self, target: String, state: Merkle<lir::Expr>, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>), lir::ModuleError> {
        let (decls, mut stmts, expr) = self.gen_expr(&state, ctx, cache)?;

        let assignments =
            match_value_typ_exprs(Some(format!("{}_{}", ctx.get_prefix().unwrap(), target)), state.port_decls(), expr);

        for (_, var_name, expr) in assignments {
            let reg_name = format!("{}_reg", var_name);
            stmts.push(Statement::nonblocking_assignment(vir::Expression::ident(reg_name), expr));
        }

        Ok((decls, stmts))
    }

    /// Generates corresponding Verilog code for Expr.
    ///
    /// Returns required declarations and statements for expr output, and the expression tree
    /// indicating the expr output. If the expr has invalid width or mismatched type, returns `Err`.
    fn gen_expr(
        &self, expr: &lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        if let Some(prefix) = cache.get(expr) {
            return Ok((
                Vec::new(),
                Vec::new(),
                CompositeExpr::from_typ(expr.port_decls(), prefix.clone()).map(|(ident, _)| Expression::ident(ident)),
            ));
        }

        match expr {
            lir::Expr::X { .. } | lir::Expr::Constant { .. } => {
                let literal = gen_expr_literal(expr).map(|s| {
                    if s.is_empty() {
                        Expression::number("0".to_string())
                    } else if s.iter().all(|x| *x == LogicValue::False) {
                        Expression::number(format!("{}'b0", s.len()))
                    } else if s.iter().all(|x| *x == LogicValue::X) {
                        Expression::number(format!("{}'bx", s.len()))
                    } else {
                        Expression::number(format!("{}'b{}", s.len(), s.to_string(),))
                    }
                });

                Ok((Vec::new(), Vec::new(), literal))
            }
            lir::Expr::BinaryOp { op, lhs, rhs } => {
                self.gen_expr_binary_op(*op, &lhs.into_expr(), &rhs.into_expr(), ctx, cache)
            }
            lir::Expr::Member { inner, index } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr(&inner.into_expr(), ctx, cache)?;

                match exprs_for_inner {
                    CompositeExpr::Struct(mut fields) => {
                        Ok((decls_for_inner, stmts_for_inner, fields.swap_remove(*index)))
                    }
                    _ => panic!("gen_expr: cannot index bits"),
                }
            }
            lir::Expr::Concat { inner, .. } => self.gen_expr(&inner.into_expr(), ctx, cache),
            lir::Expr::Fold { inner, typ_elt, func, init, acc, inner_slice } => self.gen_expr_fold(
                expr,
                &inner.into_expr(),
                typ_elt,
                &init.into_expr(),
                &acc.into_expr(),
                &inner_slice.into_expr(),
                &func.into_expr(),
                ctx,
                cache,
            ),
            lir::Expr::Map { inner, typ_elt, func } => {
                self.gen_expr_map(expr, &inner.into_expr(), typ_elt, &func.into_expr(), ctx, cache)
            }
            lir::Expr::Repeat { inner, count } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr(&inner.into_expr(), ctx, cache)?;
                let exprs = exprs_for_inner.map(|expr| expr.multiple_concat(*count));

                Ok((decls_for_inner, stmts_for_inner, exprs))
            }
            lir::Expr::Input { name, .. } => Ok((
                Vec::new(),
                Vec::new(),
                CompositeExpr::from_typ(
                    expr.port_decls(),
                    join_options("_", [ctx.get_prefix(), name.clone()]).unwrap(),
                )
                .map(|(ident, _)| Expression::ident(ident)),
            )),
            lir::Expr::Resize { inner, .. } => self.gen_expr(&inner.into_expr(), ctx, cache),
            lir::Expr::Not { inner } => self.gen_expr_unary_op(lir::UnaryOp::Negation, &inner.into_expr(), ctx, cache),
            // TODO: Use conditional expression?
            lir::Expr::Cond { cond, lhs, rhs } => {
                let (decls_for_cond, stmts_for_cond, exprs_for_cond) = self.gen_expr(&cond.into_expr(), ctx, cache)?;
                let (decls_for_lhs, stmts_for_lhs, exprs_for_lhs) = self.gen_expr(&lhs.into_expr(), ctx, cache)?;
                let (decls_for_rhs, stmts_for_rhs, exprs_for_rhs) = self.gen_expr(&rhs.into_expr(), ctx, cache)?;

                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;

                let stmt_for_conditional = Statement::Conditional(
                    exprs_for_cond.into_expr(),
                    self.assign_exprs(exprs_for_output.clone(), exprs_for_lhs)?,
                    self.assign_exprs(exprs_for_output.clone(), exprs_for_rhs)?,
                );

                let decls = [decls_for_cond, decls_for_lhs, decls_for_rhs, decls_for_output].concat();
                let stmts = [stmts_for_cond, stmts_for_lhs, stmts_for_rhs, vec![stmt_for_conditional]].concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::LeftShift { inner, rhs } => {
                self.gen_expr_binary_op(lir::BinaryOp::ShiftLeft, &inner.into_expr(), &rhs.into_expr(), ctx, cache)
            }
            lir::Expr::RightShift { inner, rhs } => {
                self.gen_expr_binary_op(lir::BinaryOp::ShiftRight, &inner.into_expr(), &rhs.into_expr(), ctx, cache)
            }
            lir::Expr::Chunk { inner, .. } => self.gen_expr(&inner.into_expr(), ctx, cache),
            lir::Expr::Get { inner, typ_elt, index } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr_to_idents(&inner.into_expr(), ctx, cache)?;
                let (decls_for_index, stmts_for_index, exprs_for_index) =
                    self.gen_expr(&index.into_expr(), ctx, cache)?;

                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;

                let exprs_for_rhs = self.indexing_exprs(
                    exprs_for_inner,
                    exprs_for_index.into_expr(),
                    typ_elt.clone(),
                    inner.into_expr().port_decls(),
                )?;

                let stmts_for_assign = self.assign_exprs(exprs_for_output.clone(), exprs_for_rhs)?;

                let decls = [decls_for_inner, decls_for_index, decls_for_output].concat();
                let stmts = [stmts_for_inner, stmts_for_index, stmts_for_assign].concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::Clip { inner, from, size, typ_elt } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr_to_idents(&inner.into_expr(), ctx, cache)?;
                let (decls_for_from, stmts_for_from, exprs_for_from) = self.gen_expr(&from.into_expr(), ctx, cache)?;

                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;

                let exprs_for_elts = self.range_indexing_exprs(
                    exprs_for_inner,
                    exprs_for_from.into_expr(),
                    Expression::number(size.to_string()),
                    typ_elt.clone(),
                )?;
                let stmts_for_assign = self.assign_exprs(exprs_for_output.clone(), exprs_for_elts)?;

                let decls = [decls_for_inner, decls_for_from, decls_for_output].concat();
                let stmts = [stmts_for_inner, stmts_for_from, stmts_for_assign].concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::Append { lhs, rhs, .. } => {
                let (decls_for_lhs, stmts_for_lhs, exprs_for_lhs) = self.gen_expr(&lhs.into_expr(), ctx, cache)?;
                let (decls_for_rhs, stmts_for_rhs, exprs_for_rhs) = self.gen_expr(&rhs.into_expr(), ctx, cache)?;

                let decls = [decls_for_lhs, decls_for_rhs].concat();
                let stmts = [stmts_for_lhs, stmts_for_rhs].concat();
                let exprs = exprs_for_lhs.zip(exprs_for_rhs).map(|(lhs, rhs)| rhs.concat(lhs));

                Ok((decls, stmts, exprs))
            }
            lir::Expr::Zip { inner, .. } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) = inner
                    .iter()
                    .map(|expr_id| self.gen_expr(&expr_id.into_expr(), ctx, cache).expect("gen_expr: zip"))
                    .fold(
                        (Vec::new(), Vec::new(), Vec::new()),
                        |(mut acc_decls, mut acc_stmts, mut acc_exprs), (decls, stmts, exprs)| {
                            acc_decls.push(decls);
                            acc_stmts.push(stmts);
                            acc_exprs.push(exprs);
                            (acc_decls, acc_stmts, acc_exprs)
                        },
                    );
                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;

                let exprs_for_zipped = CompositeExpr::Struct(exprs_for_inner);
                let stmts_for_assign = self.assign_exprs(exprs_for_output.clone(), exprs_for_zipped)?;

                let decls = [decls_for_inner.concat(), decls_for_output].concat();
                let stmts = [stmts_for_inner.concat(), stmts_for_assign].concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::Struct { inner } => {
                let (decls, stmts, exprs) = inner
                    .iter()
                    .map(|(_, inner)| self.gen_expr(&inner.into_expr(), ctx, cache).unwrap())
                    .fold((Vec::new(), Vec::new(), Vec::new()), |mut acc, mut x| {
                        acc.0.append(&mut x.0);
                        acc.1.append(&mut x.1);
                        acc.2.push(x.2);
                        acc
                    });

                Ok((decls, stmts, CompositeExpr::Struct(exprs)))
            }
            lir::Expr::Repr { inner } => self.gen_expr(&inner.into_expr(), ctx, cache),
            lir::Expr::Set { inner, index, elt } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr(&inner.into_expr(), ctx, cache)?;
                let (decls_for_index, stmts_for_index, exprs_for_index) =
                    self.gen_expr(&index.into_expr(), ctx, cache)?;
                let (decls_for_elt, stmts_for_elt, exprs_for_elt) = self.gen_expr(&elt.into_expr(), ctx, cache)?;

                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;
                let stmts_for_assign = self.assign_exprs(exprs_for_output.clone(), exprs_for_inner)?;

                let exprs_for_output_elt = self.indexing_exprs(
                    exprs_for_output.clone(),
                    exprs_for_index.into_expr(),
                    elt.into_expr().port_decls(),
                    expr.port_decls(),
                )?;
                let stmts_for_assign_elt = self.assign_exprs(exprs_for_output_elt, exprs_for_elt)?;

                let decls = [decls_for_inner, decls_for_index, decls_for_elt, decls_for_output].concat();
                let stmts =
                    [stmts_for_inner, stmts_for_index, stmts_for_elt, stmts_for_assign, stmts_for_assign_elt].concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::SetRange { inner, typ_elt, index, elts } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr(&inner.into_expr(), ctx, cache)?;
                let (decls_for_index, stmts_for_index, exprs_for_index) =
                    self.gen_expr(&index.into_expr(), ctx, cache)?;
                let (decls_for_elts, stmts_for_elts, exprs_for_elts) = self.gen_expr(&elts.into_expr(), ctx, cache)?;

                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;
                let stmts_for_assign = self.assign_exprs(exprs_for_output.clone(), exprs_for_inner)?;

                let elts_count = elts.into_expr().width() / typ_elt.width();

                let exprs_for_output_elts = self.range_indexing_exprs(
                    exprs_for_output.clone(),
                    exprs_for_index.into_expr(),
                    Expression::number(elts_count.to_string()),
                    typ_elt.clone(),
                )?;
                let stmts_for_assign_elts = self.assign_exprs(exprs_for_output_elts, exprs_for_elts)?;

                let decls = [decls_for_inner, decls_for_index, decls_for_elts, decls_for_output].concat();
                let stmts = [stmts_for_inner, stmts_for_index, stmts_for_elts, stmts_for_assign, stmts_for_assign_elts]
                    .concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::Sum { inner, width_elt } => self.gen_expr_sum(expr, &inner.into_expr(), *width_elt, ctx, cache),
            lir::Expr::GetVarArray { inner, index, .. } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr_to_idents(&inner.into_expr(), ctx, cache)?;
                let (decls_for_index, stmts_for_index, exprs_for_index) =
                    self.gen_expr(&index.into_expr(), ctx, cache)?;

                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;
                let expr_for_index = exprs_for_index.into_expr();
                let stmts_for_output = self.assign_exprs(
                    exprs_for_output.clone(),
                    exprs_for_inner.map(|expr| expr.with_range(Range::new_index(expr_for_index.clone()))),
                )?;

                let decls = [decls_for_inner, decls_for_index, decls_for_output].concat();
                let stmts = [stmts_for_inner, stmts_for_index, stmts_for_output].concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::SetVarArray { inner, index, elt } => {
                let (decls_for_inner, stmts_for_inner, exprs_for_inner) =
                    self.gen_expr(&inner.into_expr(), ctx, cache)?;
                let (decls_for_index, stmts_for_index, exprs_for_index) =
                    self.gen_expr(&index.into_expr(), ctx, cache)?;
                let (decls_for_elt, stmts_for_elt, exprs_for_elt) = self.gen_expr(&elt.into_expr(), ctx, cache)?;

                let expr_for_index = exprs_for_index.into_expr();
                let stmts_for_assign = exprs_for_inner
                    .clone()
                    .zip(exprs_for_elt)
                    .iter()
                    .map(|(expr_for_inner, expr_for_elt)| {
                        Statement::nonblocking_assignment(
                            Expression::ident(expr_for_inner.to_string())
                                .with_range(Range::new_index(expr_for_index.clone())),
                            expr_for_elt,
                        )
                    })
                    .collect::<Vec<_>>();

                let decls = [decls_for_inner, decls_for_index, decls_for_elt].concat();
                let stmts = [stmts_for_inner, stmts_for_index, stmts_for_elt, stmts_for_assign].concat();

                Ok((decls, stmts, exprs_for_inner))
            }
            lir::Expr::Case { case_expr, case_items, default } => {
                let (decls_for_case_expr, stmts_for_case_expr, exprs_for_case_expr) =
                    self.gen_expr(&case_expr.into_expr(), ctx, cache)?;

                let (
                    decls_for_case_conds,
                    stmts_for_case_conds,
                    exprs_for_case_conds,
                    decls_for_case_stmts,
                    stmts_for_case_stmts,
                    exprs_for_case_stmts,
                ) = case_items
                    .iter()
                    .map(|(cond, expr)| {
                        (
                            self.gen_expr(&cond.into_expr(), ctx, cache).unwrap(),
                            self.gen_expr(&expr.into_expr(), ctx, cache).unwrap(),
                        )
                    })
                    .fold((Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()), |mut acc, x| {
                        acc.0.push(x.0 .0.clone());
                        acc.1.push(x.0 .1.clone());
                        acc.2.push(x.0 .2.clone());
                        acc.3.push(x.1 .0.clone());
                        acc.4.push(x.1 .1.clone());
                        acc.5.push(x.1 .2);
                        acc
                    });

                let (decls_for_default, stmts_for_default, exprs_for_default) =
                    (*default).map_or((None, None, None), |d| {
                        let (decls, stmts, exprs) = self.gen_expr(&d.into_expr(), ctx, cache).unwrap();
                        (Some(decls), Some(stmts), Some(exprs))
                    });

                let decls_for_default = decls_for_default.unwrap_or_default();
                let stmts_for_default = stmts_for_default.unwrap_or_default();

                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;

                let stmt_for_case = Statement::Case(
                    exprs_for_case_expr.into_expr(),
                    itertools::izip!(exprs_for_case_conds, exprs_for_case_stmts)
                        .map(|(expr_cond, expr_stmt)| {
                            (expr_cond.into_expr(), self.assign_exprs(exprs_for_output.clone(), expr_stmt).unwrap())
                        })
                        .collect::<Vec<_>>(),
                    exprs_for_default
                        .map(|exprs| self.assign_exprs(exprs_for_output.clone(), exprs).unwrap())
                        .unwrap_or_default(),
                );

                let decls = [
                    decls_for_case_expr,
                    decls_for_case_conds.concat(),
                    decls_for_case_stmts.concat(),
                    decls_for_default,
                    decls_for_output,
                ]
                .concat();

                let stmts = [
                    stmts_for_case_expr,
                    stmts_for_case_conds.concat(),
                    stmts_for_case_stmts.concat(),
                    stmts_for_default,
                    vec![stmt_for_case],
                ]
                .concat();

                Ok((decls, stmts, exprs_for_output))
            }
            lir::Expr::Call { func_name, args, .. } => {
                let (decls_for_args, stmts_for_args, exprs_for_args) = args
                    .iter()
                    .map(|arg_expr| self.gen_expr(&arg_expr.into_expr(), ctx, cache).unwrap())
                    .fold((Vec::new(), Vec::new(), Vec::new()), |mut acc, x| {
                        acc.0.push(x.0);
                        acc.1.push(x.1);
                        acc.2.push(x.2);

                        acc
                    });

                let decls = decls_for_args.concat();
                let stmts = stmts_for_args.concat();

                let expr = Expression::function_call(
                    func_name,
                    exprs_for_args.into_iter().map(|expr| expr.into_expr()).collect::<Vec<_>>(),
                );
                let exprs = CompositeExpr::Bits(expr);

                Ok((decls, stmts, exprs))
            }
            lir::Expr::TreeFold { inner, op, lhs, rhs, acc } => self.gen_expr_tree_fold(
                expr,
                &inner.into_expr(),
                &op.into_expr(),
                &lhs.into_expr(),
                &rhs.into_expr(),
                &acc.into_expr(),
                ctx,
                cache,
            ),
            lir::Expr::ConcatArray { inner, elt_typ } => {
                let (decls_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;
                let mut assign_decls = vec![];
                let mut assign_stmts = vec![];

                for (i, expr_elt) in inner.iter().enumerate() {
                    let (decls_for_elt, stmts_for_elt, exprs_for_elt) =
                        self.gen_expr(&expr_elt.into_expr(), ctx, cache)?;
                    let stmts_for_assign = self.assign_exprs(
                        self.indexing_exprs(
                            exprs_for_output.clone(),
                            Expression::number(i.to_string()),
                            elt_typ.clone(),
                            expr.port_decls(),
                        )?,
                        exprs_for_elt,
                    )?;

                    assign_decls.extend(decls_for_elt);
                    assign_stmts.extend(vec![stmts_for_elt, stmts_for_assign].concat());
                }

                let decls = vec![decls_for_output, assign_decls].concat();
                let stmts = assign_stmts;

                Ok((decls, stmts, exprs_for_output))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_expr_tree_fold(
        &self, expr: &lir::Expr, inner: &lir::Expr, op: &lir::Expr, lhs: &lir::Expr, rhs: &lir::Expr, acc: &lir::Expr,
        ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let num_elts = inner.width() / lhs.width();

        let (decls_for_inner, stmts_for_inner, exprs_for_inner) = self.gen_expr_to_idents(inner, ctx, cache)?;

        // decls for outer for loop
        let outer_loop_int = ctx.alloc_int_id();
        let outer_loop_int_variable = format!("{}_level", outer_loop_int);
        let outer_loop_count = clog2(num_elts);
        let decl_for_outer_loop_count = Declaration::integer(outer_loop_int_variable.clone());

        let tree_fold_prefix = ctx.alloc_temp_id();
        let decl_acc_reg = Declaration::reg_with_typ(inner.port_decls(), Some(format!("{}_acc", tree_fold_prefix)));

        let mut ctx = Context::new();

        ctx.enter_scope(tree_fold_prefix.clone());

        let (decls_for_acc, stmts_for_acc, exprs_for_acc) = self.gen_expr(acc, &mut ctx, cache)?;
        let stmts_for_acc_init = self.assign_exprs(exprs_for_acc.clone(), exprs_for_inner)?;

        let inner_loop_int = ctx.alloc_int_id();
        let decl_for_inner_loop_count = Declaration::integer(inner_loop_int.clone());

        let decl_lhs_reg = Declaration::reg_with_typ(lhs.port_decls(), Some(format!("{}_lhs", tree_fold_prefix)));
        let (decls_for_lhs, stmts_for_lhs, exprs_for_lhs) = self.gen_expr(lhs, &mut ctx, cache)?;
        let stmts_for_lhs_expr = self.assign_exprs(
            exprs_for_lhs,
            self.indexing_exprs(
                exprs_for_acc.clone(),
                // idx 2*i
                Expression::binary(
                    lir::BinaryOp::Mul,
                    Expression::number(2.to_string()),
                    Expression::ident(inner_loop_int.clone()),
                ),
                lhs.port_decls(),
                inner.port_decls(),
            )?,
        )?;

        let decl_rhs_reg = Declaration::reg_with_typ(rhs.port_decls(), Some(format!("{}_rhs", tree_fold_prefix)));
        let (decls_for_rhs, stmts_for_rhs, exprs_for_rhs) = self.gen_expr(rhs, &mut ctx, cache)?;
        let stmts_for_rhs_expr = self.assign_exprs(
            exprs_for_rhs,
            self.indexing_exprs(
                exprs_for_acc.clone(),
                // idx 2*i + 1
                Expression::binary(
                    lir::BinaryOp::Add,
                    Expression::binary(
                        lir::BinaryOp::Mul,
                        Expression::number(2.to_string()),
                        Expression::ident(inner_loop_int.clone()),
                    ),
                    Expression::number(1.to_string()),
                ),
                rhs.port_decls(),
                inner.port_decls(),
            )?,
        )?;

        let (decls_for_loop_op, stmts_for_loop_op, exprs_for_loop_op) = self.gen_expr(op, &mut ctx, cache)?;
        let stmt_for_loop_body_operation = self.assign_exprs(
            self.indexing_exprs(
                exprs_for_acc.clone(),
                Expression::ident(inner_loop_int.clone()),
                lhs.port_decls(),
                inner.port_decls(),
            )?,
            exprs_for_loop_op,
        )?;

        let stmt_for_inner_loop = Statement::Loop(
            inner_loop_int,
            Expression::binary(
                lir::BinaryOp::Div,
                Expression::number(num_elts.to_string()),
                Expression::binary(
                    lir::BinaryOp::ShiftLeft,
                    Expression::number(1.to_string()),
                    Expression::binary(
                        lir::BinaryOp::Add,
                        Expression::ident(outer_loop_int_variable.clone()),
                        Expression::number(1.to_string()),
                    ),
                ),
            ),
            [stmts_for_lhs_expr, stmts_for_rhs_expr, stmts_for_loop_op, stmt_for_loop_body_operation].concat(),
        );

        let stmt_for_outer_loop = Statement::Loop(
            outer_loop_int_variable,
            Expression::binary(
                lir::BinaryOp::Sub,
                Expression::number(outer_loop_count.to_string()),
                Expression::number(1.to_string()),
            ),
            vec![stmt_for_inner_loop],
        );

        let decls_for_loop = [
            vec![decl_for_outer_loop_count, decl_for_inner_loop_count],
            decl_acc_reg,
            decl_lhs_reg,
            decl_rhs_reg,
            decls_for_acc,
            decls_for_lhs,
            decls_for_rhs,
            decls_for_loop_op,
        ]
        .concat();

        let fold_prefix = ctx.alloc_temp_id();
        let expr_for_fold_output = codegen::CompositeExpr::from_typ(lhs.port_decls(), fold_prefix.clone())
            .map(|(ident, _)| Expression::ident(ident));

        let decl_for_fold_output = Declaration::reg_with_typ(lhs.port_decls(), Some(fold_prefix.clone()));
        let stmt_epilogue = self.assign_exprs(
            expr_for_fold_output.clone(),
            self.indexing_exprs(
                exprs_for_acc,
                Expression::number(0.to_string()),
                lhs.port_decls(),
                inner.port_decls(),
            )?,
        )?;

        let decls = vec![decls_for_inner, decls_for_loop, decl_for_fold_output].concat();
        let stmts = vec![
            stmts_for_acc,
            stmts_for_lhs,
            stmts_for_rhs,
            stmts_for_inner,
            stmts_for_acc_init,
            vec![stmt_for_outer_loop],
            stmt_epilogue,
        ]
        .concat();

        cache.insert(expr.clone(), fold_prefix);

        Ok((decls, stmts, expr_for_fold_output))
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_expr_fold(
        &self, expr: &lir::Expr, inner: &lir::Expr, typ_elt: &lir::PortDecls, init: &lir::Expr, acc: &lir::Expr,
        inner_slice: &lir::Expr, func: &lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        // Step0. Declaration for intermediate values
        // Variables for loop indexing
        let loop_int = ctx.alloc_int_id();
        let loop_count = inner.width() / typ_elt.width();
        let decl_for_loop_int = Declaration::integer(loop_int.clone());

        let fold_body_input_prefix = ctx.alloc_temp_id();
        let decl_acc_reg =
            Declaration::reg_with_typ(acc.port_decls(), Some(format!("{}_{}", fold_body_input_prefix, "acc")));

        // Step 1. initialize acc with init
        let (decls_for_init, stmts_for_init, init_expr) = self.gen_expr(init, ctx, cache)?;

        let (decls_for_inner, stmts_for_inner, exprs_for_inner) = self.gen_expr_to_idents(inner, ctx, cache)?;

        let mut ctx = Context::new();

        ctx.enter_scope(fold_body_input_prefix.clone());

        let (decls_for_acc, stmts_for_acc, exprs_for_acc) = self.gen_expr(acc, &mut ctx, cache)?;
        // Step 2. logic inside loop
        let (decls_for_inner_slice, stmts_for_inner_slice, exprs_for_inner_slice) =
            self.gen_expr(inner_slice, &mut ctx, cache)?;

        let stmt_acc_initialization = self.assign_exprs(exprs_for_acc.clone(), init_expr)?;

        let decl_inner_slice_reg = Declaration::reg_with_typ(
            inner_slice.port_decls(),
            Some(format!("{}_{}", fold_body_input_prefix, "inner_slice")),
        );
        // slice the inner into exprs_for_inner_slice
        let stmts_for_body_inner_slice = self.assign_exprs(
            exprs_for_inner_slice,
            self.indexing_exprs(
                exprs_for_inner,
                Expression::ident(loop_int.clone()),
                typ_elt.clone(),
                inner.port_decls(),
            )?,
        )?;

        let (decls_for_loop_body, stmts_for_loop_body, exprs_for_loop_body) =
            self.gen_expr(func, &mut ctx, &mut HashMap::new())?;

        // assign output of closure
        let stmt_for_loop_body_output = self.assign_exprs(exprs_for_acc.clone(), exprs_for_loop_body)?;

        let stmt_for_loop = Statement::Loop(
            loop_int,
            Expression::number(loop_count.to_string()),
            [stmts_for_body_inner_slice, stmts_for_loop_body, stmt_for_loop_body_output].concat(),
        );

        let decls_for_loop =
            [[vec![decl_for_loop_int], decls_for_inner_slice, decls_for_loop_body, decls_for_acc].concat()].concat();

        // Step 3. Epiogue
        let fold_prefix = ctx.alloc_temp_id();
        let expr_for_fold_output = codegen::CompositeExpr::from_typ(acc.port_decls(), fold_prefix.clone())
            .map(|(ident, _)| Expression::ident(ident));
        let decl_epilogue_reg = Declaration::reg_with_typ(acc.port_decls(), Some(fold_prefix.clone()));
        let stmt_epilogue = self.assign_exprs(expr_for_fold_output.clone(), exprs_for_acc)?;

        let decls =
            [decl_acc_reg, decl_inner_slice_reg, decl_epilogue_reg, decls_for_inner, decls_for_init, decls_for_loop]
                .concat();
        let stmts = [
            stmts_for_acc,
            stmts_for_init,
            stmts_for_inner,
            stmt_acc_initialization,
            stmts_for_inner_slice,
            vec![stmt_for_loop],
            stmt_epilogue,
        ]
        .concat();

        cache.insert(expr.clone(), fold_prefix);

        Ok((decls, stmts, expr_for_fold_output))
    }

    fn gen_expr_unary_op(
        &self, op: lir::UnaryOp, inner: &lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (decls_for_inner, stmts_for_inner, exprs_for_inner) = self.gen_expr(inner, ctx, cache)?;

        let expr = Expression::unary(op, exprs_for_inner.into_expr());
        let exprs = CompositeExpr::Bits(expr);

        Ok((decls_for_inner, stmts_for_inner, exprs))
    }

    fn gen_expr_binary_op(
        &self, op: lir::BinaryOp, lhs: &lir::Expr, rhs: &lir::Expr, ctx: &mut Context,
        cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (decls_for_lhs, stmts_for_lhs, exprs_for_lhs) = self.gen_expr(lhs, ctx, cache)?;
        let (decls_for_rhs, stmts_for_rhs, exprs_for_rhs) = self.gen_expr(rhs, ctx, cache)?;

        let expr = Expression::binary(op, exprs_for_lhs.into_expr(), exprs_for_rhs.into_expr());

        let decls = [decls_for_lhs, decls_for_rhs].concat();
        let stmts = [stmts_for_lhs, stmts_for_rhs].concat();
        let exprs = CompositeExpr::Bits(expr);

        Ok((decls, stmts, exprs))
    }

    fn gen_expr_map(
        &self, expr: &lir::Expr, inner: &lir::Expr, typ_elt: &lir::PortDecls, func: &lir::Expr, ctx: &mut Context,
        cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (decls_for_inner, stmts_for_inner, exprs_for_inner) = self.gen_expr_to_idents(inner, ctx, cache)?;

        let loop_int = ctx.alloc_int_id();
        let loop_count = inner.width() / typ_elt.width();

        let decl_for_loop_int = Declaration::integer(loop_int.clone());

        let loop_body_input_prefix = ctx.alloc_temp_id();
        let (decls_for_loop_input, exprs_for_loop_input) = {
            let exprs = CompositeExpr::from_typ(typ_elt.clone(), loop_body_input_prefix.clone());

            let decls = exprs.iter().map(|(ident, shape)| Declaration::reg(shape, ident)).collect::<Vec<_>>();
            let exprs = exprs.map(|(ident, _)| Expression::ident(ident));

            (decls, exprs)
        };

        let (decls_for_loop_output, exprs_for_loop_output) = self.alloc_exprs(expr.clone(), ctx, cache)?;

        let stmts_for_loop_body_input = self.assign_exprs(
            exprs_for_loop_input,
            self.indexing_exprs(
                exprs_for_inner,
                Expression::ident(loop_int.clone()),
                typ_elt.clone(),
                inner.port_decls(),
            )?,
        )?;

        let mut ctx = Context::new();
        ctx.enter_scope(loop_body_input_prefix);

        let (decls_for_loop_body, stmts_for_loop_body, exprs_for_loop_body) =
            self.gen_expr(func, &mut ctx, &mut HashMap::new())?;

        let stmts_for_loop_body_output = self.assign_exprs(
            self.indexing_exprs(
                exprs_for_loop_output.clone(),
                Expression::ident(loop_int.clone()),
                func.port_decls(),
                expr.port_decls(),
            )?,
            exprs_for_loop_body,
        )?;

        let decls_for_loop =
            [vec![decl_for_loop_int], decls_for_loop_input, decls_for_loop_output, decls_for_loop_body].concat();

        let stmt_for_loop = Statement::Loop(
            loop_int,
            Expression::number(loop_count.to_string()),
            [stmts_for_loop_body_input, stmts_for_loop_body, stmts_for_loop_body_output].concat(),
        );

        let decls = [decls_for_inner, decls_for_loop].concat();
        let stmts = [stmts_for_inner, vec![stmt_for_loop]].concat();

        Ok((decls, stmts, exprs_for_loop_output))
    }

    fn gen_expr_sum(
        &self, expr: &lir::Expr, inner: &lir::Expr, width_elt: usize, ctx: &mut Context,
        cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (decls_for_inner, stmts_for_inner, exprs_for_inner) = self.gen_expr_to_idents(inner, ctx, cache)?;
        let loop_int = ctx.alloc_int_id();
        let loop_count = inner.width() / width_elt;
        let loop_body_input_prefix = ctx.alloc_temp_id();
        let decl_for_loop_int = Declaration::integer(loop_int.clone());
        let decl_for_loop_body_input =
            Declaration::reg(lir::Shape::new([expr.width()]), loop_body_input_prefix.clone())
                .with_init(Expression::number("0".to_string()));

        let expr_for_loop_acc = Expression::ident(loop_body_input_prefix.clone());

        let stmt_for_loop_int_init =
            Statement::blocking_assignment(expr_for_loop_acc.clone(), Expression::number("0".to_string()));

        let stmt_for_loop_body = Statement::blocking_assignment(
            expr_for_loop_acc.clone(),
            Expression::binary(
                lir::BinaryOp::Add,
                expr_for_loop_acc,
                self.indexing_exprs(
                    exprs_for_inner,
                    Expression::ident(loop_int.clone()),
                    expr.port_decls(),
                    inner.port_decls(),
                )?
                .into_expr(),
            ),
        );

        let stmt_for_loop =
            Statement::Loop(loop_int, Expression::number(loop_count.to_string()), vec![stmt_for_loop_body]);

        let decls_for_loop = vec![decl_for_loop_int, decl_for_loop_body_input];

        let decls = [decls_for_inner, decls_for_loop].concat();
        let stmts = [stmts_for_inner, vec![stmt_for_loop_int_init], vec![stmt_for_loop]].concat();

        cache.insert(expr.clone(), loop_body_input_prefix.clone());
        Ok((decls, stmts, CompositeExpr::Bits(Expression::ident(loop_body_input_prefix))))
    }

    fn gen_expr_to_idents(
        &self, expr: &lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (mut decls, mut stmts, exprs) = self.gen_expr(expr, ctx, cache)?;

        // If every expressions are idents, return immediately
        if exprs.iter().all(|expr| expr.is_identifier()) {
            return Ok((decls, stmts, exprs));
        }

        let (mut decls_for_alloc, new_exprs) = self.alloc_exprs(expr.clone(), ctx, &mut HashMap::new())?;
        let mut stmts_for_assign = self.assign_exprs(new_exprs.clone(), exprs)?;

        decls.append(&mut decls_for_alloc);
        stmts.append(&mut stmts_for_assign);

        Ok((decls, stmts, new_exprs))
    }

    fn indexing_exprs(
        &self, exprs: CompositeExpr<Expression>, index: Expression, typ_elt: lir::PortDecls, typ: lir::PortDecls,
    ) -> Result<CompositeExpr<Expression>, lir::ModuleError> {
        let exprs_for_elt = exprs.zip(typ_elt.into()).zip(typ.into()).map(|((expr, (_, shape_elt)), (_, shape))| {
            // `gen_expr()` considers all `lir::lir::Expr`s with width 1 as single bit, not an array.
            if shape.width() > 1 {
                expr.with_range(Range::new_range(
                    Expression::binary(
                        lir::BinaryOp::Mul,
                        index.clone(),
                        Expression::number(shape_elt.width().to_string()),
                    ),
                    Expression::number(shape_elt.width().to_string()),
                ))
            } else {
                expr
            }
        });

        Ok(exprs_for_elt)
    }

    fn range_indexing_exprs(
        &self, exprs: CompositeExpr<Expression>, base: Expression, offset: Expression, typ_elt: lir::PortDecls,
    ) -> Result<CompositeExpr<Expression>, lir::ModuleError> {
        let exprs = exprs.zip(typ_elt.into()).map(|(expr, (_, shape))| {
            expr.with_range(Range::new_range(
                Expression::binary(lir::BinaryOp::Mul, base.clone(), Expression::number(shape.width().to_string())),
                Expression::binary(lir::BinaryOp::Mul, offset.clone(), Expression::number(shape.width().to_string())),
            ))
        });

        Ok(exprs)
    }

    fn alloc_exprs(
        &self, expr: lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Declaration>, CompositeExpr<Expression>), lir::ModuleError> {
        let typ = expr.port_decls();
        let prefix = ctx.alloc_temp_id();
        let exprs = CompositeExpr::from_typ(typ, prefix.clone());

        let decls = exprs.iter().map(|(ident, shape)| Declaration::reg(shape, ident)).collect::<Vec<_>>();
        let exprs = exprs.map(|(ident, _)| Expression::ident(ident));

        cache.insert(expr, prefix);

        Ok((decls, exprs))
    }

    fn assign_exprs(
        &self, lhs: CompositeExpr<Expression>, rhs: CompositeExpr<Expression>,
    ) -> Result<Vec<Statement>, lir::ModuleError> {
        let stmts =
            lhs.zip(rhs).iter().map(|(lvalue, expr)| Statement::blocking_assignment(lvalue, expr)).collect::<Vec<_>>();

        Ok(stmts)
    }
}

//! Generates FIRRTL code.
//!
//! # Note
//!
//! In FIRRTL generation, only 1-dimensional signals are supported.

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use hashcons::merkle::Merkle;

use crate::codegen::*;
use crate::fir::*;
use crate::*;

impl Package {
    /// Generates FIRRTL code at the given directory path.
    pub fn gen_fir<P: AsRef<Path>>(self, path_dir: P) -> Result<(), PackageError> {
        fs::create_dir_all(path_dir.as_ref()).map_err(|error| PackageError::Fs { error })?;

        for module in self.modules.into_iter() {
            let path = path_dir.as_ref().join(format!("{}_inner.fir", module.get_module_name()));
            let mut file = File::create(path).map_err(|error| PackageError::Fs { error })?;

            let name = module.get_module_name();
            let module =
                gen_module::<Firgen>(name.clone(), &module).map_err(|error| PackageError::Module { error })?.into();
            let circuit = Circuit { modules: vec![module], main: name };

            writeln!(file, "{}", circuit.to_string()).map_err(|error| PackageError::Fs { error })?;
        }

        Ok(())
    }
}

impl From<codegen::Module<Firgen>> for fir::Module {
    fn from(module: codegen::Module<Firgen>) -> Self {
        fir::Module { name: module.name, ports: module.ports, body: module.body }
    }
}

/// FIRRTL Generator
#[derive(Default, Debug)]
pub struct Firgen;

impl Codegen for Firgen {
    type Body = Statement;
    type Ports = Vec<Port>;

    fn gen_port_decls(&self, module: &lir::Module) -> Result<Vec<Port>, lir::ModuleError> {
        Ok(gen_port_decls(module)?
            .into_iter()
            .map(|(direction, width, name)| {
                if name == "clk" {
                    Port::input("clk".to_string(), Type::clock())
                } else {
                    Port { name, direction, tpe: Type::uint(width) }
                }
            })
            .collect())
    }

    fn gen_module_composite(
        &self, module: &lir::CompositeModule, ctx: &mut Context,
    ) -> Result<Statement, lir::ModuleError> {
        assert!(matches!(module.module_typ, lir::CompositeModuleTyp::OneToOne), "unsupported composite module type");

        let mut stmts = vec![];

        for (name, shape) in gen_submodule_wires(module, ctx)? {
            stmts.push(Statement::def_wire(name, Type::uint(shape.width())));
        }

        let mut conts = self.gen_module_wiring(module, ctx.get_prefix())?;
        stmts.append(&mut conts);

        // TODO: Add inner submodule's logic.
        for (index, (submodule, _)) in module.submodules.iter().enumerate() {
            let comp_name = submodule.get_module_name();
            ctx.enter_scope(format!("{}_{}", comp_name, index));
            match &*submodule.inner {
                lir::ModuleInner::Composite(_, module) => {
                    stmts.push(self.gen_module_composite(module, ctx)?);
                }
                lir::ModuleInner::Fsm(module) => {
                    stmts.push(self.gen_module_fsm(module, ctx)?);
                }
                lir::ModuleInner::ModuleInst(module) => {
                    stmts.push(self.gen_module_inst(module, ctx)?);
                }
                _ => todo!(),
            }
            ctx.leave_scope();
        }

        Ok(Statement::block(stmts))
    }

    /// Generates target code for FSM.
    ///
    /// # Note
    ///
    /// Layout of generated FIRRTL code:
    ///
    /// ```firrtl
    /// reg ... // (1) state reg, wire (decls)
    ///
    /// ... <= ... // (1) state reg, wire (conts)
    ///
    /// node ... // (2) input, output logic
    ///
    /// ... <= ... // (2) input, output logic
    ///
    /// node ... // (3) state update logic
    ///
    /// ... <= ... // (3) state update logic
    /// ```
    fn gen_module_fsm(&self, module: &lir::Fsm, ctx: &mut Context) -> Result<Statement, lir::ModuleError> {
        let mut stmts = Vec::new();

        let state_init = gen_module_fsm_state_init(&module.state.into_expr(), &module.init.into_expr(), ctx)?
            .into_iter()
            .map(|(shape, name, init_value)| {
                let init_value = LogicValues::new(
                    init_value
                        .iter()
                        .map(|b| if *b == LogicValue::X { LogicValue::False } else { *b })
                        .collect::<Vec<_>>(),
                );

                (shape, name, init_value)
            })
            .collect::<Vec<_>>();

        // (1) state reg, wire (decls, conts)
        {
            let (mut decls, mut conts) = (Vec::new(), Vec::new());

            state_init.iter().for_each(|(shape, net_name, init_value)| {
                let reg_name = format!("{}_reg", net_name);

                // Add decls
                decls.push(Statement::def_reg(
                    reg_name.clone(),
                    Type::uint(shape.width()),
                    Expression::literal(init_value.clone(), Some(shape.width())),
                ));
                decls.push(Statement::def_wire(net_name.clone(), Type::uint(shape.width())));

                // Add conts
                conts
                    .push(Statement::connect(Expression::reference(net_name.clone()), Expression::reference(reg_name)));
            });

            if !decls.is_empty() {
                stmts.push(Statement::block(decls));
            }

            if !conts.is_empty() {
                stmts.push(Statement::block(conts));
            }
        }

        // TODO: Give proper names
        let output_fwd = &module.output_fwd;
        let input_bwd = &module.input_bwd;
        let state = &module.state;

        // (2) input, output logic
        {
            assert_eq!(output_fwd.into_expr().port_decls().max_dim(), 1);
            assert_eq!(input_bwd.into_expr().port_decls().max_dim(), 1);

            // input, output logic for output forward exprs
            stmts.push(self.gen_module_fsm_output(
                "out".to_string(),
                output_fwd.into_expr(),
                ctx,
                &mut HashMap::new(),
            )?);

            // input, output logic for input backward exprs.
            stmts.push(self.gen_module_fsm_output(
                "in".to_string(),
                input_bwd.into_expr(),
                ctx,
                &mut HashMap::new(),
            )?);
        }

        // (3) state update logic
        stmts.push(self.gen_module_fsm_state("st".to_string(), state.into_expr(), ctx, &mut HashMap::new())?);

        Ok(Statement::block(stmts))
    }

    fn gen_module_inst(&self, module: &lir::ModuleInst, ctx: &mut Context) -> Result<Statement, lir::ModuleError> {
        let connections = gen_connections(module, ctx)?
            .into_iter()
            .map(|(dir, port, expr)| match dir {
                Direction::Input => Statement::PartialConnect {
                    loc: Expression::sub_field(Expression::reference(module.inst_name.clone()), port),
                    expr: Expression::reference(expr),
                },
                Direction::Output => Statement::PartialConnect {
                    loc: Expression::reference(expr),
                    expr: Expression::sub_field(Expression::reference(module.inst_name.clone()), port),
                },
            })
            .collect::<Vec<_>>();

        let module_inst = Statement::def_inst(module.inst_name.clone(), module.get_module_name());

        Ok(Statement::block(vec![vec![module_inst], connections].concat()))
    }

    fn gen_module_virtual(
        &self, _module: &lir::VirtualModule, _composite_context_prefix: Option<String>, _ctx: &mut Context,
    ) -> Result<Self::Body, lir::ModuleError> {
        todo!()
    }
}

impl Firgen {
    /// Generates FIRRTL code for wirings in the module.
    fn gen_module_wiring(
        &self, module: &lir::CompositeModule, prefix: Option<String>,
    ) -> Result<Vec<Statement>, lir::ModuleError> {
        Ok(gen_wiring(module, prefix)?
            .into_iter()
            .map(|(lvalue, lvalue_range, rvalue, rvalue_range)| {
                if lvalue_range.is_some() {
                    todo!("FIRRTL does not support indexing in L-value expression");
                }

                match rvalue_range {
                    Some((index, size)) => Statement::connect(
                        Expression::reference(lvalue),
                        Expression::bits(Expression::reference(rvalue), size * index + index - 1, size * index),
                    ),
                    None => Statement::connect(Expression::reference(lvalue), Expression::reference(rvalue)),
                }
            })
            .collect())
    }

    /// Generates FSM output.
    ///
    /// Returns `Err` if types of `typ` and `output` are mismatched.
    fn gen_module_fsm_output(
        &self, target: String, output: Merkle<lir::Expr>, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<Statement, lir::ModuleError> {
        let (mut stmts, expr) = self.gen_expr(&output, ctx, cache)?;

        let assignments =
            match_value_typ_exprs(Some(format!("{}_{}", ctx.get_prefix().unwrap(), target)), output.port_decls(), expr);

        let mut conts = assignments
            .into_iter()
            .map(|(_, var_name, expr)| Statement::connect(Expression::reference(var_name), expr))
            .collect::<Vec<_>>();

        if !conts.is_empty() {
            stmts.append(&mut conts);
        }

        Ok(Statement::block(stmts))
    }

    /// Generates FSM state.
    ///
    /// Returns `Err` if types of `typ` and `state` are mismatched.
    fn gen_module_fsm_state(
        &self, target: String, state: Merkle<lir::Expr>, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<Statement, lir::ModuleError> {
        let (mut stmts, exprs) = self.gen_expr(&state, ctx, cache)?;

        let assignments =
            match_value_typ_exprs(Some(format!("{}_{}", ctx.get_prefix().unwrap(), target)), state.port_decls(), exprs);

        let mut conts = assignments
            .into_iter()
            .map(|(_, var_name, expr)| {
                let reg_name = format!("{}_reg", var_name);

                Statement::connect(Expression::reference(reg_name), expr)
            })
            .collect();

        stmts.append(&mut conts);

        Ok(Statement::block(stmts))
    }

    /// Generates corresponding FIRRTL code for Expr.
    fn gen_expr(
        &self, expr: &lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        if let Some(prefix) = cache.get(expr) {
            return Ok((
                Vec::new(),
                CompositeExpr::from_typ(expr.port_decls(), prefix.clone())
                    .map(|(ident, _)| Expression::reference(ident)),
            ));
        }

        match expr {
            lir::Expr::X { .. } | lir::Expr::Constant { .. } => {
                let literal = gen_expr_literal(expr).map(|s| {
                    if s.is_empty() {
                        0.into()
                    } else {
                        Expression::literal(s.clone(), Some(s.len()))
                    }
                });

                Ok((Vec::new(), literal))
            }
            lir::Expr::Repeat { inner, count } => {
                let (stmts_for_inner, exprs_for_inner) = self.gen_expr(&inner.into_expr(), ctx, cache)?;
                let (stmts_for_concat, exprs_for_concat) =
                    self.gen_expr_multiple_concat(&inner.into_expr(), exprs_for_inner, *count, ctx, cache)?;

                let stmts = [stmts_for_inner, stmts_for_concat].concat();

                Ok((stmts, exprs_for_concat))
            }
            lir::Expr::Input { name, .. } => Ok((
                Vec::new(),
                CompositeExpr::from_typ(
                    expr.port_decls(),
                    join_options("_", [ctx.get_prefix(), name.clone()]).unwrap(),
                )
                .map(|(ident, _)| Expression::reference(ident)),
            )),
            lir::Expr::Member { inner, index } => {
                let (stmts, exprs) = self.gen_expr(&inner.into_expr(), ctx, cache)?;

                match exprs {
                    CompositeExpr::Struct(inner) => Ok((stmts, inner[*index].clone())),
                    _ => panic!("cannot index bits"),
                }
            }
            lir::Expr::Struct { inner } => {
                let (stmts, exprs) = inner
                    .iter()
                    .map(|(_, inner)| self.gen_expr(&inner.into_expr(), ctx, cache).unwrap())
                    .fold((Vec::new(), Vec::new()), |mut acc, mut x| {
                        acc.0.append(&mut x.0);
                        acc.1.push(x.1);
                        acc
                    });

                Ok((stmts, CompositeExpr::Struct(exprs)))
            }
            lir::Expr::LeftShift { inner, rhs } => {
                self.gen_expr_binary_op(lir::BinaryOp::ShiftLeft, &inner.into_expr(), &rhs.into_expr(), ctx, cache)
            }
            lir::Expr::RightShift { inner, rhs } => {
                self.gen_expr_binary_op(lir::BinaryOp::ShiftRight, &inner.into_expr(), &rhs.into_expr(), ctx, cache)
            }
            lir::Expr::Not { inner } => self.gen_expr_unary_op(lir::UnaryOp::Negation, &inner.into_expr(), ctx, cache),
            lir::Expr::BinaryOp { op, lhs, rhs } => {
                self.gen_expr_binary_op(*op, &lhs.into_expr(), &rhs.into_expr(), ctx, cache)
            }
            lir::Expr::Map { inner, typ_elt, func } => {
                self.gen_expr_map(&inner.into_expr(), typ_elt, &func.into_expr(), ctx, cache)
            }
            lir::Expr::Get { inner, typ_elt, index } => {
                let (stmts_for_inner, exprs_for_inner) = self.gen_expr_to_idents(&inner.into_expr(), ctx, cache)?;
                let (stmts_for_index, exprs_for_index) = self.gen_expr(&index.into_expr(), ctx, cache)?;

                let exprs_for_elt =
                    self.indexing_exprs(exprs_for_inner, exprs_for_index.into_expr(), typ_elt.clone())?;
                let (stmts_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), exprs_for_elt, ctx, cache)?;

                let stmts = [stmts_for_inner, stmts_for_index, stmts_for_output].concat();

                Ok((stmts, exprs_for_output))
            }
            lir::Expr::Clip { inner, typ_elt, from, size } => {
                let (stmts_for_inner, exprs_for_inner) = self.gen_expr_to_idents(&inner.into_expr(), ctx, cache)?;
                let (stmts_for_from, exprs_for_from) = self.gen_expr(&from.into_expr(), ctx, cache)?;

                let exprs_for_elts =
                    self.range_indexing_exprs(exprs_for_inner, exprs_for_from.into_expr(), *size, typ_elt.clone())?;
                let (stmts_for_output, exprs_for_output) =
                    self.alloc_exprs(expr.clone(), exprs_for_elts, ctx, cache)?;

                let stmts = [stmts_for_inner, stmts_for_from, stmts_for_output].concat();

                Ok((stmts, exprs_for_output))
            }
            lir::Expr::Append { lhs, rhs, .. } => {
                let (stmts_for_lhs, exprs_for_lhs) = self.gen_expr(&lhs.into_expr(), ctx, cache)?;
                let (stmts_for_rhs, exprs_for_rhs) = self.gen_expr(&rhs.into_expr(), ctx, cache)?;

                let stmts = [stmts_for_lhs, stmts_for_rhs].concat();
                let exprs = exprs_for_lhs.zip(exprs_for_rhs).map(|(lhs, rhs)| Expression::cat(rhs, lhs));

                Ok((stmts, exprs))
            }
            lir::Expr::Zip { inner, .. } => {
                let (stmts_for_inner, exprs_for_inner) = inner
                    .iter()
                    .map(|expr_id| self.gen_expr(&expr_id.into_expr(), ctx, cache).expect("gen_expr: zip"))
                    .fold((Vec::new(), Vec::new()), |(mut acc_stmts, mut acc_exprs), (stmts, exprs)| {
                        acc_stmts.push(stmts);
                        acc_exprs.push(exprs);
                        (acc_stmts, acc_exprs)
                    });

                let exprs_for_zipped = CompositeExpr::Struct(exprs_for_inner);
                let (stmts_for_output, exprs_for_output) =
                    self.alloc_exprs(expr.clone(), exprs_for_zipped, ctx, cache)?;

                let stmts = [stmts_for_inner.concat(), stmts_for_output].concat();

                Ok((stmts, exprs_for_output))
            }
            lir::Expr::Concat { inner, .. } => self.gen_expr(&inner.into_expr(), ctx, cache),
            lir::Expr::Chunk { inner, .. } => self.gen_expr(&inner.into_expr(), ctx, cache),
            lir::Expr::Repr { inner } => self.gen_expr(&inner.into_expr(), ctx, cache),
            lir::Expr::Cond { cond, lhs, rhs } => {
                let (stmts_for_cond, exprs_for_cond) = self.gen_expr(&cond.into_expr(), ctx, cache)?;
                let (stmts_for_lhs, exprs_for_lhs) = self.gen_expr(&lhs.into_expr(), ctx, cache)?;
                let (stmts_for_rhs, exprs_for_rhs) = self.gen_expr(&rhs.into_expr(), ctx, cache)?;

                let exprs_for_mux = exprs_for_lhs
                    .zip(exprs_for_rhs)
                    .map(|(lhs, rhs)| Expression::mux(exprs_for_cond.clone().into_expr(), lhs, rhs));

                let (stmts_for_output, exprs_for_output) = self.alloc_exprs(expr.clone(), exprs_for_mux, ctx, cache)?;

                let stmts = [stmts_for_cond, stmts_for_lhs, stmts_for_rhs, stmts_for_output].concat();

                Ok((stmts, exprs_for_output))
            }
            _ => unimplemented!("{:?}", expr),
        }
    }

    fn gen_expr_unary_op(
        &self, op: lir::UnaryOp, inner: &lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (stmts_for_inner, exprs_for_inner) = self.gen_expr(inner, ctx, cache)?;

        let expr = Expression::do_prim(op.into(), vec![exprs_for_inner.into_expr()], Vec::new());

        let exprs = CompositeExpr::Bits(expr);

        Ok((stmts_for_inner, exprs))
    }

    fn gen_expr_binary_op(
        &self, op: lir::BinaryOp, lhs: &lir::Expr, rhs: &lir::Expr, ctx: &mut Context,
        cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (stmts_for_lhs, exprs_for_lhs) = self.gen_expr(lhs, ctx, cache)?;
        let (stmts_for_rhs, exprs_for_rhs) = self.gen_expr(rhs, ctx, cache)?;

        let expr = match op {
            lir::BinaryOp::Eq => {
                let expr =
                    Expression::binary_bitwise(PrimOp::Xor, exprs_for_lhs.into_expr(), exprs_for_rhs.into_expr());
                Expression::not(expr)
            }
            _ => Expression::do_prim(op.into(), vec![exprs_for_lhs.into_expr(), exprs_for_rhs.into_expr()], Vec::new()),
        };

        let stmts = [stmts_for_lhs, stmts_for_rhs].concat();
        let exprs = CompositeExpr::Bits(expr);

        Ok((stmts, exprs))
    }

    fn gen_expr_map(
        &self, inner: &lir::Expr, typ_elt: &lir::PortDecls, func: &lir::Expr, ctx: &mut Context,
        cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (stmts_for_inner, exprs_for_inner) = self.gen_expr_to_idents(inner, ctx, cache)?;

        let loop_count = inner.width() / typ_elt.width();

        let mut stmts_for_output = Vec::new();
        let mut exprs_for_output = Vec::new();

        for i in 0..loop_count {
            let loop_body_input_prefix = ctx.alloc_temp_id();
            let stmts_for_loop_input = {
                let exprs = CompositeExpr::from_typ(typ_elt.clone(), loop_body_input_prefix.clone());

                let exprs_for_elt = self.indexing_exprs(exprs_for_inner.clone(), i.into(), typ_elt.clone())?;

                exprs
                    .clone()
                    .zip(exprs_for_elt)
                    .iter()
                    .map(|((ident, _), expr_for_elt)| Statement::def_node(ident, expr_for_elt))
                    .collect::<Vec<_>>()
            };

            let mut ctx = Context::new();
            ctx.enter_scope(loop_body_input_prefix);

            let (stmts_for_loop_body, exprs_for_loop_body) = self.gen_expr(func, &mut ctx, &mut HashMap::new())?;

            let stmts = [stmts_for_loop_input, stmts_for_loop_body].concat();

            stmts_for_output.push(stmts);
            exprs_for_output.push(exprs_for_loop_body);
        }

        let stmts = [stmts_for_inner, stmts_for_output.concat()].concat();
        let exprs = self.concat_exprs(exprs_for_output)?;

        Ok((stmts, exprs))
    }

    fn gen_expr_multiple_concat(
        &self, expr_for_elt: &lir::Expr, exprs_for_elt: CompositeExpr<Expression>, count: usize, ctx: &mut Context,
        cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        if count == 1 {
            return Ok((Vec::new(), exprs_for_elt));
        }

        let (stmts, exprs) = self.gen_expr(
            &lir::Expr::Repeat { inner: lir::ExprId::alloc_expr(Merkle::new(expr_for_elt.clone())), count: count / 2 },
            ctx,
            cache,
        )?;

        let (stmts_for_output, exprs_for_output) = self.alloc_exprs(
            lir::Expr::Repeat { inner: lir::ExprId::alloc_expr(Merkle::new(expr_for_elt.clone())), count },
            self.concat_exprs(if count % 2 != 0 {
                vec![exprs.clone(), exprs, exprs_for_elt]
            } else {
                vec![exprs.clone(), exprs]
            })?,
            ctx,
            cache,
        )?;

        let stmts = [stmts, stmts_for_output].concat();

        Ok((stmts, exprs_for_output))
    }

    fn gen_expr_to_idents(
        &self, expr: &lir::Expr, ctx: &mut Context, cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let (mut stmts, exprs) = self.gen_expr(expr, ctx, cache)?;

        if exprs.iter().all(|expr| expr.is_reference()) {
            return Ok((stmts, exprs));
        }

        let (mut stmts_for_alloc, new_exprs) = self.alloc_exprs(expr.clone(), exprs, ctx, cache)?;

        stmts.append(&mut stmts_for_alloc);

        Ok((stmts, new_exprs))
    }

    fn indexing_exprs(
        &self, exprs: CompositeExpr<Expression>, index: Expression, typ_elt: lir::PortDecls,
    ) -> Result<CompositeExpr<Expression>, lir::ModuleError> {
        let exprs_for_elt = exprs.zip(typ_elt.into()).map(|(expr, (_, shape))| {
            let shift_amount = Expression::mul(index.clone(), shape.width().into());

            Expression::bits(Expression::dshr(expr, shift_amount), shape.width() - 1, 0)
        });

        Ok(exprs_for_elt)
    }

    fn range_indexing_exprs(
        &self, exprs: CompositeExpr<Expression>, base: Expression, offset: usize, typ_elt: lir::PortDecls,
    ) -> Result<CompositeExpr<Expression>, lir::ModuleError> {
        let exprs_for_elts = exprs.zip(typ_elt.into()).map(|(expr, (_, shape))| {
            let shift_amount = Expression::mul(base.clone(), shape.width().into());

            Expression::bits(Expression::dshr(expr, shift_amount), shape.width() * offset - 1, 0)
        });

        Ok(exprs_for_elts)
    }

    fn alloc_exprs(
        &self, expr: lir::Expr, value: CompositeExpr<Expression>, ctx: &mut Context,
        cache: &mut HashMap<lir::Expr, String>,
    ) -> Result<(Vec<Statement>, CompositeExpr<Expression>), lir::ModuleError> {
        let typ = expr.port_decls();
        let prefix = ctx.alloc_temp_id();
        let exprs = CompositeExpr::from_typ(typ, prefix.clone());

        let stmts = exprs
            .clone()
            .zip(value)
            .iter()
            .map(|((ident, _), rhs)| Statement::def_node(ident, rhs))
            .collect::<Vec<_>>();
        let exprs = exprs.map(|(ident, _)| Expression::reference(ident));

        cache.insert(expr, prefix);

        Ok((stmts, exprs))
    }

    fn concat_exprs(
        &self, exprs: Vec<CompositeExpr<Expression>>,
    ) -> Result<CompositeExpr<Expression>, lir::ModuleError> {
        Ok(exprs
            .into_iter()
            .reduce(|acc, expr| expr.zip(acc).map(|(expr, acc)| Expression::cat(expr, acc)))
            .expect("concat_exprs: no element"))
    }
}

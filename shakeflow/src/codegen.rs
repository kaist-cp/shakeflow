//! Generates target code from ShakeFlow module.

use std::collections::VecDeque;
use std::ops::*;

use itertools::*;

use crate::*;

/// IR-level module.
#[derive(Debug)]
pub struct Module<C: Codegen> {
    /// Name of the module
    pub name: String,

    /// Ports of the module
    pub ports: C::Ports,

    /// Body of the module
    pub body: C::Body,
}

impl<C: Codegen> Module<C> {
    /// Creates new module.
    fn new(name: String, ports: C::Ports, body: C::Body) -> Self { Module { name, ports, body } }
}

/// Generates target code.
pub trait Codegen: Default {
    /// Ports of module.
    type Ports;

    /// Body of module.
    type Body;

    /// Generates target code for port declarations.
    fn gen_port_decls(&self, module: &lir::Module) -> Result<Self::Ports, lir::ModuleError>;

    /// Generates target code for composite module.
    fn gen_module_composite(
        &self, module: &lir::CompositeModule, ctx: &mut Context,
    ) -> Result<Self::Body, lir::ModuleError>;

    /// Generates target code for FSM.
    fn gen_module_fsm(&self, module: &lir::Fsm, ctx: &mut Context) -> Result<Self::Body, lir::ModuleError>;

    /// Generates target code for Module instantiation.
    fn gen_module_inst(&self, module: &lir::ModuleInst, ctx: &mut Context) -> Result<Self::Body, lir::ModuleError>;

    /// Generated connections for virtual module
    fn gen_module_virtual(
        &self, module: &lir::VirtualModule, composite_context_prefix: Option<String>, ctx: &mut Context,
    ) -> Result<Self::Body, lir::ModuleError>;
}

/// Generates target code for module with given compiler.
pub fn gen_module<C: Codegen>(name: String, module: &lir::Module) -> Result<Module<C>, lir::ModuleError> {
    let compiler = C::default();

    match &*module.inner {
        lir::ModuleInner::Composite(_, composite_module) => {
            let module = Module::new(
                name,
                compiler.gen_port_decls(module)?,
                compiler.gen_module_composite(composite_module, &mut Context::new())?,
            );

            Ok(module)
        }
        _ => unimplemented!("not supported module type"),
    }
}

/// Composite of expressions.
#[derive(Debug, Clone)]
pub enum CompositeExpr<V: Clone> {
    /// Struct of expressions.
    Struct(Vec<CompositeExpr<V>>),

    /// Expression.
    Bits(V),
}

impl<V: Clone + std::fmt::Debug> CompositeExpr<V> {
    /// Converts into expression.
    pub fn into_expr(self) -> V {
        match self {
            Self::Struct(_) => panic!("Cannot convert struct of expressions into expression."),
            Self::Bits(expr) => expr,
        }
    }

    /// Iterator for `CompositeExpr`.
    pub fn iter(&self) -> CompositeExprIterator<V> { self.into_iter() }

    /// Converts primitive expressions in the tree.
    pub fn map<W: Clone, F: FnMut(V) -> W>(self, mut f: F) -> CompositeExpr<W> {
        CompositeExprMap { inner: self, f: &mut f }.collect()
    }

    /// Zips with other composite expr. Structures of the two compositions should be same.
    pub fn zip<W: Clone + std::fmt::Debug>(self, other: CompositeExpr<W>) -> CompositeExpr<(V, W)> {
        match (self, other) {
            (CompositeExpr::Struct(exprs_self), CompositeExpr::Struct(exprs_other)) => CompositeExpr::Struct(
                izip!(exprs_self.into_iter(), exprs_other.into_iter())
                    .map(|(expr_lhs, expr_rhs)| expr_lhs.zip(expr_rhs))
                    .collect(),
            ),
            (CompositeExpr::Bits(expr_self), CompositeExpr::Bits(expr_other)) => {
                CompositeExpr::Bits((expr_self, expr_other))
            }
            (CompositeExpr::Struct(exprs_self), CompositeExpr::Bits(expr_other)) => panic!("zip: two compositions CompositeExpr::Struct(\n{:#?})\nand CompositeExpr::Bits(\n{:#?})\nhave different structure", exprs_self, expr_other),
            (CompositeExpr::Bits(exprs_self), CompositeExpr::Struct(expr_other)) => panic!("zip: two compositions CompositeExpr::Bits(\n{:#?})\nand CompositeExpr::Struct(\n{:#?})\nhave different structure", exprs_self, expr_other),
        }
    }
}

#[derive(Debug)]
struct CompositeExprMap<'a, V: Clone, F> {
    inner: CompositeExpr<V>,
    f: &'a mut F,
}

impl<'a, V: Clone, W: Clone, F> CompositeExprMap<'a, V, F>
where F: FnMut(V) -> W
{
    fn collect(self) -> CompositeExpr<W> {
        match self.inner {
            CompositeExpr::Struct(inner) => CompositeExpr::Struct(
                inner.into_iter().map(|expr| CompositeExprMap { inner: expr, f: self.f }.collect()).collect(),
            ),
            CompositeExpr::Bits(expr) => CompositeExpr::Bits((self.f)(expr)),
        }
    }
}

/// Iterator for `CompositeExpr`.
#[derive(Debug)]
pub struct CompositeExprIterator<V> {
    inner: VecDeque<V>,
}

impl<V: Clone> Iterator for CompositeExprIterator<V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> { self.inner.pop_front() }
}

impl<V: Clone> IntoIterator for &CompositeExpr<V> {
    type IntoIter = CompositeExprIterator<V>;
    type Item = V;

    fn into_iter(self) -> Self::IntoIter {
        let mut iter_vec = vec![];

        match self {
            CompositeExpr::Struct(inner) => {
                for expr in inner {
                    iter_vec.extend(expr.into_iter().inner)
                }
            }
            CompositeExpr::Bits(expr) => iter_vec.push(expr.clone()),
        }

        Self::IntoIter { inner: iter_vec.into() }
    }
}

impl CompositeExpr<LogicValues> {
    /// Repeats each field in the expressions by n times.
    pub fn repeat(&self, n: usize) -> Self {
        match self {
            CompositeExpr::Struct(inner) => CompositeExpr::Struct(inner.iter().map(|expr| expr.repeat(n)).collect()),
            CompositeExpr::Bits(expr) => CompositeExpr::Bits(LogicValues(expr.0.repeat(n))),
        }
    }
}

impl From<lir::PortDecls> for CompositeExpr<(Option<String>, lir::Shape)> {
    fn from(typ: lir::PortDecls) -> Self {
        match typ {
            lir::PortDecls::Struct(inner) => CompositeExpr::Struct(
                inner
                    .into_iter()
                    .map(|(prefix, typ)| {
                        CompositeExpr::from(typ).map(|(name, shape)| (join_options("_", [prefix.clone(), name]), shape))
                    })
                    .collect(),
            ),
            lir::PortDecls::Bits(shape) => CompositeExpr::Bits((None, shape)),
        }
    }
}

impl CompositeExpr<(String, lir::Shape)> {
    /// Constructs from value type.
    pub fn from_typ(typ: lir::PortDecls, prefix: String) -> Self {
        CompositeExpr::from(typ).map(|(name, shape)| (join_options("_", [Some(prefix.clone()), name]).unwrap(), shape))
    }
}

/// Context.
#[derive(Debug, Default, Clone)]
pub struct Context {
    /// Scopes in the context
    scopes: Vec<Scope>,

    /// Genvar index
    genvar_id: usize,
}

impl Context {
    /// Creates new context.
    pub fn new() -> Self { Self::default() }

    /// Enters scope with given scope name.
    pub fn enter_scope(&mut self, scope_name: String) { self.scopes.push(Scope::new(scope_name)); }

    /// Leaves scope.
    pub fn leave_scope(&mut self) { self.scopes.pop(); }

    /// Returns prefix of the inner scope.
    pub fn get_prefix(&self) -> Option<String> {
        if self.scopes.is_empty() {
            None
        } else {
            Some(self.scopes.iter().map(|scope| scope.prefix.clone()).collect::<Vec<_>>().join("_"))
        }
    }

    /// Allocates integer.
    pub fn alloc_int_id(&mut self) -> String {
        let count = self.scopes.len();
        assert!(count > 0, "There is no scope in context");
        let int_id = self.scopes[count - 1].int_id;
        self.scopes[count - 1].int_id += 1;
        join_options("_", [self.get_prefix(), Some(format!("i{}", int_id))]).unwrap()
    }

    /// Allocates genvar.
    pub fn alloc_genvar_id(&mut self) -> String {
        let genvar_id = self.genvar_id;
        self.genvar_id += 1;
        format!("g{}", genvar_id)
    }

    /// Allocates net or reg.
    pub fn alloc_temp_id(&mut self) -> String {
        let count = self.scopes.len();
        assert!(count > 0, "There is no scope in context");
        let temp_id = self.scopes[count - 1].temp_id;
        self.scopes[count - 1].temp_id += 1;
        join_options("_", [self.get_prefix(), Some(format!("t{}", temp_id))]).unwrap()
    }
}

/// Scope.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Prefix of the scope
    prefix: String,

    /// Integer index
    int_id: usize,

    /// Net, Reg index
    temp_id: usize,
}

impl Scope {
    /// Creates new scope.
    pub fn new(prefix: String) -> Self { Self { prefix, int_id: 0, temp_id: 0 } }
}

/// Represents port in target language.
#[derive(Debug, Clone)]
struct Port {
    /// Channel type.
    channel_typ: lir::ChannelTyp,

    /// Array size.
    size: usize,
}

impl Port {
    fn new(channel_typ: lir::ChannelTyp, size: usize) -> Self { Port { channel_typ, size } }

    fn multiple(self, count: usize) -> Self { Port { channel_typ: self.channel_typ, size: self.size * count } }
}

/// Direction of port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    /// Input
    Input,

    /// Output
    Output,
}

impl ToString for Direction {
    fn to_string(&self) -> String {
        match self {
            Direction::Input => "input".to_string(),
            Direction::Output => "output".to_string(),
        }
    }
}

/// Accessor to the element in the interface.
#[derive(Default, Debug, Clone)]
struct Accessor {
    /// Prefix.
    prefix: Option<String>,

    /// Separator.
    sep: Option<String>,

    /// Index and total number of elements.
    index: Option<(usize, usize)>,
}

/// Logic value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicValue {
    /// Logic '0' or false condition
    False,
    /// Logic '1' or true condition
    True,
    /// Don't care or unknown value
    X,
    /// High impedance state (used for tri-state buffer)
    Z,
}

impl ToString for LogicValue {
    fn to_string(&self) -> String {
        match self {
            LogicValue::False => "0",
            LogicValue::True => "1",
            LogicValue::X => "x",
            LogicValue::Z => "z",
        }
        .to_string()
    }
}

/// Logic values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicValues(Vec<LogicValue>);

impl ToString for LogicValues {
    fn to_string(&self) -> String { self.0.iter().map(|b| b.to_string()).collect::<String>() }
}

impl Deref for LogicValues {
    type Target = [LogicValue];

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl LogicValues {
    /// Creates new logic values.
    pub fn new(inner: Vec<LogicValue>) -> Self { Self(inner) }

    /// Inner logic values.
    pub fn into_inner(self) -> Vec<LogicValue> { self.0 }
}

/// Generates bitarrays representing Expr. Panics if it cannot be converted into bitarrays.
///
/// Returned string contains "0", "1" and "x".
pub(super) fn gen_expr_literal(expr: &lir::Expr) -> CompositeExpr<LogicValues> {
    match expr {
        lir::Expr::X { typ } => match typ {
            lir::PortDecls::Bits(shape) => CompositeExpr::Bits(LogicValues(vec![LogicValue::X; shape.width()])),
            lir::PortDecls::Struct(inner) => CompositeExpr::Struct(
                inner.iter().map(|(_, typ)| gen_expr_literal(&lir::Expr::X { typ: typ.clone() })).collect(),
            ),
        },
        lir::Expr::Constant { bits, typ } => match typ {
            lir::PortDecls::Bits(_) => CompositeExpr::Bits(LogicValues(
                bits.iter().rev().map(|x| if *x { LogicValue::True } else { LogicValue::False }).collect(),
            )),
            lir::PortDecls::Struct(inner) => {
                let mut member_exprs = Vec::new();
                let mut offset = 0;

                for (_, typ) in inner {
                    let width = typ.width();
                    member_exprs.push(gen_expr_literal(&lir::Expr::Constant {
                        bits: bits[offset..(offset + width)].to_vec(),
                        typ: typ.clone(),
                    }));
                    offset += width;
                }

                CompositeExpr::Struct(member_exprs)
            }
        },
        lir::Expr::Struct { inner } => {
            CompositeExpr::Struct(inner.iter().map(|(_, s)| gen_expr_literal(&s.into_expr())).collect())
        }
        lir::Expr::Repeat { inner, count } => gen_expr_literal(&inner.into_expr()).repeat(*count),
        lir::Expr::Member { inner, index } => {
            let inner = gen_expr_literal(&inner.into_expr());
            match inner {
                CompositeExpr::Struct(inner) => inner[*index].clone(),
                _ => todo!(),
            }
        }
        _ => todo!("not yet implemented: {:?}", expr),
    }
}

/// Checks `values` and `exprs` have same structure and returns its matched elements.
///
/// # Returns
///
/// - `lir::Shape`: Shape of the element
/// - `String`: Name of the element
/// - `vir::Expression`: Expression of the element
pub(super) fn match_value_typ_exprs<V: Clone>(
    prefix: Option<String>, values: lir::PortDecls, exprs: CompositeExpr<V>,
) -> Vec<(lir::Shape, String, V)> {
    match (values, exprs) {
        (lir::PortDecls::Bits(shape), CompositeExpr::Bits(expr)) => {
            if shape.dim() > 1 || shape.width() == 0 {
                vec![]
            } else {
                vec![(shape, prefix.unwrap(), expr)]
            }
        }
        (lir::PortDecls::Struct(values), CompositeExpr::Struct(exprs)) => {
            assert_eq!(values.len(), exprs.len());
            izip!(values, exprs)
                .map(|((name, values), exprs)| {
                    match_value_typ_exprs(join_options("_", [prefix.clone(), name]), values, exprs)
                })
                .collect::<Vec<_>>()
                .concat()
        }
        _ => panic!("internal compiler error"),
    }
}

/// Returns FSM state initialization info.
///
/// # Returns
///
/// - `lir::Shape`: Shape of the reg
/// - `String`: Name of the reg
/// - `LogicValues`: Initial value of the reg
pub(super) fn gen_module_fsm_state_init(
    state: &lir::Expr, init: &lir::Expr, ctx: &Context,
) -> Result<Vec<(lir::Shape, String, LogicValues)>, lir::ModuleError> {
    let state_init_value = gen_expr_literal(init);

    Ok(izip!(state.port_decls().iter(), state_init_value.iter())
        .map(|((name, shape), init_value)| {
            let net_name = join_options("_", [Some(format!("{}_st", ctx.get_prefix().unwrap())), name]).unwrap();
            (shape, net_name, init_value)
        })
        .collect())
}

/// Returns a set of ports to represent given interface type.
fn gen_ports(interface_typ: &lir::InterfaceTyp) -> Vec<(Port, Accessor)> {
    match interface_typ {
        lir::InterfaceTyp::Unit => Vec::new(),
        lir::InterfaceTyp::Channel(channel_typ) => {
            vec![(Port::new(channel_typ.clone(), 1), Accessor::default())]
        }
        lir::InterfaceTyp::Array(interface_typ, count) => {
            gen_ports(interface_typ).into_iter().map(|(port, accessor)| (port.multiple(*count), accessor)).collect()
        }
        lir::InterfaceTyp::ExpansiveArray(interface_typ, count) => (0..*count)
            .flat_map(|i| {
                gen_ports(interface_typ).into_iter().map(move |(port, mut accessor)| {
                    match accessor.prefix {
                        Some(prefix) => {
                            accessor.prefix = join_options("_", [Some(i.to_string()), Some(prefix)]);
                        }
                        None => {
                            accessor.prefix = Some(i.to_string());
                            accessor.sep = None;
                        }
                    }
                    (port, accessor)
                })
            })
            .collect(),
        lir::InterfaceTyp::Struct(inner) => inner
            .into_iter()
            .flat_map(|(name, (sep, interface_typ))| {
                gen_ports(interface_typ).into_iter().map(|(port, mut accessor)| {
                    match accessor.prefix {
                        Some(prefix) => {
                            let sep = sep.clone().unwrap_or_else(|| "_".to_string());
                            accessor.prefix = join_options(&sep, [Some(name.clone()), Some(prefix)]);
                        }
                        None => {
                            accessor.prefix = Some(name.clone());
                            accessor.sep = sep.clone();
                        }
                    }
                    (port, accessor)
                })
            })
            .collect(),
    }
}

/// wires virtual modules with real registered module
///
/// # Returns
///
/// - `String`: Name of lvalue
/// - `Option<(usize, usize)>`: Index/element size of lvalue
/// - `String`: Name of rvalue
/// - `Option<(usize, usize)>`: Index/element size of rvalue
#[allow(clippy::type_complexity)]
pub(super) fn gen_virtual_wirings(
    virtual_module: &lir::VirtualModule, composite_context_prefix: Option<String>,
    submodule_context_prefix: Option<String>,
) -> Result<Vec<(String, Option<(usize, usize)>, String, Option<(usize, usize)>)>, lir::ModuleError> {
    let mut conts = Vec::new();

    for (ityp, path) in virtual_module.input_interface_typ().into_primitives() {
        let channel_typ = some_or!(ityp.clone().get_channel_typ(), continue);

        let mut ingress_accessor = gen_channel_accessor(&virtual_module.input_interface_typ(), path.clone());
        ingress_accessor.prefix = join_options("_", [Some("in".to_string()), ingress_accessor.prefix]);

        let mut registered_ingress_accessor = gen_channel_accessor(
            &virtual_module.input_interface_typ,
            virtual_module.input_endpoint().inner.into_iter().chain(path.inner.into_iter()).collect(),
        );
        registered_ingress_accessor.prefix =
            join_options("_", [Some(virtual_module.input_prefix.clone()), registered_ingress_accessor.prefix]);

        let lvalue_prefix = join_options("_", [
            composite_context_prefix.clone(),
            Some(format!("registered_{}_{}", virtual_module.get_module_name(), virtual_module.registered_index)),
            registered_ingress_accessor.prefix.clone(),
        ]);
        let rvalue_prefix = join_options("_", [submodule_context_prefix.clone(), ingress_accessor.prefix.clone()]);

        for (name, shape) in channel_typ.fwd.iter() {
            assert_eq!(shape.dim(), 1);
            let from_sep = ingress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_range = ingress_accessor.index.map(|(index, _)| (index, shape.width()));
            let to_sep = registered_ingress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_range = registered_ingress_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&to_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                to_range,
                join_options(&from_sep, [rvalue_prefix.clone(), name]).unwrap(),
                from_range,
            ));
        }

        for (name, shape) in channel_typ.bwd.iter() {
            assert_eq!(shape.dim(), 1);
            let from_sep = registered_ingress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_range = registered_ingress_accessor.index.map(|(index, _)| (index, shape.width()));
            let to_sep = ingress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_range = ingress_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&from_sep, [rvalue_prefix.clone(), name.clone()]).unwrap(),
                from_range,
                join_options(&to_sep, [lvalue_prefix.clone(), name]).unwrap(),
                to_range,
            ));
        }
    }

    for (ityp, path) in virtual_module.output_interface_typ().into_primitives() {
        let channel_typ = some_or!(ityp.clone().get_channel_typ(), continue);

        let mut egress_accessor = gen_channel_accessor(&virtual_module.output_interface_typ(), path.clone());
        egress_accessor.prefix = join_options("_", [Some("out".to_string()), egress_accessor.prefix]);

        let mut registered_egress_accessor = gen_channel_accessor(
            &virtual_module.output_interface_typ,
            virtual_module.output_endpoint().inner.into_iter().chain(path.inner.into_iter()).collect(),
        );
        registered_egress_accessor.prefix =
            join_options("_", [Some(virtual_module.output_prefix.clone()), registered_egress_accessor.prefix]);

        let lvalue_prefix = join_options("_", [submodule_context_prefix.clone(), egress_accessor.prefix.clone()]);
        let rvalue_prefix = join_options("_", [
            composite_context_prefix.clone(),
            Some(format!("registered_{}_{}", virtual_module.get_module_name(), virtual_module.registered_index)),
            registered_egress_accessor.prefix.clone(),
        ]);

        for (name, shape) in channel_typ.fwd.iter() {
            assert_eq!(shape.dim(), 1);
            let from_sep = registered_egress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_range = registered_egress_accessor.index.map(|(index, _)| (index, shape.width()));
            let to_sep = egress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_range = egress_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&to_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                to_range,
                join_options(&from_sep, [rvalue_prefix.clone(), name]).unwrap(),
                from_range,
            ));
        }

        for (name, shape) in channel_typ.bwd.iter() {
            assert_eq!(shape.dim(), 1);
            let from_sep = egress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_range = egress_accessor.index.map(|(index, _)| (index, shape.width()));
            let to_sep = registered_egress_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_range = registered_egress_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&from_sep, [rvalue_prefix.clone(), name.clone()]).unwrap(),
                from_range,
                join_options(&to_sep, [lvalue_prefix.clone(), name]).unwrap(),
                to_range,
            ));
        }
    }

    Ok(conts)
}

/// Returns connections in the module instantiation.
///
/// # Returns
///
/// - `Direction`: Direction of the port
/// - `String`: Name of the port
/// - `String`: Name of the expression
pub(super) fn gen_connections(
    module: &lir::ModuleInst, ctx: &mut Context,
) -> Result<Vec<(Direction, String, String)>, lir::ModuleError> {
    let mut connections = Vec::new();

    if module.has_clkrst {
        connections.push((Direction::Input, "clk".to_string(), "clk".to_string()));
        connections.push((Direction::Input, "rst".to_string(), "rst".to_string()));
    }

    for (port, accessor) in gen_ports(&module.input_interface_typ()) {
        let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
        let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
        let lvalue_prefix = join_options("_", [module.input_prefix.clone(), path_prefix.clone()]);
        let rvalue_prefix = join_options("_", [ctx.get_prefix(), Some("in".to_string()), path_prefix]);

        for (name, _) in port.channel_typ.fwd.iter() {
            connections.push((
                Direction::Input,
                join_options(&path_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                join_options(&path_sep, [rvalue_prefix.clone(), name]).unwrap(),
            ));
        }

        for (name, _) in port.channel_typ.bwd.iter() {
            connections.push((
                Direction::Output,
                join_options(&path_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                join_options(&path_sep, [rvalue_prefix.clone(), name.clone()]).unwrap(),
            ));
        }
    }

    for (port, accessor) in gen_ports(&module.output_interface_typ()) {
        let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
        let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
        let lvalue_prefix = join_options("_", [module.output_prefix.clone(), path_prefix.clone()]);
        let rvalue_prefix = join_options("_", [ctx.get_prefix(), Some("out".to_string()), path_prefix]);

        for (name, _) in port.channel_typ.fwd.iter() {
            connections.push((
                Direction::Output,
                join_options(&path_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                join_options(&path_sep, [rvalue_prefix.clone(), name]).unwrap(),
            ));
        }

        for (name, _) in port.channel_typ.bwd.iter() {
            connections.push((
                Direction::Input,
                join_options(&path_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                join_options(&path_sep, [rvalue_prefix.clone(), name]).unwrap(),
            ));
        }
    }

    Ok(connections)
}

/// Returns port declarations in the module.
///
/// # Returns
///
/// - `Direction`: Direction of the port (input or output)
/// - `usize`: Bitwidth of the port
/// - `String`: Name of the port
pub(super) fn gen_port_decls(module: &lir::Module) -> Result<Vec<(Direction, usize, String)>, lir::ModuleError> {
    let mut port_decls = vec![(Direction::Input, 1, "clk".to_string()), (Direction::Input, 1, "rst".to_string())];

    // Port declarations for input interface
    for (port, accessor) in gen_ports(&module.inner.input_interface_typ()) {
        let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
        let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
        let input_prefix = join_options("_", [module.inner.input_prefix(), path_prefix]);

        for (name, shape) in port.channel_typ.fwd.iter() {
            assert_eq!(shape.dim(), 1, "Port of module should be 1-dimensional.");
            port_decls.push((
                Direction::Input,
                shape.width() * port.size,
                join_options(&path_sep, [input_prefix.clone(), name]).unwrap(),
            ));
        }

        for (name, shape) in port.channel_typ.bwd.iter() {
            assert_eq!(shape.dim(), 1, "Port of module should be 1-dimensional.");
            port_decls.push((
                Direction::Output,
                shape.width() * port.size,
                join_options(&path_sep, [input_prefix.clone(), name]).unwrap(),
            ));
        }
    }

    // Port declarations for output interface
    for (port, accessor) in gen_ports(&module.inner.output_interface_typ()) {
        let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
        let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
        let output_prefix = join_options("_", [module.inner.output_prefix(), path_prefix]);

        for (name, shape) in port.channel_typ.fwd.iter() {
            assert_eq!(shape.dim(), 1, "Port of module should be 1-dimensional.");
            port_decls.push((
                Direction::Output,
                shape.width() * port.size,
                join_options(&path_sep, [output_prefix.clone(), name]).unwrap(),
            ));
        }

        for (name, shape) in port.channel_typ.bwd.iter() {
            assert_eq!(shape.dim(), 1, "Port of module should be 1-dimensional.");
            port_decls.push((
                Direction::Input,
                shape.width() * port.size,
                join_options(&path_sep, [output_prefix.clone(), name]).unwrap(),
            ));
        }
    }

    Ok(port_decls)
}

/// Returns input/output wires for submodules in the module.
///
/// # Returns
///
/// - `String`: Name of the wire
/// - `lir::Shape`: Shape of the wire
pub(super) fn gen_submodule_wires(
    module: &lir::CompositeModule, ctx: &mut Context,
) -> Result<Vec<(String, lir::Shape)>, lir::ModuleError> {
    // Add input/output wires for submodules
    let mut submodule_wires = vec![];

    ctx.enter_scope("registered".to_string());
    for (index, registered_module) in module.registered_modules.iter().enumerate() {
        let comp_name = registered_module.get_module_name();
        for (port, accessor) in gen_ports(&registered_module.inner.input_interface_typ()) {
            let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
            let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
            let input_prefix = join_options("_", [
                ctx.get_prefix(),
                Some(format!(
                    "{}_{}_{}",
                    comp_name,
                    index,
                    registered_module.inner.input_prefix().unwrap_or_else(|| "in".to_string())
                )),
                path_prefix,
            ]);

            for (name, shape) in
                ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
            {
                submodule_wires
                    .push((join_options(&path_sep, [input_prefix.clone(), name]).unwrap(), shape.multiple(port.size)));
            }
        }

        for (port, accessor) in gen_ports(&registered_module.inner.output_interface_typ()) {
            let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
            let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
            let output_prefix = join_options("_", [
                ctx.get_prefix(),
                Some(format!(
                    "{}_{}_{}",
                    comp_name,
                    index,
                    registered_module.inner.output_prefix().unwrap_or_else(|| "out".to_string())
                )),
                path_prefix,
            ]);

            for (name, shape) in
                ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
            {
                submodule_wires
                    .push((join_options(&path_sep, [output_prefix.clone(), name]).unwrap(), shape.multiple(port.size)));
            }
        }
    }
    ctx.leave_scope();

    for (index, (submodule, _)) in module.submodules.iter().enumerate() {
        let comp_name = submodule.get_module_name();
        match &*submodule.inner {
            lir::ModuleInner::Composite(_, module) => {
                // Add input wires
                for (port, accessor) in gen_ports(&module.input_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let input_prefix = join_options("_", [
                        ctx.get_prefix(),
                        Some(format!("{}_{}", comp_name, index)),
                        join_options("_", [module.input_prefix.clone(), path_prefix]),
                    ]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [input_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }

                // Add output wires
                for (port, accessor) in gen_ports(&module.output_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let output_prefix = join_options("_", [
                        ctx.get_prefix(),
                        Some(format!("{}_{}", comp_name, index)),
                        join_options("_", [module.output_prefix.clone(), path_prefix]),
                    ]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [output_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }
            }
            lir::ModuleInner::Fsm(module) => {
                // Add input wires
                for (port, accessor) in gen_ports(&module.input_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let input_prefix =
                        join_options("_", [ctx.get_prefix(), Some(format!("{}_{}_in", comp_name, index)), path_prefix]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [input_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }

                // Add output wires
                for (port, accessor) in gen_ports(&module.output_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let output_prefix = join_options("_", [
                        ctx.get_prefix(),
                        Some(format!("{}_{}_out", comp_name, index)),
                        path_prefix,
                    ]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [output_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }
            }
            lir::ModuleInner::ModuleInst(module) => {
                // Add input wires
                for (port, accessor) in gen_ports(&module.input_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let input_prefix =
                        join_options("_", [ctx.get_prefix(), Some(format!("{}_{}_in", comp_name, index)), path_prefix]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [input_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }

                // Add output wires
                for (port, accessor) in gen_ports(&module.output_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let output_prefix = join_options("_", [
                        ctx.get_prefix(),
                        Some(format!("{}_{}_out", comp_name, index)),
                        path_prefix,
                    ]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [output_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }
            }
            lir::ModuleInner::VirtualModule(module) => {
                // Add input wires
                for (port, accessor) in gen_ports(&module.input_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let input_prefix =
                        join_options("_", [ctx.get_prefix(), Some(format!("{}_{}_in", comp_name, index)), path_prefix]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [input_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }

                // Add output wires
                for (port, accessor) in gen_ports(&module.output_interface_typ()) {
                    let (path_prefix, path_sep) = (accessor.prefix, accessor.sep);
                    let path_sep = path_sep.unwrap_or_else(|| "_".to_string());
                    let output_prefix = join_options("_", [
                        ctx.get_prefix(),
                        Some(format!("{}_{}_out", comp_name, index)),
                        path_prefix,
                    ]);

                    for (name, shape) in
                        ::std::iter::empty().chain(port.channel_typ.fwd.iter()).chain(port.channel_typ.bwd.iter())
                    {
                        submodule_wires.push((
                            join_options(&path_sep, [output_prefix.clone(), name]).unwrap(),
                            shape.multiple(port.size),
                        ));
                    }
                }
            }
        }
    }

    Ok(submodule_wires)
}

/// Returns accessor to channel in interface.
fn gen_channel_accessor(interface_typ: &lir::InterfaceTyp, mut path: lir::EndpointPath) -> Accessor {
    if path.is_empty() {
        assert!(matches!(interface_typ, lir::InterfaceTyp::Channel(_)));
        return Accessor::default();
    }

    let front = path.pop_front().unwrap();
    match (&front, interface_typ) {
        (lir::EndpointNode::Index(i), lir::InterfaceTyp::Array(interface_typ_elt, count)) => {
            let mut accessor = gen_channel_accessor(interface_typ_elt, path);
            accessor.index = match accessor.index {
                Some((index, total)) => Some((total * i + index, total * count)),
                None => Some((*i, *count)),
            };
            accessor
        }
        (lir::EndpointNode::ExpansiveIndex(i), lir::InterfaceTyp::ExpansiveArray(interface_typ_elt, _)) => {
            let mut accessor = gen_channel_accessor(interface_typ_elt, path);
            accessor.prefix = join_options("_", [Some(i.to_string()), accessor.prefix]);
            accessor
        }
        (lir::EndpointNode::Field(name, _), lir::InterfaceTyp::Struct(inner)) => {
            let (sep, interface_typ_field) = inner.get(name).unwrap();
            let mut accessor = gen_channel_accessor(interface_typ_field, path);
            match accessor.prefix {
                Some(prefix) => {
                    accessor.prefix = join_options(&sep.clone().unwrap_or_else(|| "_".to_string()), [
                        Some(name.clone()),
                        Some(prefix),
                    ]);
                }
                None => {
                    accessor.prefix = Some(name.clone());
                    accessor.sep = sep.clone();
                }
            }
            accessor
        }
        _ => panic!("unmatched endpoint node and interface type: {:#?} and {:#?}", front, interface_typ),
    }
}

/// Returns wirings in the module.
///
/// # Returns
///
/// - `String`: Name of lvalue
/// - `Option<(usize, usize)>`: Index/element size of lvalue
/// - `String`: Name of rvalue
/// - `Option<(usize, usize)>`: Index/element size of rvalue
#[allow(clippy::type_complexity)]
pub(super) fn gen_wiring(
    module: &lir::CompositeModule, prefix: Option<String>,
) -> Result<Vec<(String, Option<(usize, usize)>, String, Option<(usize, usize)>)>, lir::ModuleError> {
    let mut conts = Vec::new();

    // Connections from input interface of the module and output interfaces of submodules in the module.
    let mut input_connections = Vec::new();
    let mut comp_connections = vec![Vec::new(); module.submodules.len()];

    for (submodule_index, (submodule, from)) in module.submodules.iter().enumerate() {
        for (interface, path) in from.clone().into_primitives() {
            let channel = some_or!(interface.get_channel(), continue);

            let mut comp_accessor = gen_channel_accessor(&from.typ(), path);
            comp_accessor.prefix = join_options("_", [
                Some(format!("{}_{}", submodule.get_module_name(), submodule_index)),
                match &*submodule.inner {
                    lir::ModuleInner::Composite(..) => submodule.inner.input_prefix(),
                    lir::ModuleInner::Fsm(_) | lir::ModuleInner::ModuleInst(_) | lir::ModuleInner::VirtualModule(_) => {
                        Some("in".to_string())
                    }
                },
                comp_accessor.prefix,
            ]);

            match channel.endpoint() {
                lir::Endpoint::Input { path } => {
                    let mut from_accessor = gen_channel_accessor(&module.input_interface_typ(), path);
                    from_accessor.prefix = join_options("_", [module.input_prefix.clone(), from_accessor.prefix]);

                    input_connections.push((from_accessor, comp_accessor, channel.typ()));
                }
                lir::Endpoint::Submodule { submodule_index, path } => {
                    let mut from_accessor =
                        gen_channel_accessor(&module.submodules[submodule_index].0.inner.output_interface_typ(), path);
                    from_accessor.prefix = join_options("_", [
                        Some(format!("{}_{}", module.submodules[submodule_index].0.get_module_name(), submodule_index)),
                        match &*module.submodules[submodule_index].0.inner {
                            lir::ModuleInner::Composite(..) => {
                                module.submodules[submodule_index].0.inner.output_prefix()
                            }
                            lir::ModuleInner::Fsm(_)
                            | lir::ModuleInner::ModuleInst(_)
                            | lir::ModuleInner::VirtualModule(_) => Some("out".to_string()),
                        },
                        from_accessor.prefix,
                    ]);

                    comp_connections[submodule_index].push((from_accessor, comp_accessor, channel.typ()));
                }
                _ => panic!("internal compiler error"),
            }
        }
    }

    for (interface, path) in module.output_interface.clone().into_primitives() {
        let channel = some_or!(interface.get_channel(), continue);

        let mut output_accessor = gen_channel_accessor(&module.output_interface_typ(), path);
        output_accessor.prefix = join_options("_", [module.output_prefix.clone(), output_accessor.prefix]);

        match channel.endpoint() {
            lir::Endpoint::Input { path } => {
                let mut from_accessor = gen_channel_accessor(&module.input_interface_typ(), path);
                from_accessor.prefix = join_options("_", [module.input_prefix.clone(), from_accessor.prefix]);

                input_connections.push((from_accessor, output_accessor, channel.typ()));
            }
            lir::Endpoint::Submodule { submodule_index, path } => {
                let mut from_accessor =
                    gen_channel_accessor(&module.submodules[submodule_index].0.inner.output_interface_typ(), path);
                from_accessor.prefix = join_options("_", [
                    Some(format!("{}_{}", module.submodules[submodule_index].0.get_module_name(), submodule_index,)),
                    match &*module.submodules[submodule_index].0.inner {
                        lir::ModuleInner::Composite(..) => module.submodules[submodule_index].0.inner.output_prefix(),
                        lir::ModuleInner::Fsm(_)
                        | lir::ModuleInner::ModuleInst(_)
                        | lir::ModuleInner::VirtualModule(_) => Some("out".to_string()),
                    },
                    from_accessor.prefix,
                ]);

                comp_connections[submodule_index].push((from_accessor, output_accessor, channel.typ()));
            }
            _ => panic!("internal compiler error"),
        }
    }

    for (from_accessor, to_accessor, channel_typ) in
        ::std::iter::empty().chain(input_connections.iter()).chain(comp_connections.concat().iter())
    {
        let lvalue_prefix = join_options("_", [prefix.clone(), to_accessor.prefix.clone()]);
        let rvalue_prefix = join_options("_", [prefix.clone(), from_accessor.prefix.clone()]);

        for (name, shape) in channel_typ.fwd.iter() {
            assert_eq!(shape.dim(), 1);
            let to_sep = to_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_range = to_accessor.index.map(|(index, _)| (index, shape.width()));
            let from_sep = from_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_range = from_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&to_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                to_range,
                join_options(&from_sep, [rvalue_prefix.clone(), name]).unwrap(),
                from_range,
            ));
        }

        for (name, shape) in channel_typ.bwd.iter() {
            assert_eq!(shape.dim(), 1);
            let from_sep = from_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_range = from_accessor.index.map(|(index, _)| (index, shape.width()));
            let to_sep = to_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_range = to_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&from_sep, [rvalue_prefix.clone(), name.clone()]).unwrap(),
                from_range,
                join_options(&to_sep, [lvalue_prefix.clone(), name]).unwrap(),
                to_range,
            ));
        }
    }

    Ok(conts)
}

/// Returns wirings in the array module.
///
/// # Returns
///
/// - `String`: Name of lvalue
/// - `Option<usize>`: Generate array element size of lvalue
/// - `Option<(usize, usize)>`: Index/element size of lvalue
/// - `String`: Name of rvalue
/// - `Option<usize>`: Generate array element size of rvalue
/// - `Option<(usize, usize)>`: Index/element size of rvalue
#[allow(clippy::type_complexity)]
pub(super) fn gen_wiring_array(
    module: &lir::CompositeModule, prefix: Option<String>,
) -> Result<
    Vec<(String, Option<usize>, Option<(usize, usize)>, String, Option<usize>, Option<(usize, usize)>)>,
    lir::ModuleError,
> {
    let mut conts = Vec::new();

    // Connections from input interface of the module and output interfaces of submodules in the module.
    let mut input_connections = Vec::new();
    let mut comp_connections = vec![Vec::new(); module.submodules.len()];

    for (submodule_index, (submodule, from)) in module.submodules.iter().enumerate() {
        for (interface, path) in from.clone().into_primitives() {
            let channel = some_or!(interface.get_channel(), continue);

            let mut comp_accessor = gen_channel_accessor(&from.typ(), path);
            comp_accessor.prefix = join_options("_", [
                Some(format!("{}_{}", submodule.get_module_name(), submodule_index)),
                match &*submodule.inner {
                    lir::ModuleInner::Composite(..) => submodule.inner.input_prefix(),
                    lir::ModuleInner::Fsm(_) | lir::ModuleInner::ModuleInst(_) | lir::ModuleInner::VirtualModule(_) => {
                        Some("in".to_string())
                    }
                },
                comp_accessor.prefix,
            ]);

            match channel.endpoint() {
                lir::Endpoint::Input { path } => {
                    let mut from_accessor = gen_channel_accessor(&module.input_interface_typ(), path);
                    from_accessor.prefix = join_options("_", [module.input_prefix.clone(), from_accessor.prefix]);

                    input_connections.push((from_accessor, true, comp_accessor, false, channel.typ()));
                }
                lir::Endpoint::Submodule { submodule_index, path } => {
                    let mut from_accessor =
                        gen_channel_accessor(&module.submodules[submodule_index].0.inner.output_interface_typ(), path);
                    from_accessor.prefix = join_options("_", [
                        Some(format!("{}_{}", module.submodules[submodule_index].0.get_module_name(), submodule_index)),
                        match &*module.submodules[submodule_index].0.inner {
                            lir::ModuleInner::Composite(..) => {
                                module.submodules[submodule_index].0.inner.output_prefix()
                            }
                            lir::ModuleInner::Fsm(_)
                            | lir::ModuleInner::ModuleInst(_)
                            | lir::ModuleInner::VirtualModule(_) => Some("out".to_string()),
                        },
                        from_accessor.prefix,
                    ]);

                    comp_connections[submodule_index].push((from_accessor, false, comp_accessor, false, channel.typ()));
                }
                _ => panic!("internal compiler error"),
            }
        }
    }

    for (interface, path) in module.output_interface.clone().into_primitives() {
        let channel = some_or!(interface.get_channel(), continue);

        let mut output_accessor = gen_channel_accessor(&module.output_interface_typ(), path);
        output_accessor.prefix = join_options("_", [module.output_prefix.clone(), output_accessor.prefix]);

        match channel.endpoint() {
            lir::Endpoint::Input { path } => {
                let mut from_accessor = gen_channel_accessor(&module.input_interface_typ(), path);
                from_accessor.prefix = join_options("_", [module.input_prefix.clone(), from_accessor.prefix]);

                input_connections.push((from_accessor, true, output_accessor, true, channel.typ()));
            }
            lir::Endpoint::Submodule { submodule_index, path } => {
                let mut from_accessor =
                    gen_channel_accessor(&module.submodules[submodule_index].0.inner.output_interface_typ(), path);
                from_accessor.prefix = join_options("_", [
                    Some(format!("{}_{}", module.submodules[submodule_index].0.get_module_name(), submodule_index,)),
                    match &*module.submodules[submodule_index].0.inner {
                        lir::ModuleInner::Composite(..) => module.submodules[submodule_index].0.inner.output_prefix(),
                        lir::ModuleInner::Fsm(_)
                        | lir::ModuleInner::ModuleInst(_)
                        | lir::ModuleInner::VirtualModule(_) => Some("out".to_string()),
                    },
                    from_accessor.prefix,
                ]);

                comp_connections[submodule_index].push((from_accessor, false, output_accessor, true, channel.typ()));
            }
            _ => panic!("internal compiler error"),
        }
    }

    for (from_accessor, from_generate, to_accessor, to_generate, channel_typ) in
        ::std::iter::empty().chain(input_connections.iter()).chain(comp_connections.concat().iter())
    {
        let lvalue_prefix = join_options("_", [prefix.clone(), to_accessor.prefix.clone()]);
        let rvalue_prefix = join_options("_", [prefix.clone(), from_accessor.prefix.clone()]);

        for (name, shape) in channel_typ.fwd.iter() {
            assert_eq!(shape.dim(), 1);
            let to_sep = to_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_generate = if *to_generate {
                match to_accessor.index {
                    Some((_, total)) => Some(shape.width() * total),
                    None => Some(shape.width()),
                }
            } else {
                None
            };
            let to_range = to_accessor.index.map(|(index, _)| (index, shape.width()));
            let from_sep = from_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_generate = if *from_generate {
                match from_accessor.index {
                    Some((_, total)) => Some(shape.width() * total),
                    None => Some(shape.width()),
                }
            } else {
                None
            };
            let from_range = from_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&to_sep, [lvalue_prefix.clone(), name.clone()]).unwrap(),
                to_generate,
                to_range,
                join_options(&from_sep, [rvalue_prefix.clone(), name]).unwrap(),
                from_generate,
                from_range,
            ));
        }

        for (name, shape) in channel_typ.bwd.iter() {
            assert_eq!(shape.dim(), 1);
            let from_sep = from_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let from_generate = if *from_generate {
                match from_accessor.index {
                    Some((_, total)) => Some(shape.width() * total),
                    None => Some(shape.width()),
                }
            } else {
                None
            };
            let from_range = from_accessor.index.map(|(index, _)| (index, shape.width()));
            let to_sep = to_accessor.sep.clone().unwrap_or_else(|| "_".to_string());
            let to_generate = if *to_generate {
                match to_accessor.index {
                    Some((_, total)) => Some(shape.width() * total),
                    None => Some(shape.width()),
                }
            } else {
                None
            };
            let to_range = to_accessor.index.map(|(index, _)| (index, shape.width()));
            conts.push((
                join_options(&from_sep, [rvalue_prefix.clone(), name.clone()]).unwrap(),
                from_generate,
                from_range,
                join_options(&to_sep, [lvalue_prefix.clone(), name]).unwrap(),
                to_generate,
                to_range,
            ));
        }
    }

    Ok(conts)
}

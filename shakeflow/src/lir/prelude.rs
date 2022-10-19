//! Low-level IR's prelude.

use std::collections::{HashMap, VecDeque};
use std::iter::FromIterator;
use std::ops::*;

use linked_hash_map::LinkedHashMap;

use crate::utils::join_options;

/// Shape of an array.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shape {
    inner: VecDeque<usize>,
}

impl Shape {
    /// Creates new shape.
    pub fn new<I: IntoIterator<Item = usize>>(iterable: I) -> Self { Self { inner: iterable.into_iter().collect() } }

    /// Returns dimension of array.
    pub fn dim(&self) -> usize { self.inner.len() }

    /// Returns number of elements in array.
    pub fn width(&self) -> usize { self.inner.iter().product() }

    /// TODO: Documentation
    pub fn get(&self, index: usize) -> usize {
        assert!(self.dim() > index);
        *self.inner.get(index).unwrap()
    }

    /// TODO: Documentation
    #[must_use]
    pub fn multiple(&self, n: usize) -> Self {
        let mut inner = self.inner.clone();
        let front = inner.pop_front().unwrap();
        inner.push_front(front * n);

        Self { inner }
    }

    /// TODO: Documentation
    #[must_use]
    pub fn divide(&self, n: usize) -> Self {
        let mut inner = self.inner.clone();
        let front = inner.pop_front().unwrap();
        assert_eq!(front % n, 0);
        inner.push_front(front / n);

        Self { inner }
    }
}

/// LIR value type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortDecls {
    /// Collection of channels.
    Struct(Vec<(Option<String>, PortDecls)>),

    /// Single channel which contains its width.
    Bits(Shape),
}

impl PortDecls {
    /// Width of `PortDecls`.
    pub fn width(&self) -> usize {
        match self {
            PortDecls::Struct(inner) => inner.iter().map(|(_, m)| m.width()).sum(),
            PortDecls::Bits(shape) => shape.width(),
        }
    }

    /// Maximum dimension of the primitive value types in `PortDecls`.
    pub fn max_dim(&self) -> usize { self.iter().map(|(_, shape)| shape.dim()).max().unwrap_or(1) }

    /// Iterator for `PortDecls`.
    ///
    /// # Note
    ///
    /// The iterator returns (name, width) for inner fields **ONLY** with nonzero width.
    /// This is to ignore meaningless unit types. (e.g. The unit type in `Keep<V, ()>`)
    pub fn iter(&self) -> ValueTypIterator { self.into_iter() }

    /// Consumes the `PortDecls`, returning new `PortDecls` with width of each field multiplied by `n`.
    #[must_use]
    pub fn multiple(&self, n: usize) -> Self {
        match self {
            PortDecls::Struct(inner) => {
                PortDecls::Struct(inner.clone().into_iter().map(|(name, m)| (name, m.multiple(n))).collect::<Vec<_>>())
            }
            PortDecls::Bits(shape) => PortDecls::Bits(shape.multiple(n)),
        }
    }

    /// Consumes the `PortDecls`, returning new `PortDecls` with width of each field divided by `n`.
    #[must_use]
    pub fn divide(&self, n: usize) -> Self {
        match self {
            PortDecls::Struct(inner) => {
                PortDecls::Struct(inner.clone().into_iter().map(|(name, m)| (name, m.divide(n))).collect::<Vec<_>>())
            }
            PortDecls::Bits(shape) => PortDecls::Bits(shape.divide(n)),
        }
    }

    fn iter_with_prefix(&self, prefix: Option<String>) -> ValueTypIterator {
        let mut iter_vec = vec![];

        match self {
            PortDecls::Struct(inner) => {
                for (name, member) in inner {
                    iter_vec.extend(member.iter_with_prefix(join_options("_", [prefix.clone(), name.clone()])).inner)
                }
            }
            PortDecls::Bits(shape) => {
                if shape.width() > 0 {
                    iter_vec.push((prefix, shape.clone()));
                }
            }
        }

        ValueTypIterator { inner: iter_vec.into() }
    }
}

impl IntoIterator for &PortDecls {
    type IntoIter = ValueTypIterator;
    type Item = (Option<String>, Shape);

    fn into_iter(self) -> Self::IntoIter { self.iter_with_prefix(None) }
}

/// Iterator for `PortDecls`.
#[derive(Debug)]
pub struct ValueTypIterator {
    inner: VecDeque<(Option<String>, Shape)>,
}

impl Iterator for ValueTypIterator {
    type Item = (Option<String>, Shape);

    fn next(&mut self) -> Option<Self::Item> { self.inner.pop_front() }
}

/// Channel's type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelTyp {
    /// Forward value.
    pub fwd: PortDecls,

    /// Backward value.
    pub bwd: PortDecls,
}

impl ChannelTyp {
    /// Creates a new channel type.
    pub const fn new(fwd: PortDecls, bwd: PortDecls) -> Self { Self { fwd, bwd } }
}

/// Interface's type.
#[allow(variant_size_differences)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceTyp {
    /// Unit type
    Unit,

    /// Single channel type
    Channel(ChannelTyp),

    /// Array of interface types
    Array(Box<InterfaceTyp>, usize),

    /// Expansive array of interface types
    ExpansiveArray(Box<InterfaceTyp>, usize),

    /// Struct of interface types. The first `String` of value indicates separator of the field.
    Struct(LinkedHashMap<String, (Option<String>, InterfaceTyp)>),
}

impl InterfaceTyp {
    /// TODO: Documentation
    pub fn get_channel_typ(self) -> Option<ChannelTyp> {
        if let InterfaceTyp::Channel(channel_typ) = self {
            Some(channel_typ)
        } else {
            None
        }
    }

    /// Returns primitive interface types and their endpoint paths in the interface type.
    // TODO: Change return type and consider primitives of `VarArray`.
    pub fn into_primitives(&self) -> Vec<(InterfaceTyp, EndpointPath)> {
        match self {
            InterfaceTyp::Unit | InterfaceTyp::Channel(_) => vec![(self.clone(), EndpointPath::default())],
            InterfaceTyp::Array(interface_typ, count) => (0..*count)
                .flat_map(|i| {
                    interface_typ.into_primitives().into_iter().map(move |(primitive_typ, mut path)| {
                        path.inner.push_front(EndpointNode::Index(i));
                        (primitive_typ, path)
                    })
                })
                .collect(),
            InterfaceTyp::ExpansiveArray(interface_typ, count) => (0..*count)
                .flat_map(|i| {
                    interface_typ.into_primitives().into_iter().map(move |(primitive_typ, mut path)| {
                        path.inner.push_front(EndpointNode::ExpansiveIndex(i));
                        (primitive_typ, path)
                    })
                })
                .collect(),
            InterfaceTyp::Struct(inner) => inner
                .into_iter()
                .flat_map(|(name, (sep, interface_typ))| {
                    interface_typ.into_primitives().into_iter().map(|(primitive_typ, mut path)| {
                        path.inner.push_front(EndpointNode::Field(name.clone(), sep.clone()));
                        (primitive_typ, path)
                    })
                })
                .collect(),
        }
    }

    /// Returns subinterface given a endpoint path
    pub fn get_subinterface(&self, mut path: EndpointPath) -> Self {
        if let Some(front) = path.pop_front() {
            match (front, self) {
                (EndpointNode::Index(i), InterfaceTyp::Array(typ, size)) => {
                    assert!(i < *size);
                    typ.get_subinterface(path)
                }
                (EndpointNode::ExpansiveIndex(i), InterfaceTyp::ExpansiveArray(typ, size)) => {
                    assert!(i < *size);
                    typ.get_subinterface(path)
                }
                (EndpointNode::Field(field, _), InterfaceTyp::Struct(map)) => {
                    if let Some((_, typ)) = map.get(&field) {
                        typ.get_subinterface(path)
                    } else {
                        panic!("{} does not exist in the struct", field)
                    }
                }
                _ => panic!("path and interface doesn't match"),
            }
        } else {
            self.clone()
        }
    }
}

/// Input/output channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    /// Channel's typ.
    pub typ: ChannelTyp,

    /// Channel's endpoint.
    pub endpoint: Endpoint,
}

impl Channel {
    /// Returns channel type.
    pub fn typ(&self) -> ChannelTyp { self.typ.clone() }

    /// Returns endpoint.
    pub fn endpoint(&self) -> Endpoint { self.endpoint.clone() }
}

/// Input/output interface.
#[allow(variant_size_differences)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interface {
    /// Unit
    Unit,

    /// Single channel
    Channel(Channel),

    /// Array of interfaces
    Array(Vec<Interface>),

    /// Expansive array of interfaces
    ExpansiveArray(Vec<Interface>),

    /// Struct of interfaces. The first `Option<String>` of value indicates separator of the field.
    /// If it is `None`, then separator is '_'.
    Struct(LinkedHashMap<String, (Option<String>, Interface)>),
}

impl Default for Interface {
    fn default() -> Self { Interface::Unit }
}

impl Interface {
    /// TODO: Documentation
    pub fn get_channel(self) -> Option<Channel> {
        if let Interface::Channel(channel) = self {
            Some(channel)
        } else {
            None
        }
    }

    /// Returns the interface type.
    pub fn typ(&self) -> InterfaceTyp {
        match self {
            Interface::Unit => InterfaceTyp::Unit,
            Interface::Channel(channel) => InterfaceTyp::Channel(channel.typ.clone()),
            Interface::Array(inner) => InterfaceTyp::Array(Box::new(inner[0].typ()), inner.len()),
            Interface::ExpansiveArray(inner) => InterfaceTyp::ExpansiveArray(Box::new(inner[0].typ()), inner.len()),
            Interface::Struct(inner) => InterfaceTyp::Struct(
                inner.iter().map(|(name, (sep, interface))| (name.clone(), (sep.clone(), interface.typ()))).collect(),
            ),
        }
    }

    /// Returns primitive interfaces in the interface.
    pub fn into_primitives(&self) -> Vec<(Interface, EndpointPath)> {
        match self {
            Interface::Unit | Interface::Channel(_) => vec![(self.clone(), EndpointPath::default())],
            Interface::Array(interfaces) => interfaces
                .iter()
                .enumerate()
                .flat_map(|(i, interface)| {
                    interface.into_primitives().into_iter().map(move |(primitive, mut path)| {
                        path.inner.push_front(EndpointNode::Index(i));
                        (primitive, path)
                    })
                })
                .collect(),
            Interface::ExpansiveArray(interfaces) => interfaces
                .iter()
                .enumerate()
                .flat_map(|(i, interface)| {
                    interface.into_primitives().into_iter().map(move |(primitive, mut path)| {
                        path.inner.push_front(EndpointNode::ExpansiveIndex(i));
                        (primitive, path)
                    })
                })
                .collect(),
            Interface::Struct(inner) => inner
                .iter()
                .flat_map(|(name, (sep, interface))| {
                    interface.into_primitives().into_iter().map(|(primitive, mut path)| {
                        path.inner.push_front(EndpointNode::Field(name.clone(), sep.clone()));
                        (primitive, path)
                    })
                })
                .collect(),
        }
    }
}

impl FromIterator<(Interface, EndpointPath)> for Interface {
    /// Constructs interface from primitive interfaces.
    fn from_iter<I: IntoIterator<Item = (Interface, EndpointPath)>>(iter: I) -> Self {
        let mut primitives = iter.into_iter().collect::<Vec<_>>();
        assert!(!primitives.is_empty());

        let is_primitive = primitives[0].1.inner.front().is_none();
        if is_primitive {
            assert_eq!(primitives.len(), 1);
            let (primitive, _) = primitives.pop().unwrap();
            assert!(matches!(primitive, Interface::Unit | Interface::Channel(_)));
            primitive
        } else {
            match primitives[0].1.inner.front().unwrap() {
                EndpointNode::Index(_) => {
                    let mut interfaces = HashMap::<usize, Vec<(Interface, EndpointPath)>>::new();
                    for (interface, mut path) in primitives {
                        let node = path.inner.pop_front().unwrap();
                        match node {
                            EndpointNode::Index(i) => {
                                interfaces.entry(i).or_default();
                                let primitives = interfaces.get_mut(&i).unwrap();
                                primitives.push((interface, path));
                            }
                            _ => panic!("internal compiler error"),
                        }
                    }
                    let len = interfaces.len();
                    Interface::Array(
                        (0..len).map(|i| interfaces.get(&i).unwrap().clone().into_iter().collect()).collect(),
                    )
                }
                EndpointNode::ExpansiveIndex(_) => {
                    let mut interfaces = HashMap::<usize, Vec<(Interface, EndpointPath)>>::new();
                    for (interface, mut path) in primitives {
                        let node = path.inner.pop_front().unwrap();
                        match node {
                            EndpointNode::ExpansiveIndex(i) => {
                                interfaces.entry(i).or_default();
                                let primitives = interfaces.get_mut(&i).unwrap();
                                primitives.push((interface, path));
                            }
                            _ => panic!("internal compiler error"),
                        }
                    }
                    let len = interfaces.len();
                    Interface::ExpansiveArray(
                        (0..len).map(|i| interfaces.get(&i).unwrap().clone().into_iter().collect()).collect(),
                    )
                }
                EndpointNode::Field(..) => {
                    let mut inner = LinkedHashMap::<String, (Option<String>, Vec<(Interface, EndpointPath)>)>::new();
                    for (interface, mut path) in primitives {
                        let node = path.inner.pop_front().unwrap();
                        match node {
                            EndpointNode::Field(name, sep) => {
                                inner.entry(name.clone()).or_insert((sep, Vec::new()));
                                let primitives = inner.get_mut(&name).unwrap();
                                primitives.1.push((interface, path));
                            }
                            _ => panic!("internal compiler error"),
                        }
                    }
                    Interface::Struct(
                        inner
                            .into_iter()
                            .map(|(name, (sep, primitives))| (name, (sep, primitives.into_iter().collect())))
                            .collect(),
                    )
                }
            }
        }
    }
}

/// Endpoint's node.
// TODO: Add array range types
#[allow(variant_size_differences)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EndpointNode {
    /// Element of array.
    Index(usize),

    /// Element of expansive array.
    ExpansiveIndex(usize),

    /// Field of struct. The first `String` indicates name of the field, and the second `Option<String>`
    /// indicates separator. If it is `None`, then separator is '_'.
    Field(String, Option<String>),
}

/// Endpoint's path.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct EndpointPath {
    /// List of endpoint nodes.
    pub inner: VecDeque<EndpointNode>,
}

impl FromIterator<EndpointNode> for EndpointPath {
    fn from_iter<T: IntoIterator<Item = EndpointNode>>(iter: T) -> Self { Self { inner: iter.into_iter().collect() } }
}

impl Deref for EndpointPath {
    type Target = VecDeque<EndpointNode>;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for EndpointPath {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

/// Wire's endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Input interface.
    Input {
        /// Interface's endpoint path in the input.
        path: EndpointPath,
    },

    /// Submodule endpoint.
    Submodule {
        /// Submodule's index in the module's submodules.
        submodule_index: usize,

        /// Interface's endpoint path in the submodule.
        path: EndpointPath,
    },

    /// Temporary interface used in `CompositeModule::wrap`.
    ///
    /// # Note
    ///
    /// This endpoint type does not appear in the final module. This type is only used to replace
    /// the output interface of the inner module and then updated to the original output interface
    /// of the inner module in `CompositeModule::wrap`.
    Temp {
        /// Interface's endpoint path in the output.
        path: EndpointPath,
    },
}

impl Endpoint {
    /// Creates a new endpoint on input.
    pub fn input(path: EndpointPath) -> Self { Self::Input { path } }

    /// Creates a new endpoint on submodule.
    pub fn submodule(submodule_index: usize, path: EndpointPath) -> Self { Self::Submodule { submodule_index, path } }

    /// Creates a new temporary endpoint.
    pub fn temp(path: EndpointPath) -> Self { Self::Temp { path } }

    /// Returns endpoint path.
    pub fn path(&self) -> &EndpointPath {
        match self {
            Endpoint::Input { path } => path,
            Endpoint::Submodule { path, .. } => path,
            Endpoint::Temp { path } => path,
        }
    }
}

/// Unary operators.
// TODO: Add more cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// Negation
    Negation,
}

impl ToString for UnaryOp {
    fn to_string(&self) -> String {
        match self {
            UnaryOp::Negation => "~",
        }
        .to_string()
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    /// Addition
    Add,

    /// Subtraction
    Sub,

    /// Multiplication
    Mul,

    /// Division
    Div,

    /// Modulus
    Mod,

    /// Or (bitwise)
    Or,

    /// And (bitwise)
    And,

    /// Xor (bitwise)
    Xor,

    /// Eq (bitwise, `a ~^ b`)
    Eq,

    /// Eq (arithmetic, `a == b`)
    EqArithmetic,

    /// Less than
    Less,

    /// Greater than
    Greater,

    /// Less than or equal
    LessEq,

    /// Greater than or equal
    GreaterEq,

    /// Shift left
    ShiftLeft,

    /// Shift right
    ShiftRight,
}

impl ToString for BinaryOp {
    fn to_string(&self) -> String {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Or => "|",
            BinaryOp::And => "&",
            BinaryOp::Xor => "^",
            BinaryOp::Eq => "~^",
            BinaryOp::EqArithmetic => "==",
            BinaryOp::Less => "<",
            BinaryOp::Greater => ">",
            BinaryOp::LessEq => "<=",
            BinaryOp::GreaterEq => ">=",
            BinaryOp::ShiftLeft => "<<",
            BinaryOp::ShiftRight => ">>",
        }
        .to_string()
    }
}

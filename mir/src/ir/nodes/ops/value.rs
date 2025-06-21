use air_parser::ast::{self, Identifier, QualifiedIdentifier, TraceColumnIndex, TraceSegmentId};
use miden_diagnostics::{SourceSpan, Spanned};

use crate::ir::{BackLink, Builder, Bus, Child, Link, Node, Op, Owner, Singleton};

/// A MIR operation to represent a known value, [Value].
/// Wraps a [SpannedMirValue] to represent a known value in the [MIR].
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Builder, Spanned)]
#[enum_wrapper(Op)]
pub struct Value {
    pub parents: Vec<BackLink<Owner>>,
    #[span]
    pub value: SpannedMirValue,
    pub _node: Singleton<Node>,
}

impl Value {
    pub fn create(value: SpannedMirValue) -> Link<Op> {
        Op::Value(Self { value, ..Default::default() }).into()
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self {
            value: SpannedMirValue {
                value: MirValue::Constant(ConstantValue::Felt(value as u64)),
                span: Default::default(),
            },
            ..Default::default()
        }
    }
}

impl Child for Value {
    type Parent = Owner;
    fn get_parents(&self) -> Vec<BackLink<Self::Parent>> {
        self.parents.clone()
    }
    fn add_parent(&mut self, parent: Link<Self::Parent>) {
        self.parents.push(parent.into());
    }
    fn remove_parent(&mut self, parent: Link<Self::Parent>) {
        self.parents.retain(|p| *p != parent.clone().into());
    }
}

/// Represents a known value in the [MIR].
///
/// Values are either constant, or evaluated at runtime using the context
/// provided to an AirScript program (i.e. public inputs, etc.).
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MirValue {
    /// A constant value.
    Constant(ConstantValue),
    /// A reference to a specific column in the trace segment, with an optional offset.
    TraceAccess(TraceAccess),
    /// A reference to a periodic column.
    ///
    /// The value this corresponds to is determined by the current row of the trace.
    PeriodicColumn(PeriodicColumnAccess),
    /// A reference to a specific element of a given public input
    PublicInput(PublicInputAccess),
    /// A reference to a public input table.
    PublicInputTable(PublicInputTableAccess),
    /// A reference to a specific index in the random values array.
    ///
    /// Random values are not provided by the user in the AirScript program, but are used to expand
    /// Bus constraints.
    RandomValue(usize),
    /// A binding to a set of consecutive trace columns of a given size.
    TraceAccessBinding(TraceAccessBinding),
    /// A binding to a [Bus].
    BusAccess(BusAccess),
    /// An empty bus
    Null,
    /// An unconstrained bus
    Unconstrained,
}

/// [BusAccess] is like [SymbolAccess], but is used to describe an access to a specific bus.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BusAccess {
    /// The trace segment being accessed
    pub bus: Link<Bus>,
    /// The offset from the current row.
    ///
    /// Defaults to 0, which indicates no offset/the current row.
    ///
    /// For example, if accessing a trace column with `a'`, where `a` is bound to a single column,
    /// the row offset would be `1`, as the `'` modifier indicates the "next" row.
    pub row_offset: usize,
}

impl BusAccess {
    /// Creates a new [BusAccess].
    pub const fn new(bus: Link<Bus>, row_offset: usize) -> Self {
        Self { bus, row_offset }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ConstantValue {
    Felt(u64),
    Vector(Vec<u64>),
    Matrix(Vec<Vec<u64>>),
}

/// [TraceAccess] is like [SymbolAccess], but is used to describe an access to a specific trace
/// column or columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceAccess {
    /// The trace segment being accessed
    pub segment: TraceSegmentId,
    /// The index of the first column at which the access begins
    pub column: TraceColumnIndex,
    /// The offset from the current row.
    ///
    /// Defaults to 0, which indicates no offset/the current row.
    ///
    /// For example, if accessing a trace column with `a'`, where `a` is bound to a single column,
    /// the row offset would be `1`, as the `'` modifier indicates the "next" row.
    pub row_offset: usize,
}
impl TraceAccess {
    /// Creates a new [TraceAccess].
    pub const fn new(segment: TraceSegmentId, column: TraceColumnIndex, row_offset: usize) -> Self {
        Self { segment, column, row_offset }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct TraceAccessBinding {
    pub segment: TraceSegmentId,
    /// The offset to the first column of the segment which is bound by this binding
    pub offset: usize,
    /// The number of columns which are bound
    pub size: usize,
}

/// Represents a typed value in the [MIR]
#[derive(Debug, Eq, PartialEq, Clone, Hash, Spanned)]
pub struct SpannedMirValue {
    #[span]
    pub span: SourceSpan,
    pub value: MirValue,
}

#[derive(Debug, Default, Eq, PartialEq, Clone, Hash)]
pub enum MirType {
    #[default]
    Felt,
    Vector(usize),
    Matrix(usize, usize),
}

impl From<ast::Type> for MirType {
    fn from(value: ast::Type) -> Self {
        match value {
            ast::Type::Felt => MirType::Felt,
            ast::Type::Vector(n) => MirType::Vector(n),
            ast::Type::Matrix(cols, rows) => MirType::Matrix(cols, rows),
        }
    }
}

/// Represents an access of a [PeriodicColumn], similar in nature to [TraceAccess].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PeriodicColumnAccess {
    pub name: QualifiedIdentifier,
    pub cycle: usize,
}
impl PeriodicColumnAccess {
    pub const fn new(name: QualifiedIdentifier, cycle: usize) -> Self {
        Self { name, cycle }
    }
}

/// Represents an access of a [PublicInput], similar in nature to [TraceAccess].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PublicInputAccess {
    /// The name of the public input to access
    pub name: Identifier,
    /// The index of the element in the public input to access
    pub index: usize,
}
impl PublicInputAccess {
    pub const fn new(name: Identifier, index: usize) -> Self {
        Self { name, index }
    }
}

/// Represents an access of a public input table, similar in nature to [TraceAccess].
///
/// It can only be bound to a [Bus]'s .first or .last boundary constraints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicInputTableAccess {
    /// The name of the public input to bind
    pub table_name: Identifier,
    /// The name of the bus to bind
    /// The bus name is not always known at the time of instantiation,
    /// making it an Option allows setting it later.
    bus_name: Option<Identifier>,
    /// The number of columns in the table
    pub num_cols: usize,
}

impl PublicInputTableAccess {
    pub const fn new(table_name: Identifier, num_cols: usize) -> Self {
        Self { table_name, bus_name: None, num_cols }
    }
    pub fn set_bus_name(&mut self, bus_name: Identifier) {
        self.bus_name = Some(bus_name);
    }
    pub fn bus_name(&self) -> Identifier {
        self.bus_name.expect("Bus name should have already been set")
    }
}

impl Default for SpannedMirValue {
    fn default() -> Self {
        Self {
            value: MirValue::Constant(ConstantValue::Felt(0)),
            span: Default::default(),
        }
    }
}

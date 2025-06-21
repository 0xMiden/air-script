//! This module provides AST structures which represent declarations permitted at module scope.
//!
//! There are no expressions/statements permitted in the top-level of a module, only declarations.
//! These declarations define named items which are used by functions/constraints during evaluation.
//!
//! Some declarations introduce identifiers at global scope, i.e. they are implicitly defined in all
//! modules regardless of imports.
//!
//! Certain declarations are only permitted in the root module of an AirScript program, as they are
//! also effectively global:
//!
//! * `trace_columns`
//! * `public_inputs`
//! * `boundary_constraints`
//! * `integrity_constraints`
//!
//! All other declarations are module-scoped, and must be explicitly imported by a module which
//! wishes to reference them. Not all items are importable however, only the following:
//!
//! * constants
//! * evaluators
//! * pure functions
//!
//! There is no notion of public/private visiblity, so any declaration of the above types may be
//! imported into another module, and "wildcard" imports will import all importable items.
use std::{collections::HashSet, fmt};

use miden_diagnostics::{SourceSpan, Spanned};

use super::*;

/// Represents all of the top-level items permitted at module scope.
#[derive(Debug, PartialEq, Eq, Spanned)]
pub enum Declaration {
    /// Import one or more items from the specified AirScript module to the current module
    Import(Span<Import>),
    /// A Bus section declaration
    Buses(Span<Vec<Bus>>),
    /// A constant value declaration
    Constant(Constant),
    /// An evaluator function definition
    ///
    /// Evaluator functions can be defined in any module of the program
    EvaluatorFunction(EvaluatorFunction),
    /// A pure function definition
    ///
    /// Pure functions can be defined in any module of the program
    Function(Function),
    /// A `periodic_columns` section declaration
    ///
    /// This may appear any number of times in the program, and may be declared in any module.
    PeriodicColumns(Span<Vec<PeriodicColumn>>),
    /// A `public_inputs` section declaration
    ///
    /// There may only be one of these in the entire program, and it must
    /// appear in the root AirScript module, i.e. in a module declared with `def`
    PublicInputs(Span<Vec<PublicInput>>),
    /// A `trace_bindings` section declaration
    ///
    /// There may only be one of these in the entire program, and it must
    /// appear in the root AirScript module, i.e. in a module declared with `def`
    Trace(Span<Vec<TraceSegment>>),
    /// A `boundary_constraints` section declaration
    ///
    /// There may only be one of these in the entire program, and it must
    /// appear in the root AirScript module, i.e. in a module declared with `def`
    BoundaryConstraints(Span<Vec<Statement>>),
    /// A `integrity_constraints` section declaration
    ///
    /// There may only be one of these in the entire program, and it must
    /// appear in the root AirScript module, i.e. in a module declared with `def`
    IntegrityConstraints(Span<Vec<Statement>>),
}

/// Represents a bus declaration in an AirScript module.
#[derive(Debug, Clone, Spanned)]
pub struct Bus {
    #[span]
    pub span: SourceSpan,
    pub name: Identifier,
    pub bus_type: BusType,
}
impl Bus {
    /// Creates a new bus declaration
    pub const fn new(span: SourceSpan, name: Identifier, bus_type: BusType) -> Self {
        Self { span, name, bus_type }
    }
}
#[derive(Default, Copy, Hash, Debug, Clone, PartialEq, Eq)]
pub enum BusType {
    /// A multiset bus
    #[default]
    Multiset,
    /// A logup bus
    Logup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusOperator {
    /// Insert a tuple to the bus
    Insert,
    /// Remove a tuple from the bus
    Remove,
}
impl std::fmt::Display for BusOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Insert => write!(f, "insert"),
            Self::Remove => write!(f, "remove"),
        }
    }
}

impl Eq for Bus {}
impl PartialEq for Bus {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.bus_type == other.bus_type
    }
}

/// Stores a constant's name and value. There are three types of constants:
///
/// * Scalar: 123
/// * Vector: \[1, 2, 3\]
/// * Matrix: \[\[1, 2, 3\], \[4, 5, 6\]\]
#[derive(Debug, Clone, Spanned)]
pub struct Constant {
    #[span]
    pub span: SourceSpan,
    pub name: Identifier,
    pub value: ConstantExpr,
}
impl Constant {
    /// Returns a new instance of a [Constant]
    pub const fn new(span: SourceSpan, name: Identifier, value: ConstantExpr) -> Self {
        Self { span, name, value }
    }

    /// Gets the type of the value associated with this constant
    pub fn ty(&self) -> Type {
        self.value.ty()
    }
}
impl Eq for Constant {}
impl PartialEq for Constant {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

/// Value of a constant. Constants can be of 3 value types:
///
/// * Scalar: 123
/// * Vector: \[1, 2, 3\]
/// * Matrix: \[\[1, 2, 3\], \[4, 5, 6\]\]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstantExpr {
    Scalar(u64),
    Vector(Vec<u64>),
    Matrix(Vec<Vec<u64>>),
}
impl ConstantExpr {
    /// Gets the type of this expression
    pub fn ty(&self) -> Type {
        match self {
            Self::Scalar(_) => Type::Felt,
            Self::Vector(elems) => Type::Vector(elems.len()),
            Self::Matrix(rows) => {
                let num_rows = rows.len();
                let num_cols = rows.first().unwrap().len();
                Type::Matrix(num_rows, num_cols)
            },
        }
    }

    /// Returns true if this expression is of aggregate type
    pub fn is_aggregate(&self) -> bool {
        matches!(self, Self::Vector(_) | Self::Matrix(_))
    }
}
impl fmt::Display for ConstantExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Scalar(value) => write!(f, "{value}"),
            Self::Vector(values) => {
                write!(f, "{}", DisplayList(values.as_slice()))
            },
            Self::Matrix(values) => write!(
                f,
                "{}",
                DisplayBracketed(DisplayCsv::new(
                    values.iter().map(|vs| DisplayList(vs.as_slice()))
                ))
            ),
        }
    }
}

/// An import declaration
///
/// There can be multiple of these in a given module
#[derive(Debug, Clone)]
pub enum Import {
    /// Imports all items from `module`
    All { module: ModuleId },
    /// Imports `items` from `module`
    Partial {
        module: ModuleId,
        items: HashSet<Identifier>,
    },
}
impl Import {
    pub fn module(&self) -> ModuleId {
        match self {
            Self::All { module } | Self::Partial { module, .. } => *module,
        }
    }
}
impl Eq for Import {}
impl PartialEq for Import {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::All { module: l }, Self::All { module: r }) => l == r,
            (Self::Partial { module: l, items: ls }, Self::Partial { module: r, items: rs })
                if l == r =>
            {
                ls.difference(rs).next().is_none()
            },
            _ => false,
        }
    }
}

/// Represents an item exported from a module
///
/// Currently, only constants and functions are exported.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Export<'a> {
    Constant(&'a crate::ast::Constant),
    Evaluator(&'a EvaluatorFunction),
}
impl Export<'_> {
    pub fn name(&self) -> Identifier {
        match self {
            Self::Constant(item) => item.name,
            Self::Evaluator(item) => item.name,
        }
    }

    /// Returns the type of the value associated with this export
    ///
    /// NOTE: Evaluator functions have no return value, so they have no type associated.
    /// For this reason, this function returns `Option<Type>` rather than `Type`.
    pub fn ty(&self) -> Option<Type> {
        match self {
            Self::Constant(item) => Some(item.ty()),
            Self::Evaluator(_) => None,
        }
    }
}

/// Declaration of a periodic column in an AirScript module.
///
/// Periodic columns are columns with repeating cycles of values. The values declared
/// for the periodic column should be the cycle of values that will be repeated. The
/// length of the values vector is expected to be a power of 2 with a minimum length of 2,
/// which is enforced during semantic analysis.
#[derive(Debug, Clone, Spanned)]
pub struct PeriodicColumn {
    #[span]
    pub span: SourceSpan,
    pub name: Identifier,
    pub values: Vec<u64>,
}
impl PeriodicColumn {
    pub const fn new(span: SourceSpan, name: Identifier, values: Vec<u64>) -> Self {
        Self { span, name, values }
    }

    pub fn period(&self) -> usize {
        self.values.len()
    }
}
impl Eq for PeriodicColumn {}
impl PartialEq for PeriodicColumn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.values == other.values
    }
}

/// Declaration of a public input for an AirScript program.
///
/// This declaration is only permitted in the root module.
///
/// Public inputs are represented by a named identifier which is used to identify a fixed
/// size array of length `size`.
#[derive(Debug, Clone, Spanned)]
pub enum PublicInput {
    Vector {
        #[span]
        span: SourceSpan,
        name: Identifier,
        size: usize,
    },
    Table {
        #[span]
        span: SourceSpan,
        name: Identifier,
        size: usize,
    },
}
impl PublicInput {
    #[inline]
    pub fn new_vector(span: SourceSpan, name: Identifier, size: u64) -> Self {
        Self::Vector {
            span,
            name,
            size: size.try_into().unwrap(),
        }
    }
    #[inline]
    pub fn new_table(span: SourceSpan, name: Identifier, size: u64) -> Self {
        Self::Table {
            span,
            name,
            size: size.try_into().unwrap(),
        }
    }
    #[inline]
    pub fn name(&self) -> Identifier {
        match self {
            Self::Vector { name, .. } | Self::Table { name, .. } => *name,
        }
    }
    #[inline]
    pub fn size(&self) -> usize {
        match self {
            Self::Vector { size, .. } | Self::Table { size, .. } => *size,
        }
    }
}
impl Eq for PublicInput {}
impl PartialEq for PublicInput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Vector { name: l, size: ls, .. }, Self::Vector { name: r, size: rs, .. }) => {
                l == r && ls == rs
            },
            (Self::Table { name: l, size: lc, .. }, Self::Table { name: r, size: rc, .. }) => {
                l == r && lc == rc
            },
            _ => false,
        }
    }
}

/// Evaluator functions take a vector of trace bindings as parameters where each trace binding
/// represents one or a group of columns in the execution trace that are passed to the evaluator
/// function, and enforce integrity constraints on those trace columns.
#[derive(Debug, Clone, Spanned)]
pub struct EvaluatorFunction {
    #[span]
    pub span: SourceSpan,
    pub name: Identifier,
    pub params: Vec<TraceSegment>,
    pub body: Vec<Statement>,
}
impl EvaluatorFunction {
    /// Creates a new function.
    pub const fn new(
        span: SourceSpan,
        name: Identifier,
        params: Vec<TraceSegment>,
        body: Vec<Statement>,
    ) -> Self {
        Self { span, name, params, body }
    }
}
impl Eq for EvaluatorFunction {}
impl PartialEq for EvaluatorFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.params == other.params && self.body == other.body
    }
}

/// Functions take a group of expressions as parameters and returns a value.
///
/// The result value of a function may be a felt, vector, or a matrix.
///
/// NOTE: Functions do not take trace bindings as parameters.
#[derive(Debug, Clone, Spanned)]
pub struct Function {
    #[span]
    pub span: SourceSpan,
    pub name: Identifier,
    pub params: Vec<(Identifier, Type)>,
    pub return_type: Type,
    pub body: Vec<Statement>,
}
impl Function {
    /// Creates a new function.
    pub const fn new(
        span: SourceSpan,
        name: Identifier,
        params: Vec<(Identifier, Type)>,
        return_type: Type,
        body: Vec<Statement>,
    ) -> Self {
        Self { span, name, params, return_type, body }
    }

    pub fn param_types(&self) -> Vec<Type> {
        self.params.iter().map(|(_, ty)| *ty).collect::<Vec<_>>()
    }
}

impl Eq for Function {}
impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.return_type == other.return_type
            && self.body == other.body
    }
}

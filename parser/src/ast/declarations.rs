//! This module provides AST structures which represent declarations permitted at module scope.
//!
//! There are no expressions/statements permitted in the top-level of a module, only declarations.
//! These declarations define named items which are used by functions/constraints during evaluation.
//!
//! Some declarations introduce identifiers at global scope, i.e. they are implicitly defined in all
//! modules regardless of imports. Currently, this is only the `random_values` section.
//!
//! Certain declarations are only permitted in the root module of an AirScript program, as they are
//! also effectively global:
//!
//! * `trace_columns`
//! * `public_inputs`
//! * `random_values`
//! * `boundary_constraints`
//! * `integrity_constraints`
//!
//! All other declarations are module-scoped, and must be explicitly imported by a module which wishes
//! to reference them. Not all items are importable however, only the following:
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
    /// A `random_values` section declaration
    ///
    /// There may only be one of these in the entire program, and it must
    /// appear in the root AirScript module, i.e. in a module declared with `def`
    RandomValues(RandomValues),
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
        Self {
            span,
            name,
            bus_type,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusType {
    /// A multiset bus
    Unit,
    /// A logup bus
    Mult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusOperator {
    /// Add a tuple to the bus
    Add,
    /// Remove a tuple from the bus
    Rem,
}
impl std::fmt::Display for BusOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "add"),
            Self::Rem => write!(f, "rem"),
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
            }
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
            Self::Scalar(value) => write!(f, "{}", value),
            Self::Vector(ref values) => {
                write!(f, "{}", DisplayList(values.as_slice()))
            }
            Self::Matrix(ref values) => write!(
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
            (
                Self::Partial {
                    module: l,
                    items: ls,
                },
                Self::Partial {
                    module: r,
                    items: rs,
                },
            ) if l == r => ls.difference(rs).next().is_none(),
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
pub struct PublicInput {
    #[span]
    pub span: SourceSpan,
    pub name: Identifier,
    pub size: usize,
}
impl PublicInput {
    #[inline]
    pub fn new(span: SourceSpan, name: Identifier, size: u64) -> Self {
        Self {
            span,
            name,
            size: size.try_into().unwrap(),
        }
    }
}
impl Eq for PublicInput {}
impl PartialEq for PublicInput {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.size == other.size
    }
}

/// Declaration of random values for an AirScript program.
///
/// This declaration is only permitted in the root module.
///
/// Random values are a fixed-size array bound to a given name. In addition to the name
/// of the array itself, individual elements or sub-slices of the array may be bound to
/// names which will also be globally-visible.
///
/// # Examples
///
/// A `random_values` declaration like the following is equivalent to creating a new
/// [RandomValues] instance with `RandomValues::with_size`, with the name `rand`, and
/// size `15`:
///
/// ```airscript
/// random_values {
///     rand: [15]
/// }
/// ```
///
/// A `random_values` declaration like the following however:
///
/// ```airscript
/// random_values {
///     rand: [a, b[12]]
/// }
/// ```
///
/// It is equivalent to creating it with `RandomValues::new`, with two separate bindings,
/// one for `a`, and one for `b`, with sizes `1` and `12` respectively. The size of the overall
/// [RandomValues] instance in that case would be `13`.
///
#[derive(Clone, Spanned)]
pub struct RandomValues {
    #[span]
    pub span: SourceSpan,
    /// The name bound to the `random_values` array
    pub name: Identifier,
    /// The size of the array
    pub size: usize,
    /// Zero or more bindings for individual elements or groups of elements
    pub bindings: Vec<RandBinding>,
}
impl RandomValues {
    /// Creates a new [RandomValues] array `size` elements
    pub const fn with_size(span: SourceSpan, name: Identifier, size: usize) -> Self {
        Self {
            span,
            name,
            size,
            bindings: vec![],
        }
    }

    /// Creates a new [RandomValues] array from the given bindings
    pub fn new(
        span: SourceSpan,
        name: Identifier,
        raw_bindings: Vec<Span<(Identifier, usize)>>,
    ) -> Self {
        let mut bindings = Vec::with_capacity(raw_bindings.len());
        let mut offset = 0;
        for binding in raw_bindings.into_iter() {
            let (name, size) = binding.item;
            let ty = match size {
                1 => Type::Felt,
                n => Type::Vector(n),
            };
            bindings.push(RandBinding::new(binding.span(), name, size, offset, ty));
            offset += size;
        }

        Self {
            span,
            name,
            size: offset,
            bindings,
        }
    }
}
impl Eq for RandomValues {}
impl PartialEq for RandomValues {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.size == other.size && self.bindings == other.bindings
    }
}
impl fmt::Debug for RandomValues {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RandomValues")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("bindings", &self.bindings)
            .finish()
    }
}
impl fmt::Display for RandomValues {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = self.name.as_str().strip_prefix('$') {
            write!(f, "{}: ", name)?;
        } else {
            write!(f, "{}: ", self.name)?;
        }
        if self.bindings.is_empty() {
            write!(f, "[{}]", self.size)
        } else {
            write!(f, "{}", DisplayList(self.bindings.as_slice()))
        }
    }
}

/// Declaration of a random value binding used in [RandomValues].
///
/// It is represented by a named identifier and its size.
#[derive(Copy, Clone, Spanned)]
pub struct RandBinding {
    #[span]
    pub span: SourceSpan,
    /// The name of this binding
    pub name: Identifier,
    /// The number of elements bound
    pub size: usize,
    /// The offset in the random values array where this binding begins
    pub offset: usize,
    /// The type of this binding
    pub ty: Type,
}
impl RandBinding {
    pub const fn new(
        span: SourceSpan,
        name: Identifier,
        size: usize,
        offset: usize,
        ty: Type,
    ) -> Self {
        Self {
            span,
            name,
            size,
            offset,
            ty,
        }
    }

    #[inline]
    pub fn ty(&self) -> Type {
        self.ty
    }

    #[inline]
    pub fn is_scalar(&self) -> bool {
        self.ty.is_scalar()
    }

    /// Derive a new [RandBinding] derived from the current one given an [AccessType]
    pub fn access(&self, access_type: AccessType) -> Result<Self, InvalidAccessError> {
        use super::{RangeBound, RangeExpr};
        match access_type {
            AccessType::Default => Ok(*self),
            AccessType::Slice(_) if self.is_scalar() => Err(InvalidAccessError::SliceOfScalar),
            AccessType::Slice(RangeExpr {
                start: RangeBound::Const(start),
                end: RangeBound::Const(end),
                ..
            }) if start > end => Err(InvalidAccessError::IndexOutOfBounds),
            AccessType::Slice(RangeExpr {
                start: RangeBound::Const(start),
                end: RangeBound::Const(end),
                ..
            }) => {
                let offset = self.offset + start.item;
                let size = end.item - start.item;
                Ok(Self {
                    offset,
                    size,
                    ty: Type::Vector(size),
                    ..*self
                })
            }
            AccessType::Slice(_) => {
                unreachable!("expected non-constant range bounds to have been erased by this point")
            }
            AccessType::Index(_) if self.is_scalar() => Err(InvalidAccessError::IndexIntoScalar),
            AccessType::Index(idx) if idx >= self.size => Err(InvalidAccessError::IndexOutOfBounds),
            AccessType::Index(idx) => {
                let offset = self.offset + idx;
                Ok(Self {
                    offset,
                    size: 1,
                    ty: Type::Felt,
                    ..*self
                })
            }
            AccessType::Matrix(_, _) => Err(InvalidAccessError::IndexIntoScalar),
        }
    }
}
impl Eq for RandBinding {}
impl PartialEq for RandBinding {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.size == other.size
            && self.offset == other.offset
            && self.ty == other.ty
    }
}
impl fmt::Debug for RandBinding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RandBinding")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("ty", &self.ty)
            .finish()
    }
}
impl fmt::Display for RandBinding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.size == 1 {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}[{}]", self.name, self.size)
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
        Self {
            span,
            name,
            params,
            body,
        }
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
        Self {
            span,
            name,
            params,
            return_type,
            body,
        }
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

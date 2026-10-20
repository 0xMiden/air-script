use std::fmt;

use crate::ast::{
    AccessType, BusType, FunctionType, InvalidAccessError, ScalarExpr, TraceBinding, Type,
};

/// This type provides type and contextual information about a binding,
/// i.e. not only does it tell us the type of a binding, but what type
/// of value was bound. This is used during analysis to check whether a
/// particular access is valid for the context it is in, as well as to
/// propagate type information while retaining information about where
/// the type was derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingType {
    /// A local variable whose value is not an alias of a global/module declaration
    Local(Type),
    /// A local variable that aliases a global/module declaration
    Alias(Box<BindingType>),
    /// A direct reference to a constant declaration
    Constant(Type),
    /// A type associated with a function signature
    ///
    /// The result type is None if the function is an evaluator
    Function(FunctionType),
    /// A binding to a bus definition
    Bus(BusType),
    /// A function parameter corresponding to trace columns
    TraceParam(TraceBinding),
    /// A direct reference to one or more contiguous trace columns
    TraceColumn(TraceBinding),
    /// A potentially non-contiguous set of trace columns
    Vector(Vec<BindingType>),
    /// A direct reference to a public input
    PublicInput(Type),
    /// A direct reference to a periodic column
    PeriodicColumn(usize),
}
impl BindingType {
    /// Get the value type of this binding, if applicable
    pub fn ty(&self) -> Option<Type> {
        match self {
            Self::TraceColumn(tb) | Self::TraceParam(tb) => Some(tb.ty()),
            Self::Vector(elems) => Some(Type::Vector(elems.len())),
            Self::Alias(aliased) => aliased.ty(),
            Self::Local(ty) | Self::Constant(ty) | Self::PublicInput(ty) => Some(*ty),
            Self::PeriodicColumn(_) => Some(Type::Felt),
            Self::Function(ty) => ty.result(),
            Self::Bus(_) => Some(Type::Felt),
        }
    }

    /// Produce a new [BindingType] which represents accessing the current binding via `access_type`
    pub fn access(&self, access_type: AccessType) -> Result<Self, InvalidAccessError> {
        match self {
            Self::Alias(aliased) => aliased.access(access_type),
            Self::Local(ty) => ty.access(access_type).map(Self::Local),
            Self::Constant(ty) => {
                ty.access(access_type).map(|t| Self::Alias(Box::new(Self::Constant(t))))
            },
            Self::TraceColumn(tb) => tb.access(access_type).map(Self::TraceColumn),
            Self::TraceParam(tb) => tb.access(access_type).map(Self::TraceParam),
            Self::Vector(elems) => match access_type {
                AccessType::Default => Ok(Self::Vector(elems.clone())),
                AccessType::Index(idx) => {
                    if let ScalarExpr::Const(idx) = *idx {
                        if idx.item as usize >= elems.len() {
                            Err(InvalidAccessError::IndexOutOfBounds)
                        } else {
                            Ok(elems[idx.item as usize].clone())
                        }
                    } else {
                        // Items are all of the same type, we can just return the first one for now,
                        // as we cannot determine its value for now.
                        Ok(elems[0].clone())
                    }
                },
                AccessType::Slice(range) => {
                    let slice_range = range.to_slice_range();
                    // The range must be well-ordered and within bounds, otherwise slicing the
                    // elements with it would panic.
                    if slice_range.start > slice_range.end || slice_range.end > elems.len() {
                        Err(InvalidAccessError::IndexOutOfBounds)
                    } else {
                        Ok(Self::Vector(elems[slice_range].to_vec()))
                    }
                },
                AccessType::Matrix(row, col) => {
                    if let ScalarExpr::Const(row) = *row {
                        if row.item as usize >= elems.len() {
                            Err(InvalidAccessError::IndexOutOfBounds)
                        } else {
                            elems[row.item as usize].access(AccessType::Index(col))
                        }
                    } else {
                        // Items are all of the same type, we can just return the first one for now,
                        // as we cannot determine its value for now.
                        elems[0].access(AccessType::Index(col))
                    }
                },
            },
            Self::PublicInput(ty) => ty.access(access_type).map(Self::PublicInput),
            Self::PeriodicColumn(period) => match access_type {
                AccessType::Default => Ok(Self::PeriodicColumn(*period)),
                _ => Err(InvalidAccessError::IndexIntoScalar),
            },
            Self::Function(_) => Err(InvalidAccessError::InvalidBinding),
            Self::Bus(bus) => Ok(Self::Bus(*bus)),
        }
    }
}
impl fmt::Display for BindingType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Alias(aliased) => write!(f, "{aliased}"),
            Self::Local(_) => f.write_str("local"),
            Self::Constant(_) => f.write_str("constant"),
            Self::Vector(_) => f.write_str("vector"),
            Self::Function(_) => f.write_str("function"),
            Self::TraceColumn(_) | Self::TraceParam(_) => f.write_str("trace column(s)"),
            Self::PublicInput(_) => f.write_str("public input(s)"),
            Self::PeriodicColumn(_) => f.write_str("periodic column(s)"),
            Self::Bus(_) => f.write_str("bus"),
        }
    }
}

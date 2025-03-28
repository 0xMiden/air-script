use crate::ast::{
    AccessType, BusType, FunctionType, InvalidAccessError, RandBinding, TraceBinding, Type,
};
use std::fmt;

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
    /// A direct reference to a random value binding
    RandomValue(RandBinding),
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
            Self::RandomValue(rb) => Some(rb.ty()),
            Self::Alias(aliased) => aliased.ty(),
            Self::Local(ty) | Self::Constant(ty) | Self::PublicInput(ty) => Some(*ty),
            Self::PeriodicColumn(_) => Some(Type::Felt),
            Self::Function(ty) => ty.result(),
            Self::Bus(_) => Some(Type::Felt),
        }
    }

    /// Returns true if this binding type is a trace binding
    pub fn is_trace_binding(&self) -> bool {
        match self {
            Self::TraceColumn(_) | Self::TraceParam(_) => true,
            Self::Vector(ref elems) => elems.iter().all(|e| e.is_trace_binding()),
            _ => false,
        }
    }

    /// This function is used to split the current binding into two parts, the
    /// first of which contains `n` trace columns, the second of which contains
    /// what remains of the original binding. This function returns `Ok` when
    /// there were `n` columns in the input binding type, otherwise `Err` with
    /// a binding that contains as many columns as possible.
    ///
    /// If the input binding type is a single logical binding, then the resulting
    /// binding types will be of the same type. If however, the input binding type
    /// is a vector of bindings, then the first part of the split will be a vector
    /// containing `n` elements, where each element is a single logical binding of
    /// size 1. This corresponds to the way trace column bindings are packed/unpacked
    /// using vectors/lists in AirScript
    pub fn split_columns(&self, n: usize) -> Result<(Self, Option<Self>), Self> {
        use core::cmp::Ordering;

        if n == 1 {
            return Ok(self.pop_column());
        }

        match self {
            Self::TraceColumn(ref tb) => match n.cmp(&tb.size) {
                Ordering::Equal => Ok((self.clone(), None)),
                Ordering::Less => {
                    let remaining = tb.size - n;
                    let first = Self::TraceColumn(TraceBinding { size: n, ..*tb });
                    let rest = Self::TraceColumn(TraceBinding {
                        size: remaining,
                        offset: tb.offset + n,
                        ..*tb
                    });
                    Ok((first, Some(rest)))
                }
                Ordering::Greater => Err(self.clone()),
            },
            Self::Vector(ref elems) if elems.len() == 1 => elems[0].split_columns(n),
            Self::Vector(ref elems) => {
                let mut index = 0;
                let mut remaining = n;
                let mut set = Vec::with_capacity(elems.len());
                let mut next = elems.get(index).cloned();
                while remaining > 0 {
                    match next.take() {
                        None => return Err(Self::Vector(set)),
                        Some(binding_ty) => {
                            let (col, rest) = binding_ty.pop_column();
                            set.push(col);
                            remaining -= 1;
                            next = rest.or_else(|| {
                                index += 1;
                                elems.get(index).cloned()
                            });
                        }
                    }
                }
                let leftover = elems.len() - (index + 1);
                match next {
                    None => Ok((Self::Vector(set), None)),
                    Some(mid) => {
                        index += 1;
                        let mut rest = Vec::with_capacity(leftover + 1);
                        rest.push(mid);
                        rest.extend_from_slice(&elems[index..]);
                        Ok((Self::Vector(set), Some(Self::Vector(rest))))
                    }
                }
            }
            invalid => panic!("invalid trace column(s) binding type: {:#?}", invalid),
        }
    }

    /// This function is like `split`, for the use case in which only a single
    /// column is desired. This is used internally by `split` to handle those
    /// cases, but may be used directly as well.
    pub fn pop_column(&self) -> (Self, Option<Self>) {
        match self {
            // If we have a single logical binding, return the first half as
            // a binding containing the first column of that binding, and the
            // second half as a binding representing whatever was left, or `None`
            // if it is empty.
            Self::TraceColumn(ref tb) if tb.is_scalar() => (Self::TraceColumn(*tb), None),
            Self::TraceColumn(ref tb) => {
                let first = Self::TraceColumn(TraceBinding {
                    size: 1,
                    ty: Type::Felt,
                    ..*tb
                });
                let remaining = tb.size - 1;
                if remaining == 0 {
                    (first, None)
                } else {
                    let rest = Self::TraceColumn(TraceBinding {
                        size: remaining,
                        ty: Type::Vector(remaining),
                        offset: tb.offset + 1,
                        ..*tb
                    });
                    (first, Some(rest))
                }
            }
            // If the vector has only one element, remove the vector and
            // return the result of popping a column on the first element.
            Self::Vector(ref elems) if elems.len() == 1 => elems[0].pop_column(),
            // If the vector has multiple elements, then we're going to return
            // a vector for the remainder of the split.
            Self::Vector(ref elems) => {
                // Take the first element out of the vector
                let (popped, rest) = elems.split_first().unwrap();
                // Pop a single trace column from that element
                let (first, mid) = popped.pop_column();
                // The `popped` binding must have been a TraceColumn type, as
                // as nested binding vectors are not permitted in calls to evaluators
                match mid {
                    None => (first, Some(Self::Vector(rest.to_vec()))),
                    Some(mid) => {
                        let mut mid_and_rest = Vec::with_capacity(rest.len() + 1);
                        mid_and_rest.push(mid);
                        mid_and_rest.extend_from_slice(rest);
                        (first, Some(Self::Vector(mid_and_rest)))
                    }
                }
            }
            invalid => panic!("invalid trace column(s) binding type: {:#?}", invalid),
        }
    }

    /// Produce a new [BindingType] which represents accessing the current binding via `access_type`
    pub fn access(&self, access_type: AccessType) -> Result<Self, InvalidAccessError> {
        match self {
            Self::Alias(aliased) => aliased.access(access_type),
            Self::Local(ty) => ty.access(access_type).map(Self::Local),
            Self::Constant(ty) => ty
                .access(access_type)
                .map(|t| Self::Alias(Box::new(Self::Constant(t)))),
            Self::TraceColumn(tb) => tb.access(access_type).map(Self::TraceColumn),
            Self::TraceParam(tb) => tb.access(access_type).map(Self::TraceParam),
            Self::Vector(elems) => match access_type {
                AccessType::Default => Ok(Self::Vector(elems.clone())),
                AccessType::Index(idx) if idx >= elems.len() => {
                    Err(InvalidAccessError::IndexOutOfBounds)
                }
                AccessType::Index(idx) => Ok(elems[idx].clone()),
                AccessType::Slice(range) => {
                    let slice_range = range.to_slice_range();
                    if slice_range.end > elems.len() {
                        Err(InvalidAccessError::IndexOutOfBounds)
                    } else {
                        Ok(Self::Vector(elems[slice_range].to_vec()))
                    }
                }
                AccessType::Matrix(row, _) if row >= elems.len() => {
                    Err(InvalidAccessError::IndexOutOfBounds)
                }
                AccessType::Matrix(row, col) => elems[row].access(AccessType::Index(col)),
            },
            Self::RandomValue(tb) => tb
                .access(access_type)
                .map(|tb| Self::Alias(Box::new(Self::RandomValue(tb)))),
            Self::PublicInput(ty) => ty.access(access_type).map(Self::PublicInput),
            Self::PeriodicColumn(period) => match access_type {
                AccessType::Default => Ok(Self::PeriodicColumn(*period)),
                _ => Err(InvalidAccessError::IndexIntoScalar),
            },
            Self::Function(_) => Err(InvalidAccessError::InvalidBinding),
            Self::Bus(bus) => Ok(Self::Bus(bus.clone())),
        }
    }
}
impl fmt::Display for BindingType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Alias(aliased) => write!(f, "{}", aliased),
            Self::Local(_) => f.write_str("local"),
            Self::Constant(_) => f.write_str("constant"),
            Self::Vector(_) => f.write_str("vector"),
            Self::Function(_) => f.write_str("function"),
            Self::TraceColumn(_) | Self::TraceParam(_) => f.write_str("trace column(s)"),
            Self::RandomValue(_) => f.write_str("random value(s)"),
            Self::PublicInput(_) => f.write_str("public input(s)"),
            Self::PeriodicColumn(_) => f.write_str("periodic column(s)"),
            Self::Bus(_) => f.write_str("bus"),
        }
    }
}

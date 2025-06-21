use std::fmt;

use miden_diagnostics::{SourceSpan, Spanned};

use super::*;

/// The id of a trace segment is its index in the trace_columns declaration
pub type TraceSegmentId = usize;

/// The index of a column in a particular trace segment
pub type TraceColumnIndex = usize;

#[derive(Clone, Spanned)]
pub struct TraceSegment {
    #[span]
    pub span: SourceSpan,
    /// The index of this segment in the trace_columns declaration
    pub id: TraceSegmentId,
    /// The name of this trace segment, e.g. `$main`
    ///
    /// NOTE: The name of a trace segment is always a special identifier (i.e. has the `$` prefix)
    pub name: Identifier,
    /// The number of columns in this trace segment
    pub size: usize,
    /// Bindings declared in this segment, without the segment-wide binding, e.g. `$main`
    pub bindings: Vec<TraceBinding>,
    /// A vector of `size` elements which tracks for every column whether a
    /// constraint has been applied to that column, and on what boundaries.
    pub boundary_constrained: Vec<Span<ColumnBoundaryFlags>>,
}
impl TraceSegment {
    /// Constructs a new [TraceSegment] given a span, segment id, name, and a vector of (Identifier, size) pairs.
    pub fn new(
        span: SourceSpan,
        id: TraceSegmentId,
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
            bindings.push(TraceBinding::new(
                binding.span(),
                name,
                id,
                offset,
                size,
                ty,
            ));
            offset += size;
        }

        // The size of the segment is the sum of the sizes of all the bindings
        let size = offset;
        Self {
            span,
            id,
            name,
            size,
            bindings,
            boundary_constrained: vec![
                Span::new(SourceSpan::UNKNOWN, ColumnBoundaryFlags::EMPTY);
                size
            ],
        }
    }

    /// Returns true if `column` is constrained on `boundary`
    pub fn is_boundary_constrained(&self, column: TraceColumnIndex, boundary: Boundary) -> bool {
        self.boundary_constrained[column].is_constrained(boundary)
    }

    /// Marks `column` as constrained on `boundary`, and associates it with a span
    /// that is responsible for the constraint.
    ///
    /// Returns `None` if the column was previously unconstrained on `boundary`,
    /// otherwise it returns the span responsible for the previous constraint for
    /// use in diagnostics
    pub fn mark_constrained(
        &mut self,
        span: SourceSpan,
        column: TraceColumnIndex,
        boundary: Boundary,
    ) -> Option<SourceSpan> {
        let flags = &mut self.boundary_constrained[column];
        if flags.is_constrained(boundary) {
            Some(flags.span())
        } else {
            *flags = Span::new(span, flags.item | boundary);
            None
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}
impl fmt::Debug for TraceSegment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TraceSegment")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("size", &self.size)
            .field("bindings", &self.bindings)
            .field(
                "boundary_constrained",
                &FormatConstrainedFlags(&self.boundary_constrained),
            )
            .finish()
    }
}
impl fmt::Display for TraceSegment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.name.is_generated() {
            if let Some(name) = self.name.as_str().strip_prefix('$') {
                write!(f, "{name}: ")?;
            } else {
                write!(f, "{}: ", self.name)?;
            }
        }
        if self.bindings.is_empty() {
            write!(f, "[{}]", self.size)
        } else {
            write!(f, "{}", DisplayList(self.bindings.as_slice()))
        }
    }
}
impl Eq for TraceSegment {}
impl PartialEq for TraceSegment {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.bindings == other.bindings && self.size == other.size
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct ColumnBoundaryFlags(u8);
impl ColumnBoundaryFlags {
    /// An empty flag set that indicates the column is unconstrained
    pub const EMPTY: Self = Self(0);
    /// A flag set that indicates the column is constrained on the first boundary
    pub const FIRST: Self = Self(0b001);
    /// A flag set that indicates the column is constrained on the last boundary
    pub const LAST: Self = Self(0b010);
    /// A flag set that indicates the column is constrained on both boundaries
    pub const BOTH: Self = Self(0b011);

    /// Returns true if this column is constrained on `boundary`
    pub fn is_constrained(&self, boundary: Boundary) -> bool {
        *self & boundary
    }
}
impl fmt::Debug for ColumnBoundaryFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            0b000 => f.write_str("*"),
            0b001 => f.write_str("F"),
            0b010 => f.write_str("L"),
            0b011 => f.write_str("B"),
            _ => unreachable!(),
        }
    }
}
impl std::ops::BitOr<Boundary> for ColumnBoundaryFlags {
    type Output = ColumnBoundaryFlags;

    fn bitor(self, boundary: Boundary) -> Self {
        Self(
            self.0
                | match boundary {
                    Boundary::First => Self::FIRST.0,
                    Boundary::Last => Self::LAST.0,
                },
        )
    }
}
impl std::ops::BitAnd<Boundary> for ColumnBoundaryFlags {
    type Output = bool;

    fn bitand(self, boundary: Boundary) -> bool {
        let bit = match boundary {
            Boundary::First => Self::FIRST.0,
            Boundary::Last => Self::LAST.0,
        };
        self.0 & bit == bit
    }
}

/// Used to help format the boundary constraint flags
struct FormatConstrainedFlags<'a>(&'a [Span<ColumnBoundaryFlags>]);
impl fmt::Debug for FormatConstrainedFlags<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|c| c.item))
            .finish()
    }
}

/// [TraceBinding] is used to represent one or more columns in the execution trace that are bound to
/// a name. For single columns, the size is 1. For groups, the size is the number of columns in the
/// group. The offset is the column index in the trace where the first column of the binding starts.
#[derive(Copy, Clone, Spanned)]
pub struct TraceBinding {
    #[span]
    pub span: SourceSpan,
    /// The name of this binding, if applicable
    pub name: Option<Identifier>,
    /// The id of the segment to which this binding belongs
    pub segment: TraceSegmentId,
    /// The offset to the first column of the segment which is bound by this binding
    pub offset: usize,
    /// The number of columns which are bound
    pub size: usize,
    /// The effective type of this binding
    pub ty: Type,
}
impl TraceBinding {
    /// Creates a new trace binding.
    pub const fn new(
        span: SourceSpan,
        name: Identifier,
        segment: TraceSegmentId,
        offset: usize,
        size: usize,
        ty: Type,
    ) -> Self {
        Self {
            span,
            name: Some(name),
            segment,
            offset,
            size,
            ty,
        }
    }

    /// Returns a [Type] that describes what type of value this binding represents
    #[inline]
    pub fn ty(&self) -> Type {
        self.ty
    }

    #[inline]
    pub fn is_scalar(&self) -> bool {
        self.ty.is_scalar()
    }

    /// Derive a new [TraceBinding] derived from the current one given an [AccessType]
    pub fn access(&self, access_type: AccessType) -> Result<Self, InvalidAccessError> {
        match access_type {
            AccessType::Default => Ok(*self),
            AccessType::Slice(_) if self.is_scalar() => Err(InvalidAccessError::SliceOfScalar),
            AccessType::Slice(range) => {
                let slice_range = range.to_slice_range();
                if slice_range.end > self.size {
                    Err(InvalidAccessError::IndexOutOfBounds)
                } else {
                    let offset = self.offset + slice_range.start;
                    let size = slice_range.len();
                    Ok(Self {
                        offset,
                        size,
                        ty: Type::Vector(size),
                        ..*self
                    })
                }
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
impl Eq for TraceBinding {}
impl PartialEq for TraceBinding {
    fn eq(&self, other: &Self) -> bool {
        self.segment == other.segment
            && self.name == other.name
            && self.offset == other.offset
            && self.size == other.size
            && self.ty == other.ty
    }
}
impl fmt::Debug for TraceBinding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TraceBinding")
            .field("name", &self.name)
            .field("segment", &self.segment)
            .field("offset", &self.offset)
            .field("size", &self.size)
            .field("ty", &self.ty)
            .finish()
    }
}
impl fmt::Display for TraceBinding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.size == 1 {
            write!(
                f,
                "{}",
                self.name.as_ref().map(|n| n.as_str()).unwrap_or("?")
            )
        } else {
            write!(
                f,
                "{}[{}]",
                self.name.as_ref().map(|n| n.as_str()).unwrap_or("?"),
                self.size
            )
        }
    }
}

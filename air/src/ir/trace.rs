use air_parser::ast::{TraceColumnIndex, TraceSegmentId};

/// [TraceAccess] is like [SymbolAccess], but is used to describe an access to a specific trace column or columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
        Self {
            segment,
            column,
            row_offset,
        }
    }

    /// Creates a new [TraceAccess] with a new column index that is updated according to the
    /// provided offsets. All other data is left unchanged.
    pub fn clone_with_offsets(&self, offsets: &[Vec<usize>]) -> Self {
        Self {
            column: offsets[self.segment][self.column],
            ..*self
        }
    }
}

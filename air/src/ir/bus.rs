pub use air_parser::ast::BusType;
use air_parser::ast::{self, Identifier};
pub use mir::ir::BusOpKind;

use crate::NodeIndex;

/// An Air struct to represent a Bus definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bus {
    /// The [Identifier] of the bus
    pub name: ast::Identifier,
    /// The type of bus:
    pub bus_type: ast::BusType,
    /// The initial state of the bus
    pub first: BusBoundary,
    /// The final state of the bus
    pub last: BusBoundary,
    /// The operations (insertions and removals) of this bus
    pub bus_ops: Vec<BusOp>,
    /*// Alternatively, separate the insertions and removals into two vectors
    /// The insertions into the bus
    pub inserted: Vec<BusOp>,
    /// The removals from the bus
    pub removed: Vec<BusOp>,*/
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusBoundary {
    /// A reference to a public input table.
    PublicInputTable(PublicInputTableAccess),
    /// A reference to an empty bus
    Null,
}

/// Represents an access of a public input table, similar in nature to [TraceAccess].
///
/// It can only be bound to a [Bus]'s .first or .last boundary constraints.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicInputTableAccess {
    /// The name of the public input to bind
    pub table_name: Identifier,
    pub bus_name: Identifier,
    pub num_cols: usize,
}
impl PublicInputTableAccess {
    pub const fn new(table_name: Identifier, bus_name: Identifier, num_cols: usize) -> Self {
        Self {
            table_name,
            num_cols,
            bus_name,
        }
    }
}

/// An Air struct to represent a Bus definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusOp {
    pub columns: Vec<NodeIndex>,
    pub latch: NodeIndex,
    pub op_kind: BusOpKind,
}

impl BusOp {
    pub fn new(columns: Vec<NodeIndex>, latch: NodeIndex, op_kind: BusOpKind) -> Self {
        Self {
            columns,
            latch,
            op_kind,
        }
    }
}

impl Bus {
    pub fn new(
        name: ast::Identifier,
        bus_type: ast::BusType,
        first: BusBoundary,
        last: BusBoundary,
        bus_ops: Vec<BusOp>,
    ) -> Self {
        Self {
            name,
            bus_type,
            first,
            last,
            bus_ops,
        }
    }
}

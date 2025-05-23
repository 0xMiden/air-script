use air_parser::ast;
pub use air_parser::ast::BusType;
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
    pub first: NodeIndex,
    /// The final state of the bus
    pub last: NodeIndex,
    /// The operations (insertions and removals) of this bus
    pub bus_ops: Vec<BusOp>,
    /*// Alternatively, separate the insertions and removals into two vectors
    /// The insertions into the bus
    pub inserted: Vec<BusOp>,
    /// The removals from the bus
    pub removed: Vec<BusOp>,*/
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
        first: NodeIndex,
        last: NodeIndex,
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

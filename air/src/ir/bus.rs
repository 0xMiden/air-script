use air_parser::ast;
pub use air_parser::ast::BusType;

use crate::NodeIndex;

/// An Air struct to represent a Bus definition
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Bus {
    /// The [Identifier] of the bus
    pub name: ast::Identifier,
    /// The type of bus:
    pub bus_type: ast::BusType,
    /// The initial state of the bus
    pub first: NodeIndex,
    /// The final state of the bus
    pub last: NodeIndex,
}

impl Bus {
    pub fn new(
        name: ast::Identifier,
        bus_type: ast::BusType,
        first: NodeIndex,
        last: NodeIndex,
    ) -> Self {
        Self {
            name,
            bus_type,
            first,
            last,
        }
    }
}

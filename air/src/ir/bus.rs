use air_parser::ast;
pub use air_parser::ast::BusType;

use crate::NodeIndex;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Bus {
    pub name: ast::Identifier,
    pub bus_type: ast::BusType,
    pub first: NodeIndex,
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

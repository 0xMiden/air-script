use crate::graph::NodeIndex;

use super::*;

/// [Operation] defines the various node types represented
/// in the [AlgebraicGraph].
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Operation {
    /// Evaluates to a [Value]
    ///
    /// This is always a leaf node in the graph.
    Value(Value),
    /// Evaluates by addition over two operands (given as nodes in the graph)
    Add(NodeIndex, NodeIndex),
    /// Evaluates by subtraction over two operands (given as nodes in the graph)
    Sub(NodeIndex, NodeIndex),
    /// Evaluates by multiplication over two operands (given as nodes in the graph)
    Mul(NodeIndex, NodeIndex),
}
impl Operation {
    /// Corresponds to the binding power of this [Operation]
    ///
    /// Operations with a higher binding power are applied before
    /// operations with a lower binding power. Operations with equivalent
    /// precedence are evaluated left-to-right.
    pub fn precedence(&self) -> usize {
        match self {
            Self::Add(_, _) => 1,
            Self::Sub(_, _) => 2,
            Self::Mul(_, _) => 3,
            _ => 4,
        }
    }
}

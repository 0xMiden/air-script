use super::*;
use crate::graph::NodeIndex;

/// [Operation] defines the various node types represented
/// in the [AlgebraicGraph].
#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
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
            Self::Add(..) => 1,
            Self::Sub(..) => 2,
            Self::Mul(..) => 3,
            _ => 4,
        }
    }
}

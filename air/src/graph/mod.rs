use std::collections::BTreeMap;

use crate::ir::*;

/// A unique identifier for a node in an [AlgebraicGraph]
///
/// The raw value of this identifier is an index in the `nodes` vector
/// of the [AlgebraicGraph] struct.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeIndex(usize);
impl core::ops::Add<usize> for NodeIndex {
    type Output = NodeIndex;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}
impl core::ops::Add<usize> for &NodeIndex {
    type Output = NodeIndex;

    fn add(self, rhs: usize) -> Self::Output {
        NodeIndex(self.0 + rhs)
    }
}

impl From<usize> for NodeIndex {
    fn from(u: usize) -> Self {
        Self(u)
    }
}

impl From<NodeIndex> for usize {
    fn from(val: NodeIndex) -> usize {
        val.0
    }
}

/// A node in the [AlgebraicGraph]
#[derive(Debug, Clone)]
pub struct Node {
    /// The operation represented by this node
    op: Operation,
}
impl Node {
    /// Get the underlying [Operation] represented by this node
    #[inline]
    pub const fn op(&self) -> &Operation {
        &self.op
    }
}

/// The AlgebraicGraph is a directed acyclic graph used to represent integrity constraints. To
/// store it compactly, it is represented as a vector of nodes where each node references other
/// nodes by their index in the vector.
///
/// Within the graph, constraint expressions can overlap and share subgraphs, since new expressions
/// reuse matching existing nodes when they are added, rather than creating new nodes.
///
/// - Leaf nodes (with no outgoing edges) are constants or references to trace cells (i.e. column 0
///   in the current row or column 5 in the next row).
/// - Tip nodes with no incoming edges (no parent nodes) always represent constraints, although they
///   do not necessarily represent all constraints. There could be constraints which are also
///   subgraphs of other constraints.
#[derive(Default, Debug, Clone)]
pub struct AlgebraicGraph {
    /// All nodes in the graph.
    nodes: Vec<Node>,
}
impl AlgebraicGraph {
    /// Creates a new graph from a list of nodes.
    pub const fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    /// Returns the node with the specified index.
    pub fn node(&self, index: &NodeIndex) -> &Node {
        &self.nodes[index.0]
    }

    /// Returns the number of nodes in the graph.
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the degree of the subgraph which has the specified node as its tip.
    pub fn degree(&self, index: &NodeIndex) -> IntegrityConstraintDegree {
        let mut cycles = BTreeMap::default();
        let base = self.accumulate_degree(&mut cycles, index);

        if cycles.is_empty() {
            IntegrityConstraintDegree::new(base)
        } else {
            IntegrityConstraintDegree::with_cycles(base, cycles.values().copied().collect())
        }
    }

    /// TODO: docs
    pub fn node_details(
        &self,
        index: &NodeIndex,
        default_domain: ConstraintDomain,
    ) -> Result<(TraceSegmentId, ConstraintDomain), ConstraintError> {
        // recursively walk the subgraph and infer the trace segment and domain
        match self.node(index).op() {
            Operation::Value(value) => match value {
                Value::Constant(_) => Ok((DEFAULT_SEGMENT, default_domain)),
                Value::RandomValue(_) => Ok((AUX_SEGMENT, default_domain)),
                Value::PeriodicColumn(_) => {
                    assert!(
                        !default_domain.is_boundary(),
                        "unexpected access to periodic column in boundary constraint"
                    );
                    // the default domain for [IntegrityConstraints] is `EveryRow`
                    Ok((DEFAULT_SEGMENT, ConstraintDomain::EveryRow))
                },
                Value::PublicInput(_) => {
                    assert!(
                        !default_domain.is_integrity(),
                        "unexpected access to public input in integrity constraint"
                    );
                    Ok((DEFAULT_SEGMENT, default_domain))
                },
                Value::PublicInputTable(_) => {
                    assert!(
                        !default_domain.is_integrity(),
                        "unexpected access to public input table in integrity constraint"
                    );
                    Ok((DEFAULT_SEGMENT, default_domain))
                },
                Value::TraceAccess(trace_access) => {
                    let domain = if default_domain.is_boundary() {
                        assert_eq!(
                            trace_access.row_offset, 0,
                            "unexpected trace offset in boundary constraint"
                        );
                        default_domain
                    } else {
                        ConstraintDomain::from_offset(trace_access.row_offset)
                    };

                    Ok((trace_access.segment, domain))
                },
            },
            Operation::Add(lhs, rhs) | Operation::Sub(lhs, rhs) | Operation::Mul(lhs, rhs) => {
                let (lhs_segment, lhs_domain) = self.node_details(lhs, default_domain)?;
                let (rhs_segment, rhs_domain) = self.node_details(rhs, default_domain)?;

                let trace_segment = lhs_segment.max(rhs_segment);
                let domain = lhs_domain.merge(rhs_domain)?;

                Ok((trace_segment, domain))
            },
        }
    }

    /// Insert the operation and return its node index. If an identical node already exists, return
    /// that index instead.
    pub(crate) fn insert_node(&mut self, op: Operation) -> NodeIndex {
        self.nodes.iter().position(|n| *n.op() == op).map_or_else(
            || {
                // create a new node.
                let index = self.nodes.len();
                self.nodes.push(Node { op });
                NodeIndex(index)
            },
            |index| {
                // return the existing node's index.
                NodeIndex(index)
            },
        )
    }

    /// Recursively accumulates the base degree and the cycle lengths of the periodic columns.
    pub fn accumulate_degree(
        &self,
        cycles: &mut BTreeMap<QualifiedIdentifier, usize>,
        index: &NodeIndex,
    ) -> usize {
        // recursively walk the subgraph and compute the degree from the operation and child nodes
        match self.node(index).op() {
            Operation::Value(value) => match value {
                Value::Constant(_)
                | Value::PublicInput(_)
                | Value::PublicInputTable(_)
                | Value::RandomValue(_) => 0,
                Value::TraceAccess(_) => 1,
                Value::PeriodicColumn(pc) => {
                    cycles.insert(pc.name, pc.cycle);
                    0
                },
            },
            Operation::Add(lhs, rhs) => {
                let lhs_base = self.accumulate_degree(cycles, lhs);
                let rhs_base = self.accumulate_degree(cycles, rhs);
                lhs_base.max(rhs_base)
            },
            Operation::Sub(lhs, rhs) => {
                let lhs_base = self.accumulate_degree(cycles, lhs);
                let rhs_base = self.accumulate_degree(cycles, rhs);
                lhs_base.max(rhs_base)
            },
            Operation::Mul(lhs, rhs) => {
                let lhs_base = self.accumulate_degree(cycles, lhs);
                let rhs_base = self.accumulate_degree(cycles, rhs);
                lhs_base + rhs_base
            },
        }
    }
}

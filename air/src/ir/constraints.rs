use core::fmt;

use crate::graph::{AlgebraicGraph, NodeIndex};

use super::*;

#[derive(Debug, thiserror::Error)]
pub enum ConstraintError {
    #[error("cannot merge incompatible constraint domains ({0} and {1})")]
    IncompatibleConstraintDomains(ConstraintDomain, ConstraintDomain),
}

/// [Constraints] is the algebraic graph representation of all the constraints
/// in an [AirScript]. The graph contains all of the constraints, each of which
/// is a subgraph consisting of all the expressions involved in evaluating the constraint,
/// including constants, references to the trace, public inputs, random values, and
/// periodic columns.
///
/// Internally, this struct also holds a matrix for each constraint type (boundary,
/// integrity), where each row corresponds to a trace segment (in the same order)
/// and contains a vector of [ConstraintRoot] for all of the constraints of that type
/// to be applied to that trace segment.
///
/// For example, integrity constraints for the main execution trace, which has a trace segment
/// id of 0, will be specified by the vector of constraint roots found at index 0 of the
/// `integrity_constraints` matrix.
#[derive(Default, Debug)]
pub struct Constraints {
    /// Constraint roots for all boundary constraints against the execution trace, by trace segment,
    /// where boundary constraints are any constraints that apply to either the first or the last
    /// row of the trace.
    boundary_constraints: Vec<Vec<ConstraintRoot>>,
    /// Constraint roots for all integrity constraints against the execution trace, by trace segment,
    /// where integrity constraints are any constraints that apply to every row or every frame.
    integrity_constraints: Vec<Vec<ConstraintRoot>>,
    /// A directed acyclic graph which represents all of the constraints and their subexpressions.
    graph: AlgebraicGraph,
}
impl Constraints {
    /// Constructs a new [Constraints] graph from the given parts
    pub const fn new(
        graph: AlgebraicGraph,
        boundary_constraints: Vec<Vec<ConstraintRoot>>,
        integrity_constraints: Vec<Vec<ConstraintRoot>>,
    ) -> Self {
        Self {
            graph,
            boundary_constraints,
            integrity_constraints,
        }
    }

    /// Returns the number of boundary constraints applied against the specified trace segment.
    pub fn num_boundary_constraints(&self, trace_segment: TraceSegmentId) -> usize {
        if self.boundary_constraints.len() <= trace_segment {
            return 0;
        }

        self.boundary_constraints[trace_segment].len()
    }

    /// Returns the set of boundary constraints for the given trace segment.
    ///
    /// Each boundary constraint is represented by a [ConstraintRoot] which is
    /// the root of the subgraph representing the constraint within the [AlgebraicGraph]
    pub fn boundary_constraints(&self, trace_segment: TraceSegmentId) -> &[ConstraintRoot] {
        if self.boundary_constraints.len() <= trace_segment {
            return &[];
        }

        &self.boundary_constraints[trace_segment]
    }

    /// Returns the number of integrity constraints applied against the specified trace segment.
    pub fn num_integrity_constraints(&self, trace_segment: TraceSegmentId) -> usize {
        if self.integrity_constraints.len() <= trace_segment {
            return 0;
        }

        self.integrity_constraints[trace_segment].len()
    }

    /// Returns a vector of the degrees of the integrity constraints for the specified trace segment.
    pub fn integrity_constraint_degrees(
        &self,
        trace_segment: TraceSegmentId,
    ) -> Vec<IntegrityConstraintDegree> {
        if self.integrity_constraints.len() <= trace_segment {
            return vec![];
        }

        self.integrity_constraints[trace_segment]
            .iter()
            .map(|entry_index| self.graph.degree(entry_index.node_index()))
            .collect()
    }

    /// Returns the set of integrity constraints for the given trace segment.
    ///
    /// Each integrity constraint is represented by a [ConstraintRoot] which is
    /// the root of the subgraph representing the constraint within the [AlgebraicGraph]
    pub fn integrity_constraints(&self, trace_segment: TraceSegmentId) -> &[ConstraintRoot] {
        if self.integrity_constraints.len() <= trace_segment {
            return &[];
        }

        &self.integrity_constraints[trace_segment]
    }

    /// Inserts a new constraint against `trace_segment`, using the provided `root` and `domain`
    pub fn insert_constraint(
        &mut self,
        trace_segment: TraceSegmentId,
        root: NodeIndex,
        domain: ConstraintDomain,
    ) {
        let root = ConstraintRoot::new(root, domain);
        if domain.is_boundary() {
            if self.boundary_constraints.len() <= trace_segment {
                self.boundary_constraints.resize(trace_segment + 1, vec![]);
            }
            self.boundary_constraints[trace_segment].push(root);
        } else {
            if self.integrity_constraints.len() <= trace_segment {
                self.integrity_constraints.resize(trace_segment + 1, vec![]);
            }
            self.integrity_constraints[trace_segment].push(root);
        }
    }

    /// Returns the underlying [AlgebraicGraph] representing all constraints and their sub-expressions.
    #[inline]
    pub const fn graph(&self) -> &AlgebraicGraph {
        &self.graph
    }

    /// Returns a mutable reference to the underlying [AlgebraicGraph] representing all constraints and their sub-expressions.
    #[inline]
    pub fn graph_mut(&mut self) -> &mut AlgebraicGraph {
        &mut self.graph
    }
}

/// A [ConstraintRoot] represents the entry node of a subgraph within the [AlgebraicGraph]
/// representing a constraint. It also contains the [ConstraintDomain] for the constraint, which is
/// the domain against which the constraint should be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintRoot {
    index: NodeIndex,
    domain: ConstraintDomain,
}
impl ConstraintRoot {
    /// Creates a new [ConstraintRoot] with the specified entry index and row offset.
    pub const fn new(index: NodeIndex, domain: ConstraintDomain) -> Self {
        Self { index, domain }
    }

    /// Returns the index of the entry node of the subgraph representing the constraint.
    pub const fn node_index(&self) -> &NodeIndex {
        &self.index
    }

    /// Returns the [ConstraintDomain] for this constraint, which specifies the rows against which
    /// the constraint should be applied.
    pub const fn domain(&self) -> ConstraintDomain {
        self.domain
    }
}

/// [ConstraintDomain] corresponds to the domain over which a constraint is applied.
///
/// See the docs on each variant for more details.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintDomain {
    /// For boundary constraints which apply to the first row
    FirstRow,
    /// For boundary constraints which apply to the last row
    LastRow,
    /// For constraints which apply to every row of the trace
    ///
    /// This is used for validity constraints
    EveryRow,
    /// For constraints which apply across multiple rows at once.
    ///
    /// A "frame" is a window over rows in the trace, i.e. a constraint
    /// over a frame of size 2 is a constraint that observes 2 rows at
    /// a time, at each step of the trace, e.g. current and next rows.
    /// Such a constraint verifies that certain properties hold in the
    /// transition between every pair of rows.
    ///
    /// This is used for transition constraints.
    EveryFrame(usize),
}
impl ConstraintDomain {
    /// Returns true if this domain is a boundary domain (e.g. first or last)
    pub fn is_boundary(&self) -> bool {
        matches!(self, Self::FirstRow | Self::LastRow)
    }

    /// Returns true if this domain is an integrity constraint domain.
    pub fn is_integrity(&self) -> bool {
        matches!(self, Self::EveryRow | Self::EveryFrame(_))
    }

    /// Returns a [ConstraintDomain] corresponding to the given row offset.
    ///
    /// * `offset == 0` corresponds to every row
    /// * `offset > 0` corresponds to a frame size of `offset + 1`
    pub fn from_offset(offset: usize) -> Self {
        if offset == 0 {
            Self::EveryRow
        } else {
            Self::EveryFrame(offset + 1)
        }
    }

    /// Combines two compatible [ConstraintDomain]s into a single [ConstraintDomain]
    /// that represents the maximum of the two.
    ///
    /// For example, if one domain is [ConstraintDomain::EveryFrame(2)] and the other
    /// is [ConstraintDomain::EveryFrame(3)], then the result will be [ConstraintDomain::EveryFrame(3)].
    ///
    /// NOTE: Domains for boundary constraints (FirstRow and LastRow) cannot be merged with other domains.
    pub fn merge(self, other: Self) -> Result<Self, ConstraintError> {
        if self == other {
            return Ok(other);
        }

        match (self, other) {
            (Self::EveryFrame(a), Self::EveryRow) => Ok(Self::EveryFrame(a)),
            (Self::EveryRow, Self::EveryFrame(b)) => Ok(Self::EveryFrame(b)),
            (Self::EveryFrame(a), Self::EveryFrame(b)) => Ok(Self::EveryFrame(a.max(b))),
            _ => Err(ConstraintError::IncompatibleConstraintDomains(self, other)),
        }
    }
}
impl From<Boundary> for ConstraintDomain {
    fn from(boundary: Boundary) -> Self {
        match boundary {
            Boundary::First => Self::FirstRow,
            Boundary::Last => Self::LastRow,
        }
    }
}
impl fmt::Display for ConstraintDomain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::FirstRow => write!(f, "the first row"),
            Self::LastRow => write!(f, "the last row"),
            Self::EveryRow => write!(f, "every row"),
            Self::EveryFrame(size) => {
                write!(f, "every frame of {size} consecutive rows")
            }
        }
    }
}

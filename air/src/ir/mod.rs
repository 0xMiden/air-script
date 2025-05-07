mod bus;
mod constraints;
mod degree;
mod operation;
mod trace;
mod value;

pub use self::bus::{Bus, BusType};
pub use self::constraints::{ConstraintDomain, ConstraintError, ConstraintRoot, Constraints};
pub use self::degree::IntegrityConstraintDegree;
pub use self::operation::Operation;
pub use self::trace::TraceAccess;
pub use self::value::{PeriodicColumnAccess, PublicInputAccess, PublicInputTableAccess, Value};

pub use air_parser::{
    ast::{
        AccessType, Boundary, Identifier, PeriodicColumn, PublicInput, QualifiedIdentifier,
        TraceSegmentId,
    },
    Symbol,
};

/// The default segment against which a constraint is applied is the main trace segment.
pub const DEFAULT_SEGMENT: TraceSegmentId = 0;
/// The auxiliary trace segment.
pub const AUX_SEGMENT: TraceSegmentId = 1;
/// The offset of the "current" row during constraint evaluation.
pub const CURRENT_ROW: usize = 0;
/// The minimum cycle length of a periodic column
pub const MIN_CYCLE_LENGTH: usize = 2;

use std::collections::BTreeMap;

use miden_diagnostics::{SourceSpan, Spanned};

use crate::graph::AlgebraicGraph;

/// The intermediate representation of a complete AirScript program
///
/// This structure is produced from an [air_parser::ast::Program] that has
/// been through semantic analysis, constant propagation, and inlining. It
/// is equivalent to an [air_parser::ast::Program], except that it has been
/// translated into an algebraic graph representation, on which further analysis,
/// optimization, and code generation are performed.
#[derive(Debug, Spanned)]
pub struct Air {
    /// The name of the [air_parser::ast::Program] from which this IR was derived
    #[span]
    pub name: Identifier,
    /// The widths (number of columns) of each segment of the trace, in segment order (i.e. the
    /// index in this vector matches the index of the segment in the program).
    pub trace_segment_widths: Vec<u16>,
    /// The periodic columns referenced by this program.
    ///
    /// These are taken straight from the [air_parser::ast::Program] without modification.
    pub periodic_columns: BTreeMap<QualifiedIdentifier, PeriodicColumn>,
    /// The public inputs referenced by this program.
    ///
    /// These are taken straight from the [air_parser::ast::Program] without modification.
    pub public_inputs: BTreeMap<Identifier, PublicInput>,
    /// The total number of elements in the random values array
    pub num_random_values: u16,
    /// The constraints enforced by this program, in their algebraic graph representation.
    pub constraints: Constraints,
    /// The buses referenced by this program.
    ///
    /// Only their name, type, and the first and last boundary constraints are stored here.
    pub buses: BTreeMap<Identifier, Bus>,
}
impl Default for Air {
    fn default() -> Self {
        Self::new(Identifier::new(
            SourceSpan::UNKNOWN,
            Symbol::intern("unnamed"),
        ))
    }
}
impl Air {
    /// Create a new, empty [Air] container
    ///
    /// An empty [Air] is meaningless until it has been populated with
    /// constraints and associated metadata. This is typically done by converting
    /// an [air_parser::ast::Program] to this struct using the [crate::passes::AstToAir]
    /// translation pass.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            trace_segment_widths: vec![],
            periodic_columns: Default::default(),
            public_inputs: Default::default(),
            num_random_values: 0,
            constraints: Default::default(),
            buses: Default::default(),
        }
    }

    /// Returns the name of the [air_parser::ast::Program] this [Air] was derived from, as a `str`
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn public_inputs(&self) -> impl Iterator<Item = &PublicInput> + '_ {
        self.public_inputs.values()
    }

    pub fn periodic_columns(&self) -> impl Iterator<Item = &PeriodicColumn> + '_ {
        self.periodic_columns.values()
    }

    /// Return the number of boundary constraints
    pub fn num_boundary_constraints(&self, trace_segment: TraceSegmentId) -> usize {
        self.constraints.num_boundary_constraints(trace_segment)
    }

    /// Return the set of [ConstraintRoot] corresponding to the boundary constraints
    pub fn boundary_constraints(&self, trace_segment: TraceSegmentId) -> &[ConstraintRoot] {
        self.constraints.boundary_constraints(trace_segment)
    }

    /// Return the set of [ConstraintRoot] corresponding to the integrity constraints
    pub fn integrity_constraints(&self, trace_segment: TraceSegmentId) -> &[ConstraintRoot] {
        self.constraints.integrity_constraints(trace_segment)
    }

    /// Return the set of [IntegrityConstraintDegree] corresponding to each integrity constraint
    pub fn integrity_constraint_degrees(
        &self,
        trace_segment: TraceSegmentId,
    ) -> Vec<IntegrityConstraintDegree> {
        self.constraints.integrity_constraint_degrees(trace_segment)
    }

    /// Return an [Iterator] over the validity constraints for the given trace segment
    pub fn validity_constraints(
        &self,
        trace_segment: TraceSegmentId,
    ) -> impl Iterator<Item = &ConstraintRoot> + '_ {
        self.constraints
            .integrity_constraints(trace_segment)
            .iter()
            .filter(|constraint| matches!(constraint.domain(), ConstraintDomain::EveryRow))
    }

    /// Return an [Iterator] over the transition constraints for the given trace segment
    pub fn transition_constraints(
        &self,
        trace_segment: TraceSegmentId,
    ) -> impl Iterator<Item = &ConstraintRoot> + '_ {
        self.constraints
            .integrity_constraints(trace_segment)
            .iter()
            .filter(|constraint| matches!(constraint.domain(), ConstraintDomain::EveryFrame(_)))
    }

    /// Return a reference to the raw [AlgebraicGraph] corresponding to the constraints
    #[inline]
    pub fn constraint_graph(&self) -> &AlgebraicGraph {
        self.constraints.graph()
    }

    /// Return a mutable reference to the raw [AlgebraicGraph] corresponding to the constraints
    #[inline]
    pub fn constraint_graph_mut(&mut self) -> &mut AlgebraicGraph {
        self.constraints.graph_mut()
    }
}

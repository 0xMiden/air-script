use std::collections::BTreeMap;

use air_parser::ast::TraceSegment;
pub use air_parser::{
    Symbol,
    ast::{Identifier, PeriodicColumn, PublicInput, QualifiedIdentifier},
};
use miden_diagnostics::{SourceSpan, Spanned};

use super::Graph;

/// The intermediate representation of a complete AirScript program
///
/// This structure is produced from an [air_parser::ast::Program] that has
/// been through semantic analysis and constant propagation.
/// It is equivalent to an [air_parser::ast::Program], except that it has been
/// translated into an algebraic graph representation, on which further analysis,
/// optimization, and code generation are performed.
#[derive(Debug, Spanned, PartialEq, Eq)]
pub struct Mir {
    /// The name of the [air_parser::ast::Program] from which this MIR was derived
    #[span]
    pub name: Identifier,
    /// The trace columns referenced by this program.
    ///
    /// These are taken straight from the [air_parser::ast::Program] without modification.
    pub trace_columns: Vec<TraceSegment>,
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
    /// The constraints of the program, represented as MIR Nodes
    graph: Graph,
}
impl Default for Mir {
    fn default() -> Self {
        Self::new(Identifier::new(SourceSpan::UNKNOWN, Symbol::intern("unnamed")))
    }
}
impl Mir {
    /// Create a new, empty [Mir] container
    ///
    /// An empty [Mir] is meaningless until it has been populated with
    /// constraints and associated metadata. This is typically done by converting
    /// an [air_parser::ast::Program] to this struct using the [crate::passes::AstToMir]
    /// translation pass.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            trace_columns: vec![],
            periodic_columns: Default::default(),
            public_inputs: Default::default(),
            num_random_values: 0,
            graph: Default::default(),
        }
    }

    /// Returns the name of the [air_parser::ast::Program] this [Mir] was derived from, as a `str`
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Return a reference to the raw AlgebraicGraph corresponding to the constraints
    #[inline]
    pub fn constraint_graph(&self) -> &Graph {
        &self.graph
    }

    /// Return a mutable reference to the raw AlgebraicGraph corresponding to the constraints
    #[inline]
    pub fn constraint_graph_mut(&mut self) -> &mut Graph {
        &mut self.graph
    }
}

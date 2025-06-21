use air_pass::Pass;
use miden_diagnostics::DiagnosticsHandler;

use super::visitor::Visitor;
use crate::{
    ir::{Link, Mir, Node},
    CompileError,
};

/// TODO MIR:
/// If needed, implement value numbering pass on MIR
/// See https://en.wikipedia.org/wiki/Value_numbering
///   
pub struct ValueNumbering<'a> {
    #[allow(unused)]
    diagnostics: &'a DiagnosticsHandler,
    work_stack: Vec<Link<Node>>,
}

impl Pass for ValueNumbering<'_> {
    type Input<'a> = Mir;
    type Output<'a> = Mir;
    type Error = CompileError;

    fn run<'a>(&mut self, mut ir: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        Visitor::run(self, ir.constraint_graph_mut())?;
        Ok(ir)
    }
}

impl<'a> ValueNumbering<'a> {
    #[allow(unused)]
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self {
            diagnostics,
            work_stack: vec![],
        }
    }
}

impl Visitor for ValueNumbering<'_> {
    fn work_stack(&mut self) -> &mut Vec<Link<Node>> {
        &mut self.work_stack
    }
    fn root_nodes_to_visit(
        &self,
        graph: &crate::ir::Graph,
    ) -> Vec<crate::ir::Link<crate::ir::Node>> {
        let boundary_constraints_roots_ref = graph.boundary_constraints_roots.borrow();
        let integrity_constraints_roots_ref = graph.integrity_constraints_roots.borrow();
        let combined_roots = boundary_constraints_roots_ref
            .clone()
            .into_iter()
            .map(|bc| bc.as_node())
            .chain(
                integrity_constraints_roots_ref
                    .clone()
                    .into_iter()
                    .map(|ic| ic.as_node()),
            );
        combined_roots.collect()
    }
}

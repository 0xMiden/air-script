use std::ops::Deref;

use air_parser::ast::{AccessType, BusType};
use air_pass::Pass;
use miden_diagnostics::{DiagnosticsHandler, SourceSpan, Spanned};

use super::{duplicate_node, visitor::Visitor};
use crate::{
    ir::{Accessor, Graph, Link, Mir, MirValue, Mul, Node, Op, SpannedMirValue, Sub, Value},
    CompileError,
};

/// TODO MIR:
/// If needed, implement bus operation expand pass on MIR
/// See https://github.com/0xPolygonMiden/air-script/issues/183
///   
pub struct BusOpExpand<'a> {
    #[allow(unused)]
    diagnostics: &'a DiagnosticsHandler,
    work_stack: Vec<Link<Node>>,
}

impl Pass for BusOpExpand<'_> {
    type Input<'a> = Mir;
    type Output<'a> = Mir;
    type Error = CompileError;

    fn run<'a>(&mut self, mut ir: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        let graph = ir.constraint_graph_mut();

        let buses = graph.buses.clone();

        for (ident, bus) in buses {
            let bus_type = bus.borrow().bus_type.clone();
            let columns = bus.borrow().columns.clone(); // columns are the bus_operations (add or remove of a Vec of arguments)
            let latches = bus.borrow().latches.clone(); // latches are the selectors

            let bus_access = Value::create(SpannedMirValue {
                span: bus.borrow().span().clone(),
                value: MirValue::BusAccess(bus.clone()),
            });
            let bus_access_with_offset = Accessor::create(
                duplicate_node(bus_access.clone(), &mut Default::default()),
                AccessType::Default,
                1,
                bus.borrow().span().clone(),
            );

            match bus_type {
                BusType::Unit => {
                    // Example:
                    // p.add(a, b) when s
                    // p.rem(c, d) when (1 - s)
                    // => p' * (( A0 + A1 c + A2 d ) ( 1 - s ) + s) = p * ( A0 + A1 a + A2 b ) s + 1 - s

                    // p' * ( columns removed combined with alphas ) = p * ( columns added combined with alphas )

                    let mut p_factor = Value::create(SpannedMirValue {
                        span: SourceSpan::default(),
                        value: crate::ir::MirValue::Constant(crate::ir::ConstantValue::Felt(1)),
                    });
                    let mut p_prime_factor = Value::create(SpannedMirValue {
                        span: SourceSpan::default(),
                        value: MirValue::Constant(crate::ir::ConstantValue::Felt(1)),
                    });

                    for (column, latch) in columns.iter().zip(latches.iter()) {

                        let bus_op = column.as_bus_op().unwrap();
                        let bus_op_kind = bus_op.kind.clone();
                        let bus_op_args = bus_op.args.clone();

                        // 1. Combine args with alphas
                        // 2. multiply with latch
                        // 3. add inverse of latch
                        // 4. add to p_factor or p_prime_factor (depending on bus_op_kind: add: p, rem: p_prime)

                    }

                    let p_prod = Mul::create(p_factor, bus_access, SourceSpan::default());
                    let p_prime_prod = Mul::create(
                        p_prime_factor,
                        bus_access_with_offset,
                        SourceSpan::default(),
                    );
                }
                BusType::Mult => {
                    //
                    for (columns, latches) in columns.iter().zip(latches.iter()) {}
                }
            }
        }

        Ok(ir)
    }
}

impl<'a> BusOpExpand<'a> {
    #[allow(unused)]
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self {
            diagnostics,
            work_stack: vec![],
        }
    }
}

impl Visitor for BusOpExpand<'_> {
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

    fn visit_node(&mut self, graph: &mut Graph, node: Link<Node>) -> Result<(), CompileError> {
        let updated_op: Result<Option<Link<Op>>, CompileError> = match node.borrow().deref() {
            Node::BusOp(bus_op) => {
                let bus_op_link: Link<Op> = bus_op.clone().into();
                let mut updated_node = None;

                {
                    // safe to unwrap because we just dispatched on it
                    let bus_op_ref = bus_op_link.as_bus_op().unwrap();
                    let bus = bus_op_ref.bus.clone();
                    let bus_kind = bus.borrow().bus_type.clone();
                    let bus_operator = bus_op_ref.kind.clone();
                    let args = bus_op_ref.args.clone();
                }

                Ok(updated_node)
            }
            _ => Ok(None),
        };

        // We update the node if needed
        if let Some(updated_op) = updated_op? {
            node.as_op().unwrap().set(&updated_op);
        }

        Ok(())
    }
}

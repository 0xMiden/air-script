use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use air_parser::ast::{Identifier, NamespacedIdentifier};
use miden_diagnostics::{SourceSpan, Span};
use pretty_assertions::assert_eq;

use crate::{ir::*, passes::Visitor, CompileError};

pub fn strip_spans(mir: &mut Mir) {
    let graph = mir.constraint_graph_mut();
    let mut visitor = StripSpansVisitor::default();
    match visitor.run(graph) {
        Ok(_) => {}
        Err(e) => {
            panic!("Error stripping spans: {:?}", e);
        }
    }
}

#[derive(Default)]
pub struct StripSpansVisitor {
    _done: BTreeMap<usize, bool>,
    work_stack: Vec<Link<Node>>,
}

pub fn extract_roots(
    graph: &Graph,
    include_boundary: bool,
    include_integrity: bool,
    include_bus: bool,
    include_func: bool,
    include_eval: bool,
) -> Vec<Link<Node>> {
    let mut nodes = Vec::new();
    if include_boundary {
        let boundary_ref = graph.boundary_constraints_roots.borrow();
        let boundary = boundary_ref.iter().map(|n| n.as_node());
        nodes.extend(boundary);
    }
    if include_integrity {
        let integrity_ref = graph.integrity_constraints_roots.borrow();
        let integrity = integrity_ref.iter().map(|n| n.as_node());
        nodes.extend(integrity);
    }
    if include_bus {
        let buses = graph.get_bus_nodes();
        let bus_columns = buses
            .iter()
            .flat_map(|b| b.borrow().columns.clone())
            .map(|n| n.as_node());
        let bus_latches = buses
            .iter()
            .flat_map(|b| b.borrow().latches.clone())
            .map(|n| n.as_node());
        nodes.extend(bus_columns);
        nodes.extend(bus_latches);
    }
    if include_func {
        let funcs = graph.get_function_nodes().into_iter().map(|n| n.as_node());
        nodes.extend(funcs);
    }
    if include_eval {
        let evals = graph.get_evaluator_nodes().into_iter().map(|n| n.as_node());
        nodes.extend(evals);
    }
    nodes
}

pub fn extract_all_roots(graph: &Graph) -> Vec<Link<Node>> {
    extract_roots(graph, true, true, true, true, true)
}

pub fn extract_boundary_roots(graph: &Graph) -> Vec<Link<Node>> {
    extract_roots(graph, true, false, false, false, false)
}

pub fn extract_integrity_roots(graph: &Graph) -> Vec<Link<Node>> {
    extract_roots(graph, false, true, false, false, false)
}

pub fn extract_bus_roots(graph: &Graph) -> Vec<Link<Node>> {
    extract_roots(graph, false, false, true, false, false)
}

pub fn extract_function_roots(graph: &Graph) -> Vec<Link<Node>> {
    extract_roots(graph, false, false, false, true, false)
}

pub fn extract_evaluator_roots(graph: &Graph) -> Vec<Link<Node>> {
    extract_roots(graph, false, false, false, false, true)
}

impl Visitor for StripSpansVisitor {
    fn work_stack(&mut self) -> &mut Vec<Link<Node>> {
        &mut self.work_stack
    }
    fn root_nodes_to_visit(&self, graph: &Graph) -> Vec<Link<Node>> {
        extract_all_roots(graph)
    }

    fn visit_function(
        &mut self,
        _graph: &mut Graph,
        function: Link<Root>,
    ) -> Result<(), CompileError> {
        let mut function = function.as_function_mut().unwrap();
        function.span = Default::default();
        Ok(())
    }

    fn visit_evaluator(
        &mut self,
        _graph: &mut Graph,
        evaluator: Link<Root>,
    ) -> Result<(), CompileError> {
        let mut evaluator = evaluator.as_evaluator_mut().unwrap();
        evaluator.span = Default::default();
        Ok(())
    }

    fn visit_enf(&mut self, _graph: &mut Graph, enf: Link<Op>) -> Result<(), CompileError> {
        let mut enf = enf.as_enf_mut().unwrap();
        enf.span = Default::default();
        Ok(())
    }

    fn visit_boundary(
        &mut self,
        _graph: &mut Graph,
        boundary: Link<Op>,
    ) -> Result<(), CompileError> {
        let mut boundary = boundary.as_boundary_mut().unwrap();
        boundary.span = Default::default();
        Ok(())
    }

    fn visit_add(&mut self, _graph: &mut Graph, add: Link<Op>) -> Result<(), CompileError> {
        let mut add = add.as_add_mut().unwrap();
        add.span = Default::default();
        Ok(())
    }

    fn visit_sub(&mut self, _graph: &mut Graph, sub: Link<Op>) -> Result<(), CompileError> {
        let mut sub = sub.as_sub_mut().unwrap();
        sub.span = Default::default();
        Ok(())
    }

    fn visit_mul(&mut self, _graph: &mut Graph, mul: Link<Op>) -> Result<(), CompileError> {
        let mut mul = mul.as_mul_mut().unwrap();
        mul.span = Default::default();
        Ok(())
    }

    fn visit_exp(&mut self, _graph: &mut Graph, exp: Link<Op>) -> Result<(), CompileError> {
        let mut exp = exp.as_exp_mut().unwrap();
        exp.span = Default::default();
        Ok(())
    }

    fn visit_if(&mut self, _graph: &mut Graph, if_node: Link<Op>) -> Result<(), CompileError> {
        let mut if_node = if_node.as_if_mut().unwrap();
        if_node.span = Default::default();
        Ok(())
    }

    fn visit_for(&mut self, _graph: &mut Graph, for_node: Link<Op>) -> Result<(), CompileError> {
        let mut for_node = for_node.as_for_mut().unwrap();
        for_node.span = Default::default();
        Ok(())
    }

    fn visit_call(&mut self, _graph: &mut Graph, call: Link<Op>) -> Result<(), CompileError> {
        let mut call = call.as_call_mut().unwrap();
        call.span = Default::default();
        Ok(())
    }

    fn visit_fold(&mut self, _graph: &mut Graph, fold: Link<Op>) -> Result<(), CompileError> {
        let mut fold = fold.as_fold_mut().unwrap();
        fold.span = Default::default();
        Ok(())
    }

    fn visit_vector(&mut self, _graph: &mut Graph, vector: Link<Op>) -> Result<(), CompileError> {
        let mut vector = vector.as_vector_mut().unwrap();
        vector.span = Default::default();
        Ok(())
    }

    fn visit_matrix(&mut self, _graph: &mut Graph, matrix: Link<Op>) -> Result<(), CompileError> {
        let mut matrix = matrix.as_matrix_mut().unwrap();
        matrix.span = Default::default();
        Ok(())
    }

    fn visit_accessor(
        &mut self,
        _graph: &mut Graph,
        accessor: Link<Op>,
    ) -> Result<(), CompileError> {
        let mut accessor = accessor.as_accessor_mut().unwrap();
        accessor.span = Default::default();
        Ok(())
    }

    fn visit_bus_op(&mut self, _graph: &mut Graph, bus_op: Link<Op>) -> Result<(), CompileError> {
        let mut bus_op = bus_op.as_bus_op_mut().unwrap();
        bus_op.span = Default::default();
        Ok(())
    }

    fn visit_parameter(
        &mut self,
        _graph: &mut Graph,
        parameter: Link<Op>,
    ) -> Result<(), CompileError> {
        let mut parameter = parameter.as_parameter_mut().unwrap();
        parameter.span = Default::default();
        Ok(())
    }

    fn visit_value(&mut self, _graph: &mut Graph, value: Link<Op>) -> Result<(), CompileError> {
        let mut value = value.as_value_mut().unwrap();
        value.value.span = Default::default();
        match &mut value.value.value {
            MirValue::Constant(_) => {}
            MirValue::TraceAccess(_) => {}
            MirValue::PeriodicColumn(v) => {
                v.name.module.0 = Span::new(SourceSpan::default(), v.name.module.0.item);
                match v.name.item {
                    NamespacedIdentifier::Function(f) => {
                        v.name.item = NamespacedIdentifier::Function(Identifier::new(
                            SourceSpan::default(),
                            f.0.item,
                        ));
                    }
                    NamespacedIdentifier::Binding(b) => {
                        v.name.item = NamespacedIdentifier::Binding(Identifier::new(
                            SourceSpan::default(),
                            b.0.item,
                        ));
                    }
                };
            }
            MirValue::PublicInput(v) => {
                v.name.0 = Span::new(SourceSpan::default(), v.name.0.item);
            }
            // TODO: Will be used when handling variable-length public inputs
            MirValue::PublicInputBinding(v) => {
                v.table_name.0 = Span::new(SourceSpan::default(), v.table_name.0.item);
            }
            MirValue::RandomValue(_) => {}
            MirValue::TraceAccessBinding(_) => {}
            MirValue::BusAccess(_) => {}
            MirValue::Null => {}
        }
        Ok(())
    }
}

pub fn hash<T: Hash>(val: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    val.hash(&mut hasher);
    hasher.finish()
}

fn extract_and_compare_mir(
    lhs: &mut Mir,
    rhs: &mut Mir,
    extract: impl Fn(&Graph) -> Vec<Link<Node>>,
) -> bool {
    strip_spans(lhs);
    strip_spans(rhs);
    let lhs = extract(lhs.constraint_graph());
    let rhs = extract(rhs.constraint_graph());
    hash(&lhs) == hash(&rhs)
}

pub fn compare_mir(lhs: &mut Mir, rhs: &mut Mir) -> bool {
    extract_and_compare_mir(lhs, rhs, extract_all_roots)
}

pub fn compare_boundary(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_compare_mir(lhs, rhs, extract_boundary_roots);
}

pub fn compare_integrity(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_compare_mir(lhs, rhs, extract_integrity_roots);
}

pub fn compare_bus(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_compare_mir(lhs, rhs, extract_bus_roots);
}

pub fn compare_function(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_compare_mir(lhs, rhs, extract_function_roots);
}

pub fn compare_evaluator(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_compare_mir(lhs, rhs, extract_evaluator_roots);
}

fn extract_and_assert_mir_eq(
    lhs: &mut Mir,
    rhs: &mut Mir,
    extract: impl Fn(&Graph) -> Vec<Link<Node>>,
) {
    strip_spans(lhs);
    strip_spans(rhs);
    let lhs = extract(lhs.constraint_graph());
    let rhs = extract(rhs.constraint_graph());
    assert_eq!(lhs, rhs);
}

pub fn assert_mir_eq(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_assert_mir_eq(lhs, rhs, extract_all_roots);
}

pub fn assert_boundary_eq(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_assert_mir_eq(lhs, rhs, extract_boundary_roots);
}

pub fn assert_integrity_eq(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_assert_mir_eq(lhs, rhs, extract_integrity_roots);
}

pub fn assert_bus_eq(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_assert_mir_eq(lhs, rhs, extract_bus_roots);
}

pub fn assert_function_eq(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_assert_mir_eq(lhs, rhs, extract_function_roots);
}

pub fn assert_evaluator_eq(lhs: &mut Mir, rhs: &mut Mir) {
    extract_and_assert_mir_eq(lhs, rhs, extract_evaluator_roots);
}

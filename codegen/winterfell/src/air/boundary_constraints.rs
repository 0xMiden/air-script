use core::panic;

use air_ir::{Air, AlgebraicGraph, ConstraintDomain, NodeIndex, Operation, TraceAccess, Value};

use crate::air::call_bus_boundary_varlen_pubinput;

use super::{Codegen, ElemType, Impl};

// HELPERS TO GENERATE THE WINTERFELL BOUNDARY CONSTRAINT METHODS
// ================================================================================================

/// Adds an implementation of the "get_assertions" method to the referenced Air implementation
/// based on the data in the provided IR.
/// TODO: add result types to these functions.
pub(super) fn add_fn_get_assertions(impl_ref: &mut Impl, ir: &Air) {
    // define the function
    let get_assertions = impl_ref
        .new_fn("get_assertions")
        .arg_ref_self()
        .ret("Vec<Assertion<Felt>>");

    // add the boundary constraints
    add_main_trace_assertions(get_assertions, ir);

    // return the result
    get_assertions.line("result");
}

/// Adds an implementation of the "get_aux_assertions" method to the referenced Air implementation
/// based on the data in the provided IR.
pub(super) fn add_fn_get_aux_assertions(impl_ref: &mut Impl, ir: &Air) {
    // define the function
    let get_aux_assertions = impl_ref
        .new_fn("get_aux_assertions")
        .generic("E: FieldElement<BaseField = Felt>")
        .arg_ref_self()
        .arg("aux_rand_elements", "&AuxRandElements<E>")
        .ret("Vec<Assertion<E>>");

    // add the boundary constraints
    add_aux_trace_assertions(get_aux_assertions, ir);

    // return the result
    get_aux_assertions.line("result");
}

/// Declares a result vector and adds assertions for boundary constraints to it for the main
/// trace segment
fn add_main_trace_assertions(func_body: &mut codegen::Function, ir: &Air) {
    let elem_type = ElemType::Base;
    let main_trace_segment = 0;

    // declare the result vector to be returned.
    func_body.line("let mut result = Vec::new();");

    // add the main boundary constraints
    for constraint in ir.boundary_constraints(main_trace_segment) {
        let (trace_access, expr_root) =
            split_boundary_constraint(ir.constraint_graph(), constraint.node_index());
        debug_assert_eq!(trace_access.segment, main_trace_segment);

        let expr_root_string = expr_root.to_string(ir, elem_type, main_trace_segment);

        let assertion = format!(
            "result.push(Assertion::single({}, {}, {}));",
            trace_access.column,
            domain_to_str(constraint.domain()),
            expr_root_string
        );

        func_body.line(assertion);
    }
}

/// Declares a result vector and adds assertions for boundary constraints to it for the aux
/// trace segment (used for buses boundary constraints for variable length public inputs)
fn add_aux_trace_assertions(func_body: &mut codegen::Function, ir: &Air) {
    let elem_type = ElemType::Ext;
    let aux_trace_segment = 1;

    // declare the result vector to be returned.
    func_body.line("let mut result = Vec::new();");

    // add the boundary constraints that have already be expanded in the algebraic graph
    // (currently, empty buses constraints)
    for constraint in ir.boundary_constraints(aux_trace_segment) {
        let (trace_access, expr_root) =
            split_boundary_constraint(ir.constraint_graph(), constraint.node_index());
        debug_assert_eq!(trace_access.segment, aux_trace_segment);

        let expr_root_string = expr_root.to_string(ir, elem_type, aux_trace_segment);

        let assertion = format!(
            "result.push(Assertion::single({}, {}, {}));",
            trace_access.column,
            domain_to_str(constraint.domain()),
            expr_root_string
        );

        func_body.line(assertion);
    }

    let domains = [ConstraintDomain::FirstRow, ConstraintDomain::LastRow];

    for domain in &domains {
        for (index, bus) in ir.buses.values().enumerate() {
            let bus_boundary = match domain {
                ConstraintDomain::FirstRow => &bus.first,
                ConstraintDomain::LastRow => &bus.last,
                _ => unreachable!("Invalid domain for bus boundary constraint"),
            };

            match bus_boundary {
                air_ir::BusBoundary::PublicInputTable(air_ir::PublicInputTableAccess {
                    bus_name,
                    table_name,
                    ..
                }) => {
                    let expr_root_string =
                        call_bus_boundary_varlen_pubinput(ir, *bus_name, *table_name);

                    let assertion = format!(
                        "result.push(Assertion::single({}, {}, {}));",
                        index,
                        domain_to_str(*domain),
                        expr_root_string
                    );

                    func_body.line(assertion);
                }
                air_ir::BusBoundary::Null | air_ir::BusBoundary::Unconstrained => {}
            }
        }
    }
}

/// Returns a string slice representing the provided constraint domain.
fn domain_to_str(domain: ConstraintDomain) -> String {
    match domain {
        ConstraintDomain::FirstRow => "0".to_string(),
        ConstraintDomain::LastRow => "self.last_step()".to_string(),
        // TODO: replace this with an Error once we have a Result return type.
        _ => panic!("invalid constraint domain"),
    }
}

// CONSTRAINT GRAPH HELPERS
// ================================================================================================

/// Given a node index that is expected to be the root index of a boundary constraint, returns
/// the [TraceAccess] representing the trace segment and column against which the
/// boundary constraint expression must hold, as well as the node index that represents the root
/// of the constraint expression that must equal zero during evaluation.
///
/// TODO: replace panics with Result and Error
pub fn split_boundary_constraint(
    graph: &AlgebraicGraph,
    index: &NodeIndex,
) -> (TraceAccess, NodeIndex) {
    let node = graph.node(index);
    match node.op() {
        Operation::Sub(lhs, rhs) => {
            if let Operation::Value(Value::TraceAccess(trace_access)) = graph.node(lhs).op() {
                debug_assert_eq!(trace_access.row_offset, 0);
                (*trace_access, *rhs)
            } else {
                panic!("InvalidUsage: index {index:?} is not the constraint root of a boundary constraint");
            }
        }
        _ => panic!("InvalidUsage: index {index:?} is not the root index of a constraint"),
    }
}

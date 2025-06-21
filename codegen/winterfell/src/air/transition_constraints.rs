use air_ir::{Air, TraceSegmentId};

use super::{Codegen, ElemType, Impl};

// HELPERS TO GENERATE THE WINTERFELL TRANSITION CONSTRAINT METHODS
// ================================================================================================

/// Adds an implementation of the "evaluate_transition" method to the referenced Air implementation
/// based on the data in the provided IR.
pub(super) fn add_fn_evaluate_transition(impl_ref: &mut Impl, ir: &Air) {
    // define the function.
    let evaluate_transition = impl_ref
        .new_fn("evaluate_transition")
        .arg_ref_self()
        .generic("E: FieldElement<BaseField = Felt>")
        .arg("frame", "&EvaluationFrame<E>")
        .arg("periodic_values", "&[E]")
        .arg("result", "&mut [E]");

    // declare current and next trace row arrays.
    evaluate_transition.line("let main_current = frame.current();");
    evaluate_transition.line("let main_next = frame.next();");

    // output the constraints.
    add_constraints(evaluate_transition, ir, 0);
}

/// Adds an implementation of the "evaluate_aux_transition" method to the referenced Air
/// implementation based on the data in the provided IR.
pub(super) fn add_fn_evaluate_aux_transition(impl_ref: &mut Impl, ir: &Air) {
    // define the function.
    let evaluate_aux_transition = impl_ref
        .new_fn("evaluate_aux_transition")
        .generic("F, E")
        .arg_ref_self()
        .arg("main_frame", "&EvaluationFrame<F>")
        .arg("aux_frame", "&EvaluationFrame<E>")
        .arg("_periodic_values", "&[F]")
        .arg("aux_rand_elements", "&AuxRandElements<E>")
        .arg("result", "&mut [E]")
        .bound("F", "FieldElement<BaseField = Felt>")
        .bound("E", "FieldElement<BaseField = Felt> + ExtensionOf<F>");

    // declare current and next trace row arrays.
    evaluate_aux_transition.line("let main_current = main_frame.current();");
    evaluate_aux_transition.line("let main_next = main_frame.next();");
    evaluate_aux_transition.line("let aux_current = aux_frame.current();");
    evaluate_aux_transition.line("let aux_next = aux_frame.next();");

    // output the constraints.
    add_constraints(evaluate_aux_transition, ir, 1);
}

/// Iterates through the integrity constraints in the IR, and appends a line of generated code to
/// the provided codegen function body for each constraint.
fn add_constraints(func_body: &mut codegen::Function, ir: &Air, trace_segment: TraceSegmentId) {
    for (idx, constraint) in ir.integrity_constraints(trace_segment).iter().enumerate() {
        func_body.line(format!(
            "result[{}] = {};",
            idx,
            constraint.node_index().to_string(ir, ElemType::Ext, trace_segment)
        ));
    }
}

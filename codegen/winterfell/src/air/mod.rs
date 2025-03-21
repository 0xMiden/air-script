mod public_inputs;
use public_inputs::{add_public_inputs_struct, public_input_type_to_string};

mod periodic_columns;
use periodic_columns::add_fn_get_periodic_column_values;

mod graph;
use graph::Codegen;

mod boundary_constraints;
use boundary_constraints::{add_fn_get_assertions, add_fn_get_aux_assertions};

mod transition_constraints;
use transition_constraints::{add_fn_evaluate_aux_transition, add_fn_evaluate_transition};

use air_ir::{Air, TraceSegmentId};

use super::{Impl, Scope};

// HELPER TYPES
// ================================================================================================

#[derive(Debug, Clone, Copy)]
pub enum ElemType {
    Base,
    Ext,
}

// HELPERS TO GENERATE AN IMPLEMENTATION OF THE WINTERFELL AIR TRAIT
// ================================================================================================

/// Updates the provided scope with a new Air struct and Winterfell Air trait implementation
/// which are equivalent the provided AirIR.
pub(super) fn add_air(scope: &mut Scope, ir: &Air) {
    // add the Public Inputs struct and its base implementation.
    add_public_inputs_struct(scope, ir);

    let name = ir.name();

    // add the Air struct and its base implementation.
    add_air_struct(scope, ir, name);

    // add Winterfell Air trait implementation for the provided AirIR.
    add_air_trait(scope, ir, name);
}

/// Updates the provided scope with a custom Air struct.
fn add_air_struct(scope: &mut Scope, ir: &Air, name: &str) {
    // define the custom Air struct.
    let air_struct = scope
        .new_struct(name)
        .vis("pub")
        .field("context", "AirContext<Felt>");

    // add public inputs
    for public_input in ir.public_inputs() {
        air_struct.field(
            public_input.name().as_str(),
            public_input_type_to_string(public_input),
        );
    }

    // add the custom Air implementation block
    let base_impl = scope.new_impl(name);
    // add a simple method to get the last step.
    base_impl
        .new_fn("last_step")
        .arg_ref_self()
        .vis("pub")
        .ret("usize")
        .line("self.trace_length() - self.context().num_transition_exemptions()");
}

/// Updates the provided scope with the custom Air struct and an Air trait implementation based on
/// the provided AirIR.
fn add_air_trait(scope: &mut Scope, ir: &Air, name: &str) {
    // add the implementation block for the Air trait.
    let air_impl = scope
        .new_impl(name)
        .impl_trait("Air")
        .associate_type("BaseField", "Felt")
        .associate_type("PublicInputs", "PublicInputs");

    // add default function "context".
    let fn_context = air_impl
        .new_fn("context")
        .arg_ref_self()
        .ret("&AirContext<Felt>");
    fn_context.line("&self.context");

    // add the method implementations required by the AIR trait.
    add_fn_new(air_impl, ir);

    add_fn_get_periodic_column_values(air_impl, ir);

    add_fn_get_assertions(air_impl, ir);

    add_fn_get_aux_assertions(air_impl, ir);

    add_fn_evaluate_transition(air_impl, ir);

    add_fn_evaluate_aux_transition(air_impl, ir);
}

/// Adds an implementation of the "new" method to the referenced Air implementation based on the
/// data in the provided AirIR.
fn add_fn_new(impl_ref: &mut Impl, ir: &Air) {
    // define the function.
    let new = impl_ref
        .new_fn("new")
        .arg("trace_info", "TraceInfo")
        .arg("public_inputs", "PublicInputs")
        .arg("options", "WinterProofOptions")
        .ret("Self");

    // define the integrity constraint degrees of the main trace `main_degrees`.
    add_constraint_degrees(new, ir, 0, "main_degrees");

    // define the integrity constraint degrees of the aux trace `aux_degrees`.
    add_constraint_degrees(new, ir, 1, "aux_degrees");

    // define the number of main trace boundary constraints `num_main_assertions`.
    new.line(format!(
        "let num_main_assertions = {};",
        ir.num_boundary_constraints(0)
    ));

    // define the number of aux trace boundary constraints `num_aux_assertions`.
    new.line(format!(
        "let num_aux_assertions = {};",
        ir.num_boundary_constraints(1)
    ));

    // define the context.
    let context = "
let context = AirContext::new_multi_segment(
    trace_info,
    main_degrees,
    aux_degrees,
    num_main_assertions,
    num_aux_assertions,
    options,
)
.set_num_transition_exemptions(2);";

    new.line(context);

    // get public inputs
    let mut pub_inputs = Vec::new();
    for public_input in ir.public_inputs() {
        pub_inputs.push(format!("{0}: public_inputs.{0}", public_input.name()));
    }
    // return initialized Self.
    new.line(format!("Self {{ context, {} }}", pub_inputs.join(", ")));
}

/// Iterates through the degrees of the integrity constraints in the IR, and appends a line of
/// generated code to the function body that declares all of the constraint degrees.
fn add_constraint_degrees(
    func_body: &mut codegen::Function,
    ir: &Air,
    trace_segment: TraceSegmentId,
    decl_name: &str,
) {
    let degrees = ir
        .integrity_constraint_degrees(trace_segment)
        .iter()
        .map(|degree| degree.to_string(ir, ElemType::Ext, trace_segment))
        .collect::<Vec<_>>();
    func_body.line(format!("let {decl_name} = vec![{}];", degrees.join(", ")));
}

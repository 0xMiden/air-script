mod public_inputs;

use public_inputs::{add_public_inputs_struct, public_input_type_to_string};

mod periodic_columns;
use periodic_columns::add_fn_get_periodic_column_values;

mod graph;
use graph::Codegen;

mod boundary_constraints;
use boundary_constraints::{add_fn_get_assertions, add_fn_get_aux_assertions};

mod transition_constraints;
use air_ir::{Air, Bus, BusBoundary, BusType, ConstraintDomain, Identifier, TraceSegmentId};
use transition_constraints::{add_fn_evaluate_aux_transition, add_fn_evaluate_transition};

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
    let air_struct = scope.new_struct(name).vis("pub").field("context", "AirContext<Felt>");

    // add public inputs
    for public_input in ir.public_inputs() {
        air_struct.field(public_input.name().as_str(), public_input_type_to_string(public_input));
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
    // add a method to get the variable length public inputs bus boundary constraints.
    let (mut add_bus_multiset_boundary_varlen, mut add_bus_logup_boundary_varlen) = (false, false);
    for bus in ir.buses.values() {
        // Check which bus type is refering to variable length public inputs
        let bus_constraints = [&bus.first, &bus.last];
        for fl in bus_constraints {
            if let BusBoundary::PublicInputTable(_) = fl {
                match bus.bus_type {
                    BusType::Multiset => {
                        add_bus_multiset_boundary_varlen = true;
                    },
                    BusType::Logup => {
                        add_bus_logup_boundary_varlen = true;
                    },
                }
            }
        }
    }
    if add_bus_multiset_boundary_varlen {
        impl_bus_multiset_boundary_varlen(base_impl);
    }
    if add_bus_logup_boundary_varlen {
        impl_bus_logup_boundary_varlen(base_impl);
    }
}

/// Build the Multiset bus constraint based on the public input
///
/// p: the constraint to be built
/// v: the public input
/// r: the random elements
/// n: the number of rows in v
/// c: the number of columns in v
/// p = prod(
///     r[0] + sum(p[i][j] * r[j+1] for j in 0..c)
///     for i in 0..n)
/// when n = 3, c = 2, the constraint is
/// p = ((r[0] + v[0][0] * r[1] + v[0][1] * r[2])
///    * (r[0] + v[1][0] * r[1] + v[1][1] * r[2])
///    * (r[0] + v[2][0] * r[1] + v[2][1] * r[2]))
///
/// denoting vi_j as v[i][j], and ri as r[i] for readability
/// p = ((r0 + v0_0 * r1 + v0_1 * r2)
///    * (r0 + v1_0 * r1 + v1_1 * r2)
///    * (r0 + v2_0 * r1 + v2_1 * r2))
fn impl_bus_multiset_boundary_varlen(base_impl: &mut Impl) {
    base_impl
        .new_fn("bus_multiset_boundary_varlen")
        .generic("'a")
        .generic("const N: usize")
        .generic("I: IntoIterator<Item = &'a [Felt; N]> + Clone")
        .generic("E: FieldElement<BaseField = Felt>")
        .arg("aux_rand_elements", "&AuxRandElements<E>")
        .arg("public_inputs", "&I")
        .ret("E")
        .vis("pub")
        .line("let mut bus_p_last: E = E::ONE;")
        .line("let rand = aux_rand_elements.rand_elements();")
        .line("for row in public_inputs.clone().into_iter() {")
        .line("    let mut p_last = rand[0];")
        .line("    for (c, p_i) in row.iter().enumerate() {")
        .line("        p_last += E::from(*p_i) * rand[c + 1];")
        .line("    }")
        .line("    bus_p_last *= p_last;")
        .line("}")
        .line("bus_p_last");
}

/// Build the LogUp bus constraint based on the public input
/// q: the constraint to be built
/// v: the public input
/// r: the random elements
/// n: the number of rows in v
/// c: the number of columns in v
/// q = sum(
///     1 / (r[0] + sum(p[i][j] * r[j+1] for j in 0..c))
///     for i in 0..n)
/// when n = 3, c = 2, the constraint is
/// q = (1 / (r[0] + v[0][0] * r[1] + v[0][1] * r[2])
///    + 1 / (r[0] + v[1][0] * r[1] + v[1][1] * r[2])
///    + 1 / (r[0] + v[2][0] * r[1] + v[2][1] * r[2]))
///
/// denoting vi_j as v[i][j], and ri as r[i] for readability
/// q = (1 / (r0 + v0_0 * r1 + v0_1 * r2)
///    + 1 / (r0 + v1_0 * r1 + v1_1 * r2)
///    + 1 / (r0 + v2_0 * r1 + v2_1 * r2))
///
/// Because this operation is not part of the Air, and is repeated by the Verifier,
/// we can divide in this scenario!
fn impl_bus_logup_boundary_varlen(base_impl: &mut Impl) {
    base_impl
        .new_fn("bus_logup_boundary_varlen")
        .generic("'a")
        .generic("const N: usize")
        .generic("I: IntoIterator<Item = &'a [Felt; N]> + Clone")
        .generic("E: FieldElement<BaseField = Felt>")
        .arg("aux_rand_elements", "&AuxRandElements<E>")
        .arg("public_inputs", "&I")
        .ret("E")
        .vis("pub")
        .line("let mut bus_q_last = E::ZERO;")
        .line("let rand = aux_rand_elements.rand_elements();")
        .line("for row in public_inputs.clone().into_iter() {")
        .line("    let mut q_last = rand[0];")
        .line("    for (c, p_i) in row.iter().enumerate() {")
        .line("        let p_i = *p_i;")
        .line("        q_last += E::from(p_i) * rand[c + 1];")
        .line("    }")
        .line("    bus_q_last += q_last.inv();")
        .line("}")
        .line("bus_q_last");
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
    let fn_context = air_impl.new_fn("context").arg_ref_self().ret("&AirContext<Felt>");
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
    new.line(format!("let num_main_assertions = {};", ir.num_boundary_constraints(0)));

    // define the number of aux trace boundary constraints `num_aux_assertions`.
    new.line(format!("let num_aux_assertions = {};", num_bus_boundary_constraints(ir)));

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

fn call_bus_boundary_varlen_pubinput(bus: &Bus, table_name: Identifier) -> String {
    match bus.bus_type {
        BusType::Multiset => format!(
            "Self::bus_multiset_boundary_varlen(aux_rand_elements, &self.{table_name}.iter())",
        ),
        BusType::Logup => {
            format!("Self::bus_logup_boundary_varlen(aux_rand_elements, &self.{table_name}.iter())",)
        },
    }
}

/// Helper function to count the number of bus boundary constraints in the provided AirIR.
fn num_bus_boundary_constraints(ir: &Air) -> usize {
    let mut num_bus_boundary_constraints = 0;

    let domains = [ConstraintDomain::FirstRow, ConstraintDomain::LastRow];
    for domain in &domains {
        for bus in ir.buses.values() {
            let bus_boundary = match domain {
                ConstraintDomain::FirstRow => &bus.first,
                ConstraintDomain::LastRow => &bus.last,
                _ => unreachable!("Invalid domain for bus boundary constraint"),
            };
            match bus_boundary {
                air_ir::BusBoundary::PublicInputTable(_) | air_ir::BusBoundary::Null => {
                    num_bus_boundary_constraints += 1;
                },
                air_ir::BusBoundary::Unconstrained => {},
            }
        }
    }

    num_bus_boundary_constraints
}

use crate::inputs::{AceVars, StarkInputs};
use crate::QuadFelt;
use air_ir::{Air, ConstraintDomain, NodeIndex, Operation, Value};
use miden_core::Felt;
use std::collections::BTreeMap;
use winter_math::FieldElement;

/// Evaluates the quotient polynomial of the Air.
pub fn eval_quotient(air: &Air, ace_vars: &AceVars, log_trace_len: u32) -> QuadFelt {
    let StarkInputs {
        gen_penultimate,
        gen_last,
        alpha,
        z_pow_n,
        // unused since we compute the `z^{trace_len/cycle_len} instead of
        // `z^{max_cycle_pow/cycle_len}` where `max_cycle_pow = trace_len/max_cycle_len`.
        z_max_cycle: _,
        z,
    } = ace_vars.stark;

    // Evaluate all periodic columns at appropriate z power
    let periodic: BTreeMap<_, _> = air
        .periodic_columns
        .iter()
        .map(|(ident, col)| {
            let trace_len = 1 << log_trace_len;
            let z_col_pow = trace_len / col.values.len();
            let z_col = z.exp_vartime(z_col_pow as u64);

            let mut poly: Vec<_> = col
                .values
                .iter()
                .copied()
                .map(Felt::new)
                .map(QuadFelt::from)
                .collect();
            let twiddles = winter_math::fft::get_inv_twiddles::<Felt>(poly.len());
            winter_math::fft::interpolate_poly(&mut poly, &twiddles);

            let eval = poly_eval(&poly, z_col);
            (*ident, QuadFelt::from(eval))
        })
        .collect();

    // Map public inputs from identifier to index matching the AirLayout format
    let public: BTreeMap<_, _> = air
        .public_inputs
        .keys()
        .enumerate()
        .map(|(i, ident)| (*ident, i))
        .collect();

    // Prepare a vector containing evaluations of all nodes in the Air graph.
    let graph = air.constraints.graph();
    let num_nodes = graph.num_nodes();
    let mut evals = Vec::with_capacity(num_nodes);

    // Iterate over all nodes, assuming the graph is "sorted"
    // (i.e., an operation always references a previous node)
    for node_idx in 0..num_nodes {
        let node: NodeIndex = node_idx.into();
        let op = graph.node(&node).op();
        let eval = match *op {
            Operation::Value(v) => match v {
                Value::Constant(c) => QuadFelt::from(Felt::new(c)),
                Value::TraceAccess(access) => {
                    ace_vars.segments[access.row_offset][access.segment][access.column]
                }
                Value::PeriodicColumn(access) => periodic[&access.name],
                Value::PublicInput(access) => {
                    let idx = public[&access.name];
                    ace_vars.public[idx][access.index]
                }
                Value::PublicInputTable(_) => unimplemented!(),
                Value::Null => todo!(), // REMOVE AUX/RAND FROM BACKEND PR TODO: IN CODEGEN, EVALUATE THIS VALUE BEFORE ITERATING ON ALL NODES
            },
            Operation::Add(l, r) => evals[usize::from(l)] + evals[usize::from(r)],
            Operation::Sub(l, r) => evals[usize::from(l)] - evals[usize::from(r)],
            Operation::Mul(l, r) => evals[usize::from(l)] * evals[usize::from(r)],
        };
        evals.push(eval);
    }

    // Iterator for all powers of alpha
    let mut alpha_pow_iter = std::iter::successors(Some(QuadFelt::ONE), move |alpha_prev| {
        Some(*alpha_prev * alpha)
    });

    // Evaluate linear-combination of integrity constraints.
    let integrity: QuadFelt = [0, 1]
        .into_iter()
        .flat_map(|segment| {
            air.constraints
                .integrity_constraints(segment)
                .iter()
                .map(|c| {
                    // TODO(Issue #392): Technically we should separate the transition from
                    //                   all-row constraints
                    // assert_eq!(c.domain(), ConstraintDomain::EveryFrame(2));
                    let idx = usize::from(*c.node_index());
                    evals[idx]
                })
        })
        .zip(alpha_pow_iter.by_ref())
        .fold(QuadFelt::ZERO, |acc, (eval, alpha_pow)| {
            acc + eval * alpha_pow
        });

    // Evaluate linear-combination of integrity constraints for the first row
    let boundary_first = [0, 1]
        .into_iter()
        .flat_map(|segment| {
            air.constraints
                .boundary_constraints(segment)
                .iter()
                .filter(|c| c.domain() == ConstraintDomain::FirstRow)
                .map(|r| {
                    let idx = usize::from(*r.node_index());
                    evals[idx]
                })
        })
        .zip(alpha_pow_iter.by_ref())
        .fold(QuadFelt::ZERO, |acc, (eval, alpha_pow)| {
            acc + eval * alpha_pow
        });

    // Evaluate linear-combination of integrity constraints for the last row
    let boundary_last = [0, 1]
        .into_iter()
        .flat_map(|segment| {
            air.constraints
                .boundary_constraints(segment)
                .iter()
                .filter(|c| c.domain() == ConstraintDomain::LastRow)
                .map(|r| {
                    let idx = usize::from(*r.node_index());
                    evals[idx]
                })
        })
        .zip(alpha_pow_iter.by_ref())
        .fold(QuadFelt::ZERO, |acc, (eval, alpha_pow)| {
            acc + eval * alpha_pow
        });

    // z-1 = z − g⁰
    let vanishing_first = z - QuadFelt::ONE;
    // z − g⁻² = z − gⁿ⁻²
    let vanishing_penultimate = z - gen_penultimate;
    // z − g⁻¹ = z − gⁿ⁻¹
    let vanishing_last = z - gen_last;
    // zⁿ − 1
    let vanishing_all = z_pow_n - QuadFelt::ONE;

    // Vanish only in the last two
    let vanishing_integrity = vanishing_last * vanishing_penultimate;
    // Vanish everywhere except the first row
    let vanishing_boundary_first = vanishing_all / vanishing_first;
    // Vanish everywhere except the penultimate row
    let vanishing_boundary_last = vanishing_all / vanishing_penultimate;

    // Combine linear combinations, multiplied by the polynomial which vanishes
    // where the constraint should not apply
    let composition = integrity * vanishing_integrity
        + boundary_first * vanishing_boundary_first
        + boundary_last * vanishing_boundary_last;

    // Quotient by the polynomial vanishing over the entire set,
    // ensuring each constraint must have evaluated to zero.
    composition / vanishing_all
}

/// Evaluates a polynomial given by `coeffs` at `point`
pub fn poly_eval(coeffs: &[QuadFelt], point: QuadFelt) -> QuadFelt {
    coeffs
        .iter()
        .copied()
        .rev()
        .reduce(|acc, coeff| acc * point + coeff)
        .unwrap_or(QuadFelt::ZERO)
}

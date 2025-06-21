mod builder;
mod circuit;
mod dot;
mod encoded;
mod inputs;
mod layout;
#[cfg(test)]
mod tests;

pub use crate::circuit::{Circuit as AceCircuit, Node as AceNode};
pub use crate::encoded::EncodedCircuit as EncodedAceCircuit;
pub use crate::inputs::{AceVars, AirInputs};
pub use crate::layout::Layout as AirLayout;

use crate::builder::{CircuitBuilder, LinearCombination};
use crate::layout::StarkVar;
use air_ir::{Air, ConstraintDomain};
use miden_core::{Felt, QuadExtension};

type QuadFelt = QuadExtension<Felt>;

/// Air constraints are organized in 3 main groups: integrity roots,
/// boundary-first roots and boundary-last roots.
/// The roots in each group are linearly combined with powers of a random challenge `α`:
///   - `int = ∑ᵢ int_roots[i]⋅αⁱ` for `i` from 0 to `|int_roots|`
///   - `bf = ∑ᵢ bf_roots[i]⋅αⁱ⁺ᵒ` for `i` from 0 to `|int_roots|`, and `o = |int_roots|`
///   - `bl = ∑ᵢ_i bl_roots[i]⋅αⁱ⁺ᵒ` for `i` from 0 to `|bf_roots|` and `o = |int_roots| + |bf_roots|`
///
/// This function builds a circuit that computes the following formula which must evaluate to zero:
/// `z₋₂²⋅z₋₁⋅z₀⋅int + zₙ⋅z₋₂⋅bf + zₙ⋅z₀⋅bl - Q(z)⋅zₙ⋅z₀⋅z₋₂`. The variables are given by:
///   - `z₀ = z - 1` is the vanishing polynomial for the first row,
///   - `z₋₂ = z - g⁻²` is the vanishing polynomial for the penultimate row,
///   - `z₋₁ = z - g⁻¹` is the vanishing polynomial for the last row,
///   - `zₙ = zⁿ - 1` is the vanishing polynomial for all rows,
///   - `Q(z) = Q₀(z) + Q₁(z)⋅zⁿ + ⋯ + Q₇(z)⋅z⁷ⁿ` is the reconstructed quotient,
///   - `n` is the length of the trace.
///
/// This is equivalent to the check
/// ```ignore
///     num₀/[(zⁿ - 1)/[(z - g⁻¹)(z - g⁻²)]] + num₁/(z - 1) + num₂/(z - g⁻²) = Q(z)
/// ```
///
/// The ACE chiplet expects the inputs of the original AirScript, with the order defined by
/// [`AceLayout`]:
/// - the public inputs of the AirScript e.g. `public_inputs { stack_inputs[16] }`,
/// - auxiliary randomness of the AirScript e.g. `random_values { rand: [2] }`,
/// - the main segment of trace inputs of the AirScript e.g. `trace_columns { main: [a b] }`,
/// - the aux segment of trace inputs of the AirScript e.g. `trace_columns { aux: [f] }`,
/// - the segment of 8 quotient evaluations `[Q₀(z), ..., Q₇(z)]`,
/// - the next row main segment of trace inputs of the AirScript e.g. `a' b'`,
/// - the next row aux segment of trace inputs of the AirScript e.g. `f'`,
/// - a dummy section of 8 quotient evaluation for the next row, unused by the ACE circuit.
///
/// Additionally, the ACE chiplet expects the following 5 auxiliary "STARK" inputs, whose order
/// is defined by [`StarkVar`], given by `[g⁻¹, g⁻¹, α, z, zⁿ, zᵐᵃˣ`].
pub fn build_ace_circuit(air: &Air) -> anyhow::Result<(AceNode, AceCircuit)> {
    // A circuit builder is instantiated with the inputs of the circuits plus the 13 needed by the ACE chiplet
    let mut cb = CircuitBuilder::new(air);

    let segments = [0, 1];
    let integrity_roots: Vec<_> = segments
        .iter()
        .flat_map(|&seg| air.integrity_constraints(seg))
        .filter_map(|constraint| match constraint.domain() {
            // TODO(Issue #392): Technically we should separate the transition from
            //                   all-row constraints
            ConstraintDomain::EveryRow | ConstraintDomain::EveryFrame(2) => {
                Some(cb.node_from_index(air, constraint.node_index()))
            }
            ConstraintDomain::FirstRow | ConstraintDomain::LastRow => None,
            ConstraintDomain::EveryFrame(_) => {
                panic!("invalid integrity constraint domain")
            }
        })
        .collect();

    let boundary_first_roots: Vec<_> = segments
        .iter()
        .flat_map(|&seg| air.boundary_constraints(seg))
        .filter_map(|constraint| match constraint.domain() {
            ConstraintDomain::FirstRow => Some(cb.node_from_index(air, constraint.node_index())),
            _ => None,
        })
        .collect();

    let boundary_last_roots: Vec<_> = segments
        .iter()
        .flat_map(|&seg| air.boundary_constraints(seg))
        .filter_map(|constraint| match constraint.domain() {
            ConstraintDomain::LastRow => Some(cb.node_from_index(air, constraint.node_index())),
            _ => None,
        })
        .collect();

    let one = cb.constant(1);

    let alpha = cb.layout.stark_node(StarkVar::Alpha);
    let z = cb.layout.stark_node(StarkVar::Z);
    let z_n = cb.layout.stark_node(StarkVar::ZPowN);
    let gen_last = cb.layout.stark_node(StarkVar::GenLast);
    let gen_penultimate = cb.layout.stark_node(StarkVar::GenPenultimate);

    // At this point, all the nodes of the original AirScript are copied inside the ACE circuit.
    // We now start adding new nodes to join the AirScript roots in the formula
    // described above.
    let vanish_first = cb.sub(z, one);
    let vanish_penultimate = cb.sub(z, gen_penultimate);
    let vanish_last = cb.sub(z, gen_last);
    let vanish_all = cb.sub(z_n, one);

    let mut lc = LinearCombination::new(alpha);
    let mut lhs = cb.constant(0);
    // z₋₂²⋅z₋₁⋅z₀⋅int
    {
        let int = lc.next_linear_combination(&mut cb, integrity_roots);
        let res = cb.prod([
            vanish_first,
            vanish_penultimate,
            vanish_last,
            vanish_penultimate,
            int,
        ]);
        lhs = cb.add(lhs, res);
    };

    // zₙ⋅z₋₂⋅bf
    {
        let bf = lc.next_linear_combination(&mut cb, boundary_first_roots);
        let res = cb.prod([vanish_penultimate, vanish_all, bf]);
        lhs = cb.add(lhs, res);
    };

    // zₙ⋅z₀⋅bl
    {
        let bl = lc.next_linear_combination(&mut cb, boundary_last_roots);
        let res = cb.prod([vanish_first, vanish_all, bl]);
        lhs = cb.add(lhs, res);
    };

    // Q(z)⋅zₙ⋅z₀⋅z₋₂, where Q(z) = Q₀(z) + Q₁(z)⋅zⁿ + ⋯ + Q₇(z)⋅z⁷ⁿ
    let rhs = {
        let q = cb.layout.quotient_nodes(); // [Q₀(z), ..., Q₇(z)]
        let qz = cb.poly_eval(z_n, &q); // Q(z)
        cb.prod([vanish_first, vanish_penultimate, vanish_all, qz])
    };

    let root = cb.sub(lhs, rhs);
    let circuit = cb.into_ace_circuit();
    Ok((root, circuit))
}

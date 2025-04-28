use crate::circuit::{remap_op, Circuit, CircuitBuilder};
use air_ir::{
    Air, AlgebraicGraph, ConstraintDomain, NodeIndex, Operation, PeriodicColumnAccess,
    QualifiedIdentifier, Value,
};
use miden_core::Felt;
use std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};

const NUM_ACE_AUX_INPUTS: usize = 14;
const NUM_QUOTIENT_SEGMENT_POLY: usize = 8;

const QUOTIENT_SEGMENT_POLY_OFFSETT: usize = 0;
const ALPHA_OFFSETT: usize = 8;
const Z_OFFSETT: usize = 9;
const Z_POW_N_OFFSETT: usize = 10;
const INV_TRACE_GEN_OFFSETT: usize = 11;
const Z_MIN_NUM_CYCLES_OFFSETT: usize = 12;
const INV_TRACE_GEN_SQUARE_OFFSETT: usize = 13;

/// Air constraints are organized in 3 main groups:
///
/// 1. integrity roots,
/// 2. boundary first roots and,
/// 3. boundary last roots.
///
/// The roots in each group are linearly combined with powers of a random challenge `alpha`:
/// - `int = \sum_i int_roots[i] * alpha^i` with `i` from 0 to |int_roots|
/// - `bf = \sum_i bf_roots[i] * alpha^{i+offset}` with `i` from 0 to |int_roots| and offset = |int_roots|
/// - `bl = \sum_i bl_roots[i] * alpha^{i+offset}` with `i` from 0 to |bf_roots| and offset = |int_roots| + |bf_roots|
///
/// This function builds a circuit that computes the following formula:
///
/// `z_g_2^2 * z_g * z_1 * int + zn_1 * z_g_2 * bf + zn_1 * z_1 * bl - Qz * zn_1 * z_1 * z_g_2`
///
/// where
/// - `z_g = z - g^(-1)`
/// - `z_g_2 = z - g^(-2)`
/// - `z_1 = z - 1`
/// - `zn_1 = z^n -1`
/// - `Qz = z^{0 * n} * Q_0(z) + z^{1 * n} * Q_1(z) + ... + z^{7 * n} * Q_7(z)`
/// - n is the length of the execution trace.
///
/// The constraint evaluation check
///
/// $$\frac{\mathsf{num}_0 }{\frac{z^n - 1}{z - g^{-1}}} + \frac{\mathsf{num}_1}{z-1} + \frac{\mathsf{num}_2}{z - g^{-1}} = Q(z)$$.
///
/// is then equivalent to the circuit evaluating to zero.
///
/// The ACE chiplet expects the inputs of the original AirScript in the following order:
///
/// - the public inputs of the AirScript e.g. `public_inputs { stack_inputs[16] }`
/// - auxiliary randomness of the AirScript e.g. `random_values { rand: [2] }`
/// - the main segment of trace inputs of the AirScript e.g. `trace_columns { main: [a b] }`
/// - the aux segment of trace inputs of the AirScript e.g. `trace_columns { aux: [f] }`
/// - the next row main segment of trace inputs of the AirScript e.g. `a' b'`
/// - the next row aux segment of trace inputs of the AirScript e.g. `f'`
///
/// Additionally the ACE chiplet expects following 13 auxiliary inputs:
///
/// - qi(z) with i in [0,8)
/// - alpha
/// - z
/// - z^n
/// - g^(-1)
/// - z^min_num_cycles
/// - g^(-2)
///
/// where `min_num_cycles = trace_len / max_cycle_len` as described in more details in [`periodic_columns`].
pub fn build_ace_circuit(ir: &Air) -> anyhow::Result<(NodeIndex, Circuit)> {
    // All constraints roots are extracted from the Air
    let (integrity_roots, boundary_first_roots, boundary_last_roots) = {
        let n_segments = ir.trace_segment_widths.len();
        let mut integrity_roots = vec![];
        for seg in 0..n_segments {
            for root in ir.integrity_constraints(seg) {
                integrity_roots.push(*root.node_index());
            }
        }
        let mut boundary_first_roots = vec![];
        let mut boundary_last_roots = vec![];
        for seg in 0..n_segments {
            for root in ir.boundary_constraints(seg) {
                match root.domain() {
                    ConstraintDomain::FirstRow => boundary_first_roots.push(*root.node_index()),
                    ConstraintDomain::LastRow => boundary_last_roots.push(*root.node_index()),
                    _ => unreachable!("only boundary constraints against first and last execution trace rows are supported"),
                };
            }
        }
        (integrity_roots, boundary_first_roots, boundary_last_roots)
    };

    // Using all the roots, the nodes are extracted from the Air associated with their out index.
    // Periodic columns are collected separately.
    let all_roots: Vec<NodeIndex> = integrity_roots
        .clone()
        .into_iter()
        .chain(boundary_first_roots.clone())
        .chain(boundary_last_roots.clone())
        .collect();

    // A circuit builder is instantiated with the inputs of the circuits plus the `NUM_ACE_AUX_INPUTS` needed by the ACE chiplet
    let airscript_inputs = n_random(ir) + n_public_inputs(ir) + n_trace_accesses(ir);
    let (mut cb, inputs) = CircuitBuilder::new(airscript_inputs + NUM_ACE_AUX_INPUTS);
    let qs: Vec<NodeIndex> = inputs[(airscript_inputs + QUOTIENT_SEGMENT_POLY_OFFSETT)
        ..(airscript_inputs + QUOTIENT_SEGMENT_POLY_OFFSETT + NUM_QUOTIENT_SEGMENT_POLY)]
        .to_vec();
    let alpha = inputs[airscript_inputs + ALPHA_OFFSETT];
    let z_point = inputs[airscript_inputs + Z_OFFSETT];
    let z_n = inputs[airscript_inputs + Z_POW_N_OFFSETT];
    let inverse_generator = inputs[airscript_inputs + INV_TRACE_GEN_OFFSETT];
    let z_min_num_cycles = inputs[airscript_inputs + Z_MIN_NUM_CYCLES_OFFSETT];
    let inverse_gen_2 = inputs[airscript_inputs + INV_TRACE_GEN_SQUARE_OFFSETT];

    // A mapping is necessary to convert the indexes of the Air to the new indexes of the circuit
    let mut mapping: BTreeMap<NodeIndex, NodeIndex> = BTreeMap::new();

    // The periodic columns are treated first as they are leaves in the graph and
    // other nodes might depend on them.
    let (periodic_ops, non_periodic_ops): (Vec<(NodeIndex, Operation)>, _) =
        UniqueOperationIterator::new(ir.constraint_graph(), all_roots)
            .partition(|(_, op)| matches!(op, Operation::Value(Value::PeriodicColumn(_))));
    periodic_columns(ir, &mut cb, &mut mapping, periodic_ops, z_min_num_cycles);

    // Inputs are filtered and their indexes are converted. They are not added
    // to the circuit because, unlike in the AlgebraicGraph where inputs are
    // nodes in the graph, in the ACE circuit inputs are implicit. They are
    // simply the first indexes.
    // Their converted indexes however need to be added to the mapping so that
    // any nodes using such an index can be remapped correctly.
    let (input_ops, non_input_ops): (Vec<(NodeIndex, Operation)>, _) =
        non_periodic_ops.into_iter().partition(|(_, op)| {
            matches!(
                op,
                Operation::Value(Value::TraceAccess(_))
                    | Operation::Value(Value::PublicInput(_))
                    | Operation::Value(Value::RandomValue(_))
            )
        });
    for (out, op) in input_ops {
        let new_out = convert_input(ir, op);
        mapping.insert(out, new_out);
    }

    // The remaining operations that are not periodic columns or inputs are
    // remapped and added to the circuit.
    for (out, op) in non_input_ops {
        let op = remap_op(0, op, &mapping); // In the algebraicgraph indexes start at 0
        let new_out = cb.push(op);
        mapping.insert(out, new_out);
    }

    // At this point, all the nodes of the original Airscript are copied inside the ACE circuit.
    // We now start adding new nodes to join the Airscript roots in the formula
    // described above.

    let z_minus_g = cb.sub(z_point, inverse_generator);
    let z_minus_g_2 = cb.sub(z_point, inverse_gen_2);
    let z_minus_one = cb.sub(z_point, cb.one);
    let z_n_minus_one = cb.sub(z_n, cb.one);

    let mut lc = LinearCombinationAlpha::new(alpha, cb.one);
    let lhs = {
        let mut els = vec![];
        // int * z_g_2^2 * z_g * z_1
        {
            let integrity_roots: Vec<NodeIndex> =
                integrity_roots.iter().map(|r| mapping[r]).collect();
            let lc = lc.linear_combination_alpha(&mut cb, &integrity_roots);
            let res = cb.prod(&[lc, z_minus_g, z_minus_one, z_minus_g_2, z_minus_g_2]);
            els.push(res)
        };
        // bf * zn_1 * z_g_2
        {
            let boundary_first_roots: Vec<NodeIndex> =
                boundary_first_roots.iter().map(|r| mapping[r]).collect();
            let lc = lc.linear_combination_alpha(&mut cb, &boundary_first_roots);
            let res = cb.prod(&[lc, z_n_minus_one, z_minus_g_2]);
            els.push(res)
        };
        // bl * zn_1 * z_1
        if !boundary_last_roots.is_empty() {
            let boundary_last_roots: Vec<NodeIndex> =
                boundary_last_roots.iter().map(|r| mapping[r]).collect();
            let lc = lc.linear_combination_alpha(&mut cb, &boundary_last_roots);
            let res = cb.prod(&[lc, z_n_minus_one, z_minus_one]);
            els.push(res)
        };
        cb.sum(&els)
    };
    let rhs = {
        let qz = cb.horners_method(z_n, &qs);
        cb.prod(&[qz, z_n_minus_one, z_minus_one, z_minus_g_2])
    };
    let root = cb.sub(lhs, rhs);
    Ok(cb.normalize(root))
}

/// Computes a linear combination with the powers of a random challenge alpha
/// `\sum_i alpha^(offset+i) * coeffs[i]`
/// When called multiple times, the alpha keeps being increased with
/// alpha^(offset-1) being the last power used in the last call.
struct LinearCombinationAlpha {
    alpha: NodeIndex,
    next_alpha: NodeIndex,
}

impl LinearCombinationAlpha {
    pub fn new(alpha: NodeIndex, one: NodeIndex) -> Self {
        Self {
            alpha,
            next_alpha: one,
        }
    }

    pub fn linear_combination_alpha(
        &mut self,
        cb: &mut CircuitBuilder,
        coeffs: &[NodeIndex],
    ) -> NodeIndex {
        assert!(!coeffs.is_empty());
        coeffs.iter().fold(cb.zero, |acc, root| {
            let root_alpha = cb.mul(*root, self.next_alpha);
            self.next_alpha = cb.mul(self.alpha, self.next_alpha);
            cb.add(acc, root_alpha)
        })
    }
}

// This iterator can be dropped if a simple iter() is added to AlgebraicGraph.
// It is needed now as it is the only way to extract the nodes from the
// AlgebraicGraph with the current API.
pub struct UniqueOperationIterator<'a> {
    graph: &'a AlgebraicGraph,
    visited: BTreeSet<NodeIndex>,
    processed: BTreeSet<NodeIndex>,
    to_process: Vec<NodeIndex>,
}
impl<'a> UniqueOperationIterator<'a> {
    // the visit starts from all the constraint roots
    pub fn new(graph: &'a AlgebraicGraph, process: Vec<NodeIndex>) -> Self {
        let visited = BTreeSet::new();
        let processed = BTreeSet::new();
        UniqueOperationIterator {
            graph,
            visited,
            processed,
            to_process: process,
        }
    }
}

impl Iterator for UniqueOperationIterator<'_> {
    type Item = (NodeIndex, Operation);
    fn next(&mut self) -> Option<Self::Item> {
        use Operation::*;

        while let Some(id) = self.to_process.pop() {
            let op = *self.graph.node(&id).op();
            if self.processed.contains(&id) {
                return Some((id, op));
            } else {
                self.to_process.push(id);
                self.visited.insert(id);
                self.processed.insert(id);
                match op {
                    Value(_) => {}
                    Add(l, r) | Sub(l, r) | Mul(l, r) => {
                        if !self.visited.contains(&r) {
                            self.visited.insert(r);
                            self.to_process.push(r)
                        };
                        if !self.visited.contains(&l) {
                            self.visited.insert(l);
                            self.to_process.push(l)
                        };
                    }
                };
            }
        }
        None
    }
}

// Helper functions to convert the Air inputs to ACE inputs.
// =============================================================================

fn n_random(ir: &Air) -> usize {
    ir.num_random_values as usize
}
fn n_public_inputs(ir: &Air) -> usize {
    ir.public_inputs().fold(0, |acc, pi| acc + pi.size)
}
fn n_trace_accesses(ir: &Air) -> usize {
    let main_aux = ir.trace_segment_widths.iter().sum::<u16>() as usize;
    main_aux * 2 // main + aux + main' + aux'
}
/// Inputs in Air are explicit nodes in the AlgebraicGraph while in ACE they are simply the first indexes.
/// This function converts any input operation coming from Air into the correct index of the ACE inputs.
fn convert_input(ir: &Air, op: Operation) -> NodeIndex {
    match op {
        Operation::Value(Value::PublicInput(access)) => {
            let mut idx = 0;
            for pi in ir.public_inputs() {
                if pi.name == access.name {
                    idx += access.index;
                    break;
                } else {
                    idx += pi.size;
                }
            }
            idx.into()
        }
        Operation::Value(Value::RandomValue(idx)) => (n_public_inputs(ir) + idx).into(),
        Operation::Value(Value::TraceAccess(access)) => {
            let mut idx = n_random(ir) + n_public_inputs(ir);

            assert!(ir.trace_segment_widths.len() <= 2);
            let main_trace_width = ir.trace_segment_widths[0] as usize;
            let aux_trace_width = ir.trace_segment_widths.get(1).copied().unwrap_or(0) as usize;
            match (access.segment, access.row_offset) {
                (0, 0) => idx += access.column,
                (1, 0) => {
                    idx += main_trace_width;
                    idx += access.column
                }
                (0, 1) => {
                    idx += main_trace_width + aux_trace_width;
                    idx += access.column
                }
                (1, 1) => {
                    idx += main_trace_width + main_trace_width + aux_trace_width;
                    idx += access.column
                }
                _ => unreachable!(),
            }
            idx.into()
        }
        _ => unreachable!("not an input"),
    }
}

/// For each periodic column, its `cycle_len` many values are interpolated to get a
/// polynomial P. The polynomial P is then evaluated at `z l` where
/// `l := trace_len / cycle_len`.
/// However to avoid having one input per periodic column, a single power of z is
/// received as input and the others are computed.
/// Let `max_cycle_len` be the biggest cycle length appearing in the periodic columns
/// of the circuit and let `min_num_cycles = trace_len / max_cycle_len`.
/// The circuit gets as input `z^min_num_cycles`.
/// For a given cycle_len let `k = max_cycle_len / cycle_len`, then `z^l`, where
/// `l := trace_len / cycle_len`, is nothing but `(z^min_num_cycles)^k`.
fn periodic_columns(
    ir: &Air,
    cb: &mut CircuitBuilder,
    mapping: &mut BTreeMap<NodeIndex, NodeIndex>,
    periodic_ops: Vec<(NodeIndex, Operation)>,
    z_min_num_cycles: NodeIndex,
) {
    if !periodic_ops.is_empty() {
        // The periodic columns of the Air are interpolated into polynomials
        let periodic_polys: BTreeMap<QualifiedIdentifier, Vec<u64>> = ir
            .periodic_columns
            .iter()
            .map(|(identifier, col)| {
                let mut column: Vec<Felt> = col.values.iter().map(|el| Felt::new(*el)).collect();
                let inv_twiddles = winter_math::fft::get_inv_twiddles::<Felt>(column.len());
                winter_math::fft::interpolate_poly(&mut column, &inv_twiddles);
                let column: Vec<u64> = column.into_iter().map(From::from).collect();
                (*identifier, column)
            })
            .collect();

        // The coefficients of the interpolated polynomials are stored in the circuit as constants
        let periodic_polys: BTreeMap<QualifiedIdentifier, Vec<NodeIndex>> = periodic_polys
            .iter()
            .map(|(ident, poly)| {
                let poly = poly
                    .iter()
                    .map(|c| cb.constant(*c))
                    .collect::<Vec<NodeIndex>>();
                (*ident, poly)
            })
            .collect();

        let periodic_values: Vec<(NodeIndex, PeriodicColumnAccess)> = periodic_ops
            .iter()
            .map(|(idx, op)| match op {
                Operation::Value(Value::PeriodicColumn(pca)) => (*idx, *pca),
                _ => unreachable!("periodic values already filtered"),
            })
            .collect();

        let max_cycle_len = periodic_values
            .iter()
            .map(|(_, pca)| pca.cycle)
            .max()
            .expect("should contain at least one cycle");

        // Map from cycle_len to k = max_cycle_len / cycle_len, this step
        // also removes duplicated cycle_len
        let mut ks: BTreeMap<usize, usize> = BTreeMap::new();
        periodic_values.iter().for_each(|(_, pca)| {
            let k = max_cycle_len / pca.cycle;
            ks.insert(pca.cycle, k);
        });

        // Map from cycle_len to z^l
        let z_ls: BTreeMap<usize, NodeIndex> = ks
            .into_iter()
            .map(|(cycle_len, k)| {
                let k = u32::try_from(k).unwrap();
                (cycle_len, cb.pow(z_min_num_cycles, k))
            })
            .collect();

        periodic_values.iter().for_each(|(idx, pca)| {
            let z_l = z_ls[&pca.cycle];
            let poly = &periodic_polys[&pca.name];
            let new_idx = cb.horners_method(z_l, poly);
            mapping.insert(*idx, new_idx);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::Quad;

    #[test]
    fn test_linear_combination_alpha() {
        let (mut cb, inputs) = CircuitBuilder::new(1);
        let alpha = inputs[0];
        let mut lc = LinearCombinationAlpha::new(alpha, cb.one);

        let coeffs: Vec<NodeIndex> = (1..3).map(|i| cb.constant(i)).collect();
        let res = lc.linear_combination_alpha(&mut cb, &coeffs);
        let inputs = [Quad::from(5u64)];
        let (root, circuit) = cb.normalize(res);
        assert_eq!(
            circuit.eval(root, &inputs),
            (5u64.pow(0) * 1 + 5u64.pow(1) * 2).into()
        );

        let coeffs: Vec<NodeIndex> = (3..5).map(|i| cb.constant(i)).collect();
        let res = lc.linear_combination_alpha(&mut cb, &coeffs);
        let (root, circuit) = cb.normalize(res);
        assert_eq!(
            circuit.eval(root, &inputs),
            (5u64.pow(2) * 3 + 5u64.pow(3) * 4).into()
        );
    }
}

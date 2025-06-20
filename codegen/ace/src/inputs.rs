use crate::layout::{InputRegion, Layout};
use crate::QuadFelt;
use air_ir::Air;
use miden_core::Felt;
use std::iter::zip;
use winter_math::{FieldElement, StarkField};

/// Set of all inputs required to perform the DEEP-ALI constraint evaluations check.
/// Note that these should correspond to all values included in the proof transcript,
/// and are ordered as such.
#[derive(Clone, Debug)]
pub struct AirInputs {
    /// Log of the trace length.
    pub log_trace_len: u32,
    /// Public inputs in the same order as [`Air::public_inputs`].
    pub public: Vec<Vec<QuadFelt>>,
    /// Evaluations of the *main* trace.
    pub main: [Vec<QuadFelt>; 2],
    /// Verifier challenges used to derive the *aux* trace.
    pub rand: Vec<QuadFelt>,
    /// Evaluations of the *aux* trace.
    pub aux: [Vec<QuadFelt>; 2],
    /// Evaluations of the *quotient* parts, including in the next row.
    pub quotient: [Vec<QuadFelt>; 2],
    /// Verifier challenge used to compute the linear combination of constraints.
    pub alpha: QuadFelt,
    /// Verifier challenge corresponding to the point at which the constraint evaluation check is
    /// performed.
    pub z: QuadFelt,
}

/// Set of all variables required for the evaluation of an ACE circuit.
#[derive(Clone, Debug)]
pub struct AceVars {
    pub(crate) public: Vec<Vec<QuadFelt>>,
    pub(crate) segments: [[Vec<QuadFelt>; 3]; 2],
    pub(crate) rand: Vec<QuadFelt>,
    pub(crate) stark: StarkInputs,
}

impl AirInputs {
    /// Returns the set of all variables required for the evaluation of the ACE circuit, using only
    /// the values that would be present in the proof's transcript.
    pub fn into_ace_vars(self, air: &Air) -> AceVars {
        let stark = StarkInputs::new(air, self.log_trace_len, self.alpha, self.z);
        let [main_curr, main_next] = self.main;
        let [aux_curr, aux_next] = self.aux;
        let [quotient_curr, quotient_next] = self.quotient;
        let segments = [
            [main_curr, aux_curr, quotient_curr],
            [main_next, aux_next, quotient_next],
        ];
        AceVars {
            public: self.public,
            segments,
            rand: self.rand,
            stark,
        }
    }
}

/// Set of all variables related to the STARK IOP, necessary to run the full constraints evaluation
/// check i.e., DEEP-ALI.
#[derive(Clone, Debug)]
pub(crate) struct StarkInputs {
    /// Evaluation domain point g⁻² matching the second-to-last row of the trace.
    pub(crate) gen_penultimate: QuadFelt,
    /// Evaluation domain point g⁻¹ matching the last row of the trace.
    pub(crate) gen_last: QuadFelt,
    /// Verifier challenge used to compute the linear combination of constraints.
    pub(crate) alpha: QuadFelt,
    /// The evaluation point raised to `n = trace_len`.
    pub(crate) z_pow_n: QuadFelt,
    /// The evaluation point raised to `max = trace_len / max_cycle_len`. More details can be
    /// found in [`crate::builder::CircuitBuilder::periodic_column`].
    pub(crate) z_max_cycle: QuadFelt,
    /// Verifier challenge corresponding to the point at which the constraint evaluation check is
    /// performed.
    pub(crate) z: QuadFelt,
}

impl StarkInputs {
    /// Returns a complete set of [`StarkInputs`], reconstructed from `n = 2^{log_trace_len}`,
    /// the challenge `α`, and the evaluation point `z`.
    ///
    /// The `g` is computed as the generator of the group of roots of unity of size `n`.
    ///
    /// The [`Air`] is required to compute `zᵐᵃˣ`, the power of `z` at which we evaluate the longest
    /// periodic column, and from which we derive the evaluation points of all other columns.
    pub(crate) fn new(air: &Air, log_trace_len: u32, alpha: QuadFelt, z: QuadFelt) -> Self {
        let generator = Felt::get_root_of_unity(log_trace_len);
        let gen_next = generator.square();
        let gen_penultimate = gen_next.inv().into();

        let gen_last = generator.inv().into();

        let n = 1 << log_trace_len;
        let z_pow_n = z.exp_vartime(n);

        let max_cycle_len = air
            .periodic_columns
            .values()
            .map(|col| col.values.len() as u64)
            .max();
        let z_max_cycle_pow = max_cycle_len.map(|cycle_len| n / cycle_len).unwrap_or(0);
        let z_max_cycle = z.exp_vartime(z_max_cycle_pow);

        Self {
            gen_penultimate,
            gen_last,
            alpha,
            z,
            z_pow_n,
            z_max_cycle,
        }
    }

    /// Returns all values as a `Vec` in the same order as [`crate::StarkVar`].
    pub(crate) fn to_vec(&self) -> Vec<QuadFelt> {
        vec![
            self.gen_penultimate,
            self.gen_last,
            self.alpha,
            self.z,
            self.z_pow_n,
            self.z_max_cycle,
        ]
    }
}

impl AceVars {
    /// Generates a vector containing all inputs, respecting the required memory alignment for the
    /// recursive verifier.
    pub fn to_memory_vec(&self, layout: &Layout) -> Vec<QuadFelt> {
        let mut mem = vec![QuadFelt::ZERO; layout.num_inputs];

        let store = |mem: &mut Vec<QuadFelt>, region: &InputRegion, vars: &[QuadFelt]| {
            assert_eq!(region.width, vars.len());
            mem[region.range()].copy_from_slice(vars);
        };

        // Public values, ordered by identifiers
        assert_eq!(layout.public_inputs.len(), self.public.len());
        for (pi_region, inputs) in zip(layout.public_inputs.values(), &self.public) {
            store(&mut mem, pi_region, inputs)
        }

        // Random values
        store(&mut mem, &layout.random_values, &self.rand);

        // Trace values
        for row_offset in [0, 1] {
            for (segment_row, region) in zip(
                &self.segments[row_offset],
                &layout.trace_segments[row_offset],
            ) {
                store(&mut mem, region, segment_row.as_slice());
            }
        }

        // Stark vars
        store(&mut mem, &layout.stark_vars, &self.stark.to_vec());
        mem
    }
}

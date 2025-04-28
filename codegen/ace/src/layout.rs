use crate::circuit::Node;
use air_ir::{Air, Identifier, PublicInputAccess, TraceAccess};
use std::collections::BTreeMap;
use std::ops::Range;

/// For each set of inputs read from the transcript, we treat them as extension field elements
/// and pad them with zeros to the next multiple of 4. They can then be unhashed to a double-word
/// aligned region in memory.
const HASH_ALIGNMENT: usize = 4;

const NUM_QUOTIENT_PARTS: usize = 8;

/// Describes the layout of inputs given to an ACE circuit.
/// Each set of variables is aligned to the next multiple of 4, ensuring they can be efficiently
/// unhashed from the transcript and that each input region is aligned to [`HASH_ALIGNMENT`].
///
/// We assume the following about the underlying `Air` from which the layout is constructed
/// - The proof always contains a `main` and `aux` segment, even when the latter is unused,
/// - The maximal degree of an [`Air`] is `9`, such that the quotient can be decomposed in 8 chunks.
///   TODO(Issue: #391): Derive the degree generically.  
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Layout {
    /// Region for each set of public inputs, sorted by `Identifier`
    pub public_inputs: BTreeMap<Identifier, InputRegion>,
    /// Region for auxiliary random inputs.
    pub random_values: InputRegion,
    /// Regions containing the evaluations of each segment, ordered by `trace[row_offset][segment]`.
    ///
    /// # Detail:
    /// Note that we make the following assumptions which do not affect the evaluation of the
    /// circuit but facilitate the implementation of the overall MASM verifier. In particular,
    /// these properties facilitate the computation of the DEEP composition polynomial and
    /// can lead to more efficient unhashing from the transcript into memory.
    /// - The [`Air`] from which the layout is derived can only contain a *main* and *aux* trace
    ///   (the latter can be empty).
    /// - We treat the *quotient* as the third trace, handling it in the same way as the witness
    ///   traces. This requires the prover to provide the out-of-domain evaluations in
    ///   the *next* row of each quotient part. These are unused by the circuit.
    /// - The rows must be ordered as follows:
    ///   ```ignore
    ///   main_curr, aux_curr, quotient_curr, main_next, aux_next, quotient_next.
    ///   ```
    /// - Each trace must be padded with zero columns such that each row is word-aligned.
    ///
    /// # TODO(Issue #391):
    /// The degree of the quotient is fixed to 8 matching the degree of the VM constraints, but
    /// the actual degree can be derived from the [`Air`].
    pub trace_segments: [[InputRegion; 3]; 2],
    /// Index of the first auxiliary input describing variables
    pub stark_vars: InputRegion,
    /// Total number of inputs
    pub num_inputs: usize,
}

impl Layout {
    /// Returns a new [`Layout`] from a description of an [`Air`]. All regions are padded according
    /// to [`HASH_ALIGNMENT`], ensuring that each section starts at a word-aligned memory pointer.
    pub fn new(air: &Air) -> Self {
        let mut inputs_offset = 0;

        fn next_region(current_offset: &mut usize, width: usize) -> InputRegion {
            let offset = *current_offset;
            *current_offset += width.next_multiple_of(HASH_ALIGNMENT);
            InputRegion { offset, width }
        }

        let public_inputs: BTreeMap<_, _> = air
            .public_inputs
            .iter()
            .map(|(ident, pi)| (*ident, next_region(&mut inputs_offset, pi.size())))
            .collect();

        let random_values = next_region(&mut inputs_offset, air.num_random_values as usize);

        // TODO(Issue: #391): Use the following to derive the degree generically, and maybe add it
        // to `Air`
        // let degree = [0, 1]
        //     .into_iter()
        //     .flat_map(|segment| air.integrity_constraint_degrees(segment.into()))
        //     .map(|deg| deg.base())
        //     .max()
        //     .unwrap();
        //
        // let num_quotient_elements = (degree - 1)
        //     .next_power_of_two()
        //     .next_multiple_of(HASH_ALIGNMENT);
        // let quotient_degree = Self::quotient_parts(air);
        let num_quotient_parts = NUM_QUOTIENT_PARTS;

        // For better uniformity, the proof will include the evaluations of the quotient
        // at the shifted point. Even if these are not used, they facilitate uniform evaluation
        // of the DEEP composition polynomial.
        let segment_widths = [
            // The Air always contains a main segment
            air.trace_segment_widths[0] as usize,
            // If there is no aux segment, we set it to 0.
            air.trace_segment_widths.get(1).copied().unwrap_or(0) as usize,
            // Quotient is stored as a segment
            num_quotient_parts,
        ];
        let trace_segments = [0, 1]
            .map(|_row_offset| segment_widths.map(|width| next_region(&mut inputs_offset, width)));

        let stark_vars = next_region(&mut inputs_offset, StarkVar::num_vars());

        Self {
            public_inputs,
            trace_segments,
            random_values,
            stark_vars,
            num_inputs: inputs_offset,
        }
    }

    /// Input node associated with a public input variable.
    pub fn public_input_node(&self, public_input: &PublicInputAccess) -> Option<Node> {
        self.public_inputs
            .get(&public_input.name)
            .and_then(|region| region.as_node(public_input.index))
    }

    /// Input node associated with a trace variable.
    pub fn trace_access_node(&self, trace_access: &TraceAccess) -> Option<Node> {
        let TraceAccess {
            segment,
            column,
            row_offset,
        } = *trace_access;
        // We should only be able to access the main and aux segments.
        if segment > 1 {
            return None;
        };
        let segments_in_row = self.trace_segments.get(row_offset)?;
        let segment_region = segments_in_row.get(segment)?;
        segment_region.as_node(column)
    }

    /// Input node associated with a random challenge variable.
    pub fn random_value_node(&self, index: usize) -> Option<Node> {
        self.random_values.as_node(index)
    }

    /// Input nodes associated with the quotient polynomial coefficients.
    pub fn quotient_nodes(&self) -> Vec<Node> {
        self.trace_segments[0][2].iter_nodes().collect()
    }

    /// Input node associated with an auxiliary STARK challenge/variable.
    pub fn stark_node(&self, stark_var: StarkVar) -> Node {
        self.stark_vars.as_node(stark_var.into()).unwrap()
    }
}

/// An [`InputRegion`] is a section of indices within the overall list of inputs to the
/// [`AceCircuit`](crate::AceCircuit).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct InputRegion {
    pub offset: usize,
    pub width: usize,
}

impl InputRegion {
    /// Returns the index within the overall input section of a variable in this region,
    /// as long as it contains `index`.
    pub fn index(&self, index: usize) -> Option<usize> {
        (index < self.width).then(|| self.offset + index)
    }

    /// Returns an input [`Node`] for the input at `index` in this region if it is within bounds.
    pub fn as_node(&self, index: usize) -> Option<Node> {
        self.index(index).map(Node::Input)
    }

    /// Returns the range for all indices of inputs in this region.
    pub fn range(&self) -> Range<usize> {
        self.offset..(self.offset + self.width)
    }

    /// Returns an iterator of all input [`Node`]s in this region.
    pub fn iter_nodes(&self) -> impl Iterator<Item = Node> + use<'_> {
        self.range().map(Node::Input)
    }
}

/// List of STARK variables and challenges, derived from the public parameters and proof transcript.
#[derive(Copy, Clone, Debug)]
pub enum StarkVar {
    /// The variable g⁻² corresponding to the penultimate point in the subgroup over which the
    /// trace is interpolated.
    GenPenultimate = 0,
    /// The variable g⁻¹ corresponding to the last point in the subgroup over which the trace is
    /// interpolated.
    GenLast = 1,
    /// The variable α used as for random linear-combination of constraints.
    Alpha = 2,
    /// The variable z at which the constraints evaluation check is performed.
    Z = 3,
    /// The variable zⁿ, where `n = trace_len`
    ZPowN = 4,
    /// The variable `zᵐᵃˣ`, where `max` is equal to `trace_len / max_cycle_len`. Details can be
    /// found in [`crate::builder::CircuitBuilder::periodic_column`]
    ZMaxCycle = 5,
}

impl StarkVar {
    pub const fn num_vars() -> usize {
        6
    }
}

impl TryFrom<usize> for StarkVar {
    type Error = usize;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::GenPenultimate),
            1 => Ok(Self::GenLast),
            2 => Ok(Self::Alpha),
            3 => Ok(Self::Z),
            4 => Ok(Self::ZPowN),
            5 => Ok(Self::ZMaxCycle),
            _ => Err(value),
        }
    }
}

impl From<StarkVar> for usize {
    fn from(value: StarkVar) -> usize {
        value as usize
    }
}

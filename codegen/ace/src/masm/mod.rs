use std::collections::{BTreeMap, HashMap};

use air_ir::Air;
use anyhow::Ok;
use constants::generate_constants_module;
use ood_frames::generate_ood_frames_module;
use public_inputs::generate_public_inputs;

use crate::{
    masm::{
        deep_queries::generate_deep_queries_module,
        fri_verifier::{generate_fri_helpers_module, generate_fri_verifier_module},
        random_coin::generate_random_coin_module,
        utils::generate_utils_module,
        verifier::generate_verifier_module,
    },
    AceCircuit,
};

mod constants;
mod deep_queries;
mod fri_verifier;
mod ood_frames;
mod public_inputs;
mod random_coin;
mod utils;
mod verifier;

// CONSTANTS
// ================================================================================================

const FIELD_EXTENSION_DEGREE: usize = 2;
const FRI_FOLDING_FACTOR: usize = 4;
const BLOWUP_FACTOR: usize = 8;
const NUM_CONSTRAINTS_COMPOSITION_POLYS: usize = 8;
const DOUBLE_WORD_SIZE: usize = 8;

// MASM CODE GENERATOR
// ================================================================================================

/// Generates the modules of the Miden assembly (MASM) STARK verifier.
///
/// The following assumptions are made:
///
/// 1. Number of constraints composition polynomials is set to 8,
/// 2. FRI folding factor is set to 4,
/// 3. Extension degree of the cryptographic field is 2.
pub fn generate_masm_verifier(
    air: &Air,
    proof_options: ProofOptions,
    circuit: &AceCircuit,
) -> anyhow::Result<MasmVerifier> {
    // get the encoded circuit for the ACE chiplet
    let masm_verifier_parameters = MasmVerifierParameters::from_air(air);
    let encoded_circuit = circuit.to_ace();

    // generate the different moduless

    let constants: String = generate_constants_module(&masm_verifier_parameters, circuit);
    let public_inputs: String = generate_public_inputs(&masm_verifier_parameters);
    let utils: String = generate_utils_module(&encoded_circuit);
    let random_coin: String =
        generate_random_coin_module(&masm_verifier_parameters, &proof_options);

    let deep_queries: String =
        generate_deep_queries_module(&masm_verifier_parameters, &proof_options);
    let ood_frames: String = generate_ood_frames_module(&masm_verifier_parameters, &proof_options);

    let fri_verifier: String = generate_fri_verifier_module(&proof_options);
    let fri_helper: String = generate_fri_helpers_module(&proof_options);

    let verifier: String = generate_verifier_module(&proof_options);

    Ok(MasmVerifier {
        fri_verifier: MasmFriVerifier {
            helper: fri_helper,
            verifier: fri_verifier,
        },
        constants,
        deep_queries,
        ood_frames,
        public_inputs,
        random_coin,
        utils,
        verifier,
    })
}

// HELPER STRUCTS
// ================================================================================================

/// Collects the modules making up the MASM STARK verifier.
#[derive(Debug, Default)]
pub struct MasmVerifier {
    fri_verifier: MasmFriVerifier,
    constants: String,
    deep_queries: String,
    ood_frames: String,
    public_inputs: String,
    random_coin: String,
    utils: String,
    verifier: String,
}

impl MasmVerifier {
    pub fn deep_queries(&self) -> &str {
        &self.deep_queries
    }

    pub fn constants(&self) -> &str {
        &self.constants
    }

    pub fn fri_verifier(&self) -> &str {
        self.fri_verifier.verifier()
    }

    pub fn fri_verifier_helper(&self) -> &str {
        self.fri_verifier.helper()
    }

    pub fn ood_frames(&self) -> &str {
        &self.ood_frames
    }

    pub fn public_inputs(&self) -> &str {
        &self.public_inputs
    }

    pub fn random_coin(&self) -> &str {
        &self.random_coin
    }

    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    pub fn utils(&self) -> &str {
        &self.utils
    }
}

/// Parameters derived from [Air] and used in building the MASM verifier.
struct MasmVerifierParameters {
    num_auxiliary_randomness: u16,
    max_cycle_len_log: u32,

    main_trace_width: u16,
    aux_trace_width: Option<u16>,
    constraints_composition_trace_width: usize,

    variable_len_pub_inputs_sizes: BTreeMap<air_ir::Identifier, usize>,
    fixed_len_pub_inputs_total_size: usize,
    num_constraints: usize,
}

impl MasmVerifierParameters {
    fn from_air(air: &Air) -> Self {
        let main_trace_width = air.trace_segment_widths[0].next_multiple_of(8);
        let aux_trace_width = air
            .trace_segment_widths
            .get(1)
            .map(|width| width.next_multiple_of(8));

        let max_cycle_length = air.periodic_columns().map(|col| col.period()).max();
        let max_cycle_len_log = max_cycle_length.unwrap_or(1).ilog2();

        let num_auxiliary_randomness = air.num_random_values;
        let public_inputs = air.public_inputs();
        let mut fixed_len_pub_inputs_total_size = 0;
        let mut variable_len_pub_inputs_sizes = BTreeMap::new();
        for pub_input in public_inputs {
            match pub_input {
                air_ir::PublicInput::Vector { size, .. } => fixed_len_pub_inputs_total_size += size,
                air_ir::PublicInput::Table { name, size, .. } => {
                    let _ = variable_len_pub_inputs_sizes.insert(*name, *size);
                }
            }
        }

        let num_constraints: usize = [0, 1]
            .iter()
            .map(|trace_id| {
                air.num_boundary_constraints(*trace_id) + air.num_integrity_constraints(*trace_id)
            })
            .sum();

        Self {
            num_auxiliary_randomness,
            max_cycle_len_log,
            main_trace_width,
            aux_trace_width,
            num_constraints,
            fixed_len_pub_inputs_total_size,
            variable_len_pub_inputs_sizes,
            constraints_composition_trace_width: NUM_CONSTRAINTS_COMPOSITION_POLYS,
        }
    }

    fn num_constraints(&self) -> usize {
        self.num_constraints
    }

    fn num_auxiliary_randomness(&self) -> u16 {
        self.num_auxiliary_randomness
    }

    fn max_cycle_len_log(&self) -> u32 {
        self.max_cycle_len_log
    }

    fn main_trace_width(&self) -> u16 {
        self.main_trace_width
    }

    fn aux_trace_width(&self) -> Option<u16> {
        self.aux_trace_width
    }

    fn constraints_composition_trace_width(&self) -> usize {
        self.constraints_composition_trace_width
    }

    fn variable_len_pub_inputs_sizes(&self) -> &BTreeMap<air_ir::Identifier, usize> {
        &self.variable_len_pub_inputs_sizes
    }

    fn fixed_len_pub_inputs_total_size(&self) -> usize {
        self.fixed_len_pub_inputs_total_size
    }
}

/// Modules defining the FRI verifier that is part of the STARK verifier.
#[derive(Debug, Default)]
struct MasmFriVerifier {
    helper: String,
    verifier: String,
}

impl MasmFriVerifier {
    fn helper(&self) -> &str {
        &self.helper
    }

    fn verifier(&self) -> &str {
        &self.verifier
    }
}

/// Proof options need by the MASM STARK verifier.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProofOptions {
    field_extension_degree: usize,
    blowup_factor: usize,
    fri_folding_factor: usize,
    fri_remainder_max_degree: usize,
}

impl ProofOptions {
    pub fn new(fri_remainder_max_degree: usize) -> Self {
        ProofOptions {
            field_extension_degree: FIELD_EXTENSION_DEGREE,
            blowup_factor: BLOWUP_FACTOR,
            fri_folding_factor: FRI_FOLDING_FACTOR,
            fri_remainder_max_degree,
        }
    }

    pub fn blowup_factor(&self) -> usize {
        self.blowup_factor
    }

    pub fn fri_folding_factor(&self) -> usize {
        self.fri_folding_factor
    }

    pub fn fri_remainder_max_degree(&self) -> usize {
        self.fri_remainder_max_degree
    }

    pub fn field_extension_degree(&self) -> usize {
        self.field_extension_degree
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Given a map with keys section labels placeholders and values the `String` to assign to
/// the placholders, returns the resulting filled `String`.
fn generate_with_map_sections(file: &mut String, sections_map: HashMap<&'static str, String>) {
    for (section_name, code) in sections_map {
        let begin_marker = format!("# BEGIN_SECTION:{section_name}",);
        let end_marker = format!("# END_SECTION:{section_name}",);

        // find the region of insertion
        if let Some(begin_index) = file.find(&begin_marker) {
            if let Some(end_index) = file.find(&end_marker) {
                let before = &file[..begin_index + begin_marker.len()];
                let after = &file[end_index..];
                *file = format!("{before}\n{code}\n{after}");
            }
        }
    }
}

/// Given a section placeholder identifier and a `String` value to assign to the placholder,
/// returns the resulting filled `String`.
fn add_section(file: &mut String, section_placeholder_id: &'static str, section: &str) {
    let mut sections_map = HashMap::new();
    sections_map.insert(section_placeholder_id, section.to_string());
    generate_with_map_sections(file, sections_map)
}

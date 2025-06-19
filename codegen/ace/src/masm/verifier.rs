use crate::masm::{add_section, ProofOptions};

/// Generates the main MASM module of the STARK verifier.
pub fn generate_verifier_module(proof_options: &ProofOptions) -> String {
    let mut file = VERIFIER
        .to_string()
        .replace(
            "{FRI_REMAINDER_POLY_MAX_DEGREE}",
            &proof_options.fri_remainder_max_degree().to_string(),
        )
        .replace(
            "{BLOWUP_FACTOR}",
            &proof_options.blowup_factor().to_string(),
        );

    add_section(
        &mut file,
        "AUX_TRACE_PROCESSING_PART",
        AUX_TRACE_PROCESSING_PART,
    );

    file
}

const VERIFIER: &str = r#"
use.std::crypto::fri::frie2f4
use.std::crypto::fri::helper

use.std::crypto::stark::deep_queries
use.std::crypto::stark::random_coin
use.std::crypto::stark::ood_frames
use.std::crypto::stark::public_inputs
use.std::crypto::stark::constants
use.std::crypto::stark::utils

#!   Verify a STARK proof attesting to the correct execution of a program in the Miden VM.
#!   The following simplifying assumptions are currently made:
#!
#!   - The blowup factor is set to {BLOWUP_FACTOR}.
#!   - The maximal allowed degree of the remainder polynomial is {FRI_REMAINDER_POLY_MAX_DEGREE}.
#!   - To boost soundness, the protocol is run on a quadratic extension field and this means that
#!     the OOD evaluation frame is composed of elements in a quadratic extension field i.e. tuples.
#!     Similarly, elements of the auxiliary trace, if it exists, are quadratic extension field elements.
#!     The random values for computing random linear combinations are also in this extension field.
#!
#! Input: [log(trace_length), num_queries, grinding,  ...]
#! Output: [...]
export.verify

    #==============================================================================================
    #       I)  Hash proof context and hash-&-load public inputs
    #==============================================================================================

    # Validate inputs
    exec.utils::validate_inputs
    # => [log(trace_length), num_queries, grinding, ...]

    # Initialize the seed using proof context
    exec.random_coin::init_seed
    # => [C, ...]

    # Load public inputs
    exec.public_inputs::process_public_inputs

    #==============================================================================================
    #       II) Generate the auxiliary trace random elements
    #==============================================================================================

    # Load main trace commitment and re-seed with it
    padw
    adv_loadw
    exec.constants::main_trace_com_ptr mem_storew
    # => [main_trace_commitment, ...]
    exec.random_coin::reseed
    # => [...]

    # BEGIN_SECTION:AUX_TRACE_PROCESSING_PART
    # END_SECTION:AUX_TRACE_PROCESSING_PART

    #==============================================================================================
    #       III) Draw constraint composition coefficients
    #==============================================================================================

    exec.random_coin::generate_constraint_composition_coefficients
    # => [...]

    #==============================================================================================
    #       IV) Reseed with commitment to constraint composition polynomial H evaluations over LDE
    #          and generate the Out-of-Domain (OOD) challenge z
    #==============================================================================================

    padw
    adv_loadw
    exec.constants::composition_poly_com_ptr mem_storew
    exec.random_coin::reseed
    exec.random_coin::generate_z_zN
    # => [...]

    #==============================================================================================
    #       V) Read the OOD frames for the main trace, auxiliary trace and the trace of evaluations
    #           of H over the LDE domain. This also computes some values needed for the computation
    #           of the DEEP queries.
    #==============================================================================================

    exec.ood_frames::load_and_horner_eval_ood_frames

    #==============================================================================================
    #       VI) Constraint evaluation check
    #==============================================================================================

    exec.utils::constraint_evaluation_check

    #==============================================================================================
    #       VII) FRI
    #==============================================================================================

    #============================================
    #   1) Draw random coefficients for computing
    #       DEEP composition polynomial.
    #============================================

    exec.random_coin::generate_deep_composition_random_coefficients

    #============================================
    #   2) Compute constants needed for computing
    #       FRI queries. These are:
    #       -   LDE domain generator.
    #       -   Trace domain generator `g`.
    #       -   `gz`.
    #       -   Number of FRI layers.
    #============================================

    exec.helper::generate_fri_parameters
    # => [...]

    #============================================
    #   3) Load and reseed with FRI layer commitments
    #      and draw the folding challenges for
    #      computing the degree respecting projection
    #============================================

    exec.helper::load_fri_layer_commitments
    # => [...]

    #============================================
    #   4) Load and check commitment to remainder
    #      polynomial.
    #============================================

    exec.helper::load_and_verify_remainder
    # => [...]

    #============================================
    #   5) Check PoW nonce
    #============================================

    exec.random_coin::check_pow
    # => [...]

    #============================================
    #   6) Compute evaluations of DEEP composition
    #   polynomial at randomly chosen query positions
    #============================================

    # Compute the pointer to the first query using the pointer to
    # the first layer commitment and the total number of queries.
    exec.helper::compute_query_pointer

    # Draw random query indices
    exec.random_coin::generate_list_indices
    # => [...]

    # Compute deep composition polynomial queries
    exec.deep_queries::compute_deep_composition_polynomial_queries
    # => [...]

    #============================================
    #   7) Call the FRI verifier
    #============================================

    # Call FRI verifier
    exec.frie2f4::verify
end
"#;

const AUX_TRACE_PROCESSING_PART: &str = r#"
    # Draw random ExtFelt for the auxiliary trace
    exec.random_coin::generate_aux_randomness
    # => [...]

    # Reseed with auxiliary trace commitment
    padw
    adv_loadw
    exec.constants::aux_trace_com_ptr mem_storew
    exec.random_coin::reseed
    # => [...]
"#;

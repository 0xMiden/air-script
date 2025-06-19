use crate::masm::ProofOptions;

/// Generates the MASM module of the STARK verifier for defining helper procedures used by
/// the FRI verifier.
pub fn generate_fri_helpers_module(proof_options: &ProofOptions) -> String {
    let fri_remainder_poly_max_degree_plus_1 = proof_options.fri_remainder_max_degree() + 1;
    let fri_remainder_poly_max_degree_plus_1_half = fri_remainder_poly_max_degree_plus_1 >> 1;

    let fri_remainder_codeword_max_size =
        proof_options.blowup_factor() * fri_remainder_poly_max_degree_plus_1;
    let fri_remainder_codeword_max_size_half = fri_remainder_codeword_max_size >> 1;

    let fri_remainder_codeword_max_size_log = fri_remainder_codeword_max_size.ilog2();
    let fri_remainder_codeword_max_size_half_log = fri_remainder_codeword_max_size_half.ilog2();

    let field_extension_degree = proof_options.field_extension_degree();
    let num_iterations_hash_fri_remainder_poly_max_degree_plus_1 =
        (fri_remainder_poly_max_degree_plus_1 * field_extension_degree).div_ceil(8);
    let num_iterations_hash_fri_remainder_poly_max_degree_plus_1_half =
        (fri_remainder_poly_max_degree_plus_1_half * field_extension_degree).div_ceil(8);

    FRI_HELPERS
        .to_string()
        .replace(
            "{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}",
            &fri_remainder_poly_max_degree_plus_1_half.to_string(),
        )
        .replace(
            "{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}",
            &fri_remainder_poly_max_degree_plus_1.to_string(),
        )
        .replace(
            "{FRI_REMAINDER_CODEWORD_MAX_SIZE}",
            &fri_remainder_codeword_max_size.to_string(),
        )
        .replace(
            "{FRI_REMAINDER_CODEWORD_MAX_SIZE_HALF}",
            &fri_remainder_codeword_max_size_half.to_string(),
        )
        .replace(
            "{FRI_REMAINDER_CODEWORD_MAX_SIZE_LOG}",
            &fri_remainder_codeword_max_size_log.to_string(),
        )
        .replace(
            "{FRI_REMAINDER_CODEWORD_MAX_SIZE_HALF_LOG}",
            &fri_remainder_codeword_max_size_half_log.to_string(),
        )
        .replace(
            "{FRI_FOLDING_FACTOR}",
            &proof_options.fri_folding_factor().to_string(),
        )
        .replace(
            "{FRI_FOLDING_FACTOR_LOG}",
            &proof_options.fri_folding_factor().ilog2().to_string(),
        )
        .replace(
            "{BLOWUP_FACTOR}",
            &proof_options.blowup_factor().to_string(),
        )
        .replace(
            "{BLOWUP_FACTOR_LOG}",
            &proof_options.blowup_factor().ilog2().to_string(),
        )
        .replace(
            "{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}",
            &num_iterations_hash_fri_remainder_poly_max_degree_plus_1.to_string(),
        )
        .replace(
            "{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}",
            &num_iterations_hash_fri_remainder_poly_max_degree_plus_1_half.to_string(),
        )
}

const FRI_HELPERS: &str = r#"
use.std::crypto::fri::ext2fri
use.std::crypto::stark::random_coin
use.std::crypto::stark::constants

#! Compute the number of FRI layers given log2 of the size of LDE domain. It also computes the
#! LDE domain generator and, from it, the trace generator and store these for later use.
#!
#! Input: [...]
#! Output: [num_fri_layers, ...]
export.generate_fri_parameters
    # Load FRI verifier data
    padw exec.constants::get_lde_domain_info_word
    #=> [lde_size, log(lde_size), lde_g, 0, ...]

    # Compute the number of FRI layers
    dup
    dup.2
    dup
    is_odd
    if.true
        push.{FRI_REMAINDER_CODEWORD_MAX_SIZE_HALF}
        swap
        sub.{FRI_REMAINDER_CODEWORD_MAX_SIZE_HALF_LOG}
        div.{FRI_FOLDING_FACTOR_LOG}
    else
        push.{FRI_REMAINDER_CODEWORD_MAX_SIZE}
        swap
        sub.{FRI_REMAINDER_CODEWORD_MAX_SIZE_LOG}
        div.{FRI_FOLDING_FACTOR_LOG}
    end
    # => [num_fri_layers, remainder_size, lde_size, lde_size, log2(lde_size), domain_gen, 0, ...]

    exec.constants::set_num_fri_layers
    div.{BLOWUP_FACTOR}
    exec.constants::set_remainder_poly_size
    # => [lde_size, lde_size, log2(lde_size), domain_gen, 0, ...]

    dropw
    drop
    # => [...]
end

#! Get FRI layer commitments and reseed with them in order to draw folding challenges i.e. alphas.
#!
#! Input: [...]
#! Output: [...]
export.load_fri_layer_commitments
    # We need to store the current FRI layer LDE domain size and its logarithm.
    padw exec.constants::get_lde_domain_info_word 
    exec.constants::tmp1 mem_storew
    # => [Y, ...] where `Y` is as "garbage" word

    # Address containing the first layer commitment
    push.0.0
    exec.constants::fri_com_ptr
    exec.constants::get_num_fri_layers
    # => [num_layers, ptr_layer, y, y, Y, ...] where `y` are considered as "garbage" values

    dup
    push.0
    neq
    while.true
        swapw
        adv_loadw
        # => [COM, num_layers, ptr_layer, y, y, ...]

        # Save FRI layer commitment
        dup.5
        add.4
        swap.6
        mem_storew
        #=> [COM, num_layers, ptr_layer + 4, y, y, ...]

        # Reseed
        exec.random_coin::reseed
        # => [num_layers, ptr_layer + 4, y, y, ...]

        push.0.0.0.0
        exec.random_coin::get_rate_1
        #=> [R1, ZERO, num_layers, ptr_layer + 4, y, y, ... ]
        push.0.0
        exec.constants::tmp1 mem_loadw
        # => [lde_size, log2(lde_size), lde_generator, 0, a1, a0, Y, num_layers, ptr_layer + 4, y, y, ...]

        # Compute and save to memory new lde_size and its new logarithm
        div.{FRI_FOLDING_FACTOR}
        swap
        sub.{FRI_FOLDING_FACTOR_LOG}
        swap
        exec.constants::tmp1 mem_storew
        # => [lde_size / {FRI_FOLDING_FACTOR}, log2(lde_size) - {FRI_FOLDING_FACTOR_LOG}, lde_generator, 0, a1, a0, num_layers, ptr_layer + 4, y, y, ...]

        # Move the pointer higher up the stack
        movup.2 drop
        movup.2 drop
        swapw
        dropw
        # => [lde_size, log2(lde_size), a1, a0, num_layers, ptr_layer + 4, y, y, Y, ...]

        # Save [a0, a1, log2(lde_size) - {FRI_FOLDING_FACTOR_LOG}, lde_size / {FRI_FOLDING_FACTOR}] in memory next to the layer commitment
        dup.5
        add.4
        swap.6
        mem_storew
        swapw
        # => [num_layers, ptr_layer + 8, y, y, lde_size / {FRI_FOLDING_FACTOR}, log2(lde_size) - {FRI_FOLDING_FACTOR_LOG}, a1, a0, ...]

        # Decrement the FRI layer counter
        sub.1
        dup
        push.0
        neq
    end
    # => [Y, Y, ...]
    dropw
    dropw
    #=> [...]
end

#! Load and save the remainder polynomial from the advice provider and check that its hash
#! corresponds to its commitment and reseed with the latter.
#!
#! Input: [...]
#! Output: [...]
export.load_and_verify_remainder
    # Load remainder commitment and save it at `TMP1`
    padw
    adv_loadw
    exec.constants::tmp1 mem_storew
    #=> [COM, ...]

    # Reseed with remainder commitment
    exec.random_coin::reseed
    #=> [...]

    # `adv_pipe` the remainder codeword
    ## Get the numbers of FRI layers
    exec.constants::get_num_fri_layers
    ## Compute the correct remainder pointer, note that the remainder poly is laid out just after
    ## the FRI layer commitments, each saved in a word, and folding challenges, also saved in
    ## a word, and this explains the multiplication by 8
    mul.8 exec.constants::fri_com_ptr add
    #=> [fri_com_ptr, 8 * num_fri_layers, ...]
    ## Store for later use
    dup exec.constants::set_remainder_poly_address
    #=> [remainder_poly_ptr, ...]

    exec.constants::get_remainder_poly_size
    push.{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}

    eq
    if.true
        # Remainder polynomial degree less than {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}
        push.0.0.0.0
        push.0.0.0.0
        push.0.0.0.0
        # => [Y, Y, 0, 0, 0, 0 remainder_poly_ptr, remainder_size, y, y]

        # adv_load remainder polynomial
        repeat.{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}
            adv_pipe hperm
        end

        # Compare Remainder_poly_com with the read commitment
        exec.constants::tmp1 mem_loadw
        movup.4
        assert_eq
        movup.3
        assert_eq
        movup.2
        assert_eq
        assert_eq
        
    else
        # Remainder polynomial degree less than {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}
        push.0.0.0.0
        push.0.0.0.0
        push.0.0.0.0
        # => [Y, Y, 0, 0, 0, 0 remainder_poly_ptr, remainder_size, y, y]

        # adv_load remainder polynomial
        repeat.{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}
            adv_pipe hperm
        end
        # => [Y, Remainder_poly_com, Y, remainder_poly_ptr, remainder_size, y, y]

        # Compare Remainder_poly_com with the read commitment
        exec.constants::tmp1 mem_loadw
        movup.4
        assert_eq
        movup.3
        assert_eq
        movup.2
        assert_eq
        assert_eq
        
    end
    dropw dropw
    #=> [...]
end

#! Compute the pointer to the first word storing the FRI queries.
#!
#! Since the FRI queries are laid out just before the FRI commitments, we compute the address
#! to the first FRI query by subtracting from the pointer to the first FRI layer commitment
#! the total number of queries.
#!
#! Input: [...]
#! Output: [...]
#!
#! Cycles: 7
export.compute_query_pointer
    exec.constants::fri_com_ptr
    exec.constants::get_number_queries
    mul.4
    # => [num_queries*4, fri_com_ptr,  ...]

    sub
    # => [query_ptr, ...]

    exec.constants::set_fri_queries_address
    # => [...]
end
"#;

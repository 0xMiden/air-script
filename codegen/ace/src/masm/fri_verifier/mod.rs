use crate::masm::ProofOptions;

mod helpers;
pub(crate) use helpers::generate_fri_helpers_module;

/// Generates the MASM module of the STARK verifier for defining the FRI verifier.
///
/// The main parameter for defining the module is the maximum degree of the FRI remainder polynomial.
pub fn generate_fri_verifier_module(proof_options: &ProofOptions) -> String {
    let fri_remainder_poly_max_degree_plus_1 = proof_options.fri_remainder_max_degree() + 1;
    let fri_remainder_poly_max_degree_plus_1_half = fri_remainder_poly_max_degree_plus_1 >> 1;

    let field_extension_degree = proof_options.field_extension_degree();

    let num_iterations_hash_fri_remainder_poly_max_degree_plus_1 =
        (fri_remainder_poly_max_degree_plus_1 * field_extension_degree).div_ceil(8);
    let num_iterations_hash_fri_remainder_poly_max_degree_plus_1_half =
        (fri_remainder_poly_max_degree_plus_1_half * field_extension_degree).div_ceil(8);

    FRI_VERIFIER
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
            "{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}",
            &num_iterations_hash_fri_remainder_poly_max_degree_plus_1.to_string(),
        )
        .replace(
            "{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}",
            &num_iterations_hash_fri_remainder_poly_max_degree_plus_1_half.to_string(),
        )
}

const FRI_VERIFIER: &str = r#"
#! Checks that, for a query with index p at layer i, the folding procedure to create layer (i + 1)
#! was performed correctly. This also advances layer_ptr by 8 to point to the next query layer.
#!
#! Input:  [layer_ptr, layer_ptr, poe, p, e1, e0, layer_ptr, rem_ptr, x, x, x, x, x, x, x, x, ...]
#! Output: [is_not_last_layer, layer_ptr+8, layer_ptr+8, poe^4, f_pos, ne1, ne0, layer_ptr+8, rem_ptr, x, x, x, x, x, x, x, x, ...]
#!
#! Cycles: 83
export.verify_query_layer.12

    # load layer commitment C as well as [a0, a1, t_depth, d_size] (7 cycles)
    swapdw
    movup.8
    add.4
    mem_loadw   # load [a0, a1, t_depth, d_size] from layer_ptr + 4
    swapw
    movup.8
    mem_loadw   # load C from layer_ptr
    # => [C, d_size, t_depth, a1, a0, poe, p, e1, e0, layer_ptr, rem_ptr, ...]

    # verify Merkle auth path for (index = f_pos, depth = t_depth, Root = C) (19 cycles)
    swapw.2             # [poe, p, e1, e0, d_size, t_depth, a1, a0, C, layer_ptr, rem_ptr, ...]
    swap                # [p, poe, e1, e0, d_size, t_depth, a1, a0, C, layer_ptr, rem_ptr, ...]
    movup.4             # [d_size, p, poe, e1, e0, t_depth, a1, a0, C, layer_ptr, rem_ptr, ...]
    u32divmod           # p and d_size must be u32 values
    movup.5
    movupw.2
    dup.5
    movup.5             # [t_depth, f_pos, C, f_pos, d_seg, poe, e1, e0, a1, a0, layer_ptr, rem_ptr, ...]
    mtree_get           # [V, C, f_pos, d_seg, poe, e1, e0, a1, a0, layer_ptr, rem_ptr, ...]
    adv.push_mapval
    swapw
    # => [V, C, f_pos, d_seg, poe, e1, e0, a1, a0, layer_ptr, rem_ptr, ...]
    # where f_pos = p % d_size and d_seg = p / 4

    # unhash V and save the pre-image in locaddr.0 and locaddr.4; we don't clear values of C
    # because adv_pipe overwrites the first 8 elements of the stack (15 cycles)
    exec.constants::tmp3
    movdn.4
    padw
    swapw
    padw
    adv_pipe
    hperm
    # => [T2, T1, T0, ptr, V, f_pos, d_seg, poe, e1, e0, a1, a0, layer_ptr, rem_ptr, ..]

    # assert T1 == V (16 cycles)
    swapw.3
    drop
    movup.3
    assert_eq
    movup.2
    assert_eq
    assert_eq
    movup.9
    assert_eq

    # load (v7, ..v0) from memory (8 cycles)
    exec.constants::tmp3
    mem_loadw
    swapw
    exec.constants::tmp4
    mem_loadw
    # => [v7, ..., v0, f_pos, d_seg, poe, e1, e0, a1, a0, layer_ptr, rem_ptr, ...]

    # fold by 4 (1 cycle)
    fri_ext2fold4
    # => [x, x, x, x, x, x, x, x, x, x, layer_ptr + 8, poe^4, f_pos, ne1, ne0, rem_ptr, ...]

    # prepare for next iteration (10 cycles)
    swapdw
    # => [x, x, layer_ptr + 8, poe^4, f_pos, ne1, ne0, rem_ptr, x, x, x, x, x, x, x, x, ...]
    dup.2     # [layer_ptr+8, x, x, layer_ptr+8, poe^4, f_pos, ne1, ne0, rem_ptr, ]
    movdn.7   # [x, x, layer_ptr+8, poe^4, f_pos, ne1, ne0, layer_ptr+8, rem_ptr, ...]
    drop      
    drop      # [layer_ptr+8, poe^4, f_pos, ne1, ne0, layer_ptr+8, rem_ptr, ...]
    dup       # [layer_ptr+8, layer_ptr+8, poe^4, f_pos, ne1, ne0, layer_ptr+8, rem_ptr, ...]
    dup.7     # [rem_ptr, layer_ptr+8, layer_ptr+8, poe^4, f_pos, ne1, ne0, layer_ptr+8, rem_ptr, ...]
    dup.1     # [layer_ptr+8, rem_ptr, layer_ptr+8, layer_ptr+8, poe^4, f_pos, ne1, ne0, layer_ptr+8, rem_ptr, ...]
    neq       
    # => [is_not_last_layer, layer_ptr+8, layer_ptr+8, poe^4, f_pos, ne1, ne0, layer_ptr+8, rem_ptr, x, x, x, x, x, x, x, x, ...]
end

#! Verifies one FRI query.
#!
#! This procedure is specialized to the case when the remainder polynomial, used in the final check,
#! is expected to have degree at most {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}.
#! This procedure is exactly the same as `verify_query_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}` except for the remainder polynomial check,
#! thus any change to one procedure will imply an equivalent change to the other one.
#!
#! Input:  [poe, p, e1, e0, layer_ptr, rem_ptr, ...]
#! Output: [x, x, x, x, x, x, x, x, x, x, x, x, ...] (12 "garbage" elements)
#!
#! - poe is g^p.
#! - p is a query index at the first layer.
#! - (e0, e1) is an extension field element corresponding to the value of the first layer at index p.
#! - layer_ptr is the memory address of the layer data (Merkle tree root, alpha etc.) for the next
#!   layer.
#! - rem_ptr is the memory address of the remainder polynomial.
export.verify_query_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}

    # prepare stack to be in a form that leverages the fri_ext2fold4 instruction output stack state
    # (16 cycles)
    dup.5
    dup.5
    padw
    padw
    swapdw
    dup
    dup
    movup.3
    neq
    # => [?, layer_ptr, layer_ptr, poe, p, e1, e0, layer_ptr, rem_ptr, 0, 0, 0, 0, 0, 0, 0, 0, ...]

    # verify correctness of layer folding
    while.true
        exec.verify_query_layer
    end
    # => [rem_ptr, rem_ptr, poe^(2^n), f_pos, ne1, ne0, rem_ptr, rem_ptr, x, x, x, x, x, x, x, x, ...]

    movup.2 mul.7
    exec.constants::tmp2 mem_store
    # => [rem_ptr, rem_ptr, f_pos, ne1, ne0, rem_ptr, rem_ptr, x, x, x, x, x, x, x, x, ...]

    push.0 exec.constants::tmp1 mem_loadw
    # => [P, ne1, ne0, rem_ptr, rem_ptr, x, x, x, x, x, x, x, x, ...]

    swapw swapdw
    # => [x, x, x, x, x, x, x, x, ne1, ne0, rem_ptr, rem_ptr, P, ...]

    repeat.{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}
        mem_stream
        horner_eval_ext
    end
    # => [x, x, x, x, x, x, x, x, ne1, ne0, rem_ptr, rem_ptr, P, ...]

    swapdw
    # => [ne1, ne0, rem_ptr, rem_ptr, P, x, x, x, x, x, x, x, x, ...]
    movup.6 assert_eq
    movup.5 assert_eq
    # => [X, x, x, x, x, x, x, x, x, ...]
end

#! Verifies one FRI query.
#!
#! This procedure is specialized to the case when the remainder polynomial, used in the final check,
#! is expected to have degree at most {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}.
#! This procedure is exactly the same as `verify_query_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}` except for the remainder polynomial check,
#! thus any change to one procedure will imply an equivalent change to the other one.
#!
#! Input:  [poe, p, e1, e0, layer_ptr, rem_ptr, ...]
#! Output: [x, x, x, x, x, x, x, x, x, x, x, x, ...] (12 "garbage" elements)
#!
#! - poe is g^p.
#! - p is a query index at the first layer.
#! - (e0, e1) is an extension field element corresponding to the value of the first layer at index p.
#! - layer_ptr is the memory address of the layer data (Merkle tree root, alpha etc.) for the next
#!   layer.
#! - rem_ptr is the memory address of the remainder polynomial.
export.verify_query_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}

    # prepare stack to be in a form that leverages the fri_ext2fold4 instruction output stack state
    # (16 cycles)
    dup.5
    dup.5
    padw
    padw
    swapdw
    dup
    dup
    movup.3
    neq
    # => [?, layer_ptr, layer_ptr, poe, p, e1, e0, layer_ptr, rem_ptr, 0, 0, 0, 0, 0, 0, 0, 0, ...]

    # verify correctness of layer folding
    while.true
        exec.verify_query_layer
    end
    # => [rem_ptr, rem_ptr, poe^(2^n), f_pos, ne1, ne0, rem_ptr, rem_ptr, x, x, x, x, x, x, x, x, ...]

    movup.2 mul.7
    exec.constants::tmp2 mem_store
    # => [rem_ptr, rem_ptr, f_pos, ne1, ne0, rem_ptr, rem_ptr, x, x, x, x, x, x, x, x, ...]

    push.0 exec.constants::tmp1 mem_loadw
    # => [P, ne1, ne0, rem_ptr, rem_ptr, x, x, x, x, x, x, x, x, ...]

    swapw swapdw
    # => [x, x, x, x, x, x, x, x, ne1, ne0, rem_ptr, rem_ptr, P, ...]

    repeat.{NUM_ITERATIONS_HASH_FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}
        mem_stream
        horner_eval_ext
    end
    
    # => [x, x, x, x, x, x, x, x, ne1, ne0, rem_ptr, rem_ptr, P, ...]
    swapdw
    # => [ne1, ne0, rem_ptr, rem_ptr, P, x, x, x, x, x, x, x, x, ...]
    movup.6 assert_eq
    movup.5 assert_eq
    # => [X, x, x, x, x, x, x, x, x, ...]
end


#! Verifies a FRI proof where the proof was generated over the quadratic extension of the base
#! field and layer folding was performed using folding factor 4 when the degree of the remainder
#! polynomial is less than {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}.
#! This procedure is exactly the same as `verify_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}` except for the remainder polynomial check,
#! thus any change to one procedure will imply an equivalent change to the other one.
#!
#! Input:  [query_ptr, layer_ptr, rem_ptr, g, ...]
#! Output: [...]
#!
#! - query_ptr is a pointer to a list of tuples of the form (e0, e1, p, poe) where poe is equal
#!   to g^p with g being the initial FRI domain generator. p is the query index at the first layer
#!   and (e0, e1) is an extension field element corresponding to the value of the first layer at index p.
#! - layer_ptr is a pointer to the first layer commitment denoted throughout the code by C.
#!   layer_ptr + 1 points to the first [alpha0, alpha1, t_depth, d_size] where d_size is the size
#!   of initial domain divided by 4, t_depth is the depth of the Merkle tree commitment to the
#!   first layer and (alpha0, alpha1) is the first challenge used in folding the first layer.
#!   Both t_depth and d_size are expected to be smaller than 2^32. Otherwise, the result of
#!   this procedure is undefined.
#! - rem_ptr is a pointer to the first tuple of two consecutive degree 2 extension field
#!   elements making up the remainder polynomial. This procedure is specialized to the case when
#!   the the degree of the latter is less than {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}.
#!
#! The memory referenced above is used contiguously, as follows:
#!
#!   [query_ptr ... layer_ptr ... rem_ptr ...]
#!
#! This means for example that:
#! 1. rem_ptr - 1 points to the last (alpha0, alpha1, t_depth, d_size) tuple.
#! 2. layer_ptr - 1 points to the last (e0, e1, p, poe) tuple.
proc.verify_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}.4

    # store [query_ptr, layer_ptr, rem_ptr, g] to keep track of all queries
    loc_storew.0

    # [(query_ptr == layer_ptr), query_ptr, layer_ptr, rem_ptr, g]
    dup
    dup.2
    neq

    # Save a word containing a fresh accumulator for Horner evaluating the remainder polynomial,
    # a pointer to the evaluation point and a pointer to the location of the polynomial.
    push.0.0 
    exec.constants::tmp2 exec.constants::get_remainder_poly_address
    exec.constants::tmp1 mem_storew
    movup.4

    while.true
        # load [e0, e1, p, poe] from memory i.e. next query data
        movup.4
        mem_loadw
        # => [poe, p, e1, e0, layer_ptr, rem_ptr, g, ...]

        # we now have everything to verify query p
        exec.verify_query_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}

        # prepare for next iteration (18 cycles)
        # => [x, x, x, x, x, x, x, x, x, x, x, x, g, ...]
        dropw drop
        # => [x, x, x, x, x, x, x, g, ...]
        loc_loadw.0   # load [query_ptr, layer_ptr, rem_ptr, g]
        add.4
        loc_storew.0  # store [query_ptr + 4, layer_ptr, rem_ptr, g]
        swapw
        # => [x, x, x, x, query_ptr + 4, layer_ptr, rem_ptr, g, ...]
        dup.5
        dup.5
        neq
        #=> [?, x, x, x, x, query_ptr + 4, layer_ptr, rem_ptr, g, ...]
    end
    #=> [x, x, x, x, x, x, x, x, ...]

    dropw dropw
end

#! Verifies a FRI proof where the proof was generated over the quadratic extension of the base
#! field and layer folding was performed using folding factor 4 when the degree of the remainder
#! polynomial is less than {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}.
#! This procedure is exactly the same as `verify_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}` except for the remainder polynomial check,
#! thus any change to one procedure will imply an equivalent change to the other one.
#!
#! Input:  [query_ptr, layer_ptr, rem_ptr, g, ...]
#! Output: [...]
#!
#! - query_ptr is a pointer to a list of tuples of the form (e0, e1, p, poe) where poe is equal
#!   to g^p with g being the initial FRI domain generator. p is the query index at the first layer
#!   and (e0, e1) is an extension field element corresponding to the value of the first layer at index p.
#! - layer_ptr is a pointer to the first layer commitment denoted throughout the code by C.
#!   layer_ptr + 1 points to the first [alpha0, alpha1, t_depth, d_size] where d_size is the size
#!   of initial domain divided by 4, t_depth is the depth of the Merkle tree commitment to the
#!   first layer and (alpha0, alpha1) is the first challenge used in folding the first layer.
#!   Both t_depth and d_size are expected to be smaller than 2^32. Otherwise, the result of
#!   this procedure is undefined.
#! - rem_ptr is a pointer to the first tuple of two consecutive degree 2 extension field
#!   elements making up the remainder polynomial. This procedure is specialized to the case when
#!   the the degree of the latter is less than {FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}.
#!
#! The memory referenced above is used contiguously, as follows:
#!
#!   [query_ptr ... layer_ptr ... rem_ptr ...]
#!
#! This means for example that:
#! 1. rem_ptr - 1 points to the last (alpha0, alpha1, t_depth, d_size) tuple.
#! 2. layer_ptr - 1 points to the last (e0, e1, p, poe) tuple.
proc.verify_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}.4

    # store [query_ptr, layer_ptr, rem_ptr, g] to keep track of all queries
    loc_storew.0

    # [(query_ptr == layer_ptr), query_ptr, layer_ptr, rem_ptr, g]
    dup
    dup.2
    neq

    # Save a word containing a fresh accumulator for Horner evaluating the remainder polynomial,
    # a pointer to the evaluation point and a pointer to the location of the polynomial.
    push.0.0 
    exec.constants::tmp2 exec.constants::get_remainder_poly_address
    exec.constants::tmp1 mem_storew
    movup.4
    
    while.true
        # load [e0, e1, p, poe] from memory i.e. next query data
        movup.4
        mem_loadw
        # => [poe, p, e1, e0, layer_ptr, rem_ptr, g, ...]

        # we now have everything to verify query p
        exec.verify_query_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}

        # prepare for next iteration
        # => [x, x, x, x, x, x, x, x, x, x, x, x, g, ...]
        dropw drop
        # => [x, x, x, x, x, x, x, g, ...]
        loc_loadw.0   # load [query_ptr, layer_ptr, rem_ptr, g]
        add.4
        loc_storew.0  # store [query_ptr + 4, layer_ptr, rem_ptr, g]
        swapw
        # => [x, x, x, x, query_ptr + 4, layer_ptr, rem_ptr, g, ...]
        dup.5
        dup.5
        neq
        #=> [?, x, x, x, x, query_ptr + 4, layer_ptr, rem_ptr, g, ...]
    end
    #=> [x, x, x, x, x, x, x, x, ...]

    dropw dropw
end

#! Verifies a FRI proof where the proof was generated over the quadratic extension of the base
#! field and layer folding was performed using folding factor 4.
#!
#! Input:  [...]
#! Output: [...]
export.verify

    # Get domain generator and pointer to the remainder codeword
    exec.constants::get_lde_domain_generator
    exec.constants::get_remainder_poly_address
    # => [remainder_poly_ptr, g, ...]

    # Get the pointer to the first layer commitment
    exec.constants::fri_com_ptr
    # => [fri_layer_ptr, remainder_poly_ptr, g, ...]

    # Get the pointer to the first FRI query to the top
    exec.constants::get_fri_queries_address
    # => [query_ptr, fri_layer_ptr, remainder_poly_ptr, g, ...]

    
    exec.constants::get_remainder_poly_size
    push.{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}
    eq

    if.true
        exec.verify_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1_HALF}
    else
        exec.verify_{FRI_REMAINDER_POLY_MAX_DEGREE_PLUS_1}
    end
    # => [...]
end
"#;

use std::collections::HashMap;

use super::MasmVerifierParameters;
use crate::{masm::generate_with_map_sections, AceCircuit};

/// Generates the MASM module of the STARK verifier for defining constants and the memory layout
/// used during verification.
///
/// The main logic is contained in the following points:
///
/// 1. define procedures and constants related to variable length public inputs, if relevant,
/// 2. add the description of the ACE circuit so that it can be added to the advice map.
pub fn generate_constants_module(
    masm_verifier_parameters: &MasmVerifierParameters,
    circuit: &AceCircuit,
) -> String {
    let encoded_circuit = circuit.to_ace();
    let digest = encoded_circuit.circuit_hash();
    let num_inputs_circuit = encoded_circuit.num_vars();
    let num_eval_gates_circuit = encoded_circuit.num_eval_rows();

    let mut file = CONSTANTS_MASM
        .to_string()
        .replace(
            "NUM_CONSTRAINTS_VALUE",
            &masm_verifier_parameters.num_constraints().to_string(),
        )
        .replace(
            "MAX_CYCLE_LEN_LOG_VALUE",
            &masm_verifier_parameters.max_cycle_len_log().to_string(),
        )
        .replace("DIGEST_0_VALUE", &digest[0].to_string())
        .replace("DIGEST_1_VALUE", &digest[1].to_string())
        .replace("DIGEST_2_VALUE", &digest[2].to_string())
        .replace("DIGEST_3_VALUE", &digest[3].to_string())
        .replace("NUM_INPUTS_CIRCUIT_VALUE", &num_inputs_circuit.to_string())
        .replace(
            "NUM_EVAL_GATES_CIRCUIT_VALUE",
            &num_eval_gates_circuit.to_string(),
        )
        .replace(
            "NUM_FIXED_LEN_PUBLIC_INPUTS_VALUE",
            &masm_verifier_parameters
                .fixed_len_pub_inputs_total_size()
                .to_string(),
        );

    let mut op_labels_constants = String::new();
    let mut op_labels_getters = String::new();
    for (op_label, var_len_pi_id) in masm_verifier_parameters
        .variable_len_pub_inputs_sizes()
        .iter()
    {
        op_labels_constants += &format!("\t\nconst.{var_len_pi_id}_VAR_LEN_OP_LABEL={op_label}");
        op_labels_getters += &format!("\t\nexport.{var_len_pi_id}_th_var_len_pub_inputs_op_label");
        op_labels_getters += &format!("\t\n\tpush.{var_len_pi_id}_VAR_LEN_OP_LABEL");
        op_labels_getters += "\t\nend";
    }
    let mut sections_map = HashMap::new();
    sections_map.insert("OP_LABELS_VAR_LEN_PUB_INPUTS", op_labels_constants);
    sections_map.insert("OP_LABELS_VAR_LEN_PUB_INPUTS_GETTERS", op_labels_getters);
    generate_with_map_sections(&mut file, sections_map);

    file
}

const CONSTANTS_MASM: &str = r#"
# CONSTANTS
# =================================================================================================

# General constants
const.ROOT_UNITY=7277203076849721926
const.DOMAIN_OFFSET=7
const.DOMAIN_OFFSET_INV=2635249152773512046

# Number of random extension field coefficients related to the auxiliary trace (i.e. the alphas)
const.NUM_AUX_TRACE_COEFS=2

# Number of constraints, both boundary and transitional
const.NUM_CONSTRAINTS=NUM_CONSTRAINTS_VALUE

# Blowup factor
const.BLOWUP_FACTOR=8
const.BLOWUP_FACTOR_LOG=3

# Max cycle length for periodic columns
const.MAX_CYCLE_LEN_LOG=MAX_CYCLE_LEN_LOG_VALUE

# Constraint evaluation circuit digest
const.DIGEST_0=DIGEST_0_VALUE
const.DIGEST_1=DIGEST_1_VALUE
const.DIGEST_2=DIGEST_2_VALUE
const.DIGEST_3=DIGEST_3_VALUE

# Number of inputs to the constraint evaluation circuit
const.NUM_INPUTS_CIRCUIT=NUM_INPUTS_CIRCUIT_VALUE

# Number of evaluation gates in the constraint evaluation circuit
const.NUM_EVAL_GATES_CIRCUIT=NUM_EVAL_GATES_CIRCUIT_VALUE

# Number of fixed length public inputs with padding (in field elements)
const.NUM_FIXED_LEN_PUBLIC_INPUTS=NUM_FIXED_LEN_PUBLIC_INPUTS_VALUE

# Op label for each group among the variable length inputs
# BEGIN_SECTION:OP_LABELS_VAR_LEN_PUB_INPUTS
# END_SECTION:OP_LABELS_VAR_LEN_PUB_INPUTS

# MEMORY POINTERS
# =================================================================================================

## General
## Starts at address 3223322624 = 2**31 + 2**30 + 2**21 and the memory region grows forward and is
## of constant size.

### Addresses to store the LDE domain parameters
const.LDE_DOMAIN_INFO_PTR=3223322624
const.LDE_DOMAIN_GEN_PTR=3223322625
const.LDE_DOMAIN_LOG_SIZE_PTR=3223322626
const.LDE_DOMAIN_SIZE_PTR=3223322627

### Address to store the number of FRI queries
const.NUM_QUERIES_PTR=3223322628

### Address to store the size of the FRI remainder polynomial, which is basically the number of its
### coefficients or equivalently its degree plus one.
const.REMAINDER_POLY_SIZE_PTR=3223322629

### Address to store the number of FRI folded oracles
const.NUM_FRI_LAYERS_PTR=3223322630

### The first address of the region of memory storing the FRI remainder polynomial
const.REMAINDER_POLY_ADDRESS_PTR=3223322631

### Address to store the length of the execution trace
const.TRACE_LENGTH_PTR=3223322632

### The first address of the region of memory storing the FRI queries, together with some useful
### data for running FRI check
const.FRI_QUERIES_ADDRESS_PTR=3223322633

### Address to store the logarithm of the execution trace length
const.TRACE_LENGTH_LOG_PTR=3223322634

### Address to store the number of grinding bits
const.GRINDING_FACTOR_PTR=3223322635

### Addresses to store the commitments to main, auxiliary and constraints composition polynomials traces
const.MAIN_TRACE_COM_PTR=3223322636
const.AUX_TRACE_COM_PTR=3223322640
const.COMPOSITION_POLY_COM_PTR=3223322644

### Address to store the OOD evaluation point
const.Z_PTR=3223322648

### Address to store the zero word, mainly used in the context of RPO
const.ZERO_WORD_PTR=3223322652

### Address to store the non-deterministically provided DEEP polynomial batching randomness
const.ALPHA_DEEP_ND_PTR=3223322656

### Address to store the fixed terms, across all queries, of the DEEP queries.
const.OOD_FIXED_TERM_HORNER_EVALS_PTR=3223322660

### Address to the number of public inputs (in field elements)
const.NUM_PUBLIC_INPUTS_PTR=3223322664

### Address to trace domain generator
const.TRACE_DOMAIN_GENERATOR_PTR=3223322668

### Address to variable length public inputs
const.VARIABLE_LEN_PUBLIC_INPUTS_ADDRESS_PTR=3223322672

### Address to the public inputs
const.PUBLIC_INPUTS_ADDRESS_PTR=3223322676

### Addresses to store the state of RPO-based random coin
const.C_PTR=3223322680
const.R1_PTR=3223322684
const.R2_PTR=3223322688

### Addresses used for storing temporary values
const.TMP1=3223322692
const.TMP2=3223322696
const.TMP3=3223322700
const.TMP4=3223322704

### Address to the word holding the non-deterministically loaded 2 random challenges, which will be
### checked for correctness once we receive the commitment to the auxiliary trace and are able to
### generate the auxiliary randomness
const.AUX_RAND_ND_PTR=3223322708

### Address to the randomness used in computing the constraints composition polynomial
const.COMPOSITION_COEF_PTR=3223322712

### Address to the randomness used in computing the DEEP polynomial
const.DEEP_RAND_CC_PTR=3223322716

## ACE related
## Starts at address 3225419776 = 2**31 + 2**30 + 2**22 and the memory region grows backward
## and forward and is of variable length in both directions.
## In the backward direction, the size is determined by the size of the fixed public inputs and
## the number of (groups) of variable length inputs.
##
## In the forward direction, the size is determined by the number of OOD evaluations, which itself
## is a function of the number of columns in all traces, the size of the ACE circuit description,
## and the number of auxiliary ACE inputs, which is fixed to 12 base field elements.

### We use 2 extension field elements for a total of 4 base field elements.
const.AUX_RAND_ELEM_PTR=3225419776

### OOD evaluations require a total of (80 + 8) * 2 * 2 field elements for current and next trace
### polynomials and 8 * 2 * 2 field elements for current and next constraint composition polynomials
const.OOD_EVALUATIONS_PTR=3225419780        # AUX_RAND_ELEM_PTR + 4

### We need to allocate for 12 field
const.AUXILIARY_ACE_INPUTS_PTR=3225420164   # AUXILIARY_ACE_INPUTS_PTR + (80 + 8 + 8) * 2 * 2

### Address at the start of the memory region holding the arithmetic circuit for constraint evaluation
const.ACE_CIRCUIT_PTR=3225420176            # AUXILIARY_ACE_INPUTS_PTR + 12

## FRI
##
##       (FRI_COM_PTR - 600)     ---|
##              .
##              .                   | <- FRI queries
##              .
##         FRI_COM_PTR           ---|
##              .
##              .                   | <- FRI layer commitments and folding challenges
##              .
##       (FRI_COM_PTR + 256)     ---|
##              .
##              .                   | <- Remainder polynomial
##              .
##       (FRI_COM_PTR + 512-1)   ---|
##
## For each FRI layer, we need 8 memory slots, one for storing the FRI layer commitment and one for
## storing the word [a0, a1, log2(lde_size), lde_size] where a := (a0, a1) is the folding randomness
## and lde_size is the size of the LDE domain of the corresponding FRI layer.
## Since we are using a folding factor of 4 and the maximal degree of the remainder polynomial
## that we allow is 127, an upper limit of 32 FRI layers is ample and the number of memory slots
## we thus allocate for this is 256. Moreover, we allocate an additional 256 slots for the remainder
## polynomial which is expected to be laid out right after the FRI commitments.
##
## Starts at address 3229614080 = 2**31 + 2**30 + 2**23 and the memory region grows backward and forward
## and is of variable length in both directions.
## As described above, the size in the backward direction is determined by the number of FRI layers i.e.,
## the number of FRI oracles. The size in the forward direction is determined by the size of the FRI
## remainder polynomial.
const.FRI_COM_PTR=4294912800

## Current trace row
## 80 field elements for main portion of trace, 8 * 2 field elements for auxiliary portion of trace
## and 8 * 2 field elements for constraint composition polynomials, i.e., the number of slots
## required is 80 + 16 + 16 = 112
##
## Starts at address 3238002688 = 2**31 + 2**30 + 2**24 and the memory region grows only forward.
## Its size is determined mainly by the sum of widths of all traces. 
const.CURRENT_TRACE_ROW_PTR=3238002688

# ACCESSORS
# =================================================================================================

export.get_root_unity
    push.ROOT_UNITY
end

export.get_domain_offset
    push.DOMAIN_OFFSET
end

export.get_domain_offset_inv
    push.DOMAIN_OFFSET_INV
end

export.get_num_aux_trace_coefs
    push.NUM_AUX_TRACE_COEFS
end

export.get_num_constraints
    push.NUM_CONSTRAINTS
end

export.get_blowup_factor
    push.BLOWUP_FACTOR
end

export.get_blowup_factor_log
    push.BLOWUP_FACTOR_LOG
end

export.get_max_cycle_length_log
    push.MAX_CYCLE_LEN_LOG
end

export.get_arithmetic_circuit_eval_digest
    push.DIGEST_0
    push.DIGEST_1
    push.DIGEST_2
    push.DIGEST_3
end

export.get_arithmetic_circuit_eval_number_inputs
    push.NUM_INPUTS_CIRCUIT
end

export.get_arithmetic_circuit_eval_number_eval_gates
    push.NUM_EVAL_GATES_CIRCUIT
end

export.get_num_fixed_len_public_inputs
    push.NUM_FIXED_LEN_PUBLIC_INPUTS
end

#! Store details about the LDE domain.
#!
#! The info stored is `[lde_size, log(lde_size), lde_g, 0]`.
export.set_lde_domain_info_word
    push.LDE_DOMAIN_INFO_PTR mem_storew
end

#! Load details about the LDE domain.
#!
#! The info stored is `[lde_size, log(lde_size), lde_g, 0]`.
export.get_lde_domain_info_word
    push.LDE_DOMAIN_INFO_PTR mem_loadw
end

export.set_lde_domain_generator
    push.LDE_DOMAIN_GEN_PTR mem_store
end

export.get_lde_domain_generator
    push.LDE_DOMAIN_GEN_PTR mem_load
end

export.set_lde_domain_log_size
    push.LDE_DOMAIN_LOG_SIZE_PTR mem_store 
end

export.get_lde_domain_log_size
    push.LDE_DOMAIN_LOG_SIZE_PTR mem_load
end

export.set_lde_domain_size
    push.LDE_DOMAIN_SIZE_PTR mem_store
end

export.get_lde_domain_size
    push.LDE_DOMAIN_SIZE_PTR mem_load
end

export.set_number_queries
    push.NUM_QUERIES_PTR mem_store
end

export.get_number_queries
    push.NUM_QUERIES_PTR mem_load
end

export.set_remainder_poly_size
    push.REMAINDER_POLY_SIZE_PTR mem_store
end

export.get_remainder_poly_size
    push.REMAINDER_POLY_SIZE_PTR mem_load
end

export.set_num_fri_layers
    push.NUM_FRI_LAYERS_PTR mem_store
end

export.get_num_fri_layers
    push.NUM_FRI_LAYERS_PTR mem_load
end

export.set_remainder_poly_address
    push.REMAINDER_POLY_ADDRESS_PTR mem_store
end

export.get_remainder_poly_address
    push.REMAINDER_POLY_ADDRESS_PTR mem_load
end

export.set_trace_length
    push.TRACE_LENGTH_PTR mem_store
end

export.get_trace_length
    push.TRACE_LENGTH_PTR mem_load
end

export.set_fri_queries_address
    push.FRI_QUERIES_ADDRESS_PTR mem_store
end

export.get_fri_queries_address
    push.FRI_QUERIES_ADDRESS_PTR mem_load
end

export.set_trace_length_log
    push.TRACE_LENGTH_LOG_PTR mem_store
end

export.get_trace_length_log
    push.TRACE_LENGTH_LOG_PTR mem_load
end

export.set_grinding_factor
    push.GRINDING_FACTOR_PTR mem_store
end

export.get_grinding_factor
    push.GRINDING_FACTOR_PTR mem_load
end

export.main_trace_com_ptr
    push.MAIN_TRACE_COM_PTR
end

export.aux_trace_com_ptr
    push.AUX_TRACE_COM_PTR
end

export.composition_poly_com_ptr
    push.COMPOSITION_POLY_COM_PTR
end

#! Address for the point `z` and its exponentiation `z^N` where `N=trace_len`.
#!
#! The word stored is `[z_0, z_1, z^n_0, z^n_1]`.
export.z_ptr
    push.Z_PTR
end

export.zero_word_ptr
    push.ZERO_WORD_PTR
end

export.deep_rand_alpha_nd_ptr
    push.ALPHA_DEEP_ND_PTR
end

export.ood_fixed_term_horner_evaluations_ptr
    push.OOD_FIXED_TERM_HORNER_EVALS_PTR
end

export.num_public_inputs_ptr
    push.NUM_PUBLIC_INPUTS_PTR
end

export.set_trace_domain_generator
    push.TRACE_DOMAIN_GENERATOR_PTR mem_store
end

export.get_trace_domain_generator
    push.TRACE_DOMAIN_GENERATOR_PTR mem_load
end

export.variable_length_public_inputs_address_ptr
    push.VARIABLE_LEN_PUBLIC_INPUTS_ADDRESS_PTR
end

export.public_inputs_address_ptr
    push.PUBLIC_INPUTS_ADDRESS_PTR
end

#! Returns the pointer to the capacity word of the RPO-based random coin.
export.c_ptr
    push.C_PTR
end

#! Returns the pointer to the first rate word of the RPO-based random coin.
export.r1_ptr
    push.R1_PTR
end

#! Returns the pointer to the second rate word of the RPO-based random coin.
export.r2_ptr
    push.R2_PTR
end

export.tmp1
    push.TMP1
end

export.tmp2
    push.TMP2
end

export.tmp3
    push.TMP3
end

export.tmp4
    push.TMP4
end

export.aux_rand_nd_ptr
    push.AUX_RAND_ND_PTR
end

export.composition_coef_ptr
    push.COMPOSITION_COEF_PTR
end

export.deep_rand_coef_ptr
    push.DEEP_RAND_CC_PTR
end

export.aux_rand_elem_ptr
    push.AUX_RAND_ELEM_PTR
end

export.ood_evaluations_ptr
    push.OOD_EVALUATIONS_PTR
end

export.auxiliary_ace_inputs_ptr
    push.AUXILIARY_ACE_INPUTS_PTR
end

export.get_arithmetic_circuit_ptr
    push.ACE_CIRCUIT_PTR
end

export.fri_com_ptr
    push.FRI_COM_PTR
end

export.current_trace_row_ptr
    push.CURRENT_TRACE_ROW_PTR
end

# HELPER
# =================================================================================================

#! Overwrites the top stack word with zeros.
export.zeroize_stack_word
    exec.zero_word_ptr mem_loadw
end

# OP LABELS VARIABLE LENGTH PUBLIC INPUTS
# =================================================================================================

# BEGIN_SECTION:OP_LABELS_VAR_LEN_PUB_INPUTS_GETTERS
# END_SECTION:OP_LABELS_VAR_LEN_PUB_INPUTS_GETTERS

"#;

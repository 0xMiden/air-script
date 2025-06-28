use air_ir::Air;
use rand::Rng;
use winter_utils::Randomizable;

use crate::{
    AceVars, QuadFelt,
    inputs::StarkInputs,
    layout::{InputRegion, Layout},
    tests::quotient::{eval_quotient, poly_eval},
};

impl InputRegion {
    /// Generates a random list of values that fit in this region.
    pub fn random(&self) -> Vec<QuadFelt> {
        rand_quad_vec(self.width)
    }
}

impl AceVars {
    /// Samples fully random inputs for the ACE circuit.
    pub fn random(air: &Air, log_trace_len: u32) -> Self {
        let layout = Layout::new(air);
        let public = layout.public_inputs.values().map(|pi| pi.random()).collect();
        let reduced_tables = layout.reduced_tables_region.random();
        let segments = layout
            .trace_segments
            .map(|segment_row| segment_row.map(|row_region| row_region.random()));
        let rand = layout.random_values.random();
        let stark = StarkInputs::random(air, log_trace_len);
        Self {
            public,
            reduced_tables,
            segments,
            rand,
            stark,
        }
    }

    /// Samples a random set of inputs to the ACE circuit, correcting the
    /// quotient to ensure the final evaluation of the circuit is 0.
    pub fn random_with_valid_quotient(air: &Air, log_trace_len: u32) -> Self {
        let mut random_vars = Self::random(air, log_trace_len);

        // The target evaluation of the quotient at z should be r, denoted as q'(z),
        // where q' is the corrected quotient.
        let quotient_eval = eval_quotient(air, &random_vars, log_trace_len); // q'(z)

        let z = random_vars.stark.z_pow_n;
        // Evaluate the existing quotient at z
        let quotient = &mut random_vars.segments[0][2]; // q(X)
        let curr_quotient_eval = poly_eval(quotient, z); // q(z)

        // q'(X) = q(X) - q(z) + q'(z)
        // => q'(0) = q(0) + q'(z) - q(z)
        quotient[0] += quotient_eval - curr_quotient_eval;
        // Ensure the polynomial now evaluates to quotient, i.e., r = q'(z)
        assert_eq!(poly_eval(quotient, z), quotient_eval);

        random_vars
    }
}

impl StarkInputs {
    /// Generates a partially randomized set of STARK inputs from randomized Air inputs
    /// (alpha and z) and deriving the remaining variables correctly.
    pub fn random(air: &Air, log_trace_len: u32) -> Self {
        let alpha = rand_quad();
        let z = rand_quad();

        Self::new(air, log_trace_len, alpha, z)
    }
}

/// Generates a random extension field element.
pub fn rand_quad() -> QuadFelt {
    for _ in 0..1000 {
        let bytes = rand::rng().random::<[u8; QuadFelt::VALUE_SIZE]>();
        if let Some(value) = QuadFelt::from_random_bytes(&bytes) {
            return value;
        }
    }
    panic!()
}

/// Generates a vector of length `len` of random extension field elements.
pub fn rand_quad_vec(len: usize) -> Vec<QuadFelt> {
    let mut vec = Vec::with_capacity(len);
    vec.resize_with(len, rand_quad);
    vec
}

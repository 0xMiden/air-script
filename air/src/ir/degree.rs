//! The [IntegrityConstraintDegree] struct and documentation contained in this file is a duplicate
//! of the [TransitionConstraintDegree] struct defined in the Winterfell STARK prover library
//! (https://github.com/novifinancial/winterfell), which is licensed under the MIT license. The
//! implementation in this file is a subset of the Winterfell code.
//!
//! The original code is available in the Winterfell library in the `air` crate:
//! https://github.com/novifinancial/winterfell/blob/main/air/src/air/transition/degree.rs

use super::MIN_CYCLE_LENGTH;

/// Degree descriptor of an integrity constraint.
///
/// Describes constraint degree as a combination of multiplications of periodic and trace
/// columns. For example, degree of a constraint which requires multiplication of two trace
/// columns can be described as: `base: 2, cycles: []`. A constraint which requires
/// multiplication of 3 trace columns and a periodic column with a period of 32 steps can be
/// described as: `base: 3, cycles: [32]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrityConstraintDegree {
    base: usize,
    cycles: Vec<usize>,
}

impl IntegrityConstraintDegree {
    pub fn base(&self) -> usize {
        self.base
    }

    pub fn cycles(&self) -> &[usize] {
        &self.cycles
    }

    /// Creates a new integrity constraint degree descriptor for constraints which involve
    /// multiplications of trace columns only.
    ///
    /// For example, if a constraint involves multiplication of two trace columns, `degree`
    /// should be set to 2. If a constraint involves multiplication of three trace columns,
    /// `degree` should be set to 3 etc.
    pub fn new(degree: usize) -> Self {
        assert!(degree > 0, "integrity constraint degree must be at least one, but was zero");
        Self { base: degree, cycles: vec![] }
    }

    /// Creates a new integrity degree descriptor for constraints which involve multiplication
    /// of trace columns and periodic columns.
    ///
    /// For example, if a constraint involves multiplication of two trace columns and one
    /// periodic column with a period length of 32 steps, `base_degree` should be set to 2,
    /// and `cycles` should be set to `vec![32]`.
    ///
    /// # Panics
    /// Panics if:
    /// * Any of the values in the `cycles` vector is smaller than two or is not powers of two.
    pub fn with_cycles(base_degree: usize, cycles: Vec<usize>) -> Self {
        assert!(
            base_degree > 0,
            "integrity constraint degree must be at least one, but was zero"
        );
        for (i, &cycle) in cycles.iter().enumerate() {
            assert!(
                cycle >= MIN_CYCLE_LENGTH,
                "cycle length must be at least {MIN_CYCLE_LENGTH}, but was {cycle} for cycle {i}"
            );
            assert!(
                cycle.is_power_of_two(),
                "cycle length must be a power of two, but was {cycle} for cycle {i}"
            );
        }
        Self { base: base_degree, cycles }
    }
}

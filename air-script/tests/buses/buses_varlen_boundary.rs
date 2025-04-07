use winter_air::{
    Air, AirContext, Assertion, AuxTraceRandElements, EvaluationFrame,
    ProofOptions as WinterProofOptions, TraceInfo, TransitionConstraintDegree,
};
use winter_math::fields::f64::BaseElement as Felt;
use winter_math::{ExtensionOf, FieldElement, ToElements};
use winter_utils::collections::Vec;
use winter_utils::{ByteWriter, Serializable};

pub struct PublicInputs {
    inputs: Vec<[Felt; 2]>,
}
impl ToElements<Felt> for PublicInputs {
    fn to_elements(&self) -> Vec<Felt> {
        self.inputs.iter().flat_map(|x| x.iter().cloned()).collect()
    }
}

impl PublicInputs {
    pub fn new(inputs: Vec<[Felt; 2]>) -> Self {
        Self { inputs }
    }
}

impl Serializable for PublicInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(self.inputs.as_slice());
    }
}

pub struct BusesAir {
    context: AirContext<Felt>,
    inputs: Vec<[Felt; 2]>,
}

impl BusesAir {
    pub fn last_step(&self) -> usize {
        self.trace_length() - self.context().num_transition_exemptions()
    }
    /// Build the Unit bus constraint based on the public input
    /// p: the constraint to be built
    /// v: the public input
    /// r: the random elements
    /// n: the number of rows in v
    /// c: the number of columns in v
    /// p = prod(
    ///     r[0] + sum(p[i][j] * r[j+1] for j in 0..c)
    ///     for i in 0..n)
    /// when n = 3, c = 2, the constraint is
    /// p = ((r[0] + v[0][0] * r[1] + v[0][1] * r[2])
    ///    * (r[0] + v[1][0] * r[1] + v[1][1] * r[2])
    ///    * (r[0] + v[2][0] * r[1] + v[2][1] * r[2]))
    ///
    /// denoting vi_j as v[i][j], and ri as r[i] for readability
    /// p = ((r0 + v0_0 * r1 + v0_1 * r2)
    ///    * (r0 + v1_0 * r1 + v1_1 * r2)
    ///    * (r0 + v2_0 * r1 + v2_1 * r2))
    ///
    pub fn get_bus_unit_last<E: FieldElement<BaseField = Felt>>(
        &self,
        aux_rand_elements: &AuxTraceRandElements<E>,
    ) -> E {
        let mut bus_p_last: E = E::ONE;
        let rand = aux_rand_elements.get_segment_elements(0);
        let public_inputs = self.inputs.as_slice();
        for row in public_inputs.iter() {
            let mut p_last = rand[0];
            for (c, p_i) in row.iter().enumerate() {
                p_last += E::from(*p_i) * rand[c + 1];
            }
            bus_p_last *= p_last;
        }
        bus_p_last
    }
    /// Build the Multiplicity bus constraint based on the public input
    /// q: the constraint to be built
    /// v: the public input
    /// r: the random elements
    /// n: the number of rows in v
    /// c: the number of columns in v
    /// q = sum(
    ///     1 / (r[0] + sum(p[i][j] * r[j+1] for j in 0..c))
    ///     for i in 0..n)
    /// when n = 3, c = 2, the constraint is
    /// q = (1 / (r[0] + v[0][0] * r[1] + v[0][1] * r[2])
    ///    + 1 / (r[0] + v[1][0] * r[1] + v[1][1] * r[2])
    ///    + 1 / (r[0] + v[2][0] * r[1] + v[2][1] * r[2]))
    ///
    /// denoting vi_j as v[i][j], and ri as r[i] for readability
    /// q = (1 / (r0 + v0_0 * r1 + v0_1 * r2)
    ///    + 1 / (r0 + v1_0 * r1 + v1_1 * r2)
    ///    + 1 / (r0 + v2_0 * r1 + v2_1 * r2))
    ///
    /// Because this operation is not part of the Air, and is repeated by the Verifier,
    /// we can divide in this scenario!
    pub fn get_bus_mult_last<E: FieldElement<BaseField = Felt>>(
        &self,
        aux_rand_elements: &AuxTraceRandElements<E>,
    ) -> E {
        let mut bus_q_last = E::ZERO;
        let public_inputs = self.inputs.as_slice();
        let rand = aux_rand_elements.get_segment_elements(0);
        for row in public_inputs.iter() {
            let mut q_last = rand[0];
            for (c, p_i) in row.iter().enumerate() {
                let p_i = *p_i;
                q_last += E::from(p_i) * rand[c + 1];
            }
            bus_q_last += q_last.inv();
        }
        bus_q_last
    }
}

impl Air for BusesAir {
    type BaseField = Felt;
    type PublicInputs = PublicInputs;

    fn context(&self) -> &AirContext<Felt> {
        &self.context
    }

    fn new(
        trace_info: TraceInfo,
        public_inputs: PublicInputs,
        options: WinterProofOptions,
    ) -> Self {
        let main_degrees = vec![];
        let aux_degrees = vec![
            TransitionConstraintDegree::new(1),
            TransitionConstraintDegree::new(1),
        ];
        let num_main_assertions = 0;
        let num_aux_assertions = 4;

        let context = AirContext::new_multi_segment(
            trace_info,
            main_degrees,
            aux_degrees,
            num_main_assertions,
            num_aux_assertions,
            options,
        )
        .set_num_transition_exemptions(2);
        Self {
            context,
            inputs: public_inputs.inputs,
        }
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Felt>> {
        vec![]
    }

    fn get_assertions(&self) -> Vec<Assertion<Felt>> {
        let mut result = Vec::new();
        result
    }

    fn get_aux_assertions<E: FieldElement<BaseField = Felt>>(
        &self,
        aux_rand_elements: &AuxTraceRandElements<E>,
    ) -> Vec<Assertion<E>> {
        let bus_p_last = self.get_bus_unit_last(aux_rand_elements);
        let bus_q_last = self.get_bus_mult_last(aux_rand_elements);
        let mut result = Vec::new();
        result.push(Assertion::single(0, 0, E::ONE));
        result.push(Assertion::single(1, 0, E::ZERO));
        result.push(Assertion::single(0, self.last_step(), bus_p_last));
        result.push(Assertion::single(1, self.last_step(), bus_q_last));
        result
    }

    fn evaluate_transition<E: FieldElement<BaseField = Felt>>(
        &self,
        frame: &EvaluationFrame<E>,
        periodic_values: &[E],
        result: &mut [E],
    ) {
        let main_current = frame.current();
        let main_next = frame.next();
    }

    fn evaluate_aux_transition<F, E>(
        &self,
        main_frame: &EvaluationFrame<F>,
        aux_frame: &EvaluationFrame<E>,
        _periodic_values: &[F],
        aux_rand_elements: &AuxTraceRandElements<E>,
        result: &mut [E],
    ) where
        F: FieldElement<BaseField = Felt>,
        E: FieldElement<BaseField = Felt> + ExtensionOf<F>,
    {
        let main_current = main_frame.current();
        let main_next = main_frame.next();
        let aux_current = aux_frame.current();
        let aux_next = aux_frame.next();
        result[0] = (E::ONE
            + (aux_rand_elements.get_segment_elements(0)[0]
                + E::ONE * aux_rand_elements.get_segment_elements(0)[1])
                * E::ONE
            + E::ONE
            - E::ONE)
            * aux_current[0]
            - (E::ONE
                + (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1])
                    * E::ONE
                + E::ONE
                - E::ONE)
                * aux_next[0];
        result[1] = E::ONE
            * (aux_rand_elements.get_segment_elements(0)[0]
                + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
            * (aux_rand_elements.get_segment_elements(0)[0]
                + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
            * (aux_rand_elements.get_segment_elements(0)[0]
                + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
            * aux_current[1]
            + E::ZERO
            + E::ONE
                * (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                    + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                * (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                    + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                * E::ONE
            + E::ONE
                * (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                    + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                * (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                    + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                * E::ONE
            - (E::ONE
                * (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                    + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                * (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                    + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                * (aux_rand_elements.get_segment_elements(0)[0]
                    + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                    + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                * aux_next[1]
                + E::ZERO
                + E::ONE
                    * (aux_rand_elements.get_segment_elements(0)[0]
                        + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                        + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                    * (aux_rand_elements.get_segment_elements(0)[0]
                        + E::ONE * aux_rand_elements.get_segment_elements(0)[1]
                        + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2])
                    * E::from(2_u64));
    }
}

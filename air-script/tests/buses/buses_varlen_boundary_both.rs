use winter_air::{Air, AirContext, Assertion, AuxTraceRandElements, EvaluationFrame, ProofOptions as WinterProofOptions, TransitionConstraintDegree, TraceInfo};
use winter_math::fields::f64::BaseElement as Felt;
use winter_math::{ExtensionOf, FieldElement};
use winter_utils::collections::Vec;
use winter_utils::{ByteWriter, Serializable};

pub struct PublicInputs {
    inputs: Vec<[Felt; 4]>,
    outputs: Vec<[Felt; 2]>,
}

impl PublicInputs {
    pub fn new(inputs: Vec<[Felt; 4]>, outputs: Vec<[Felt; 2]>) -> Self {
        Self { inputs, outputs }
    }
}

impl Serializable for PublicInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(self.inputs.as_slice());
        target.write(self.outputs.as_slice());
    }
}

pub struct BusesAir {
    context: AirContext<Felt>,
    inputs: Vec<[Felt; 4]>,
    outputs: Vec<[Felt; 2]>,
}

impl BusesAir {
    pub fn last_step(&self) -> usize {
        self.trace_length() - self.context().num_transition_exemptions()
    }

    pub fn bus_multiset_boundary_varlen<E: FieldElement<BaseField = Felt>>(&self, aux_rand_elements: &AuxTraceRandElements<E>, public_inputs: &PublicInputs) -> E {
        let mut bus_p_last: E = E::ONE;
        let rand = aux_rand_elements.get_segment_elements(0);
        for row in public_inputs.as_slice().iter() {
            let mut p_last = rand[0];
            for (c, p_i) in row.iter().enumerate() {
                p_last += E::from(*p_i) * rand[c + 1];
            }
            bus_p_last *= p_last;
        }
        bus_p_last
    }

    pub fn bus_logup_boundary_varlen<E: FieldElement<BaseField = Felt>>(&self, aux_rand_elements: &AuxTraceRandElements<E>, public_inputs: &PublicInputs) -> E {
        let mut bus_q_last = E::ZERO;
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

    fn new(trace_info: TraceInfo, public_inputs: PublicInputs, options: WinterProofOptions) -> Self {
        let main_degrees = vec![];
        let aux_degrees = vec![TransitionConstraintDegree::new(2), TransitionConstraintDegree::new(1)];
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
        Self { context, inputs: public_inputs.inputs, outputs: public_inputs.outputs }
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Felt>> {
        vec![]
    }

    fn get_assertions(&self) -> Vec<Assertion<Felt>> {
        let mut result = Vec::new();
        result
    }

    fn get_aux_assertions<E: FieldElement<BaseField = Felt>>(&self, aux_rand_elements: &AuxTraceRandElements<E>) -> Vec<Assertion<E>> {
        let mut result = Vec::new();
        result.push(Assertion::single(0, 0, self.bus_multiset_boundary_varlen(aux_rand_elements, self.inputs)));
        result.push(Assertion::single(1, 0, self.bus_logup_boundary_varlen(aux_rand_elements, self.inputs)));
        result.push(Assertion::single(0, self.last_step(), self.bus_multiset_boundary_varlen(aux_rand_elements, self.outputs)));
        result.push(Assertion::single(1, self.last_step(), self.bus_logup_boundary_varlen(aux_rand_elements, self.outputs)));
        result
    }

    fn evaluate_transition<E: FieldElement<BaseField = Felt>>(&self, frame: &EvaluationFrame<E>, periodic_values: &[E], result: &mut [E]) {
        let main_current = frame.current();
        let main_next = frame.next();
    }

    fn evaluate_aux_transition<F, E>(&self, main_frame: &EvaluationFrame<F>, aux_frame: &EvaluationFrame<E>, _periodic_values: &[F], aux_rand_elements: &AuxTraceRandElements<E>, result: &mut [E])
    where F: FieldElement<BaseField = Felt>,
          E: FieldElement<BaseField = Felt> + ExtensionOf<F>,
    {
        let main_current = main_frame.current();
        let main_next = main_frame.next();
        let aux_current = aux_frame.current();
        let aux_next = aux_frame.next();
        result[0] = ((aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1]) * E::from(main_current[0]) + E::ONE - E::from(main_current[0])) * aux_current[0] - ((aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1]) * (E::from(main_current[0]) - E::ONE) + E::ONE - (E::from(main_current[0]) - E::ONE)) * aux_next[0];
        result[1] = (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * aux_current[1] + (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(main_current[0]) + (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(main_current[0]) - ((aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * aux_next[1] + (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(2_u64));
    }
}
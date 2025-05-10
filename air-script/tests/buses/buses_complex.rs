use winter_air::{Air, AirContext, Assertion, AuxTraceRandElements, EvaluationFrame, ProofOptions as WinterProofOptions, TransitionConstraintDegree, TraceInfo};
use winter_math::fields::f64::BaseElement as Felt;
use winter_math::{ExtensionOf, FieldElement};
use winter_utils::collections::Vec;
use winter_utils::{ByteWriter, Serializable};

pub struct PublicInputs {
    inputs: [Felt; 2],
}

impl PublicInputs {
    pub fn new(inputs: [Felt; 2]) -> Self {
        Self { inputs }
    }
}

impl Serializable for PublicInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.inputs.write_into(target);
    }
}

pub struct BusesAir {
    context: AirContext<Felt>,
    inputs: [Felt; 2],
}

impl BusesAir {
    pub fn last_step(&self) -> usize {
        self.trace_length() - self.context().num_transition_exemptions()
    }
}

impl Air for BusesAir {
    type BaseField = Felt;
    type PublicInputs = PublicInputs;

    fn context(&self) -> &AirContext<Felt> {
        &self.context
    }

    fn new(trace_info: TraceInfo, public_inputs: PublicInputs, options: WinterProofOptions) -> Self {
        let main_degrees = vec![TransitionConstraintDegree::new(2), TransitionConstraintDegree::new(2)];
        let aux_degrees = vec![TransitionConstraintDegree::new(5), TransitionConstraintDegree::new(4)];
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
        Self { context, inputs: public_inputs.inputs }
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
        result.push(Assertion::single(0, 0, E::ONE));
        result.push(Assertion::single(1, 0, E::ZERO));
        result.push(Assertion::single(0, self.last_step(), E::ONE));
        result.push(Assertion::single(1, self.last_step(), E::ZERO));
        result
    }

    fn evaluate_transition<E: FieldElement<BaseField = Felt>>(&self, frame: &EvaluationFrame<E>, periodic_values: &[E], result: &mut [E]) {
        let main_current = frame.current();
        let main_next = frame.next();
        result[0] = main_current[2] * main_current[2] - main_current[2];
        result[1] = main_current[3] * main_current[3] - main_current[3];
    }

    fn evaluate_aux_transition<F, E>(&self, main_frame: &EvaluationFrame<F>, aux_frame: &EvaluationFrame<E>, _periodic_values: &[F], aux_rand_elements: &AuxTraceRandElements<E>, result: &mut [E])
    where F: FieldElement<BaseField = Felt>,
          E: FieldElement<BaseField = Felt> + ExtensionOf<F>,
    {
        let main_current = main_frame.current();
        let main_next = main_frame.next();
        let aux_current = aux_frame.current();
        let aux_next = aux_frame.next();
        result[0] = ((aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(main_current[2]) + E::ONE - E::from(main_current[2])) * ((aux_rand_elements.get_segment_elements(0)[0] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[1]) * aux_rand_elements.get_segment_elements(0)[2]) * (E::ONE - E::from(main_current[2])) + E::ONE - (E::ONE - E::from(main_current[2]))) * aux_current[0] - ((aux_rand_elements.get_segment_elements(0)[0] + E::ONE * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[1]) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(main_current[3]) + E::ONE - E::from(main_current[3])) * ((aux_rand_elements.get_segment_elements(0)[0] + E::from(2_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (E::ONE - E::from(main_current[3])) + E::ONE - (E::ONE - E::from(main_current[3]))) * aux_next[0];
        result[1] = (aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::from(4_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[1]) * aux_rand_elements.get_segment_elements(0)[2]) * aux_current[1] + (aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::from(4_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[1]) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(main_current[2]) + (aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::from(4_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[1]) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(main_current[2]) - ((aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::from(4_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[1]) * aux_rand_elements.get_segment_elements(0)[2]) * aux_next[1] + (aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * (aux_rand_elements.get_segment_elements(0)[0] + E::from(3_u64) * aux_rand_elements.get_segment_elements(0)[1] + E::from(main_current[0]) * aux_rand_elements.get_segment_elements(0)[2]) * E::from(main_current[4]));
    }
}
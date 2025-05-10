use winter_air::{Air, AirContext, Assertion, AuxTraceRandElements, EvaluationFrame, ProofOptions as WinterProofOptions, TransitionConstraintDegree, TraceInfo};
use winter_math::fields::f64::BaseElement as Felt;
use winter_math::{ExtensionOf, FieldElement};
use winter_utils::collections::Vec;
use winter_utils::{ByteWriter, Serializable};

pub struct PublicInputs {
    overflow_addrs: [Felt; 4],
    program_hash: [Felt; 4],
    stack_inputs: [Felt; 4],
    stack_outputs: [Felt; 20],
}

impl PublicInputs {
    pub fn new(overflow_addrs: [Felt; 4], program_hash: [Felt; 4], stack_inputs: [Felt; 4], stack_outputs: [Felt; 20]) -> Self {
        Self { overflow_addrs, program_hash, stack_inputs, stack_outputs }
    }
}

impl Serializable for PublicInputs {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.overflow_addrs.write_into(target);
        self.program_hash.write_into(target);
        self.stack_inputs.write_into(target);
        self.stack_outputs.write_into(target);
    }
}

pub struct ConstantsAir {
    context: AirContext<Felt>,
    overflow_addrs: [Felt; 4],
    program_hash: [Felt; 4],
    stack_inputs: [Felt; 4],
    stack_outputs: [Felt; 20],
}

impl ConstantsAir {
    pub fn last_step(&self) -> usize {
        self.trace_length() - self.context().num_transition_exemptions()
    }
}

impl Air for ConstantsAir {
    type BaseField = Felt;
    type PublicInputs = PublicInputs;

    fn context(&self) -> &AirContext<Felt> {
        &self.context
    }

    fn new(trace_info: TraceInfo, public_inputs: PublicInputs, options: WinterProofOptions) -> Self {
        let main_degrees = vec![TransitionConstraintDegree::new(1), TransitionConstraintDegree::new(1), TransitionConstraintDegree::new(1), TransitionConstraintDegree::new(1), TransitionConstraintDegree::new(1)];
        let aux_degrees = vec![];
        let num_main_assertions = 6;
        let num_aux_assertions = 0;

        let context = AirContext::new_multi_segment(
            trace_info,
            main_degrees,
            aux_degrees,
            num_main_assertions,
            num_aux_assertions,
            options,
        )
        .set_num_transition_exemptions(2);
        Self { context, overflow_addrs: public_inputs.overflow_addrs, program_hash: public_inputs.program_hash, stack_inputs: public_inputs.stack_inputs, stack_outputs: public_inputs.stack_outputs }
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Felt>> {
        vec![]
    }

    fn get_assertions(&self) -> Vec<Assertion<Felt>> {
        let mut result = Vec::new();
        result.push(Assertion::single(0, 0, Felt::ONE));
        result.push(Assertion::single(1, 0, Felt::ONE));
        result.push(Assertion::single(2, 0, Felt::ZERO));
        result.push(Assertion::single(3, 0, Felt::ONE - Felt::new(2) + Felt::new(2) - Felt::ZERO));
        result.push(Assertion::single(4, 0, Felt::ONE));
        result.push(Assertion::single(4, self.last_step(), Felt::ZERO));
        result
    }

    fn get_aux_assertions<E: FieldElement<BaseField = Felt>>(&self, aux_rand_elements: &AuxTraceRandElements<E>) -> Vec<Assertion<E>> {
        let mut result = Vec::new();
        result
    }

    fn evaluate_transition<E: FieldElement<BaseField = Felt>>(&self, frame: &EvaluationFrame<E>, periodic_values: &[E], result: &mut [E]) {
        let main_current = frame.current();
        let main_next = frame.next();
        result[0] = main_next[0] - (main_current[0] + E::ONE);
        result[1] = main_next[1] - E::ZERO * main_current[1];
        result[2] = main_next[2] - E::ONE * main_current[2];
        result[3] = main_next[4] - (main_current[4] + E::ONE + E::ZERO);
        result[4] = main_current[4] - E::ONE;
    }

    fn evaluate_aux_transition<F, E>(&self, main_frame: &EvaluationFrame<F>, aux_frame: &EvaluationFrame<E>, _periodic_values: &[F], aux_rand_elements: &AuxTraceRandElements<E>, result: &mut [E])
    where F: FieldElement<BaseField = Felt>,
          E: FieldElement<BaseField = Felt> + ExtensionOf<F>,
    {
        let main_current = main_frame.current();
        let main_next = main_frame.next();
        let aux_current = aux_frame.current();
        let aux_next = aux_frame.next();
    }
}
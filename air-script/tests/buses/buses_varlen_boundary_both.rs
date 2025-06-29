use winter_air::{Air, AirContext, Assertion, AuxRandElements, EvaluationFrame, ProofOptions as WinterProofOptions, TransitionConstraintDegree, TraceInfo};
use winter_math::fields::f64::BaseElement as Felt;
use winter_math::{ExtensionOf, FieldElement, ToElements};
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
        self.inputs.write_into(target);
        self.outputs.write_into(target);
    }
}

impl ToElements<Felt> for PublicInputs {
    fn to_elements(&self) -> Vec<Felt> {
        let mut elements = Vec::new();
        self.inputs.iter().for_each(|row| elements.extend_from_slice(row));
        self.outputs.iter().for_each(|row| elements.extend_from_slice(row));
        elements
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

    pub fn bus_multiset_boundary_varlen<'a, const N: usize, I: IntoIterator<Item = &'a [Felt; N]> + Clone, E: FieldElement<BaseField = Felt>>(aux_rand_elements: &AuxRandElements<E>, public_inputs: &I) -> E {
        let mut bus_p_last: E = E::ONE;
        let rand = aux_rand_elements.rand_elements();
        for row in public_inputs.clone().into_iter() {
            let mut p_last = rand[0];
            for (c, p_i) in row.iter().enumerate() {
                p_last += E::from(*p_i) * rand[c + 1];
            }
            bus_p_last *= p_last;
        }
        bus_p_last
    }

    pub fn bus_logup_boundary_varlen<'a, const N: usize, I: IntoIterator<Item = &'a [Felt; N]> + Clone, E: FieldElement<BaseField = Felt>>(aux_rand_elements: &AuxRandElements<E>, public_inputs: &I) -> E {
        let mut bus_q_last = E::ZERO;
        let rand = aux_rand_elements.rand_elements();
        for row in public_inputs.clone().into_iter() {
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

    fn get_aux_assertions<E: FieldElement<BaseField = Felt>>(&self, aux_rand_elements: &AuxRandElements<E>) -> Vec<Assertion<E>> {
        let mut result = Vec::new();
        result.push(Assertion::single(0, 0, Self::bus_multiset_boundary_varlen(aux_rand_elements, &self.inputs.iter())));
        result.push(Assertion::single(1, 0, Self::bus_logup_boundary_varlen(aux_rand_elements, &self.inputs.iter())));
        result.push(Assertion::single(0, self.last_step(), Self::bus_multiset_boundary_varlen(aux_rand_elements, &self.outputs.iter())));
        result.push(Assertion::single(1, self.last_step(), Self::bus_logup_boundary_varlen(aux_rand_elements, &self.outputs.iter())));
        result
    }

    fn evaluate_transition<E: FieldElement<BaseField = Felt>>(&self, frame: &EvaluationFrame<E>, periodic_values: &[E], result: &mut [E]) {
        let main_current = frame.current();
        let main_next = frame.next();
    }

    fn evaluate_aux_transition<F, E>(&self, main_frame: &EvaluationFrame<F>, aux_frame: &EvaluationFrame<E>, _periodic_values: &[F], aux_rand_elements: &AuxRandElements<E>, result: &mut [E])
    where F: FieldElement<BaseField = Felt>,
          E: FieldElement<BaseField = Felt> + ExtensionOf<F>,
    {
        let main_current = main_frame.current();
        let main_next = main_frame.next();
        let aux_current = aux_frame.current();
        let aux_next = aux_frame.next();
        result[0] = ((aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1]) * E::from(main_current[0]) + E::ONE - E::from(main_current[0])) * aux_current[0] - ((aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1]) * (E::from(main_current[0]) - E::ONE) + E::ONE - (E::from(main_current[0]) - E::ONE)) * aux_next[0];
        result[1] = (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * aux_current[1] + (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * E::from(main_current[0]) + (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * E::from(main_current[0]) - ((aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * aux_next[1] + (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * (aux_rand_elements.rand_elements()[0] + E::ONE * aux_rand_elements.rand_elements()[1] + E::from(Felt::new(2_u64)) * aux_rand_elements.rand_elements()[2]) * E::from(Felt::new(2_u64)));
    }
}
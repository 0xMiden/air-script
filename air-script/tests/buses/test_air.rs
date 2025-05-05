use winter_air::{Air, AuxRandElements};
use winter_math::fields::f64::BaseElement as Felt;
use winterfell::{matrix::ColMatrix, AuxTraceWithMetadata, Trace, TraceTable};

use crate::{
    buses::buses_complex::{BusesAir, PublicInputs},
    helpers::{AirTester, MyTraceTable},
};

#[derive(Clone)]
struct BusesAirTester {}

impl AirTester for BusesAirTester {
    type PubInputs = PublicInputs;

    fn build_main_trace(&self, length: usize) -> MyTraceTable {
        let trace_width = 5;
        let start = Felt::new(0);
        let mut trace = TraceTable::new(trace_width, length);

        trace.fill(
            |state| {
                state[0] = start;
                state[1] = start;
                state[2] = start;
                state[3] = start;
                state[4] = start;
            },
            |_, state| {
                state[0] = Felt::new(1) - state[0];
                state[1] = Felt::new(1) - state[1];
                state[2] = Felt::new(1) - state[2];
                state[3] = Felt::new(1) - state[3];
                state[4] = Felt::new(1) - state[4];
            },
        );

        MyTraceTable::new(trace, 2)
    }

    fn public_inputs(&self) -> PublicInputs {
        let zero = Felt::new(0);
        PublicInputs::new([zero; 2])
    }

    fn build_aux_trace(&self, length: usize) -> Option<AuxTraceWithMetadata<Felt>> {
        let aux_trace_width = 2;
        let num_rand_values = 3;
        let mut aux_trace = ColMatrix::new(vec![vec![Felt::new(0); length]; aux_trace_width]);
        aux_trace.update_row(0, &[Felt::new(1), Felt::new(0)]);
        aux_trace.update_row(length - 2, &[Felt::new(1), Felt::new(0)]);
        let aux_rand_elements = AuxRandElements::new(vec![Felt::new(0); num_rand_values]);

        let aux_trace_with_meta = AuxTraceWithMetadata {
            aux_trace,
            aux_rand_elements,
        };
        Some(aux_trace_with_meta)
    }
}

#[test]
fn test_buses_air() {
    let air_tester = Box::new(BusesAirTester {});
    let length = 1024;

    let main_trace = air_tester.build_main_trace(length);
    let aux_trace = air_tester.build_aux_trace(length);
    let pub_inputs = air_tester.public_inputs();
    let trace_info = air_tester.build_trace_info(length);
    let options = air_tester.build_proof_options();

    let air = BusesAir::new(trace_info, pub_inputs, options);
    main_trace.validate::<BusesAir, Felt>(&air, aux_trace.as_ref());
}

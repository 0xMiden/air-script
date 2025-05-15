use winter_air::Air;
use winter_math::fields::f64::BaseElement as Felt;
use winterfell::{Trace, TraceTable};

use crate::{
    helpers::{AirTester, MyTraceTable},
    selectors::selectors_with_evaluators_with_mir::{PublicInputs, SelectorsAir},
};

#[derive(Clone)]
struct SelectorsAirTester {}

impl AirTester for SelectorsAirTester {
    type PubInputs = PublicInputs;

    fn build_main_trace(&self, length: usize) -> MyTraceTable {
        let trace_width = 4;
        let mut trace = TraceTable::new(trace_width, length);
        let start = Felt::new(0);

        trace.fill(
            |state| {
                state[0] = start;
                state[1] = start;
                state[2] = start;
                state[3] = Felt::new(0);
            },
            |_, state| {
                state[3] = Felt::new(1);
            },
        );

        MyTraceTable::new(trace, 0)
    }

    fn public_inputs(&self) -> PublicInputs {
        let zero = Felt::new(0);
        PublicInputs::new([zero; 16])
    }
}

#[test]
fn test_selectors_with_evaluators_air() {
    let air_tester = Box::new(SelectorsAirTester {});
    let length = 1024;

    let main_trace = air_tester.build_main_trace(length);
    let aux_trace = air_tester.build_aux_trace(length);
    let pub_inputs = air_tester.public_inputs();
    let trace_info = air_tester.build_trace_info(length);
    let options = air_tester.build_proof_options();

    let air = SelectorsAir::new(trace_info, pub_inputs, options);
    main_trace.validate::<SelectorsAir, Felt>(&air, aux_trace.as_ref());
}

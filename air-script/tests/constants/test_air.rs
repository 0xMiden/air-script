use winter_air::Air;
use winter_math::fields::f64::BaseElement as Felt;
use winterfell::{Trace, TraceTable};

use crate::{
    constants::constants::{ConstantsAir, PublicInputs},
    helpers::{AirTester, MyTraceTable},
};

#[derive(Clone)]
struct ConstantsAirTester {}

impl AirTester for ConstantsAirTester {
    type PubInputs = PublicInputs;

    fn build_main_trace(&self, length: usize) -> MyTraceTable {
        let trace_width = 7;
        let mut trace = TraceTable::new(trace_width, length);

        trace.fill(
            |state| {
                state[0] = Felt::new(1);
                state[1] = Felt::new(1);
                state[2] = Felt::new(0);
                state[3] = Felt::new(1);
                state[4] = Felt::new(1);
                state[5] = Felt::new(0);
                state[6] = Felt::new(0);
            },
            |_, state| {
                state[0] += Felt::new(1);
                state[1] = Felt::new(0);
                state[5] += Felt::new(1);
            },
        );

        MyTraceTable::new(trace, 0)
    }

    fn public_inputs(&self) -> PublicInputs {
        let zero = Felt::new(0);
        PublicInputs::new([zero; 4], [zero; 4], [zero; 4], [zero; 20])
    }
}

#[test]
fn test_constants_air() {
    let air_tester = Box::new(ConstantsAirTester {});
    let length = 1024;

    let main_trace = air_tester.build_main_trace(length);
    let aux_trace = air_tester.build_aux_trace(length);
    let pub_inputs = air_tester.public_inputs();
    let trace_info = air_tester.build_trace_info(length);
    let options = air_tester.build_proof_options();

    let air = ConstantsAir::new(trace_info, pub_inputs, options);
    main_trace.validate::<ConstantsAir, Felt>(&air, aux_trace.as_ref());
}

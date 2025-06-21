#[allow(unused_imports)]
use winter_air::{Air, AuxRandElements};
use winter_math::fields::f64::BaseElement as Felt;
#[allow(unused_imports)]
use winterfell::{AuxTraceWithMetadata, Trace, TraceTable, matrix::ColMatrix};

use crate::{
    bitwise::bitwise::{BitwiseAir, PublicInputs},
    helpers::{AirTester, MyTraceTable},
};

#[derive(Clone)]
struct BitwiseAirTester {}

impl AirTester for BitwiseAirTester {
    type PubInputs = PublicInputs;

    fn build_main_trace(&self, length: usize) -> MyTraceTable {
        let trace_width = 14;
        let mut trace = TraceTable::new(trace_width, length);
        let start = Felt::new(0);

        trace.fill(
            |state| {
                state[0] = start;
                state[1] = start;
                state[2] = start;
                state[3] = start;
                state[4] = start;
                state[5] = start;
                state[6] = start;
                state[7] = start;
                state[8] = start;
                state[9] = start;
                state[10] = start;
                state[11] = start;
                state[12] = start;
                state[13] = start;
            },
            |_, state| {},
        );

        MyTraceTable::new(trace, 0)
    }

    fn public_inputs(&self) -> PublicInputs {
        let zero = Felt::new(0);
        PublicInputs::new([zero; 16])
    }
}

#[test]
fn test_bitwise_air() {
    let air_tester = Box::new(BitwiseAirTester {});
    let length = 1024;

    let main_trace = air_tester.build_main_trace(length);
    let aux_trace = air_tester.build_aux_trace(length);
    let pub_inputs = air_tester.public_inputs();
    let trace_info = air_tester.build_trace_info(length);
    let options = air_tester.build_proof_options();

    let air = BitwiseAir::new(trace_info, pub_inputs, options);
    main_trace.validate::<BitwiseAir, Felt>(&air, aux_trace.as_ref());
}

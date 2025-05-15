#[allow(unused_imports)]
use winter_air::{Air, AuxRandElements};
use winter_math::fields::f64::BaseElement as Felt;
#[allow(unused_imports)]
use winterfell::{matrix::ColMatrix, AuxTraceWithMetadata, Trace, TraceTable};

use crate::{
    fibonacci::fibonacci::{FibonacciAir, PublicInputs},
    helpers::{AirTester, MyTraceTable},
};

#[derive(Clone)]
struct FibonacciAirTester {}

impl AirTester for FibonacciAirTester {
    type PubInputs = PublicInputs;

    fn build_main_trace(&self, length: usize) -> MyTraceTable {
        let trace_width = 2;
        let mut trace = TraceTable::new(trace_width, length);
        let start_a = Felt::new(1);
        let start_b = Felt::new(1);

        trace.fill(
            |state| {
                state[0] = start_a;
                state[1] = start_b;
            },
            |_, state| {
                let cur_a = state[0];
                let cur_b = state[1];
                state[1] = cur_a + cur_b;
                state[0] = cur_b;
            },
        );

        MyTraceTable::new(trace, 0)
    }

    fn public_inputs(&self) -> PublicInputs {
        let one = Felt::new(1);
        let last = Felt::new(2178309); // 32nd Fibonacci number
        PublicInputs::new([one, one], [last])
    }
}

#[test]
fn test_fibonacci_air() {
    let air_tester = Box::new(FibonacciAirTester {});
    let length = 32;

    let main_trace = air_tester.build_main_trace(length);
    let aux_trace = air_tester.build_aux_trace(length);
    let pub_inputs = air_tester.public_inputs();
    let trace_info = air_tester.build_trace_info(length);
    let options = air_tester.build_proof_options();

    let air = FibonacciAir::new(trace_info, pub_inputs, options);
    main_trace.validate::<FibonacciAir, Felt>(&air, aux_trace.as_ref());
}

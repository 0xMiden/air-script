use air_codegen_masm::constants;
use miden_assembly::Assembler;
use miden_processor::{
    math::{Felt, FieldElement},
    AdviceInputs, Kernel, MemAdviceProvider, Process, QuadExtension, StackInputs,
};
use winter_air::{Assertion, ConstraintDivisor};

mod utils;
use utils::{codegen, test_code, to_stack_order, Data};

static SIMPLE_INTEGRITY_AIR: &str = "
def SimpleIntegrityAux

trace_columns {
    main: [a],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a = 0;
}";

#[test]
fn test_integrity_divisor() {
    let code = codegen(SIMPLE_INTEGRITY_AIR);

    let exemptions = 2;
    let one = QuadExtension::new(Felt::new(1), Felt::ZERO);
    let a = QuadExtension::new(Felt::new(3), Felt::ZERO);
    let a_prime = a;
    let z = QuadExtension::new(Felt::new(5), Felt::new(7));

    for power in 3..32 {
        let trace_len = 2u64.pow(power);

        let code = test_code(
            code.clone(),
            vec![
                Data {
                    data: to_stack_order(&[a, a_prime]),
                    address: constants::OOD_FRAME_ADDRESS,
                    descriptor: "main_trace",
                },
                Data {
                    data: to_stack_order(&[one; 2]),
                    address: constants::COMPOSITION_COEF_ADDRESS,
                    descriptor: "composition_coefficients",
                },
            ],
            trace_len,
            z,
            &["cache_z_exp", "compute_integrity_constraint_divisor"],
        );
        let program = Assembler::default().compile(code).unwrap();

        let constraint_divisor =
            ConstraintDivisor::<Felt>::from_transition(trace_len.try_into().unwrap(), exemptions);
        let divisor = constraint_divisor.evaluate_at(z);

        let mut process: Process<MemAdviceProvider> = Process::new(
            Kernel::new(&[]),
            StackInputs::new(vec![]),
            AdviceInputs::default().into(),
        );
        let program_outputs = process.execute(&program).expect("execution failed");
        let result_stack = program_outputs.stack();

        // results are in stack-order
        #[rustfmt::skip]
        let expected = to_stack_order(&[
            divisor,
        ]);

        assert!(
            result_stack
                .iter()
                .zip(expected.iter())
                .all(|(l, r)| l == r),
            "results don't match trace_len={} power={} result={:?} expected={:?}",
            trace_len,
            power,
            result_stack,
            expected,
        );
    }
}

static SIMPLE_BOUNDARY_AIR: &str = "
def MixedBoundaryAux

trace_columns {
    main: [a, b],
}

public_inputs {
    stack_inputs: [1],
}

boundary_constraints {
    enf a.first = 3;
    enf b.last = 5;
}

integrity_constraints {
    enf a = 0;
}";

// ignore `aux` tests
#[test]
fn test_boundary_divisor() {
    let code = codegen(SIMPLE_BOUNDARY_AIR);

    let exemptions = 2;
    let one = QuadExtension::new(Felt::new(1), Felt::ZERO);
    let a = QuadExtension::new(Felt::new(13), Felt::ZERO);
    let a_prime = a;
    let a_column = 0;
    let b = QuadExtension::new(Felt::new(17), Felt::ZERO);
    let b_prime = b;
    let b_column = 1;
    let z = QuadExtension::new(Felt::new(19), Felt::new(23));

    for power in 3..32 {
        let trace_len = 2u64.pow(power);
        let first_step = 0;
        let last_step: usize = (trace_len - exemptions).try_into().unwrap();

        let code = test_code(
            code.clone(),
            vec![
                Data {
                    data: to_stack_order(&[a, a_prime, b, b_prime]),
                    address: constants::OOD_FRAME_ADDRESS,
                    descriptor: "main_trace",
                },
                Data {
                    data: to_stack_order(&[]),
                    address: constants::OOD_AUX_FRAME_ADDRESS,
                    descriptor: "aux_trace",
                },
                Data {
                    data: to_stack_order(&[one; 5]),
                    address: constants::COMPOSITION_COEF_ADDRESS,
                    descriptor: "composition_coefficients",
                },
            ],
            trace_len,
            z,
            &[
                "cache_z_exp",
                // The exemption point is cached as part of the integrity constraint computation
                "compute_integrity_constraint_divisor",
                "evaluate_boundary_constraints",
            ],
        );
        let program = Assembler::default().compile(code).unwrap();

        let a_first_assertion = Assertion::<Felt>::single(a_column, first_step, Felt::new(3));
        let b_last_assertion = Assertion::<Felt>::single(b_column, last_step, Felt::new(5));

        let mut result = QuadExtension::ZERO;
        for (assertion, numerator) in [
            (a_first_assertion, a - QuadExtension::from(Felt::new(3))),
            (b_last_assertion, b - QuadExtension::from(Felt::new(5))),
        ] {
            let divisor = ConstraintDivisor::<Felt>::from_assertion(
                &assertion,
                trace_len.try_into().unwrap(),
            );
            let group_result = numerator / divisor.evaluate_at(z);
            result += group_result;
        }

        let mut process: Process<MemAdviceProvider> = Process::new(
            Kernel::new(&[]),
            StackInputs::new(vec![]),
            AdviceInputs::default().into(),
        );
        let program_outputs = process.execute(&program).expect("execution failed");
        let result_stack = program_outputs.stack();

        // results are in stack-order
        #[rustfmt::skip]
        let expected = to_stack_order(&[
            result,
        ]);

        assert!(
            result_stack
                .iter()
                .zip(expected.iter())
                .all(|(l, r)| l == r),
            "results don't match trace_len={} power={} result={:?} expected={:?}",
            trace_len,
            power,
            result_stack,
            expected,
        );
    }
}

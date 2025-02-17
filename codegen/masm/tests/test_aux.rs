use air_codegen_masm::constants;
use miden_assembly::Assembler;
use miden_processor::{
    math::{Felt, FieldElement},
    AdviceInputs, Kernel, MemAdviceProvider, Process, QuadExtension, StackInputs,
};

mod utils;
use utils::{codegen, test_code, to_stack_order, Data};

static SIMPLE_AUX_AIR: &str = "
def SimpleAux

trace_columns {
    main: [a],
    aux: [b],
}

public_inputs {
    stack_inputs: [16],
}

boundary_constraints {
    enf a.first = 0;
}

integrity_constraints {
    enf a + a = 0;
    enf a - a = 0;
    enf a * a = 0;

    enf b + a = 0;
    enf b - a = 0;
    enf b * a = 0;
}";

// Ignore `aux` tests
#[test]
#[ignore]
fn test_simple_aux() {
    let code = codegen(SIMPLE_AUX_AIR);

    let trace_len = 2u64.pow(4);
    let one = QuadExtension::new(Felt::new(1), Felt::ZERO);
    let z = one;
    let a = QuadExtension::new(Felt::new(3), Felt::ZERO);
    let b = QuadExtension::new(Felt::new(7), Felt::ZERO);
    let a_prime = a;
    let b_prime = b;

    let code = test_code(
        code,
        vec![
            Data {
                data: to_stack_order(&[a, a_prime]),
                address: constants::OOD_FRAME_ADDRESS,
                descriptor: "main_trace",
            },
            Data {
                data: to_stack_order(&[b, b_prime]),
                address: constants::OOD_AUX_FRAME_ADDRESS,
                descriptor: "aux_trace",
            },
            Data {
                data: to_stack_order(&[one; 6]),
                address: constants::COMPOSITION_COEF_ADDRESS,
                descriptor: "composition_coefficients",
            },
        ],
        trace_len,
        z,
        &["compute_integrity_constraints"],
    );
    let program = Assembler::default().compile(code).unwrap();

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
        b * a, // result: 21
        b - a, // result: 4
        b + a, // result: 10
        a * a, // result: 9
        a - a, // result: 0
        a + a, // result: 6
    ]);

    assert!(
        result_stack
            .iter()
            .zip(expected.iter())
            .all(|(l, r)| l == r),
        "results don't match result={:?} expected={:?}",
        result_stack,
        expected,
    );
}

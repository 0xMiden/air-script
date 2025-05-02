use air_codegen_masm::constants;
use miden_assembly::Assembler;
use miden_processor::{
    math::{Felt, FieldElement},
    AdviceInputs, Kernel, MemAdviceProvider, Process, QuadExtension, StackInputs,
};

mod utils;
use utils::{codegen, test_code, to_stack_order, Data};

static SIMPLE_BOUNDARY_AIR: &str = "
def SimpleBoundary

trace_columns {
    main: [a, b, len],
}

public_inputs {
    target: [1],
}

boundary_constraints {
    enf a.first = 1;
    enf b.first = 1;

    enf len.first = 0;
    enf len.last = target[0];
}

integrity_constraints {
    enf a' = a + b;
    enf b' = a;
}";

#[test]
fn test_simple_boundary() {
    let code = codegen(SIMPLE_BOUNDARY_AIR);

    let trace_len = 32u64;
    let one = QuadExtension::ONE;
    let z = one;
    let a = QuadExtension::new(Felt::new(514229), Felt::ZERO);
    let b = QuadExtension::new(Felt::new(317811), Felt::ZERO);
    let len = QuadExtension::new(Felt::new(27), Felt::ZERO);
    let a_prime = QuadExtension::new(Felt::new(514229 + 317811), Felt::ZERO);
    let b_prime = a;

    let code = test_code(
        code,
        vec![
            Data {
                data: to_stack_order(&[a, a_prime, b, b_prime, len, len]),
                address: constants::OOD_FRAME_ADDRESS,
                descriptor: "main_trace",
            },
            Data {
                data: to_stack_order(&[one; 6]),
                address: constants::COMPOSITION_COEF_ADDRESS,
                descriptor: "composition_coefficients",
            },
            Data {
                data: to_stack_order(&[len]),
                address: constants::PUBLIC_INPUTS_ADDRESS,
                descriptor: "public_inputs",
            },
        ],
        trace_len,
        z,
        &[
            "compute_boundary_constraints_main_first",
            "compute_boundary_constraints_main_last",
            // there are no auxiliary boundary constraints on this AIR defnition
            // "compute_boundary_constraints_aux_first",
            // "compute_boundary_constraints_aux_last",
        ],
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
        QuadExtension::ZERO,    // enf len.last = target[0]
        len,                    // enf len.first = 0
        b - QuadExtension::ONE, // enf b.first = 1
        a - QuadExtension::ONE, // enf a.first = 1
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

static COMPLEX_BOUNDARY_AIR: &str = "
def ComplexBoundary

const A = 1;
const B = [0, 1];
const C = [[1, 2], [2, 0]];

trace_columns {
    main: [a, b, c, d, e[2]],
}

buses {
    multiset f,
}

public_inputs {
    stack_inputs: [2],
    stack_outputs: [2],
}


boundary_constraints {
    enf a.first = stack_inputs[0];
    enf b.first = stack_inputs[1];
    enf a.last = stack_outputs[0];
    enf b.last = stack_outputs[1];

    enf c.first = (B[0] - C[1][1]) * A;
    enf d.first = 1;

    enf e[0].first = 0;
    enf e[1].first = 1;

    enf f.first = null;
    enf f.last = null;
}

integrity_constraints {
    enf a + b = 0;
}";

#[test]
fn test_complex_boundary() {
    let code = codegen(COMPLEX_BOUNDARY_AIR);

    let trace_len = 32u64;
    let one = QuadExtension::new(Felt::new(1), Felt::ZERO);
    let z = one;

    let public_inputs = [
        // stack_inputs
        QuadExtension::new(Felt::new(2), Felt::ZERO),
        QuadExtension::new(Felt::new(3), Felt::ZERO),
        // stack_outputs
        QuadExtension::new(Felt::new(5), Felt::ZERO),
        QuadExtension::new(Felt::new(7), Felt::ZERO),
    ];

    let a = QuadExtension::new(Felt::new(11), Felt::ZERO);
    let b = QuadExtension::new(Felt::new(13), Felt::ZERO);
    let c = QuadExtension::new(Felt::new(17), Felt::ZERO);
    let d = QuadExtension::new(Felt::new(19), Felt::ZERO);
    let e = [
        QuadExtension::new(Felt::new(23), Felt::ZERO),
        QuadExtension::new(Felt::new(29), Felt::ZERO),
    ];
    let f = QuadExtension::new(Felt::new(31), Felt::ZERO);

    let rand = [
        QuadExtension::new(Felt::new(37), Felt::ZERO),
        QuadExtension::new(Felt::new(41), Felt::ZERO),
    ];

    let a_prime = a + one;
    let b_prime = b + one;
    let c_prime = c + one;
    let d_prime = d + one;
    let e_prime = [e[0] + one, e[1] + one];
    let f_prime = f + one;

    let code = test_code(
        code,
        vec![
            Data {
                data: to_stack_order(&[
                    a, a_prime, b, b_prime, c, c_prime, d, d_prime, e[0], e_prime[0], e[1],
                    e_prime[1],
                ]),
                address: constants::OOD_FRAME_ADDRESS,
                descriptor: "main_trace",
            },
            Data {
                data: to_stack_order(&[f, f_prime]),
                address: constants::OOD_AUX_FRAME_ADDRESS,
                descriptor: "aux_trace",
            },
            Data {
                data: to_stack_order(&public_inputs),
                address: constants::PUBLIC_INPUTS_ADDRESS,
                descriptor: "public_inputs",
            },
            Data {
                data: to_stack_order(&[one; 12]),
                address: constants::COMPOSITION_COEF_ADDRESS,
                descriptor: "composition_coefficients",
            },
            Data {
                data: to_stack_order(&rand),
                address: constants::AUX_RAND_ELEM_PTR,
                descriptor: "aux_random_elements",
            },
        ],
        trace_len,
        z,
        &[
            "compute_boundary_constraints_main_first",
            "compute_boundary_constraints_main_last",
            "compute_boundary_constraints_aux_first",
            "compute_boundary_constraints_aux_last",
        ],
    );
    let program = Assembler::default().compile(code).unwrap();

    let mut process: Process<MemAdviceProvider> = Process::new(
        Kernel::new(&[]),
        StackInputs::new(vec![]),
        AdviceInputs::default().into(),
    );
    let program_outputs = process.execute(&program).expect("execution failed");
    let result_stack = program_outputs.stack();

    // Note: The order of the results is _not_ the same as the definition order in the AirScript.
    // The order below is:
    //
    // 1. First row boundary constraints for the MAIN trace
    // 2. Last row boundary constraints for the MAIN trace
    // 3. First row boundary constraints for the AUX trace
    // 4. Last row boundary constraints for the AUX trace
    //
    // Results are in stack-order.
    #[rustfmt::skip]
    let expected = to_stack_order(&[
        // last row aux trace
        f - one,          // enf f.last = null

        // first row aux trace
        f - one,          // enf f.first = null

        // last row main trace
        b - public_inputs[3], // enf b.last = stack_outputs[1]
        a - public_inputs[2], // enf a.last = stack_outputs[0]

        // first row main trace
        e[1] - one,           // enf e[1].first = 1
        e[0],                 // enf e[0].first = 0

        d - one,              // enf d.first = 1
        c,                    // enf c.first = (B[0] - C[1][1]) * A

        b - public_inputs[1], // enf b.first = stack_inputs[1]
        a - public_inputs[0], // enf a.first = stack_inputs[0]
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

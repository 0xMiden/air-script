use air_pass::Pass;
use miden_diagnostics::SourceSpan;
use pretty_assertions::assert_eq;

use super::ParseTest;
use crate::{ast::*, transforms::ConstantPropagation};

#[test]
fn test_constant_propagation() {
    let root = r#"
    def root

    use lib::*;

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    const A = [2, 4, 6, 8];
    const B = [[1, 1], [2, 2]];
    const TWO = 2;

    integrity_constraints {
        enf test_constraint(b);
        let x = 2^EXP;
        let y = A[0..2];
        enf a + y[1] = c + (x + 1);
    }

    boundary_constraints {
        let x = B[0];
        enf a.first = x[0];
    }
    "#;
    let lib = r#"
    mod lib

    const EXP = 2;

    ev test_constraint([b0, b1]) {
        let x = EXP;
        let y = 2^x;
        enf b0 + x = b1 + y;
    }"#;

    let test = ParseTest::new();
    let path = std::env::current_dir().unwrap().join("lib.air");
    test.add_virtual_file(path, lib.to_string());

    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pass = ConstantPropagation::new(&test.diagnostics);
    let program = pass.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    expected.constants.insert(ident!(root, A), constant!(A = [2, 4, 6, 8]));
    expected.constants.insert(ident!(root, B), constant!(B = [[1, 1], [2, 2]]));
    expected.constants.insert(ident!(lib, EXP), constant!(EXP = 2));
    // When constant propagation is done, the boundary constraints should look like:
    //     enf a.first = 1
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(a, Boundary::First, Type::Felt), int!(1))));
    // When constant propagation is done, the integrity constraints should look like:
    //     enf test_constraint(b)
    //     enf a + 4 = c + 5
    expected
        .integrity_constraints
        .push(enforce!(call!(lib::test_constraint(expr!(access!(b, Type::Vector(2)))))));
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(a, Type::Felt), int!(4)),
        add!(access!(c, Type::Felt), int!(5))
    )));
    // The test_constraint function should look like:
    //     enf b0 + 2 = b1 + 4
    let body = vec![enforce!(eq!(
        add!(access!(b0, Type::Felt), int!(2)),
        add!(access!(b1, Type::Felt), int!(4))
    ))];
    expected.evaluators.insert(
        function_ident!(lib, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(b0, 1), (b1, 1)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

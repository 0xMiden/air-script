use air_pass::Pass;
use miden_diagnostics::SourceSpan;
use pretty_assertions::assert_eq;

use super::ParseTest;
use crate::{
    ast::*,
    transforms::{ConstantPropagation, Inlining},
};

/// This test inlines an evaluator function into the root
/// integrity constraints. The evaluator is called with a
/// single trace column binding representing two columns
/// in the main trace, which is split into two individual
/// bindings via the evaluator function signature.
///
/// It is expected that the resulting evaluator function
/// body will have its references to those parameters rewritten
/// to refer to the input binding, but with appropriate accesses
/// inserted to match the semantics of the function signature
#[test]
fn test_inlining_with_evaluator_split_input_binding() {
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

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

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
    // When constant propagation and inlining is done, the boundary constraints should look like:
    //     enf a.first = 1
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(a, Boundary::First, Type::Felt), int!(1))));
    // When constant propagation and inlining is done, the integrity constraints should look like:
    //     enf b[0] + 2 = b[1] + 4
    //     enf a + 4 = c + 5
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(b[0], Type::Felt), int!(2)),
        add!(access!(b[1], Type::Felt), int!(4))
    )));
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(a, Type::Felt), int!(4)),
        add!(access!(c, Type::Felt), int!(5))
    )));
    // The test_constraint function before inlining should look like:
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

/// This test inlines an evaluator function into the root
/// integrity constraints. The evaluator is called with two
/// disjoint bindings representing three columns from the main
/// trace, packed using a vector literal. The evaluator function
/// then destructures that vector into a set of two bindings which
/// recombines the input columns into different groupings, and then
/// expresses a constraint using accesses into those groups.
///
/// It is expected that the resulting evaluator function
/// body will have its references to those parameters rewritten
/// to accesses relative to the input bindings, or to direct accesses
/// to the corresponding trace segment declaration.
#[test]
fn test_inlining_with_vector_literal_binding_regrouped() {
    let root = r#"
    def root

    use lib::*;

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint([clk, b]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    "#;
    let lib = r#"
    mod lib

    ev test_constraint([pair[2], b1]) {
        enf pair[0] + pair[1] = b1;
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

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf clk + b[0] = b[1]
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(clk, Type::Felt), access!(b[0], Type::Felt)),
        access!(b[1], Type::Felt)
    )));
    // The test_constraint function before inlining should look like:
    //     enf pair[0] + pair[1] = b1
    let body = vec![enforce!(eq!(
        add!(access!(pair[0], Type::Felt), access!(pair[1], Type::Felt)),
        access!(b1, Type::Felt)
    ))];
    expected.evaluators.insert(
        function_ident!(lib, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(pair, 2), (b1, 1)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

/// This test checks that there are no assumptions about the ordering of
/// arguments to an evaluator, i.e. there is no assumption that two consecutive
/// columns necessarily appear in that order in the trace_columns declaration
#[test]
fn test_inlining_with_vector_literal_binding_unordered() {
    let root = r#"
    def root

    use lib::*;

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint([b, clk]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;
    let lib = r#"
    mod lib

    ev test_constraint([b0, pair[2]]) {
        enf pair[1] + b0 = pair[0];
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

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf clk + b[0] = b[1]
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(clk, Type::Felt), access!(b[0], Type::Felt)),
        access!(b[1], Type::Felt)
    )));
    // The test_constraint function before inlining should look like:
    //     enf pair[1] + b0 = pair[0]
    let body = vec![enforce!(eq!(
        add!(access!(pair[1], Type::Felt), access!(b0, Type::Felt)),
        access!(pair[0], Type::Felt)
    ))];
    expected.evaluators.insert(
        function_ident!(lib, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(b0, 1), (pair, 2)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

/// This test checks the behavior when there are not only disjoint args/params
/// in a call to an evaluator, but that the number of arguments and parameters
/// is different, with more input arguments than parameter bindings.
#[test]
fn test_inlining_with_vector_literal_binding_different_arity_many_to_few() {
    let root = r#"
    def root

    use lib::*;

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint([clk, b, a]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;
    let lib = r#"
    mod lib

    ev test_constraint([pair[3], foo]) {
        enf pair[0] + pair[1] = foo + pair[2];
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

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf clk + b[0] = a + b[1]
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(clk, Type::Felt), access!(b[0], Type::Felt)),
        add!(access!(a, Type::Felt), access!(b[1], Type::Felt))
    )));
    // The test_constraint function before inlining should look like:
    //     enf pair[0] + pair[1] = a + pair[2]
    let body = vec![enforce!(eq!(
        add!(access!(pair[0], Type::Felt), access!(pair[1], Type::Felt)),
        add!(access!(foo, Type::Felt), access!(pair[2], Type::Felt))
    ))];
    expected.evaluators.insert(
        function_ident!(lib, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(pair, 3), (foo, 1)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

/// This test checks the behavior when there are not only disjoint args/params
/// in a call to an evaluator, but that the number of arguments and parameters
/// is different, with more parameter bindings than input arguments.
#[test]
fn test_inlining_with_vector_literal_binding_different_arity_few_to_many() {
    let root = r#"
    def root

    use lib::*;

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint([b, a]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;
    let lib = r#"
    mod lib

    ev test_constraint([x, y, z]) {
        enf x + y = z;
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

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf b[0] + b[1] = a
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(b[0], Type::Felt), access!(b[1], Type::Felt)),
        access!(a, Type::Felt)
    )));
    // The test_constraint function before inlining should look like:
    //     enf x + y = z
    let body = vec![enforce!(eq!(
        add!(access!(x, Type::Felt), access!(y, Type::Felt)),
        access!(z, Type::Felt)
    ))];
    expected.evaluators.insert(
        function_ident!(lib, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(x, 1), (y, 1), (z, 1)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

/// This test checks the behavior when inlining across multiple modules with
/// nested calls to evaluators, with a mix of parameter/argument binding configurations
#[test]
fn test_inlining_across_modules_with_nested_evaluators_variant1() {
    let root = r#"
    def root

    use lib1::test_constraint;

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint([clk, b, a]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;
    let lib1 = r#"
    mod lib1

    use lib2::*;

    ev test_constraint([tuple[3], z]) {
        enf helper_constraint([z, tuple[1..3]]);
    }"#;
    let lib2 = r#"
    mod lib2

    ev helper_constraint([x[2], y]) {
        enf x[0] + x[1] = y;
    }"#;

    let test = ParseTest::new();
    let path = std::env::current_dir().unwrap().join("lib1.air");
    test.add_virtual_file(path, lib1.to_string());
    let path = std::env::current_dir().unwrap().join("lib2.air");
    test.add_virtual_file(path, lib2.to_string());

    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf a + b[0] = b[1]
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(a, Type::Felt), access!(b[0], Type::Felt)),
        access!(b[1], Type::Felt)
    )));
    // The test_constraint function before inlining should look like:
    //     enf helper_constraint([z, tuple[1..3]])
    let body = vec![enforce!(call!(lib2::helper_constraint(vector!(
        access!(z, Type::Felt),
        slice!(tuple, 1..3, Type::Vector(2))
    ))))];
    expected.evaluators.insert(
        function_ident!(lib1, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(tuple, 3), (z, 1)])],
            body,
        ),
    );
    // The helper_constraint function before inlining should look like:
    //     enf x[0] + x[1] = y
    let body = vec![enforce!(eq!(
        add!(access!(x[0], Type::Felt), access!(x[1], Type::Felt)),
        access!(y, Type::Felt)
    ))];
    expected.evaluators.insert(
        function_ident!(lib2, helper_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(helper_constraint),
            vec![trace_segment!(0, "%0", [(x, 2), (y, 1)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

/// This test is like *_variant1, but with a different mix of parameter/argument configurations
#[test]
fn test_inlining_across_modules_with_nested_evaluators_variant2() {
    let root = r#"
    def root

    use lib1::test_constraint;

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint([clk, b[0..2], a]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;
    let lib1 = r#"
    mod lib1

    use lib2::*;

    ev test_constraint([tuple[3], z]) {
        enf helper_constraint([z, tuple[1], tuple[2..3]]);
    }"#;
    let lib2 = r#"
    mod lib2

    ev helper_constraint([x[2], y]) {
        enf x[0] + x[1] = y;
    }"#;

    let test = ParseTest::new();
    let path = std::env::current_dir().unwrap().join("lib1.air");
    test.add_virtual_file(path, lib1.to_string());
    let path = std::env::current_dir().unwrap().join("lib2.air");
    test.add_virtual_file(path, lib2.to_string());

    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf a + b[0] = b[1]
    expected.integrity_constraints.push(enforce!(eq!(
        add!(access!(a, Type::Felt), access!(b[0], Type::Felt)),
        access!(b[1], Type::Felt)
    )));
    // The test_constraint function before inlining should look like:
    //     enf helper_constraint([z, tuple[1..3]])
    let body = vec![enforce!(call!(lib2::helper_constraint(vector!(
        access!(z, Type::Felt),
        access!(tuple[1], Type::Felt),
        slice!(tuple, 2..3, Type::Vector(1))
    ))))];
    expected.evaluators.insert(
        function_ident!(lib1, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(tuple, 3), (z, 1)])],
            body,
        ),
    );
    // The helper_constraint function before inlining should look like:
    //     enf x[0] + x[1] = y
    let body = vec![enforce!(eq!(
        add!(access!(x[0], Type::Felt), access!(x[1], Type::Felt)),
        access!(y, Type::Felt)
    ))];
    expected.evaluators.insert(
        function_ident!(lib2, helper_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(helper_constraint),
            vec![trace_segment!(0, "%0", [(x, 2), (y, 1)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

/// This test verifies that constraint comprehensions (without a selector) are unrolled properly
/// during inlining
///
/// In this variant, we do not involve other modules to keep the test focused on just the
/// comprehension unrolling behavior. Other tests will expand on this to test it when combined
/// with other inlining behavior.
#[test]
fn test_inlining_constraint_comprehensions_no_selector() {
    let root = r#"
    def root

    const YS = [2, 4, 6, 8];

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        # We're expecting this to expand to:
        #
        #    enf b[0]' = 2
        #    enf b[1]' = 4
        #
        enf x' = y for (x, y) in (b, YS[0..2]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;

    let test = ParseTest::new();
    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected.constants.insert(ident!(root, YS), constant!(YS = [2, 4, 6, 8]));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf b[0]' = 2
    //     enf b[1]' = 4
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[0], 1, Type::Felt), int!(2))));
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[1], 1, Type::Felt), int!(4))));

    assert_eq!(program, expected);
}

/// This test verifies that constraint comprehensions (with a selector) are unrolled properly during
/// inlining
///
/// In this variant, we do not involve other modules to keep the test focused on just the
/// comprehension unrolling behavior. Other tests will expand on this to test it when combined
/// with other inlining behavior.
#[test]
fn test_inlining_constraint_comprehensions_with_selector() {
    let root = r#"
    def root

    const YS = [2, 4, 6, 8];

    trace_columns {
        main: [clk, a, b[2], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        # We're expecting this to expand to:
        #
        #    enf b[0]' = 2 when c
        #    enf b[1]' = 4 when c
        #
        enf x' = y for (x, y) in (b, YS[0..2]) when c;
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;

    let test = ParseTest::new();
    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected.constants.insert(ident!(root, YS), constant!(YS = [2, 4, 6, 8]));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 2), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf b[0]' = 2 when c
    //     enf b[1]' = 4 when c
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[0], 1, Type::Felt), int!(2)), when access!(c, Type::Felt)));
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[1], 1, Type::Felt), int!(4)), when access!(c, Type::Felt)));

    assert_eq!(program, expected);
}

/// This test verifies that constraint comprehensions (with a selector) are unrolled properly during
/// inlining. Specifically, in this case we expect that because the selector is constant, only
/// constraints for which the selector is "truthy" (i.e. non-zero) remain, and that the selector has
/// been elided.
///
/// In this variant, we do not involve other modules to keep the test focused on just the
/// comprehension unrolling behavior. Other tests will expand on this to test it when combined
/// with other inlining behavior.
#[test]
fn test_inlining_constraint_comprehensions_with_constant_selector() {
    let root = r#"
    def root

    const YS = [0, 4, 0, 8];

    trace_columns {
        main: [clk, a, b[4], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        # We're expecting this to expand to:
        #
        #    enf b[1]' = 4
        #    enf b[3]' = 8
        #
        enf x' = y for (x, y) in (b, YS) when y;
    }

    boundary_constraints {
        enf clk.first = 0;
    }
    "#;

    let test = ParseTest::new();
    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected.constants.insert(ident!(root, YS), constant!(YS = [0, 4, 0, 8]));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 4), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf b[1]' = 4
    //     enf b[3]' = 8
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[1], 1, Type::Felt), int!(4))));
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[3], 1, Type::Felt), int!(8))));

    assert_eq!(program, expected);
}

/// This test verifies that constraint comprehensions present in evaluators are inlined into call
/// sites correctly
///
/// This test exercises multiple edge cases in constant propagation/inlining in conjunction to make
/// sure that all of the pieces integrate together even in odd scenarios
#[test]
fn test_inlining_constraint_comprehensions_in_evaluator() {
    let root = r#"
    def root

    const YS = [0, 4, 0, 8];

    trace_columns {
        main: [clk, a, b[4], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint(b[1..4]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    ev test_constraint([i, j[2]]) {
        let ys = [x^2 for x in YS];
        let k = j[0];
        let l = j[1];
        let xs = [i, k, l];
        enf x' = y for (x, y) in (xs, ys[1..4]) when y;
    }"#;

    let test = ParseTest::new();
    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected.constants.insert(ident!(root, YS), constant!(YS = [0, 4, 0, 8]));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 4), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     enf b[1]' = 16
    //     enf b[3]' = 64
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[1], 1, Type::Felt), int!(16))));
    expected
        .integrity_constraints
        .push(enforce!(eq!(access!(b[3], 1, Type::Felt), int!(64))));
    // The evaluator definition is never modified by inlining, but is by constant propagation:
    //
    // ev test_constraint([i, j[2]]) {
    //     let k = j[0]
    //     let l = j[1]
    //     let xs = [i, k, l]
    //     enf x' = y for (x, y) in (xs, [16, 0, 64]) when y
    // }
    let body = vec![let_!(k = expr!(access!(j[0], Type::Felt))
        => let_!(l = expr!(access!(j[1], Type::Felt))
            => let_!(xs = vector!(access!(i, Type::Felt), access!(k, Type::Felt), access!(l, Type::Felt))
                => enforce_all!(lc!(((x, expr!(access!(xs, Type::Vector(3)))), (y, vector!(16, 0, 64)))
                    => eq!(access!(x, 1, Type::Felt), access!(y, Type::Felt)), when access!(y, Type::Felt)))))
    )];
    expected.evaluators.insert(
        function_ident!(root, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(i, 1), (j, 2)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

/// This test verifies that constraints involving let-bound, folded comprehensions behave as
/// expected
#[test]
fn test_inlining_constraints_with_folded_comprehensions_in_evaluator() {
    let root = r#"
    def root

    trace_columns {
        main: [clk, a, b[4], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        enf test_constraint(b[1..4]);
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    ev test_constraint([x, ys[2]]) {
        let y = sum([col^7 for col in ys]);
        let z = prod([col^7 for col in ys]);
        enf x = y + z;
    }"#;

    let test = ParseTest::new();
    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 4), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // When constant propagation and inlining is done, integrity_constraints should look like:
    //     let y =
    //         let %lc0 = b[2]^7
    //         let %lc1 = b[3]^7
    //         %lc0 + %lc1
    //     in
    //     let z =
    //         let %lc2 = b[2]^7
    //         let %lc3 = b[3]^7
    //         %lc2 * %lc3
    //     in
    //     enf b[1] = y + z
    expected
        .integrity_constraints
        .push(let_!(y = expr!(
            let_!("%lc0" = expr!(exp!(access!(b[2], Type::Felt), int!(7)))
            => let_!("%lc1" = expr!(exp!(access!(b[3], Type::Felt), int!(7)))
            => statement!(add!(access!("%lc0", Type::Felt), access!("%lc1", Type::Felt)))))
        ) =>
            let_!(z = expr!(
                let_!("%lc2" = expr!(exp!(access!(b[2], Type::Felt), int!(7)))
                => let_!("%lc3" = expr!(exp!(access!(b[3], Type::Felt), int!(7)))
                => statement!(mul!(access!("%lc2", Type::Felt), access!("%lc3", Type::Felt)))))
            ) =>
              enforce!(eq!(access!(b[1], Type::Felt), add!(access!(y, Type::Felt), access!(z, Type::Felt))))
            )
        ));
    // The evaluator definition is never modified by constant propagation or inlining
    let body = vec![
        let_!(y = expr!(call!(sum(expr!(lc!(((col, expr!(access!(ys, Type::Vector(2))))) => exp!(access!(col, Type::Felt), int!(7)))))))
            => let_!(z = expr!(call!(prod(expr!(lc!(((col, expr!(access!(ys, Type::Vector(2))))) => exp!(access!(col, Type::Felt), int!(7)))))))
                => enforce!(eq!(access!(x, Type::Felt), add!(access!(y, Type::Felt), access!(z, Type::Felt)))))),
    ];
    expected.evaluators.insert(
        function_ident!(root, test_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test_constraint),
            vec![trace_segment!(0, "%0", [(x, 1), (ys, 2)])],
            body,
        ),
    );

    assert_eq!(program, expected);
}

#[test]
fn test_inlining_with_function_call_as_binary_operand() {
    let root = r#"
    def root

    trace_columns {
        main: [clk, a, b[4], c],
    }

    public_inputs {
        inputs: [0],
    }

    integrity_constraints {
        let complex_fold = fold_sum(b) * fold_vec(b);
        enf complex_fold = 1;
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    fn fold_sum(a: felt[4]) -> felt {
        return a[0] + a[1] + a[2] + a[3];
    }

    fn fold_vec(a: felt[4]) -> felt {
        let m = a[0] * a[1];
        let n = m * a[2];
        let o = n * a[3];
        return o;
    }
    "#;

    let test = ParseTest::new();
    let program = match test.parse_program(root) {
        Err(err) => {
            test.diagnostics.emit(err);
            panic!("expected parsing to succeed, see diagnostics for details");
        },
        Ok(ast) => ast,
    };

    let mut pipeline =
        ConstantPropagation::new(&test.diagnostics).chain(Inlining::new(&test.diagnostics));
    let program = pipeline.run(program).unwrap();

    let mut expected = Program::new(ident!(root));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (a, 1), (b, 4), (c, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 0));
    expected.functions.insert(
        function_ident!(root, fold_sum),
        Function::new(
            SourceSpan::UNKNOWN,
            ident!(fold_sum),
            vec![(ident!(a), Type::Vector(4))],
            Type::Felt,
            vec![return_!(expr!(add!(
                add!(
                    add!(access!(a[0], Type::Felt), access!(a[1], Type::Felt)),
                    access!(a[2], Type::Felt)
                ),
                access!(a[3], Type::Felt)
            )))],
        ),
    );
    expected.functions.insert(
        function_ident!(root, fold_vec),
        Function::new(
            SourceSpan::UNKNOWN,
            ident!(fold_vec),
            vec![(ident!(a), Type::Vector(4))],
            Type::Felt,
            vec![let_!("m" = expr!(mul!(access!(a[0], Type::Felt), access!(a[1], Type::Felt)))
            => let_!("n" = expr!(mul!(access!(m, Type::Felt), access!(a[2], Type::Felt)))
            => let_!("o" = expr!(mul!(access!(n, Type::Felt), access!(a[3], Type::Felt)))
            => return_!(expr!(access!(o, Type::Felt)))
            )))],
        ),
    );
    // The sole boundary constraint is already minimal
    expected
        .boundary_constraints
        .push(enforce!(eq!(bounded_access!(clk, Boundary::First, Type::Felt), int!(0))));
    // With constant propagation and inlining done
    //
    // let complex_fold =
    //   (b[0] + b[1] + b[2] + b[3]) *
    //   (let m = b[0] * b[1]
    //   let n = m * b[2]
    //   let o = n * b[3] in o)
    // enf complex_fold = 1
    expected.integrity_constraints.push(
        let_!(complex_fold = expr!(mul!(
            add!(add!(add!(access!(b[0], Type::Felt), access!(b[1], Type::Felt)), access!(b[2], Type::Felt)), access!(b[3], Type::Felt)),
            scalar!(let_!(m = expr!(mul!(access!(b[0], Type::Felt), access!(b[1], Type::Felt)))
            => let_!(n = expr!(mul!(access!(m, Type::Felt), access!(b[2], Type::Felt)))
            => let_!(o = expr!(mul!(access!(n, Type::Felt), access!(b[3], Type::Felt))) => return_!(expr!(access!(o, Type::Felt)))))))
        )) => enforce!(eq!(access!(complex_fold, Type::Felt), int!(1))))
    );

    assert_eq!(program, expected);
}

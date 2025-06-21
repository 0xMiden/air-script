use expect_test::expect_file;

use super::helpers::{Pipeline, Target, Test};

// tests_wo_mir
// ================================================================================================

#[test]
fn binary() {
    let generated_air = Test::new("tests/binary/binary.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../binary/binary.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn buses_simple() {
    Test::new("tests/buses/buses_simple.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .expect_err("Buses should not be supported in the WithoutMIR pipeline");
}

#[test]
fn buses_complex() {
    Test::new("tests/buses/buses_complex.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .expect_err("Buses should not be supported in the WithoutMIR pipeline");
}

#[test]
fn periodic_columns() {
    let generated_air = Test::new("tests/periodic_columns/periodic_columns.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../periodic_columns/periodic_columns.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn pub_inputs() {
    let generated_air = Test::new("tests/pub_inputs/pub_inputs.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../pub_inputs/pub_inputs.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn system() {
    let generated_air = Test::new("tests/system/system.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../system/system.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn bitwise() {
    let generated_air = Test::new("tests/bitwise/bitwise.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../bitwise/bitwise.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn constants() {
    let generated_air = Test::new("tests/constants/constants.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../constants/constants.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn constant_in_range() {
    let generated_air = Test::new("tests/constant_in_range/constant_in_range.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../constant_in_range/constant_in_range.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn evaluators() {
    let generated_air = Test::new("tests/evaluators/evaluators.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../evaluators/evaluators.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn fibonacci() {
    let generated_air = Test::new("tests/fibonacci/fibonacci.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../fibonacci/fibonacci.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn functions_simple() {
    let generated_air = Test::new("tests/functions/functions_simple.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../functions/functions_simple.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn functions_simple_inlined() {
    // make sure that the constraints generated using inlined functions are the same as the ones
    // generated using regular functions
    let generated_air = Test::new("tests/functions/inlined_functions_simple.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../functions/functions_simple.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn functions_complex() {
    let generated_air = Test::new("tests/functions/functions_complex.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../functions/functions_complex.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn variables() {
    let generated_air = Test::new("tests/variables/variables.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../variables/variables.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn trace_col_groups() {
    let generated_air = Test::new("tests/trace_col_groups/trace_col_groups.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../trace_col_groups/trace_col_groups.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn indexed_trace_access() {
    let generated_air =
        Test::new("tests/indexed_trace_access/indexed_trace_access.air".to_string())
            .transpile(Target::Winterfell, Pipeline::WithoutMIR)
            .unwrap();

    let expected = expect_file!["../indexed_trace_access/indexed_trace_access.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn list_comprehension() {
    let generated_air = Test::new("tests/list_comprehension/list_comprehension.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../list_comprehension/list_comprehension.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn list_folding() {
    let generated_air = Test::new("tests/list_folding/list_folding.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../list_folding/list_folding.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn selectors() {
    let generated_air = Test::new("tests/selectors/selectors.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../selectors/selectors.rs"];
    expected.assert_eq(&generated_air);

    let generated_air = Test::new("tests/selectors/selectors_with_evaluators.air".to_string())
        .transpile(Target::Winterfell, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../selectors/selectors.rs"];
    expected.assert_eq(&generated_air);
}

#[test]
fn constraint_comprehension() {
    let generated_air =
        Test::new("tests/constraint_comprehension/constraint_comprehension.air".to_string())
            .transpile(Target::Winterfell, Pipeline::WithoutMIR)
            .unwrap();

    let expected = expect_file!["../constraint_comprehension/constraint_comprehension.rs"];
    expected.assert_eq(&generated_air);

    let generated_air =
        Test::new("tests/constraint_comprehension/cc_with_evaluators.air".to_string())
            .transpile(Target::Winterfell, Pipeline::WithoutMIR)
            .unwrap();

    let expected = expect_file!["../constraint_comprehension/constraint_comprehension.rs"];
    expected.assert_eq(&generated_air);
}

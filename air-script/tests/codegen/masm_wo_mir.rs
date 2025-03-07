use super::helpers::{Pipeline, Target, Test};
use expect_test::expect_file;

// tests_wo_mir
// ================================================================================================

#[test]
fn aux_trace() {
    let generated_masm = Test::new("tests/aux_trace/aux_trace.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../aux_trace/aux_trace.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn binary() {
    let generated_masm = Test::new("tests/binary/binary.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../binary/binary.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn periodic_columns() {
    let generated_masm = Test::new("tests/periodic_columns/periodic_columns.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../periodic_columns/periodic_columns.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn pub_inputs() {
    let generated_masm = Test::new("tests/pub_inputs/pub_inputs.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../pub_inputs/pub_inputs.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn system() {
    let generated_masm = Test::new("tests/system/system.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../system/system.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn bitwise() {
    let generated_masm = Test::new("tests/bitwise/bitwise.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../bitwise/bitwise.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn constants() {
    let generated_masm = Test::new("tests/constants/constants.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../constants/constants.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn constant_in_range() {
    let generated_masm = Test::new("tests/constant_in_range/constant_in_range.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../constant_in_range/constant_in_range.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn evaluators() {
    let generated_masm = Test::new("tests/evaluators/evaluators.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../evaluators/evaluators.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn functions_simple() {
    let generated_masm = Test::new("tests/functions/functions_simple.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../functions/functions_simple.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn functions_simple_inlined() {
    // make sure that the constraints generated using inlined functions are the same as the ones
    // generated using regular functions
    let generated_masm = Test::new("tests/functions/inlined_functions_simple.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();
    let expected = expect_file!["../functions/functions_simple.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn functions_complex() {
    let generated_masm = Test::new("tests/functions/functions_complex.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../functions/functions_complex.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn variables() {
    let generated_masm = Test::new("tests/variables/variables.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../variables/variables.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn trace_col_groups() {
    let generated_masm = Test::new("tests/trace_col_groups/trace_col_groups.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../trace_col_groups/trace_col_groups.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn indexed_trace_access() {
    let generated_masm =
        Test::new("tests/indexed_trace_access/indexed_trace_access.air".to_string())
            .transpile(Target::Masm, Pipeline::WithoutMIR)
            .unwrap();

    let expected = expect_file!["../indexed_trace_access/indexed_trace_access.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
#[ignore] // TODO: There is some non-determinism in the IR creation, unskip this test once it is fixed
fn random_values() {
    let generated_masm = Test::new("tests/random_values/random_values_simple.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();
    let expected = expect_file!["../random_values/random_values.masm"];
    expected.assert_eq(&generated_masm);

    let generated_masm = Test::new("tests/random_values/random_values_bindings.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();
    let expected = expect_file!["../random_values/random_values.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn list_comprehension() {
    let generated_masm = Test::new("tests/list_comprehension/list_comprehension.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../list_comprehension/list_comprehension.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn list_folding() {
    let generated_masm = Test::new("tests/list_folding/list_folding.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();

    let expected = expect_file!["../list_folding/list_folding.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
#[ignore] // TODO: There is some non-determinism in the IR creation, unskip this test once it is fixed
fn selectors() {
    let generated_masm = Test::new("tests/selectors/selectors.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();
    let expected = expect_file!["../selectors/selectors.masm"];
    expected.assert_eq(&generated_masm);

    let generated_masm = Test::new("tests/selectors/selectors_with_evaluators.air".to_string())
        .transpile(Target::Masm, Pipeline::WithoutMIR)
        .unwrap();
    let expected = expect_file!["../selectors/selectors.masm"];
    expected.assert_eq(&generated_masm);
}

#[test]
fn constraint_comprehension() {
    let generated_masm =
        Test::new("tests/constraint_comprehension/constraint_comprehension.air".to_string())
            .transpile(Target::Masm, Pipeline::WithoutMIR)
            .unwrap();

    let expected = expect_file!["../constraint_comprehension/constraint_comprehension.masm"];
    expected.assert_eq(&generated_masm);

    let generated_masm =
        Test::new("tests/constraint_comprehension/cc_with_evaluators.air".to_string())
            .transpile(Target::Masm, Pipeline::WithoutMIR)
            .unwrap();

    let expected = expect_file!["../constraint_comprehension/constraint_comprehension.masm"];
    expected.assert_eq(&generated_masm);
}

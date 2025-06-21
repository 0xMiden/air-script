use miden_diagnostics::SourceSpan;

use crate::ast::*;

use super::ParseTest;

// CONSTANTS
// ================================================================================================

#[test]
fn constants_scalars() {
    let source = "
    mod test

    const A = 1;
    const B = 2;";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.constants.insert(
        ident!(A),
        Constant::new(SourceSpan::UNKNOWN, ident!(A), ConstantExpr::Scalar(1)),
    );
    expected.constants.insert(
        ident!(B),
        Constant::new(SourceSpan::UNKNOWN, ident!(B), ConstantExpr::Scalar(2)),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn constants_vectors() {
    let source = "
    mod test

    const A = [1, 2, 3, 4];
    const B = [5, 6, 7, 8];";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.constants.insert(
        ident!(A),
        Constant::new(
            SourceSpan::UNKNOWN,
            ident!(A),
            ConstantExpr::Vector(vec![1, 2, 3, 4]),
        ),
    );
    expected.constants.insert(
        ident!(B),
        Constant::new(
            SourceSpan::UNKNOWN,
            ident!(B),
            ConstantExpr::Vector(vec![5, 6, 7, 8]),
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn constants_matrices() {
    let source = "
    mod test

    const A = [[1, 2], [3, 4]];
    const B = [[5, 6], [7, 8]];";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.constants.insert(
        ident!(A),
        Constant::new(
            SourceSpan::UNKNOWN,
            ident!(A),
            ConstantExpr::Matrix(vec![vec![1, 2], vec![3, 4]]),
        ),
    );
    expected.constants.insert(
        ident!(B),
        Constant::new(
            SourceSpan::UNKNOWN,
            ident!(B),
            ConstantExpr::Matrix(vec![vec![5, 6], vec![7, 8]]),
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn err_const_matrix_unequal_number_of_cols() {
    // This is invalid since the number of columns for the two rows are unequal. However this
    // validation happens at the IR level.
    let source = "
    mod test

    const A = [[1, 2], [3, 4, 5]];";

    ParseTest::new()
        .expect_module_diagnostic(source, "invalid matrix literal: mismatched dimensions");
}

#[test]
fn err_incomplete_constant_declaration_missing_name() {
    let source = "
    mod test

    const
    ";
    assert!(ParseTest::new().parse_module(source).is_err());
}

#[test]
fn err_incomplete_constant_declaration_missing_value() {
    let source = "
    mod test

    const A
    ";
    assert!(ParseTest::new().parse_module(source).is_err());
}

#[test]
fn err_lowercase_constant_name() {
    let source = "
    mod test

    const Ab = [[1, 2], [3, 4]];
    const C = [[5, 6], [7, 8]];";
    ParseTest::new().expect_module_diagnostic(source, "constant identifiers must be uppercase");
}

#[test]
fn err_consts_with_non_int_values() {
    let source = "
    def test

    const A = a;
    const B = 2;";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_const_vectors_with_non_int_values() {
    let source = "
    def test

    const A = [1, a];
    const B = [2, 4];";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_vector_with_trailing_comma() {
    let source = "
    def test

    const A = [1, ];";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_matrix_with_trailing_comma() {
    let source = "
    def test

    const A = [[1, 2], ];";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_matrix_mixed_element_types() {
    let source = "
    def test

    const A = [1, [1, 2]];";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_invalid_matrix_element() {
    let source = "
    def test

    const A = [[1, 2], [3, [4, 5]]];";
    ParseTest::new().expect_unrecognized_token(source);
}

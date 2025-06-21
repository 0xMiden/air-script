use miden_diagnostics::SourceSpan;

use crate::ast::*;

use super::ParseTest;

// VARIABLES
// ================================================================================================
#[test]
fn variables_with_and_operators() {
    let source = "
    mod test

    ev test([clk]) {
        let flag = n1 & !n2;
        enf clk' = clk + 1 when flag;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    // The constraint is converted into a comprehension constraint by the parser, which
    // involves generating an iterable with one element and giving it a generated binding
    let body = vec![let_!(flag = expr!(and!(access!(n1), not!(access!(n2)))) =>
                  enforce_all!(lc!((("%1", range!(0..1))) => eq!(access!(clk, 1), add!(access!(clk), int!(1))), when access!(flag))))];
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            body,
        ),
    );

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn variables_with_or_operators() {
    let source = "
    mod test

    ev test([clk]) {
        let flag = n1 | !n2';
        enf clk' = clk + 1 when flag;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    let body = vec![
        let_!(flag = expr!(or!(access!(n1), not!(access!(n2, 1)))) =>
                   enforce_all!(lc!((("%1", range!(0..1))) => eq!(access!(clk, 1), add!(access!(clk), int!(1))), when access!(flag)))),
    ];
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            body,
        ),
    );

    ParseTest::new().expect_module_ast(source, expected);
}

// VARIABLES INVALID USAGE
// ================================================================================================

#[test]
fn err_let_bound_variable_at_top_level() {
    let source = "
    def test

    const A = 1;

    let a = 0;";

    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_vector_variable_with_trailing_comma() {
    let source = "
    def test

    integrity_constraints {
        let a = [1, ];";

    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_matrix_variable_with_trailing_comma() {
    let source = "
    def test

    integrity_constraints {
        let a = [[1, 2], ];";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_matrix_variable_mixed_element_types() {
    let source = "
    def test

    integrity_constraints {
        let a = [[1, 2], 1];";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_invalid_matrix_element() {
    let source = "
    def test

    integrity_constraints {
        let a = [[1, 2], [3, [4, 5]]];";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_matrix_variable_from_vector_and_reference() {
    let source = "
    def test

    integrity_constraints {
        let a = [[1, 2], [3, 4]];
        let b = [5, 6];
        let c = [b, [7, 8]];
        let d = [[7, 8], a[0]];";
    ParseTest::new().expect_unrecognized_token(source);
}

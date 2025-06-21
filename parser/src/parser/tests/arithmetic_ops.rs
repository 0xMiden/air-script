use miden_diagnostics::SourceSpan;

use super::ParseTest;
use crate::ast::*;

// EXPRESSIONS
// ================================================================================================

#[test]
fn single_addition() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' + clk = 0;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(add!(access!(clk, 1), access!(clk)), int!(0)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn multi_addition() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' + clk + 2 = 0;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(add!(add!(access!(clk, 1), access!(clk)), int!(2)), int!(0)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn single_subtraction() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' - clk = 0;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(sub!(access!(clk, 1), access!(clk)), int!(0)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn multi_subtraction() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' - clk - 1 = 0;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(sub!(sub!(access!(clk, 1), access!(clk)), int!(1)), int!(0)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn single_multiplication() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' * clk = 0;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(mul!(access!(clk, 1), access!(clk)), int!(0)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn multi_multiplication() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' * clk * 2 = 0;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(mul!(mul!(access!(clk, 1), access!(clk)), int!(2)), int!(0)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn unit_with_parens() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf (2) + 1 = 3;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(add!(int!(2), int!(1)), int!(3)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ops_with_parens() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf (clk' + clk) * 2 = 4;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(mul!(add!(access!(clk, 1), access!(clk)), int!(2)), int!(4)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn const_exponentiation() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk'^2 = 1;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(exp!(access!(clk, 1), int!(2)), int!(1)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn non_const_exponentiation() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk'^(clk + 2) = 1;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(exp!(access!(clk, 1), add!(access!(clk), int!(2))), int!(1)))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn err_ops_without_matching_closing_parens() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf (clk' + clk * 2 = 4
    }";
    ParseTest::new().expect_unrecognized_token(source)
}

#[test]
fn err_closing_paren_without_opening_paren() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' + clk) * 2 = 4
    }";
    ParseTest::new().expect_unrecognized_token(source)
}

#[test]
fn multi_arithmetic_ops_same_precedence() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' - clk - 2 + 1 = 0;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(
                add!(sub!(sub!(access!(clk, 1), access!(clk)), int!(2)), int!(1)),
                int!(0)
            ))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn multi_arithmetic_ops_different_precedence() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk'^2 - clk * 2 - 1 = 0;
    }";

    // The precedence order of operations here is:
    // 1. Exponentiation
    // 2. Multiplication
    // 3. Addition/Subtraction
    // These operations are evaluated in the order of decreasing precedence.

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(
                sub!(sub!(exp!(access!(clk, 1), int!(2)), mul!(access!(clk), int!(2))), int!(1)),
                int!(0)
            ))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn multi_arithmetic_ops_different_precedence_w_parens() {
    // the operation must be put into a source section, or parsing will fail
    let source = "
    mod test

    ev test([clk]) {
        enf clk' - clk^2 * (2 - 1) = 0;
    }";

    // The precedence order of operations here is:
    // 1. Parentheses
    // 2. Exp
    // 3. Multiplication
    // 4. Addition/Subtraction
    // These operations are evaluated in the order of decreasing precedence.
    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce!(eq!(
                sub!(access!(clk, 1), mul!(exp!(access!(clk), int!(2)), sub!(int!(2), int!(1)))),
                int!(0)
            ))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

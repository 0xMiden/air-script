use miden_diagnostics::SourceSpan;

use super::ParseTest;
use crate::ast::*;

#[test]
fn call_fold_identifier() {
    let source = "
    mod test

    ev test([a, c[2]]) {
        let x = sum(c);
        let y = prod(c);
        enf a = x + y;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    let body = vec![let_!(x = expr!(call!(sum(expr!(access!(c))))) =>
                  let_!(y = expr!(call!(prod(expr!(access!(c))))) =>
                        enforce!(eq!(access!(a), add!(access!(x), access!(y))))))];
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(a, 1), (c, 2)])],
            body,
        ),
    );

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn call_fold_vector_literal() {
    let source = "
    mod test

    ev test([a, b, c[4]]) {
        let x = sum([a, b, c[0]]);
        let y = prod([a, b, c[0]]);
        enf a = x + y;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    let body = vec![let_!(x = expr!(call!(sum(vector!(access!(a), access!(b), access!(c[0]))))) =>
                  let_!(y = expr!(call!(prod(vector!(access!(a), access!(b), access!(c[0]))))) =>
                        enforce!(eq!(access!(a), add!(access!(x), access!(y))))))];
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(a, 1), (b, 1), (c, 4)])],
            body,
        ),
    );

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn call_fold_list_comprehension() {
    let source = "
    mod test

    ev test([a, b, c[4]]) {
        let x = sum([col^7 for col in c]);
        let y = prod([col^7 for col in c]);
        enf a = x + y;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    let body = vec![
        let_!(x = expr!(call!(sum(lc!(((col, expr!(access!(c)))) => exp!(access!(col), int!(7))).into()))) =>
                  let_!(y = expr!(call!(prod(lc!(((col, expr!(access!(c)))) => exp!(access!(col), int!(7))).into()))) =>
                        enforce!(eq!(access!(a), add!(access!(x), access!(y)))))),
    ];
    expected.evaluators.insert(
        ident!(test),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(test),
            vec![trace_segment!(0, "%0", [(a, 1), (b, 1), (c, 4)])],
            body,
        ),
    );

    ParseTest::new().expect_module_ast(source, expected);
}

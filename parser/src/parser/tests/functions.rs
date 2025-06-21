use miden_diagnostics::{SourceSpan, Span};

use super::ParseTest;
use crate::ast::*;

// PURE FUNCTIONS
// ================================================================================================

#[test]
fn fn_def_with_scalars() {
    let source = "
    mod test

    fn fn_with_scalars(a: felt, b: felt) -> felt {
        return a + b;
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.functions.insert(
        ident!(fn_with_scalars),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(fn_with_scalars),
            vec![(ident!(a), Type::Felt), (ident!(b), Type::Felt)],
            Type::Felt,
            vec![return_!(expr!(add!(access!(a), access!(b))))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn fn_def_with_vectors() {
    let source = "
    mod test

    fn fn_with_vectors(a: felt[12], b: felt[12]) -> felt[12] {
        return [x + y for (x, y) in (a, b)];
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.functions.insert(
        ident!(fn_with_vectors),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(fn_with_vectors),
            vec![(ident!(a), Type::Vector(12)), (ident!(b), Type::Vector(12))],
            Type::Vector(12),
            vec![return_!(expr!(lc!(((x, expr!(access!(a))), (y, expr!(access!(b)))) =>
                add!(access!(x), access!(y)))))],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn fn_use_scalars_and_vectors() {
    let source = "
        def root

        public_inputs {
            stack_inputs: [16],
        }

        trace_columns {
            main: [a, b[12]],
        }

        fn fn_with_scalars_and_vectors(a: felt, b: felt[12]) -> felt {
            return sum([a + x for x in b]);
        }

        boundary_constraints {
            enf a.first = 0;
        }

        integrity_constraints {
            enf a' = fn_with_scalars_and_vectors(a, b);
        }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(root));

    expected.functions.insert(
        ident!(fn_with_scalars_and_vectors),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(fn_with_scalars_and_vectors),
            vec![(ident!(a), Type::Felt), (ident!(b), Type::Vector(12))],
            Type::Felt,
            vec![return_!(expr!(call!(sum(expr!(
                lc!(((x, expr!(access!(b)))) => add!(access!(a), access!(x)))
            )))))],
        ),
    );

    expected.trace_columns.push(trace_segment!(0, "$main", [(a, 1), (b, 12)]));

    expected.public_inputs.insert(
        ident!(stack_inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(stack_inputs), 16),
    );

    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            access!(a, 1),
            call!(fn_with_scalars_and_vectors(expr!(access!(a)), expr!(access!(b))))
        ))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn fn_call_in_fn() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a, b[12]],
    }

    fn fold_vec(a: felt[12]) -> felt {
        return sum([x for x in a]);
    }

    fn fold_scalar_and_vec(a: felt, b: felt[12]) -> felt {
        return a + fold_vec(b);
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        enf a' = fold_scalar_and_vec(a, b);
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(root));

    expected.functions.insert(
        ident!(fold_vec),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(fold_vec),
            vec![(ident!(a), Type::Vector(12))],
            Type::Felt,
            vec![return_!(expr!(call!(sum(expr!(lc!(((x, expr!(access!(a)))) => access!(x)))))))],
        ),
    );

    expected.functions.insert(
        ident!(fold_scalar_and_vec),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(fold_scalar_and_vec),
            vec![(ident!(a), Type::Felt), (ident!(b), Type::Vector(12))],
            Type::Felt,
            vec![return_!(expr!(add!(access!(a), call!(fold_vec(expr!(access!(b)))))))],
        ),
    );

    expected.trace_columns.push(trace_segment!(0, "$main", [(a, 1), (b, 12)]));

    expected.public_inputs.insert(
        ident!(stack_inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(stack_inputs), 16),
    );

    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));

    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            access!(a, 1),
            call!(fold_scalar_and_vec(expr!(access!(a)), expr!(access!(b))))
        ))],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn fn_call_in_ev() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a, b[12]],
    }

    fn fold_vec(a: felt[12]) -> felt {
        return sum([x for x in a]);
    }

    fn fold_scalar_and_vec(a: felt, b: felt[12]) -> felt {
        return a + fold_vec(b);
    }

    ev evaluator([a, b[12]]) {
        enf a' = fold_scalar_and_vec(a, b);
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        enf evaluator(a, b);
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(root));

    expected.functions.insert(
        ident!(fold_vec),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(fold_vec),
            vec![(ident!(a), Type::Vector(12))],
            Type::Felt,
            vec![return_!(expr!(call!(sum(expr!(lc!(((x, expr!(access!(a)))) => access!(x)))))))],
        ),
    );

    expected.functions.insert(
        ident!(fold_scalar_and_vec),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(fold_scalar_and_vec),
            vec![(ident!(a), Type::Felt), (ident!(b), Type::Vector(12))],
            Type::Felt,
            vec![return_!(expr!(add!(access!(a), call!(fold_vec(expr!(access!(b)))))))],
        ),
    );

    expected.evaluators.insert(
        ident!(evaluator),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(evaluator),
            vec![trace_segment!(0, "%0", [(a, 1), (b, 12)])],
            vec![enforce!(eq!(
                access!(a, 1),
                call!(fold_scalar_and_vec(expr!(access!(a)), expr!(access!(b))))
            ))],
        ),
    );

    expected.trace_columns.push(trace_segment!(0, "$main", [(a, 1), (b, 12)]));

    expected.public_inputs.insert(
        ident!(stack_inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(stack_inputs), 16),
    );

    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));

    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(call!(evaluator(expr!(access!(a)), expr!(access!(b)))))],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn fn_as_lc_iterables() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a[12], b[12]],
    }

    fn operation(a: felt, b: felt) -> felt {
        let x = a^b + 1;
        return b^x;
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        enf a' = sum([operation(x, y) for (x, y) in (a, b)]);
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(root));

    expected.functions.insert(
        ident!(operation),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(operation),
            vec![(ident!(a), Type::Felt), (ident!(b), Type::Felt)],
            Type::Felt,
            vec![let_!(x = expr!(add!(exp!(access!(a), access!(b)), int!(1))) =>
                return_!(expr!(exp!(access!(b), access!(x)))))],
        ),
    );

    expected.trace_columns.push(trace_segment!(0, "$main", [(a, 12), (b, 12)]));

    expected.public_inputs.insert(
        ident!(stack_inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(stack_inputs), 16),
    );

    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));

    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            access!(a, 1),
            call!(sum(expr!(
                lc!(((x, expr!(access!(a))), (y, expr!(access!(b)))) => call!(operation(
                        expr!(access!(x)),
                        expr!(access!(y))
                    ))
                )
            )))
        ))],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn fn_call_in_binary_ops() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a[12], b[12]],
    }

    fn operation(a: felt[12], b: felt[12]) -> felt {
        return sum([x + y for (x, y) in (a, b)]);
    }

    boundary_constraints {
        enf a[0].first = 0;
    }

    integrity_constraints {
        enf a[0]' = a[0] * operation(a, b);
        enf b[0]' = b[0] * operation(a, b);
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(root));

    expected.functions.insert(
        ident!(operation),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(operation),
            vec![(ident!(a), Type::Vector(12)), (ident!(b), Type::Vector(12))],
            Type::Felt,
            vec![return_!(expr!(call!(sum(expr!(
                lc!(((x, expr!(access!(a))), (y, expr!(access!(b)))) => add!(
                    access!(x),
                    access!(y)
                ))
            )))))],
        ),
    );

    expected.trace_columns.push(trace_segment!(0, "$main", [(a, 12), (b, 12)]));

    expected.public_inputs.insert(
        ident!(stack_inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(stack_inputs), 16),
    );

    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a[0], Boundary::First), int!(0)))],
    ));

    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce!(eq!(
                access!(a[0], 1),
                mul!(access!(a[0], 0), call!(operation(expr!(access!(a)), expr!(access!(b)))))
            )),
            enforce!(eq!(
                access!(b[0], 1),
                mul!(access!(b[0], 0), call!(operation(expr!(access!(a)), expr!(access!(b)))))
            )),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn fn_call_in_vector_def() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a[12], b[12]],
    }

    fn operation(a: felt[12], b: felt[12]) -> felt[12] {
        return [x + y for (x, y) in (a, b)];
    }

    boundary_constraints {
        enf a[0].first = 0;
    }

    integrity_constraints {
        let d = [a[0] * operation(a, b), b[0] * operation(a, b)];
        enf a[0]' = d[0];
        enf b[0]' = d[1];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(root));

    expected.functions.insert(
        ident!(operation),
        Function::new(
            SourceSpan::UNKNOWN,
            function_ident!(operation),
            vec![(ident!(a), Type::Vector(12)), (ident!(b), Type::Vector(12))],
            Type::Vector(12),
            vec![return_!(expr!(lc!(((x, expr!(access!(a))), (y, expr!(access!(b)))) => add!(
                access!(x),
                access!(y)
            ))))],
        ),
    );

    expected.trace_columns.push(trace_segment!(0, "$main", [(a, 12), (b, 12)]));

    expected.public_inputs.insert(
        ident!(stack_inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(stack_inputs), 16),
    );

    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a[0], Boundary::First), int!(0)))],
    ));

    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![let_!(
                d = vector!(
                    mul!(access!(a[0]), call!(operation(expr!(access!(a)), expr!(access!(b))))),
                    mul!(access!(b[0]), call!(operation(expr!(access!(a)), expr!(access!(b))))))
                    =>
            enforce!(eq!(access!(a[0], 1), access!(d[0]))),
            enforce!(eq!(access!(b[0], 1), access!(d[1]))))],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

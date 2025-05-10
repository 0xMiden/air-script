use miden_diagnostics::SourceSpan;

use crate::ast::*;

use super::ParseTest;

#[test]
fn use_declaration() {
    let source = "
    mod test

    use foo::*;
    ";
    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.imports.insert(ident!(foo), import_all!(foo));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn import_declaration() {
    let source = "
    mod test

    use foo::bar;
    ";
    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.imports.insert(ident!(foo), import!(foo, bar));
    ParseTest::new().expect_module_ast(source, expected);
}

// This test performs a realistic test involving compilation of a program consisting of
// items in 3 different modules, which tests the following:
//
// * Import resolution works
// * Dependency graph construction and dead-code elimination
// * Symbol resolution (locals vs globals vs imports)
//
// Even though a variety of validations are performed during this test, we are not testing
// errors such as undefined variables/functions, shadowing, invalid constraints, etc. These
// are handled by other tests.
#[test]
fn modules_integration_test() {
    let mut expected = Program::new(ident!(import_example));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (fmp, 1), (ctx, 1)]));
    expected.periodic_columns.insert(
        ident!(foo, k0),
        PeriodicColumn::new(SourceSpan::UNKNOWN, ident!(k0), vec![1, 1, 0, 0]),
    );
    expected.periodic_columns.insert(
        ident!(bar, k0),
        PeriodicColumn::new(SourceSpan::UNKNOWN, ident!(k0), vec![1, 0]),
    );

    // NOTE: We only end up with the used evaluators in the final program.
    // Even though `import_example` imports everything from `foo`, and `foo`
    // defines an evaluator `other_constraint`, that evaluator is never called
    // so it is treated as dead code and stripped from the program

    // ev bar_constraint([clk]) {
    //    enf clk' = clk + k0 when k0
    // }
    expected.evaluators.insert(
        function_ident!(bar, bar_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(bar_constraint),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce_all!(lc!((("%1", range!(0..1))) => eq!(
                access!(clk, 1, Type::Felt),
                add!(access!(clk, Type::Felt), access!(bar, k0, Type::Felt))
            ), when access!(bar, k0, Type::Felt)))],
        ),
    );
    // ev foo_constraint([clk]) {
    //    enf clk' = clk + 1 when k0
    // }
    expected.evaluators.insert(
        function_ident!(foo, foo_constraint),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(foo_constraint),
            vec![trace_segment!(0, "%0", [(clk, 1)])],
            vec![enforce_all!(lc!((("%1", range!(0..1))) => eq!(access!(clk, 1, Type::Felt), add!(access!(clk, Type::Felt), int!(1))), when access!(foo, k0, Type::Felt)))],
        ),
    );
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected
        .integrity_constraints
        .push(enforce!(call!(foo::foo_constraint(vector!(access!(
            clk,
            Type::Felt
        ))))));
    expected
        .integrity_constraints
        .push(enforce!(call!(bar::bar_constraint(vector!(access!(
            clk,
            Type::Felt
        ))))));
    expected.boundary_constraints.push(enforce!(eq!(
        bounded_access!(clk, Boundary::First, Type::Felt),
        int!(0)
    )));

    ParseTest::new()
        .expect_program_ast_from_file("src/parser/tests/input/import_example.air", expected);
}

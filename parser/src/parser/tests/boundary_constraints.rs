use miden_diagnostics::{SourceSpan, Span};

use crate::ast::*;

use super::ParseTest;

// BOUNDARY STATEMENTS
// ================================================================================================

const BASE_MODULE: &str = r#"
def test

trace_columns {
    main: [clk],
}

buses {
    multiset p,
    logup q,
}

public_inputs {
    inputs: [2],
}

integrity_constraints {
    enf clk = 0;

}"#;

/// Constructs a module containing the following:
///
/// ```airscript
/// def test
///
/// trace_columns {
///     main: [clk]
/// }
///
/// buses {
///     multiset p,
///     logup q,
/// }
///
/// public_inputs {
///     inputs: [2]
/// }
///
/// integrity_constraints {
///     enf clk = 0
/// }
/// ```
///
/// This is used as a common base for most tests in this module
fn test_module() -> Module {
    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.buses.insert(
        ident!(p),
        Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset),
    );
    expected.buses.insert(
        ident!(q),
        Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup),
    );
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(access!(clk), int!(0)))],
    ));
    expected
}

#[test]
fn boundary_constraint_at_first() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.first = 0;
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn boundary_constraint_at_last() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.last = 15;
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::Last),
            int!(15)
        ))],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn boundary_constraint_with_buses() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf p.first = null;
        enf q.last = null;
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce!(eq!(bounded_access!(p, Boundary::First), null!())),
            enforce!(eq!(bounded_access!(q, Boundary::Last), null!())),
        ],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn error_invalid_boundary() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.0 = 15
    }}"
    );

    ParseTest::new().expect_unrecognized_token(&source);
}

#[test]
fn multiple_boundary_constraints() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.first = 0;
        enf clk.last = 1;
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce!(eq!(bounded_access!(clk, Boundary::First), int!(0))),
            enforce!(eq!(bounded_access!(clk, Boundary::Last), int!(1))),
        ],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn boundary_constraint_with_pub_input() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.first = inputs[0];
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            access!(inputs[0])
        ))],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn boundary_constraint_with_expr() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.first = 5 + inputs[1] + 6;
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            add!(add!(int!(5), access!(inputs[1])), int!(6))
        ))],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn boundary_constraint_with_const() {
    let source = format!(
        "
    {BASE_MODULE}

    const A = 1;
    const B = [0, 1];
    const C = [[0, 1], [1, 0]];

    boundary_constraints {{
        enf clk.first = A + B[1] - C[0][1];
    }}"
    );

    let mut expected = test_module();
    expected.constants.insert(ident!(A), constant!(A = 1));
    expected.constants.insert(ident!(B), constant!(B = [0, 1]));
    expected
        .constants
        .insert(ident!(C), constant!(C = [[0, 1], [1, 0]]));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            sub!(add!(access!(A), access!(B[1])), access!(C[0][1]))
        ))],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn boundary_constraint_with_variables() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        let a = 2^2;
        let b = [a, 2 * a];
        let c = [[a - 1, a^2], [b[0], b[1]]];
        enf clk.first = 5 + a[3] + 6;
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![let_!(a = expr!(exp!(int!(2), int!(2))) =>
                   let_!(b = vector!(access!(a), mul!(int!(2), access!(a))) =>
                         let_!(c = matrix!([sub!(access!(a), int!(1)), exp!(access!(a), int!(2))], [access!(b[0]), access!(b[1])]) =>
                             enforce!(eq!(bounded_access!(clk, Boundary::First), add!(add!(int!(5), access!(a[3])), int!(6)))))))],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

// CONSTRAINT COMPREHENSION
// ================================================================================================

#[test]
fn bc_comprehension_one_iterable_identifier() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf x.first = 0 for x in inputs;
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, expr!(access!(inputs)))) => eq!(bounded_access!(x, Boundary::First), int!(0)))
        )],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn bc_comprehension_one_iterable_range() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf x.first = 0 for x in (0..4);
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, range!(0..4))) => eq!(bounded_access!(x, Boundary::First), int!(0)))
        )],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn bc_comprehension_one_iterable_slice() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf x.first = 0 for x in inputs[0..1];
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, expr!(slice!(inputs, 0..1)))) => eq!(bounded_access!(x, Boundary::First), int!(0)))
        )],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn bc_comprehension_two_iterable_identifiers() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf x.first = y for (x, y) in (inputs, inputs);
    }}"
    );

    let mut expected = test_module();
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, expr!(access!(inputs))), (y, expr!(access!(inputs)))) => eq!(bounded_access!(x, Boundary::First), access!(y)))
        )],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

// INVALID BOUNDARY CONSTRAINT COMPREHENSION
// ================================================================================================

#[test]
fn err_bc_comprehension_one_member_two_iterables() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.first = c for c in (inputs, inputs);
    }}"
    );

    ParseTest::new()
        .expect_module_diagnostic(&source, "bindings and iterables lengths are mismatched");
}

#[test]
fn err_bc_comprehension_two_members_one_iterables() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        enf clk.first = c + d for (c, d) in inputs;
    }}"
    );

    ParseTest::new()
        .expect_module_diagnostic(&source, "bindings and iterables lengths are mismatched");
}

// INVALID BOUNDARY CONSTRAINTS
// ================================================================================================

#[test]
fn err_invalid_variable() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        let a = 2^2 + [1];
    }}"
    );
    ParseTest::new().expect_unrecognized_token(&source);
}

#[test]
fn err_missing_boundary_constraint() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{
        let a = 2^2;
        let b = [a, 2 * a];
        let c = [[a - 1, a^2], [b[0], b[1]]];
    }}"
    );
    ParseTest::new().expect_module_diagnostic(&source, "expected one of: '\"enf\"', '\"let\"'");
}

#[test]
fn err_empty_boundary_constraints() {
    let source = format!(
        "
    {BASE_MODULE}

    boundary_constraints {{}}
    "
    );
    assert_module_error!(&source, crate::parser::ParseError::UnrecognizedToken { .. });
}

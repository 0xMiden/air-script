use miden_diagnostics::{SourceSpan, Span};

use super::ParseTest;
use crate::ast::*;

// LIST COMPREHENSION
// ================================================================================================

#[test]
fn bc_one_iterable_identifier_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    integrity_constraints {
        enf a = 0;
    }

    boundary_constraints {
        # raise value in the current row to power 7
        let x = [col^7 for col in c];

        enf a.first = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.integrity_constraints =
        Some(Span::new(SourceSpan::UNKNOWN, vec![enforce!(eq!(access!(a), int!(0)))]));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(x = lc!(((col, expr!(access!(c)))) => exp!(access!(col), int!(7))).into() =>
                  enforce!(eq!(bounded_access!(a, Boundary::First), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3]))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn bc_identifier_and_range_lc() {
    let source = "
    def test

    const THREE = 3;

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    integrity_constraints {
        enf a = 0;
    }

    boundary_constraints {
        let x = [2^i * c for (i, c) in (0..THREE, c)];
        enf a.first = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected.constants.insert(ident!(THREE), constant!(THREE = 3));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.integrity_constraints =
        Some(Span::new(SourceSpan::UNKNOWN, vec![enforce!(eq!(access!(a), int!(0)))]));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(x = lc!(((i, range!(0usize, ident!(THREE))), (c, expr!(access!(c)))) => mul!(exp!(int!(2), access!(i)), access!(c))).into() =>
                  enforce!(eq!(bounded_access!(a, Boundary::First), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3]))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn bc_iterable_slice_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    integrity_constraints {
        enf a = 0;
    }

    boundary_constraints {
        let x = [c for c in c[0..3]];
        enf a.first = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.integrity_constraints =
        Some(Span::new(SourceSpan::UNKNOWN, vec![enforce!(eq!(access!(a), int!(0)))]));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![let_!(x = lc!(((c, expr!(slice!(c, 0..3)))) => access!(c)).into() =>
                  enforce!(eq!(bounded_access!(a, Boundary::First), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3])))))],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn bc_two_iterable_identifier_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4], d[4]],
    }

    public_inputs {
        inputs: [2],
    }

    integrity_constraints {
        enf a = 0;
    }

    boundary_constraints {
        let diff = [x - y for (x, y) in (c, d)];
        enf a.first = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4), (d, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.integrity_constraints =
        Some(Span::new(SourceSpan::UNKNOWN, vec![enforce!(eq!(access!(a), int!(0)))]));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(diff = lc!(((x, expr!(access!(c))), (y, expr!(access!(d)))) => sub!(access!(x), access!(y))).into() =>
                  enforce!(eq!(bounded_access!(a, Boundary::First), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3]))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn bc_multiple_iterables_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b[3], c[4], d[4]],
    }

    public_inputs {
        inputs: [2],
    }

    integrity_constraints {
        enf a = 0;
    }

    boundary_constraints {
        let diff = [w + x - y - z for (w, x, y, z) in (0..3, b, c[0..3], d[0..3])];
        enf a.first = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 3), (c, 4), (d, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.integrity_constraints =
        Some(Span::new(SourceSpan::UNKNOWN, vec![enforce!(eq!(access!(a), int!(0)))]));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(diff = lc!(((w, range!(0..3)), (x, expr!(access!(b))), (y, expr!(slice!(c, 0..3))), (z, expr!(slice!(d, 0..3)))) =>
                             sub!(sub!(add!(access!(w), access!(x)), access!(y)), access!(z))).into() =>
                  enforce!(eq!(bounded_access!(a, Boundary::First), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3]))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_one_iterable_identifier_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        # raise value in the current row to power 7
        let x = [col^7 for col in c];

        # raise value in the next row to power 7
        let y = [col'^7 for col in c];
        enf a = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(x = lc!(((col, expr!(access!(c)))) => exp!(access!(col), int!(7))).into() =>
                let_!(y = lc!(((col, expr!(access!(c)))) => exp!(access!(col, 1), int!(7))).into() =>
                  enforce!(eq!(access!(a), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3])))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_iterable_identifier_range_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        let x = [2^i * c for (i, c) in (0..3, c)];
        enf a = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(x = lc!(((i, range!(0..3)), (c, expr!(access!(c)))) => mul!(exp!(int!(2), access!(i)), access!(c))).into() =>
                  enforce!(eq!(access!(a), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3]))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_iterable_slice_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        let x = [c for c in c[0..3]];
        enf a = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![let_!(x = lc!(((c, expr!(slice!(c, 0..3)))) => access!(c)).into() =>
                   enforce!(eq!(access!(a), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3])))))],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_two_iterable_identifier_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4], d[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        let diff = [x - y for (x, y) in (c, d)];
        enf a = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4), (d, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(diff = lc!(((x, expr!(access!(c))), (y, expr!(access!(d)))) => sub!(access!(x), access!(y))).into() =>
                  enforce!(eq!(access!(a), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3]))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_multiple_iterables_lc() {
    let source = "
    def test

    trace_columns {
        main: [a, b[3], c[4], d[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        let diff = [w + x - y - z for (w, x, y, z) in (0..3, b, c[0..3], d[0..3])];
        enf a = x[0] + x[1] + x[2] + x[3];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 3), (c, 4), (d, 4)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(a, Boundary::First), int!(0)))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            let_!(diff = lc!(((w, range!(0..3)), (x, expr!(access!(b))), (y, expr!(slice!(c, 0..3))), (z, expr!(slice!(d, 0..3)))) =>
                             sub!(sub!(add!(access!(w), access!(x)), access!(y)), access!(z))).into() =>
                  enforce!(eq!(access!(a), add!(add!(add!(access!(x[0]), access!(x[1])), access!(x[2])), access!(x[3]))))),
        ],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

// INVALID LIST COMPREHENSION
// ================================================================================================

#[test]
fn err_bc_lc_one_member_two_iterables() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    boundary_constraints {
        let x = [c for c in (c, d)];
        enf a.first = x;
    }";

    ParseTest::new()
        .expect_module_diagnostic(source, "bindings and iterables lengths are mismatched");
}

#[test]
fn err_bc_lc_two_members_one_iterables() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    boundary_constraints {
        let x = [c + d for (c, d) in c];
        enf a.first = x;
    }";

    ParseTest::new()
        .expect_module_diagnostic(source, "bindings and iterables lengths are mismatched");
}

#[test]
fn err_ic_lc_one_member_two_iterables() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    integrity_constraints {
        let x = [c for c in (c, d)];
        enf a = x;
    }";

    ParseTest::new()
        .expect_module_diagnostic(source, "bindings and iterables lengths are mismatched");
}

#[test]
fn err_ic_lc_two_members_one_iterable() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    integrity_constraints {
        let x = [c + d for (c, d) in c];
        enf a = x;
    }";

    ParseTest::new()
        .expect_module_diagnostic(source, "bindings and iterables lengths are mismatched");
}

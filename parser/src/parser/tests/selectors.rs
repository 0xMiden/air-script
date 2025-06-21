use miden_diagnostics::{SourceSpan, Span};

use crate::ast::*;

use super::ParseTest;

// SELECTORS
// ================================================================================================

#[test]
fn single_selector() {
    let source = r#"
    def test

    trace_columns {
        main: [clk, n1],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk' = clk when n1;
    }"#;
    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1), (n1, 1)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!((("%0", range!(0..1))) => eq!(access!(clk, 1), access!(clk)), when access!(n1))
        )],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn chained_selectors() {
    let source = r#"
    def test

    trace_columns {
        main: [clk, n1, n2, n3],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk' = clk when (n1 & !n2) | !n3;
    }"#;
    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected.trace_columns.push(trace_segment!(
        0,
        "$main",
        [(clk, 1), (n1, 1), (n2, 1), (n3, 1)]
    ));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!((("%0", range!(0..1))) => eq!(access!(clk, 1), access!(clk)), when or!(and!(access!(n1), not!(access!(n2))), not!(access!(n3))))
        )],
    ));

    ParseTest::new().expect_module_ast(source, expected);
}

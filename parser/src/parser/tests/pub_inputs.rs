use miden_diagnostics::{SourceSpan, Span};

use crate::{ast::*, parser::ParseError};

use super::ParseTest;

// PUBLIC INPUTS
// ================================================================================================

#[test]
fn public_inputs_vec() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    public_inputs {
        program_hash: [4],
        stack_inputs: [16],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk = 0;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.public_inputs.insert(
        ident!(program_hash),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(program_hash), 4),
    );
    expected.public_inputs.insert(
        ident!(stack_inputs),
        PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(stack_inputs), 16),
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
        vec![enforce!(eq!(access!(clk), int!(0)))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn public_inputs_table() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    public_inputs {
        a: [[4]],
        b: [[32]],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk = 0;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.public_inputs.insert(
        ident!(a),
        PublicInput::new_table(SourceSpan::UNKNOWN, ident!(a), 4),
    );
    expected.public_inputs.insert(
        ident!(b),
        PublicInput::new_table(SourceSpan::UNKNOWN, ident!(b), 32),
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
        vec![enforce!(eq!(access!(clk), int!(0)))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn error_no_public_input() {
    let source = "
    def test

    trace_columns {
        main: [clk]
    }

    public_inputs { }
    ";
    assert_module_error!(source, ParseError::UnrecognizedToken { .. });
}

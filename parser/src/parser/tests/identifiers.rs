use super::ParseTest;

// TODO: clean up this test file
// IDENTIFIERS
// ================================================================================================

#[test]
fn error_invalid_int() {
    let num: u128 = u64::MAX as u128 + 1;
    let source = format!(
        r#"
    def test

    trace_columns {{
        main: [clk],
    }}

    integrity_constraints {{
        enf clk' = clk + {num}
    }}
    "#,
    );

    // Integers can only be of type u64.
    ParseTest::new().expect_program_diagnostic(&source, "value is too big");
}

// UNRECOGNIZED TOKEN ERRORS
// ================================================================================================

#[test]
fn error_constraint_without_section() {
    // Constraints outside of valid sections are not allowed.
    let source = r#"
    def test

    enf clk' = clk + 1
    "#;

    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn error_identifier_starting_with_int() {
    // Identifiers cannot start with numeric characters.
    // lexer considers the integer 1 and alphabetic clk' to be separate tokens
    // hence this fails at parser level since a valid identifier is expected
    // at that position which 1 is not.
    let source = r#"
    def test

    trace_columns {
        main: [clk]
    }

    integrity_constraints {
        enf 1clk' = clk + 1
    }
    "#;

    ParseTest::new().expect_unrecognized_token(source);
}

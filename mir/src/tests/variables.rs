use super::{compile, expect_diagnostic};

#[test]
fn let_scalar_constant_in_boundary_constraint() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = 1 + 8;
        enf clk.first = a;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn let_vector_constant_in_boundary_constraint() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let b = [1, 5];
        enf clk.first = b[0];
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn multi_constraint_nested_let_with_expressions_in_boundary_constraint() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = 1 + 8;
        let b = [a, a*a];
        enf clk.first = a + b[0];

        let c = [[b[0], b[1]], [clk, 2^2]];
        enf clk.last = c[1][1];
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn let_scalar_constant_in_boundary_constraint_both_domains() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = 1 + 8;
        enf clk.first = a;
        enf clk.last = a;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn invalid_column_offset_in_boundary_constraint() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = clk';
        enf clk.first = 0;
        enf clk.last = a;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(source, "invalid access of a trace column with offset");
}

#[test]
fn nested_let_with_expressions_in_integrity_constraint() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 0;
        enf clk.last = 1;
    }
    integrity_constraints {
        let a = 1;
        let b = [a, a*a];
        let c = [[clk' - clk, clk - a], [1 + 8, 2^2]];
        enf c[0][0] = 1;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn nested_let_with_vector_access_in_integrity_constraint() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 7;
        enf clk.last = 8;
    }
    integrity_constraints {
        let a = [[1, 2], [3, 4]];
        let b = a[1];
        let c = b;
        let d = [a[0], a[1], b];
        let e = d;
        enf clk' = c[0] + e[2][0] + e[0][1];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn invalid_matrix_literal_with_leading_vector_binding() {
    // We can not parse matrix variable that consists of inlined vector and scalar elements.
    // VariableBinding `d` is parsed as a vector and can not contain inlined vectors.
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 7;
        enf clk.last = 8;
    }
    integrity_constraints {
        let a = [[1, 2], [3, 4]];
        let d = [a[0], [3, 4]];
        enf clk' = d[0][0];
    }";

    expect_diagnostic(source, "expected one of: '\"!\"', '\"(\"', '\"null\"', '\"unconstrained\"', 'decl_ident_ref', 'function_identifier', 'identifier', 'int'");
}

#[test]
fn invalid_matrix_literal_with_trailing_vector_binding() {
    // We can not parse matrix variable that consists of inlined vector and scalar elements
    // VariableBinding `d` is parsed as a matrix and can not contain references to vectors.
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 7;
        enf clk.last = 8;
    }
    integrity_constraints {
        let a = [[1, 2], [3, 4]];
        let d = [[3, 4], a[0]];
        enf clk' = d[0][0];
    }";

    expect_diagnostic(source, "expected one of: '\"[\"'");
}

#[test]
fn invalid_variable_access_before_declaration() {
    let source = "
    def test
    const A = [[2, 3], [1, 0]];
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = a;
        let a = 0;
        enf clk.last = 1;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(source, "this variable / bus is not defined");
}

#[test]
fn invalid_trailing_let() {
    let source = "
    def test
    const A = [[2, 3], [1, 0]];
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 0;
        enf clk.last = 1;
    }
    integrity_constraints {
        enf clk' = clk + a;
        let a = 1;
    }";

    expect_diagnostic(source, "expected one of: '\"enf\"', '\"let\"'");
}

#[test]
fn invalid_reference_to_variable_defined_in_other_section() {
    let source = "
    def test
    const A = [[2, 3], [1, 0]];
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = 1;
        enf clk.first = 0;
        enf clk.last = 1;
    }
    integrity_constraints {
        enf clk' = clk + a;
    }";

    expect_diagnostic(source, "this variable / bus is not defined");
}

#[test]
fn invalid_vector_variable_access_out_of_bounds() {
    let source = "
    def test
    const A = [[2, 3], [1, 0]];
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = [1, 2];
        enf clk.first = a[2];
        enf clk.last = 1;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn invalid_matrix_column_variable_access_out_of_bounds() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = [[1, 2, 3], [4, 5, 6]];
        enf clk.first = a[1][3];
        enf clk.last = 1;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn invalid_matrix_row_variable_access_out_of_bounds() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        let a = [[1, 2, 3], [4, 5, 6]];
        enf clk.first = 0;
        enf clk.last = a[2][0];
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn invalid_index_into_scalar_variable() {
    let source = "
    def test
    const A = 123;
    const B = [1, 2, 3];
    const C = [[1, 2, 3], [4, 5, 6]];
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 1;
    }
    integrity_constraints {
        let a = A;
        enf clk' = clk + a[0];
    }";

    expect_diagnostic(source, "attempted to index into a scalar value");
}

#[test]
fn trace_binding_access_in_integrity_constraint() {
    let source = "
    def test
    const A = 123;
    const B = [1, 2, 3];
    const C = [[1, 2, 3], [4, 5, 6]];
    trace_columns {
        main: [clk, x[4]],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 1;
    }
    integrity_constraints {
        let a = x;
        enf clk' = clk + a[0];
    }";

    assert!(compile(source).is_ok());
}

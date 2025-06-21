use super::{Pipeline, compile, expect_diagnostic};

#[test]
fn boundary_constraint_with_constants() {
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
        enf clk.first = A;
        enf clk.last = B[0] + C[0][1];
    }
    integrity_constraints {
        enf clk' = clk - 1;
    }";

    assert!(compile(source, Pipeline::WithoutMIR).is_ok());
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

#[test]
fn integrity_constraint_with_constants() {
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
        enf clk.first = 0;
    }
    integrity_constraints {
        enf clk' = clk + A + B[1] - C[1][2];
    }";

    assert!(compile(source, Pipeline::WithoutMIR).is_ok());
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

#[test]
fn invalid_matrix_constant() {
    let source = "
    def test
    const A = [[2, 3], [1, 0, 2]];
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
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "invalid matrix literal: mismatched dimensions",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "invalid matrix literal: mismatched dimensions",
        Pipeline::WithMIR,
    );
}

use super::{expect_diagnostic, Pipeline};

#[test]
fn invalid_vector_access_in_boundary_constraint() {
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
        enf clk.first = A + B[3] - C[1][2];
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithMIR,
    );
}

#[test]
fn invalid_matrix_row_access_in_boundary_constraint() {
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
        enf clk.first = A + B[1] - C[3][2];
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithMIR,
    );
}

#[test]
fn invalid_matrix_column_access_in_boundary_constraint() {
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
        enf clk.first = A + B[1] - C[1][3];
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithMIR,
    );
}

#[test]
fn invalid_vector_access_in_integrity_constraint() {
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
        enf clk' = clk + A + B[3] - C[1][2];
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithMIR,
    );
}

#[test]
fn invalid_matrix_row_access_in_integrity_constraint() {
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
        enf clk' = clk + A + B[1] - C[3][2];
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithMIR,
    );
}

#[test]
fn invalid_matrix_column_access_in_integrity_constraint() {
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
        enf clk' = clk + A + B[1] - C[1][3];
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
        Pipeline::WithMIR,
    );
}

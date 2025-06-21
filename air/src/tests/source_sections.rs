use super::{Pipeline, expect_diagnostic};

#[test]
fn err_trace_cols_empty() {
    // if trace columns is empty, an error should be returned at parser level.
    let source = "
    def test
    trace_columns {}
    public_inputs {
        stack_inputs: [16]
    boundary_constraints {
        enf clk.first = 0
    integrity_constraints {
        enf clk' = clk + 1";

    expect_diagnostic(source, "missing 'main' declaration in this section", Pipeline::WithoutMIR);
    expect_diagnostic(source, "missing 'main' declaration in this section", Pipeline::WithMIR);
}

#[test]
fn err_trace_cols_omitted() {
    // returns an error if trace columns declaration is missing
    let source = "
    def test
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 0;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(source, "missing trace_columns section", Pipeline::WithoutMIR);
    expect_diagnostic(source, "missing trace_columns section", Pipeline::WithMIR);
}

#[test]
fn err_pub_inputs_empty() {
    // if public inputs are empty, an error should be returned at parser level.
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {}
    boundary_constraints {
        enf clk.first = 0;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(source, "expected one of: 'identifier'", Pipeline::WithoutMIR);
    expect_diagnostic(source, "expected one of: 'identifier'", Pipeline::WithMIR);
}

#[test]
fn err_pub_inputs_omitted() {
    // if public inputs are omitted, an error should be returned at IR level.
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    boundary_constraints {
        enf clk.first = 0;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "root module must contain a public_inputs section",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "root module must contain a public_inputs section",
        Pipeline::WithMIR,
    );
}

#[test]
fn err_bc_empty() {
    // if boundary constraints are empty, an error should be returned at parser level.
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {}
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(source, "expected one of: '\"enf\"', '\"let\"'", Pipeline::WithoutMIR);
    expect_diagnostic(source, "expected one of: '\"enf\"', '\"let\"'", Pipeline::WithMIR);
}

#[test]
fn err_bc_omitted() {
    // if boundary constraints are omitted, an error should be returned at IR level.
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    expect_diagnostic(
        source,
        "root module must contain both boundary_constraints and integrity_constraints sections",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "root module must contain both boundary_constraints and integrity_constraints sections",
        Pipeline::WithMIR,
    );
}

#[test]
fn err_ic_empty() {
    // if integrity constraints are empty, an error should be returned at parser level.
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
    }
    integrity_constraints {}";

    expect_diagnostic(source, "expected one of: '\"enf\"', '\"let\"'", Pipeline::WithoutMIR);
    expect_diagnostic(source, "expected one of: '\"enf\"', '\"let\"'", Pipeline::WithMIR);
}

#[test]
fn err_ic_omitted() {
    // if integrity constraints are omitted, an error should be returned at IR level.
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
    }";

    expect_diagnostic(
        source,
        "root module must contain both boundary_constraints and integrity_constraints sections",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "root module must contain both boundary_constraints and integrity_constraints sections",
        Pipeline::WithMIR,
    );
}

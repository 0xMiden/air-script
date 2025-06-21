use super::{Pipeline, compile, expect_diagnostic};

mod comprehension;

#[test]
fn integrity_constraints() {
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
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    assert!(compile(source, Pipeline::WithoutMIR).is_ok());
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

#[test]
fn ic_using_parens() {
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
    integrity_constraints {
        enf clk' = (clk + 1);
    }";

    assert!(compile(source, Pipeline::WithoutMIR).is_ok());
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

#[test]
fn ic_op_mul() {
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
    integrity_constraints {
        enf clk' * clk = 1;
    }";

    assert!(compile(source, Pipeline::WithoutMIR).is_ok());
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

#[test]
fn ic_op_exp() {
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
    integrity_constraints {
        enf clk'^2 - clk = 1;
    }";

    assert!(compile(source, Pipeline::WithoutMIR).is_ok());
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

#[test]
fn err_non_const_exp_outside_lc() {
    // non const exponents are not allowed outside of list comprehensions
    let source = "
    def test
    trace_columns {
        main: [clk, fmp[2], ctx, a, b, c[4], d[4]],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf c[2].first = 0;
    }
    integrity_constraints {
        enf clk = 2^ctx;
    }";

    expect_diagnostic(
        source,
        "expected exponent to be a constant",
        Pipeline::WithoutMIR,
    );
    expect_diagnostic(
        source,
        "expected exponent to be a constant",
        Pipeline::WithMIR,
    );
}

use super::super::{compile, expect_diagnostic};

#[test]
fn list_comprehension() {
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
        let x = [fmp for fmp in fmp];
        enf clk = x[1];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn lc_with_const_exp() {
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
        let y = [col^7 for col in c];
        let z = [col'^7 - col for col in c];
        enf clk = y[1] + z[1];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn lc_with_two_lists() {
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
        let diff = [x - y for (x, y) in (c, d)];
        enf clk = diff[0];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn lc_with_two_slices() {
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
        let diff = [x - y for (x, y) in (c[0..2], d[1..3])];
        enf clk = diff[1];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn lc_with_multiple_lists() {
    let source = "
    def test
    trace_columns {
        main: [a, b[3], c[4], d[4]],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf c[2].first = 0;
    }
    integrity_constraints {
        let x = [w + x - y - z for (w, x, y, z) in (0..3, b, c[0..3], d[0..3])];
        enf a = x[0] + x[1] + x[2];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn err_index_out_of_range_lc_ident() {
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
        let x = [fmp for fmp in fmp];
        enf clk = x[2];
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn err_index_out_of_range_lc_slice() {
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
        let x = [z for z in c[1..3]];
        enf clk = x[3];
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn err_duplicate_member() {
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
        let duplicate_member_lc = [c * d for (c, c) in (c, d)];
        enf clk = duplicate_member_lc[1];
    }";

    expect_diagnostic(source, "this name is already bound in this comprehension");
}

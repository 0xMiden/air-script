use super::{compile, expect_diagnostic};

#[test]
fn simple_evaluator() {
    let source = "
    def test
    ev advance_clock([clk]) {
        enf clk' = clk + 1;
    }
    
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
        enf advance_clock([clk]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn evaluator_with_variables() {
    let source = "
    def test
    ev advance_clock([clk]) {
        let z = clk + 1;
        enf clk' = z;
    }
    
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
        enf advance_clock([clk]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn evaluator_with_main_and_aux_cols() {
    let source = "
    def test
    ev enforce_constraints([clk], [a, b]) {
        let z = a + b;
        enf clk' = clk + 1;
        enf a' = a + z;
    }
    
    trace_columns {
        main: [clk],
        aux: [a, b],
    }

    public_inputs {
        stack_inputs: [16],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf enforce_constraints([clk], [a, b]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn ev_call_with_aux_only() {
    let source = "
    def test
    ev enforce_a([], [a, b]) {
        enf a' = a + 1;
    }
    
    trace_columns {
        main: [clk],
        aux: [a, b],
    }

    public_inputs {
        stack_inputs: [16],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf enforce_a([], [a, b]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn ev_call_inside_evaluator_with_main() {
    let source = "
    def test
    ev enforce_clk([clk]) {
        enf clk' = clk + 1;
    }
    
    ev enforce_all_constraints([clk]) {
        enf enforce_clk([clk]);
    }
    
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
        enf enforce_all_constraints([clk]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn ev_call_inside_evaluator_with_aux() {
    let source = "
    def test
    ev enforce_clk([clk]) {
        enf clk' = clk + 1;
    }
    
    ev enforce_a([], [a, b]) {
        enf a' = a + 1;
    }
    
    ev enforce_all_constraints([clk], [a, b]) {
        enf enforce_clk([clk]);
        enf enforce_a([], [a, b]);
    }
    
    trace_columns {
        main: [clk],
        aux: [a, b],
    }

    public_inputs {
        stack_inputs: [16],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf enforce_all_constraints([clk], [a, b]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn ev_fn_call_with_column_group() {
    let source = "
    def test
    ev clk_selectors([selectors[3], a, clk]) {
        enf (clk' - clk) * selectors[0] * selectors[1] * selectors[2] = 0;
    }
    
    trace_columns {
        main: [s[3], a, clk],
    }

    public_inputs {
        stack_inputs: [16],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk_selectors([s, clk, a]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn err_ev_fn_call_wrong_segment_columns() {
    let source = "
    def test
    ev is_binary([x]) {
        enf x^2 = x;
    }
    
    trace_columns {
        main: [b],
        aux: [c],
    }

    public_inputs {
        stack_inputs: [16],
    }

    boundary_constraints {
        enf b.first = 0;
    }

    integrity_constraints {
        enf is_binary([c]);
    }";

    expect_diagnostic(source, "callee expects columns from the $main trace");
}

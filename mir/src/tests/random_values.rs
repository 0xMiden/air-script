use super::{compile, expect_diagnostic};

#[test]
fn random_values_indexed_access() {
    let source = "
    def test
    trace_columns {
        main: [a, b[12]],
        aux: [c, d],
    }
    public_inputs {
        stack_inputs: [16],
    }
    random_values {
        rand: [16],
    }
    boundary_constraints {
        enf c.first = $rand[10] * 2;
        enf c.last = 1;
    }
    integrity_constraints {
        enf c' = $rand[3] + 1;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn random_values_custom_name() {
    let source = "
    def test
    trace_columns {
        main: [a, b[12]],
        aux: [c, d],
    }
    public_inputs {
        stack_inputs: [16],
    }
    random_values {
        alphas: [16],
    }
    boundary_constraints {
        enf c.first = $alphas[10] * 2;
        enf c.last = 1;
    }
    integrity_constraints {
        enf c' = $alphas[3] + 1;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn random_values_named_access() {
    let source = "
    def test
    trace_columns {
        main: [a, b[12]],
        aux: [c, d],
    }
    public_inputs {
        stack_inputs: [16],
    }
    random_values {
        rand: [m, n[4]],
    }
    boundary_constraints {
        enf c.first = (n[1] - $rand[0]) * 2;
        enf c.last = m;
    }
    integrity_constraints {
        enf c' = m + n[2] + $rand[1];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn err_random_values_out_of_bounds_no_bindings() {
    let source = "
    def test
    trace_columns {
        main: [a, b[12]],
        aux: [c, d],
    }
    public_inputs {
        stack_inputs: [16],
    }
    random_values {
        rand: [4],
    }
    boundary_constraints {
        enf a.first = $rand[10] * 2;
        enf a.last = 1;
    }
    integrity_constraints {
        enf a' = $rand[4] + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn err_random_values_out_of_bounds_binding_ref() {
    let source = "
    def test
    trace_columns {
        main: [a, b[12]],
        aux: [c, d],
    }
    public_inputs {
        stack_inputs: [16],
    }
    random_values {
        rand: [m, n[4]],
    }
    boundary_constraints {
        enf a.first = n[5] * 2;
        enf a.last = 1;
    }
    integrity_constraints {
        enf a' = m + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn err_random_values_out_of_bounds_global_ref() {
    let source = "
    def test
    trace_columns {
        main: [a, b[12]],
        aux: [c, d],
    }
    public_inputs {
        stack_inputs: [16],
    }
    random_values {
        rand: [m, n[4]],
    }
    boundary_constraints {
        enf a.first = $rand[10] * 2;
        enf a.last = 1;
    }
    integrity_constraints {
        enf a' = m + 1;
    }";

    expect_diagnostic(
        source,
        "attempted to access an index which is out of bounds",
    );
}

#[test]
fn err_random_values_without_aux_cols() {
    let source = "
    def test
    trace_columns {
        main: [a, b[12]],
    }
    public_inputs {
        stack_inputs: [16],
    }
    random_values {
        rand: [16],
    }
    boundary_constraints {
        enf a.first = 2;
        enf a.last = 1;
    }
    integrity_constraints {
        enf a' = a + 1;
    }";

    expect_diagnostic(
        source,
        "declaring random_values requires an aux trace_columns declaration",
    );
}

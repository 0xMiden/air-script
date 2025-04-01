use super::{compile, expect_diagnostic, Pipeline};

#[test]
fn buses_in_boundary_constraints() {
    let source = "
        def test

    trace_columns {
        main: [a],
    }

    buses {
        multiset p,
        logup q,
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf p.first = null;
        enf q.first = null;
        enf p.last = null;
        enf q.last = null;
        # TODO: to be used when we have support for variable-length public inputs
        #enf p.last = inputs;
        #enf q.last = inputs;
    }

    integrity_constraints {
        enf a = 0;
    }";

    expect_diagnostic(
        source,
        "buses are not implemented for this Pipeline",
        Pipeline::WithoutMIR,
    );
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

#[test]
fn buses_in_integrity_constraints() {
    let source = "
        def test

    trace_columns {
        main: [a],
    }

    fn double(a: felt) -> felt {
        return a+a;
    }

    buses {
        multiset p,
        logup q,
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf p.first = null;
        enf q.first = null;
        enf p.last = null;
        enf q.last = null;
        # TODO: to be used when we have support for variable-length public inputs
        #enf p.last = inputs;
        #enf q.last = inputs;
    }

    integrity_constraints {
        p.insert(double(a)) when 1;
        p.remove(1) when 1;
        q.insert(1, 2) when 1;
        q.insert(1, 2) when 1;
        q.remove(1, 2) with 2;
    }";

    expect_diagnostic(
        source,
        "buses are not implemented for this Pipeline",
        Pipeline::WithoutMIR,
    );
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

// Tests that should return errors
#[test]
fn err_buses_boundaries_to_const() {
    let source = "
        def test

    trace_columns {
        main: [a],
    }

    buses {
        multiset p,
        logup q,
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf p.first = 0;
        enf q.last = null;
    }

    integrity_constraints {
        enf a = 0;
    }";

    expect_diagnostic(source, "error: invalid constraint", Pipeline::WithoutMIR);
    expect_diagnostic(source, "error: invalid constraint", Pipeline::WithMIR);
}

#[test]
fn err_trace_columns_constrained_with_null() {
    let source = "
        def test

    trace_columns {
        main: [a],
    }

    buses {
        multiset p,
        logup q,
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf a.last = null;
    }

    integrity_constraints {
        enf a = 0;
    }";

    expect_diagnostic(source, "error: invalid constraint", Pipeline::WithoutMIR);
    expect_diagnostic(source, "error: invalid constraint", Pipeline::WithMIR);
}

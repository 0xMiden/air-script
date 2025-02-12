use crate::tests::{parse, translate};

use super::{compile, expect_diagnostic};

#[test]
fn buses_in_boundary_constraints() {
    let source = "
        def test

    trace_columns {
        main: [a],
    }

    buses {
        unit p,
        mult q,
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf p.first = null;
        enf q.last = null;
    }

    integrity_constraints {
        enf a = 0;
    }";
    let ast = parse(source);
    eprintln!("{:#?}", ast);
    assert!(compile(source).is_ok());
}

#[test]
fn buses_in_integrity_constraints() {
    let source = "
        def test

    trace_columns {
        main: [a],
    }

    buses {
        unit p,
        mult q,
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf p.first = null;
        enf q.last = null;
    }

    integrity_constraints {
        p.add(1) when 1;
        p.rem(1) when 1;
        q.add(1, 2) when 1;
        q.add(1, 2) when 1;
        q.rem(1, 2) for 2;
    }";

    assert!(compile(source).is_ok());
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
        unit p,
        mult q,
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

    expect_diagnostic(source, "error: invalid constraint");
}

#[test]
fn err_trace_columns_constrained_with_null() {
    let source = "
        def test

    trace_columns {
        main: [a],
    }

    buses {
        unit p,
        mult q,
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

    expect_diagnostic(source, "error: invalid constraint");
}

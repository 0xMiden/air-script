use air_parser::ast;

use super::{compile, expect_diagnostic};
use crate::ir::{Link, MirValue, Op, PublicInputTableAccess};

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
    }

    integrity_constraints {
        enf a = 0;
    }";
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
    }

    integrity_constraints {
        p.insert(1) when 1;
        p.remove(1) when 1;
        q.insert(1, 2) when 1;
        q.insert(1, 2) when 1;
        q.remove(1, 2) with 2;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn buses_table_in_boundary_constraints() {
    let source = "
    def test
    trace_columns {
        main: [a],
    }

    public_inputs {
        x: [[2]],
        y: [[3]],
    }

    buses {
        multiset p,
    }

    boundary_constraints {
        enf p.first = x;
        enf p.last = y;
    }

    integrity_constraints {
        enf a = 0;
    }";

    let result = compile(source);
    assert!(result.is_ok());

    let get_name = |op: &Link<Op>| -> (ast::Identifier, usize) {
        let MirValue::PublicInputTable(PublicInputTableAccess { table_name, num_cols, .. }) =
            op.as_value().unwrap().value.value
        else {
            panic!("Expected a public input, got {op:#?}");
        };
        (table_name, num_cols)
    };
    let mir = result.unwrap();
    let bus = mir.constraint_graph().buses.values().next().unwrap();
    let p = bus.borrow();
    let (first, first_nc) = get_name(&p.get_first());
    let (last, last_nc) = get_name(&p.get_last());
    let public_inputs = &mir.public_inputs;
    let mut pi = public_inputs.keys();
    let (x, y) = (pi.next().unwrap(), pi.next().unwrap());
    assert_eq!(first, x);
    assert_eq!(last, y);
    assert_eq!(first_nc, 2);
    assert_eq!(last_nc, 3);
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

    expect_diagnostic(source, "error: invalid constraint");
}

#[test]
fn err_bus_constrained_with_bus_access() {
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
        enf p.last = p.first;
    }

    integrity_constraints {
        enf a = 0;
    }";

    expect_diagnostic(source, "error: invalid constraint");
}

#[test]
fn err_trace_columns_constrained_with_bus_access() {
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
        enf a.last = p.first;
    }

    integrity_constraints {
        enf a = 0;
    }";

    expect_diagnostic(source, "error: invalid expression");
}

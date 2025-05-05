use crate::{
    ir::{
        assert_bus_eq, Add, Builder, Bus, BusVariableBoundary, Fold, FoldOperator, Link, Mir,
        MirValue, Op, Vector,
    },
    tests::translate,
};
use air_parser::{ast, Symbol};
use miden_diagnostics::SourceSpan;

use super::{compile, expect_diagnostic};

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
fn buses_args_expr_in_integrity_expr() {
    let source = "
    def test

    trace_columns {
        main: [a],
    }

    public_inputs {
        inputs: [2],
    }

    buses {
        multiset p,
    }

    boundary_constraints {
        enf p.first = null;
    }

    integrity_constraints {
        let vec = [x for x in 0..3];
        let b = 41;
        let x = sum(vec) + b;
        p.insert(x) when 1;
        p.remove(x) when 0;
    }";
    assert!(compile(source).is_ok());
    let mut result_mir = translate(source).unwrap();
    let bus = Bus::create(
        ast::Identifier::new(SourceSpan::default(), Symbol::new(0)),
        ast::BusType::Multiset,
        SourceSpan::default(),
    );
    let vec_op = Vector::builder()
        .size(3)
        .elements(From::from(0))
        .elements(From::from(1))
        .elements(From::from(2))
        .span(SourceSpan::default())
        .build();
    let b: Link<Op> = From::from(41);
    let vec_sum = Fold::builder()
        .iterator(vec_op)
        .operator(FoldOperator::Add)
        .initial_value(From::from(0))
        .span(SourceSpan::default())
        .build();
    let x: Link<Op> = Add::builder()
        .lhs(vec_sum)
        .rhs(b.clone())
        .span(SourceSpan::default())
        .build();
    let sel: Link<Op> = From::from(1);
    let _p_add = bus.insert(&[x.clone()], sel.clone(), SourceSpan::default());
    let not_sel: Link<Op> = From::from(0);
    let _p_rem = bus.remove(&[x.clone()], not_sel.clone(), SourceSpan::default());
    let bus_ident = result_mir.constraint_graph().buses.keys().next().unwrap();
    let mut expected_mir = Mir::new(result_mir.name);
    let _ = expected_mir
        .constraint_graph_mut()
        .insert_bus(*bus_ident, bus.clone().clone());
    assert_bus_eq(&mut expected_mir, &mut result_mir);
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
        let MirValue::PublicInputBinding(BusVariableBoundary {
            table_name,
            bus_name: _,
            num_cols,
        }) = op.as_value().unwrap().value.value
        else {
            panic!("Expected a public input, got {:#?}", op);
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

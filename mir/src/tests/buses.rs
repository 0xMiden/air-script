use crate::{
    ir::{assert_bus_eq, Add, Builder, Bus, Fold, FoldOperator, Link, Mir, Op, Vector},
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
        # TODO: to be used when we have support for variable-length public inputs
        #enf p.last = inputs;
        #enf q.last = inputs;
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
        # TODO: to be used when we have support for variable-length public inputs
        #enf p.last = inputs;
        #enf q.last = inputs;
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
        p.add(x) when 1;
        p.rem(x) when 0;
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
    let _p_add = bus.add(&[x.clone()], sel.clone(), SourceSpan::default());
    let not_sel: Link<Op> = From::from(0);
    let _p_rem = bus.rem(&[x.clone()], not_sel.clone(), SourceSpan::default());
    let bus_ident = result_mir.constraint_graph().buses.keys().next().unwrap();
    let mut expected_mir = Mir::new(result_mir.name);
    let _ = expected_mir
        .constraint_graph_mut()
        .insert_bus(*bus_ident, bus.clone().clone());
    assert_bus_eq(&mut expected_mir, &mut result_mir);
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

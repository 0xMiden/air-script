use miden_diagnostics::{SourceSpan, Span};

use crate::ast::*;

use super::ParseTest;

// INTEGRITY STATEMENTS
// ================================================================================================

#[test]
fn integrity_constraints() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk' = clk + 1;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(access!(clk, 1), add!(access!(clk), int!(1))))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn integrity_constraints_with_buses() {
    let source = "
    def test

    trace_columns {
        main: [clk],
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
        enf q.last = null;
    }

    integrity_constraints {
        p.insert(1) when 1;
        p.remove(1) when 1;
        q.insert(1, 2) when 1;
        q.insert(1, 2) when 1;
        q.remove(1, 2) for 2;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce!(eq!(bounded_access!(p, Boundary::First), null!())),
            enforce!(eq!(bounded_access!(q, Boundary::Last), null!())),
        ],
    ));
    expected.buses.insert(
        ident!(p),
        Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset),
    );
    expected.buses.insert(
        ident!(q),
        Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup),
    );

    let mut bus_enforces = Vec::new();

    // p.insert(1) when 1;
    bus_enforces.push(bus_enforce!(lc!(
        (("%0", range!(0..1))) => 
        bus_insert!(p, vec![expr!(int!(1))]),
        when int!(1))));

    //p.remove(1) when 1;
    bus_enforces.push(bus_enforce!(lc!(
        (("%1", range!(0..1))) => 
        bus_remove!(p, vec![expr!(int!(1))]),
        when int!(1))));

    //q.insert(1, 2) when 1;
    bus_enforces.push(bus_enforce!(lc!(
        (("%2", range!(0..1))) => 
        bus_insert!(q, vec![expr!(int!(1)), expr!(int!(2))]),
        when int!(1))));

    //q.insert(1, 2) when 1;
    bus_enforces.push(bus_enforce!(lc!(
        (("%3", range!(0..1))) => 
        bus_insert!(q, vec![expr!(int!(1)), expr!(int!(2))]),
        when int!(1))));

    //q.remove(1, 2) for 2;
    bus_enforces.push(bus_enforce!(lc!(
        (("%4", range!(0..1))) => 
        bus_remove!(q, vec![expr!(int!(1)), expr!(int!(2))]),
        for int!(2))));

    expected.integrity_constraints = Some(Span::new(SourceSpan::UNKNOWN, bus_enforces));

    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn err_integrity_constraints_invalid() {
    let source = "
    def test

    trace_columns {
        main: [clk]
    }

    public_inputs {
        inputs: [2]
    }

    boundary_constraints {
        enf clk.first = 0
    }

    integrity_constraints {
        enf clk' = clk = 1
    }";

    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn multiple_integrity_constraints() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk' = clk + 1;
        enf clk' - clk = 1;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce!(eq!(access!(clk, 1), add!(access!(clk), int!(1)))),
            enforce!(eq!(sub!(access!(clk, 1), access!(clk)), int!(1))),
        ],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn integrity_constraint_with_periodic_col() {
    let source = "
    def test

    trace_columns {
        main: [b],
    }

    public_inputs {
        inputs: [2],
    }

    periodic_columns {
        k0: [1, 0],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf k0 + b = 0;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(b, 1)]));
    expected.periodic_columns.insert(
        ident!(k0),
        PeriodicColumn::new(SourceSpan::UNKNOWN, ident!(k0), vec![1, 0]),
    );
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(add!(access!(k0), access!(b)), int!(0)))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn integrity_constraint_with_random_value() {
    let source = "
    def test

    trace_columns {
        main: [a],
        aux: [aux0[2]],
    }

    public_inputs {
        inputs: [2],
    }

    random_values {
        rand: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf a + $rand[1] = 0;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1)]));
    expected
        .trace_columns
        .push(trace_segment!(1, "$aux", [(aux0, 2)]));
    expected.random_values = Some(random_values!("$rand", 2));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            add!(access!(a), access!("$rand"[1])),
            int!(0)
        ))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn integrity_constraint_with_constants() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    const A = 0;
    const B = [0, 1];
    const C = [[0, 1], [1, 0]];

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf clk + A = B[1] + C[1][1];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.constants.insert(ident!(A), constant!(A = 0));
    expected.constants.insert(ident!(B), constant!(B = [0, 1]));
    expected
        .constants
        .insert(ident!(C), constant!(C = [[0, 1], [1, 0]]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            add!(access!(clk), access!(A)),
            add!(access!(B[1]), access!(C[1][1]))
        ))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn integrity_constraint_with_variables() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        let a = 2^2;
        let b = [a, 2 * a];
        let c = [[a - 1, a^2], [b[0], b[1]]];
        enf clk + a = b[1] + c[1][1];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(clk, 1)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![let_!(a = expr!(exp!(int!(2), int!(2))) =>
                 let_!(b = vector!(access!(a), mul!(int!(2), access!(a))) =>
                     let_!(c = matrix!([sub!(access!(a), int!(1)), exp!(access!(a), int!(2))], [access!(b[0]), access!(b[1])]) =>
                         enforce!(eq!(add!(access!(clk), access!(a)), add!(access!(b[1]), access!(c[1][1])))))))],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn integrity_constraint_with_indexed_trace_access() {
    let source = "
    def test

    trace_columns {
        main: [a, b],
        aux: [c, d],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf $main[0]' = $main[1] + 1;
        enf $aux[0]' - $aux[1] = 1;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1)]));
    expected
        .trace_columns
        .push(trace_segment!(1, "$aux", [(c, 1), (d, 1)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce!(eq!(
                access!("$main"[0], 1),
                add!(access!("$main"[1]), int!(1))
            )),
            enforce!(eq!(
                sub!(access!("$aux"[0], 1), access!("$aux"[1])),
                int!(1)
            )),
        ],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

// CONSTRAINT COMPREHENSION
// ================================================================================================

#[test]
fn ic_comprehension_one_iterable_identifier() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf x = a + b for x in c;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, expr!(access!(c)))) => eq!(access!(x), add!(access!(a), access!(b))))
        )],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_comprehension_one_iterable_range() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf x = a + b for x in (1..4);
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, range!(1..4))) => eq!(access!(x), add!(access!(a), access!(b))))
        )],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_comprehension_with_selectors() {
    let source = "
    def test

    trace_columns {
        main: [s[2], a, b, c[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf x = a + b for x in c when s[0] & s[1];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(s, 2), (a, 1), (b, 1), (c, 4)]));
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, expr!(access!(c)))) => eq!(access!(x), add!(access!(a), access!(b))), when and!(access!(s[0]), access!(s[1])))
        )],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_comprehension_with_evaluator_call() {
    let source = "
    def test

    ev is_binary([x]) {
        enf x^2 = x;
    }

    trace_columns {
        main: [a, b, c[4], d[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf is_binary([x]) for x in c;
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected
        .trace_columns
        .push(trace_segment!(0, "$main", [(a, 1), (b, 1), (c, 4), (d, 4)]));
    expected.evaluators.insert(
        ident!(is_binary),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(is_binary),
            vec![trace_segment!(0, "%0", [(x, 1)])],
            vec![enforce!(eq!(exp!(access!(x), int!(2)), access!(x)))],
        ),
    );
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, expr!(access!(c)))) => call!(is_binary(vector!(access!(x)))))
        )],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_comprehension_with_evaluator_and_selectors() {
    let source = "
    def test

    ev is_binary([x]) {
        enf x^2 = x;
    }

    trace_columns {
        main: [s[2], a, b, c[4], d[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf is_binary([x]) for x in c when s[0] & s[1];
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected.trace_columns.push(trace_segment!(
        0,
        "$main",
        [(s, 2), (a, 1), (b, 1), (c, 4), (d, 4)]
    ));
    expected.evaluators.insert(
        ident!(is_binary),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(is_binary),
            vec![trace_segment!(0, "%0", [(x, 1)])],
            vec![enforce!(eq!(exp!(access!(x), int!(2)), access!(x)))],
        ),
    );
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce_all!(
            lc!(((x, expr!(access!(c)))) => call!(is_binary(vector!(access!(x)))), when and!(access!(s[0]), access!(s[1])))
        )],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn ic_match_constraint() {
    let source = "
    def test

    ev is_binary([x]) {
        enf x^2 = x;
    }

    trace_columns {
        main: [s[2], a, b, c[4], d[4]],
    }

    public_inputs {
        inputs: [2],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        enf match {
            case s[0] & s[1]: is_binary([c[0]]),
            case s[0]: c[1] = c[2],
        };
    }";

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    expected.trace_columns.push(trace_segment!(
        0,
        "$main",
        [(s, 2), (a, 1), (b, 1), (c, 4), (d, 4)]
    ));
    expected.evaluators.insert(
        ident!(is_binary),
        EvaluatorFunction::new(
            SourceSpan::UNKNOWN,
            ident!(is_binary),
            vec![trace_segment!(0, "%0", [(x, 1)])],
            vec![enforce!(eq!(exp!(access!(x), int!(2)), access!(x)))],
        ),
    );
    expected.public_inputs.insert(
        ident!(inputs),
        PublicInput::new(SourceSpan::UNKNOWN, ident!(inputs), 2),
    );
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(
            bounded_access!(clk, Boundary::First),
            int!(0)
        ))],
    ));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce_all!(
                lc!((("%1", range!(0..1))) => call!(is_binary(vector!(access!(c[0])))), when and!(access!(s[0]), access!(s[1])))
            ),
            enforce_all!(
                lc!((("%2", range!(0..1))) => eq!(access!(c[1]), access!(c[2])), when access!(s[0]))
            ),
        ],
    ));
    ParseTest::new().expect_module_ast(source, expected);
}

// INVALID INTEGRITY CONSTRAINT COMPREHENSION
// ================================================================================================

#[test]
fn err_ic_comprehension_one_member_two_iterables() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    integrity_constraints {
        enf a = c for c in (c, d);
    }";

    ParseTest::new()
        .expect_module_diagnostic(source, "bindings and iterables lengths are mismatched");
}

#[test]
fn err_ic_comprehension_two_members_one_iterable() {
    let source = "
    def test

    trace_columns {
        main: [a, b, c[4]],
    }

    integrity_constraints {
        enf a = c + d for (c, d) in c;
    }";

    ParseTest::new()
        .expect_module_diagnostic(source, "bindings and iterables lengths are mismatched");
}

// INVALID INTEGRITY CONSTRAINTS
// ================================================================================================

#[test]
fn err_missing_integrity_constraint() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    integrity_constraints {
        let a = 2^2;
        let b = [a, 2 * a];
        let c = [[a - 1, a^2], [b[0], b[1]]];
    }";
    ParseTest::new().expect_module_diagnostic(source, "expected one of: '\"enf\"', '\"let\"'");
}

#[test]
fn ic_invalid() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    integrity_constraints {
        enf clk' = clk = 1
    }";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn error_invalid_next_usage() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    integrity_constraints {
        enf clk'' = clk + 1
    }";
    ParseTest::new().expect_unrecognized_token(source);
}

#[test]
fn err_empty_integrity_constraints() {
    let source = "
    def test

    trace_columns {
        main: [clk],
    }

    integrity_constraints {}
        
    boundary_constraints {
        enf clk.first = 1
    }";
    ParseTest::new().expect_unrecognized_token(source);
}

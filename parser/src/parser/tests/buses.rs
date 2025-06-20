use miden_diagnostics::{SourceSpan, Span};

use super::ParseTest;
use crate::ast::*;

const BASE_MODULE: &str = r#"
def test

trace_columns {
    main: [clk],
}

public_inputs {
    inputs: [2],
}"#;

fn add_base_expectations(expected: &mut Module) {
    expected.trace_columns.push(trace_segment!(0, "$main", [(clk, 1)]));
    expected
        .public_inputs
        .insert(ident!(inputs), PublicInput::new_vector(SourceSpan::UNKNOWN, ident!(inputs), 2));
}

fn add_base_boundary_expectation(expected: &mut Module) {
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![enforce!(eq!(bounded_access!(clk, Boundary::First), int!(0)))],
    ));
}

fn add_base_integrity_expectation(expected: &mut Module) {
    expected.integrity_constraints =
        Some(Span::new(SourceSpan::UNKNOWN, vec![enforce!(eq!(access!(clk), int!(0)))]));
}

#[test]
fn buses() {
    let source = format!(
        "
    {BASE_MODULE}

    buses {{
        multiset p,
        logup q,
    }}
    
    boundary_constraints {{
        enf clk.first = 0;
    }}

    integrity_constraints {{
        enf clk = 0;
    }}
    "
    );

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    add_base_expectations(&mut expected);
    add_base_boundary_expectation(&mut expected);
    add_base_integrity_expectation(&mut expected);
    expected
        .buses
        .insert(ident!(p), Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset));
    expected
        .buses
        .insert(ident!(q), Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn boundary_constraints_buses() {
    let source = format!(
        "
    {BASE_MODULE}

    buses {{
        multiset p,
        logup q,
    }}
    
    boundary_constraints {{
        enf p.first = null;
        enf q.last = null;
    }}

    integrity_constraints {{
        enf clk = 0;
    }}
    "
    );

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    add_base_expectations(&mut expected);
    add_base_integrity_expectation(&mut expected);
    expected
        .buses
        .insert(ident!(p), Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset));
    expected
        .buses
        .insert(ident!(q), Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup));
    expected.boundary_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            enforce!(eq!(bounded_access!(p, Boundary::First), null!())),
            enforce!(eq!(bounded_access!(q, Boundary::Last), null!())),
        ],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn integrity_constraints_buses() {
    let source = format!(
        "
    {BASE_MODULE}

    buses {{
        multiset p,
        logup q,
    }}

    boundary_constraints {{
        enf clk.first = 0;
    }}
    
    integrity_constraints {{
        p.insert(1) when 1;
        p.remove(1) when 1;
        q.insert(1, 2) when 1;
        q.insert(1, 2) when 1;
        q.remove(1, 2) with 2;
    }}
    "
    );

    let mut expected = Module::new(ModuleType::Root, SourceSpan::UNKNOWN, ident!(test));
    add_base_expectations(&mut expected);
    add_base_boundary_expectation(&mut expected);
    expected
        .buses
        .insert(ident!(p), Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset));
    expected
        .buses
        .insert(ident!(q), Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup));
    expected.integrity_constraints = Some(Span::new(
        SourceSpan::UNKNOWN,
        vec![
            bus_enforce!(
                lc!((("%0", range!(0..1))) =>  bus_insert!(p, vec![expr!(int!(1))]), when int!(1))
            ),
            bus_enforce!(
                lc!((("%1", range!(0..1))) =>  bus_remove!(p, vec![expr!(int!(1))]), when int!(1))
            ),
            bus_enforce!(
                lc!((("%2", range!(0..1))) =>  bus_insert!(q, vec![expr!(int!(1)), expr!(int!(2))]), when int!(1))
            ),
            bus_enforce!(
                lc!((("%3", range!(0..1))) =>  bus_insert!(q, vec![expr!(int!(1)), expr!(int!(2))]), when int!(1))
            ),
            bus_enforce!(
                lc!((("%4", range!(0..1))) =>  bus_remove!(q, vec![expr!(int!(1)), expr!(int!(2))]), with int!(2))
            ),
        ],
    ));
    ParseTest::new().expect_module_ast(&source, expected);
}

#[test]
fn err_empty_buses() {
    let source = "
    mod test

    buses{}";

    ParseTest::new()
        .expect_module_diagnostic(source, "expected one of: '\"logup\"', '\"multiset\"'");
}

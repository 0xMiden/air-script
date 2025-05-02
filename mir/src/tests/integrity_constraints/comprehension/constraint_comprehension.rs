use super::super::compile;

#[test]
fn constraint_comprehension() {
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
        enf c = d for (c, d) in (c, d);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn ic_comprehension_with_selectors() {
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
        enf c = d for (c, d) in (c, d) when !fmp[0];
    }";

    assert!(compile(source).is_ok());
}

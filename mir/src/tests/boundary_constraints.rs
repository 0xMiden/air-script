use super::compile;

#[test]
fn boundary_constraints() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = 0;
        enf clk.last = 1;
    }
    integrity_constraints {
        enf clk' = clk + 1;
    }";

    assert!(compile(source).is_ok());
}

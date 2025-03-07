use super::{compile, Pipeline};

#[test]
fn bc_with_public_inputs() {
    let source = "
    def test
    trace_columns {
        main: [clk],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf clk.first = stack_inputs[0]^3;
    }
    integrity_constraints {
        enf clk' = clk - 1;
    }";

    assert!(compile(source, Pipeline::WithoutMIR).is_ok());
    assert!(compile(source, Pipeline::WithMIR).is_ok());
}

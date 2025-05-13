use super::compile;

#[test]
fn list_comprehension_nested_nobind() {
    let source_explicit = "
    def ListComprehensionAir

    const TABLE = [
      [0, 1, 2],
      [3, 4, 5],
      [6, 7, 8],
      [9, 10, 11]];

    trace_columns {
        main: [clk],
    }

    public_inputs {
        input: [1],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        let state = [6, 5, 4];
        let expected = [13, 58, 103, 148];
        let result = [inner_loop(state, row) for row in TABLE];
        enf expected = result;
    }

    fn inner_loop(st: felt[3], row: felt[3]) -> felt {
        return sum([s * m for (s, m) in (st, row)]);
    }
    ";
    let source_nested = "
    def ListComprehensionAir

    const TABLE = [
      [0, 1, 2],
      [3, 4, 5],
      [6, 7, 8],
      [9, 10, 11],
    ];

    trace_columns {
        main: [clk],
    }

    public_inputs {
        input: [1],
    }

    boundary_constraints {
        enf clk.first = 0;
    }

    integrity_constraints {
        let state = [6, 5, 4];
        let expected = [13, 58, 103, 148];
        let result = [sum([s * m for (s, m) in (state, row)]) for row in TABLE];
        enf expected = result;
    }";

    let Ok(explicit) = compile(source_explicit) else {
        panic!("Failed to compile the explicit version\n{source_explicit}");
    };
    let Ok(nested) = compile(source_nested) else {
        panic!("Failed to compile the nested version\n{source_nested}");
    };
    assert_eq!(explicit, nested);
}

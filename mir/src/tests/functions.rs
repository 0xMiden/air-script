use crate::tests::compile;

#[test]
fn fn_def_complex_case() {
    let source = "
    def test

    trace_columns {
        main: [a],
    }

    public_inputs {
        stack_inputs: [16],
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        enf a' = double_and_add_with_six(a, a);
    }

    fn double_and_add_with_six(a: felt, b: felt) -> felt {
        let c = double(a);
        let d = double(b);
        
        return add_six(c+d);
    }

    fn double(a: felt) -> felt {
        return 2*a;
    }

    fn add_six(a: felt) -> felt {
        let vec = [double(x) for x in 0..3];
        let vec_sum = sum(vec);

        return a + vec_sum;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_def_with_scalars() {
    let source = "
    def test

    trace_columns {
        main: [a],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf a.first = 0;
    }
    integrity_constraints {
        enf a' = fn_with_scalars(a, a);
    }

    fn fn_with_scalars(a: felt, b: felt) -> felt {
        return a + b;
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_def_with_vectors() {
    let source = "
    def test

    trace_columns {
        main: [a[12], b[12]],
    }
    public_inputs {
        stack_inputs: [16],
    }
    boundary_constraints {
        enf a[0].first = 0;
    }
    integrity_constraints {
        let c = fn_with_vectors(a, b);
        let d = sum(c);
        enf d = 0;
    }

    fn fn_with_vectors(a: felt[12], b: felt[12]) -> felt[12] {
        return [x + y for (x, y) in (a, b)];
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_use_scalars_and_vectors() {
    let source = "
        def root

        public_inputs {
            stack_inputs: [16],
        }

        trace_columns {
            main: [a, b[12]],
        }

        fn fn_with_scalars_and_vectors(a: felt, b: felt[12]) -> felt {
            return sum([a + x for x in b]);
        }

        boundary_constraints {
            enf a.first = 0;
        }

        integrity_constraints {
            enf a' = fn_with_scalars_and_vectors(a, b);
        }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_call_in_fn() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a, b],
    }

    fn double(a: felt) -> felt {
        return 2*a;
    }

    fn double_b_and_add(a: felt, b: felt) -> felt {
        return a + double(b);
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        enf a' = double_b_and_add(a, b);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_call_in_ev() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a, b[12]],
    }

    fn double(a: felt) -> felt {
        return 2*a;
    }

    ev evaluator([a]) {
        enf a' = double(a);
    }

    boundary_constraints {
        enf a.first = 0;
    }

    integrity_constraints {
        enf evaluator([a]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_as_lc_iterables() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a[2], b[2], c],
    }

    fn operation(a: felt, b: felt) -> felt {
        let x = a^2 + b;
        return x^3;
    }

    boundary_constraints {
        enf c.first = 0;
    }

    integrity_constraints {
        enf c' = sum([operation(x, y) for (x, y) in (a, b)]);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_call_in_binary_ops() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a[12], b[12]],
    }

    fn operation(a: felt[12], b: felt[12]) -> felt {
        return sum([x + y for (x, y) in (a, b)]);
    }

    boundary_constraints {
        enf a[0].first = 0;
    }

    integrity_constraints {
        enf a[0]' = a[0] * operation(a, b);
        enf b[0]' = b[0] * operation(a, b);
    }";

    assert!(compile(source).is_ok());
}

#[test]
fn fn_call_in_vector_def() {
    let source = "
    def root

    public_inputs {
        stack_inputs: [16],
    }

    trace_columns {
        main: [a[12], b[12]],
    }

    fn operation(a: felt[12], b: felt[12]) -> felt[12] {
        return [x + y for (x, y) in (a, b)];
    }

    boundary_constraints {
        enf a[0].first = 0;
    }

    integrity_constraints {
        let d = [a[0] * sum(operation(a, b)), b[0] * sum(operation(a, b))];
        enf a[0]' = d[0];
        enf b[0]' = d[1];
    }";

    assert!(compile(source).is_ok());
}

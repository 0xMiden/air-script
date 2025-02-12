use crate::tests::{compile, translate};

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_translate() {
        let code = "
        def test_translate
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
        }
        ";
        let _mir = translate(code).unwrap();
        //eprintln!("{:#?}", mir);
    }
}

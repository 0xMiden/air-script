use crate::tests::compile;
#[cfg(test)]
mod tests {
    use super::*;

    //use crate::graph::pretty;
    //use crate::ConstantValue;
    //use crate::ir2::Mir;
    //use crate::MirGraph;
    //use crate::MirType;
    //use crate::MirValue;
    //use crate::Node;
    //use crate::NodeIndex;
    //use crate::Operation;
    //use crate::SpannedMirValue;

    #[test]
    #[ignore]
    fn test_inlining() {
        let code = "
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
        }
        ";
        let _mir = compile(code).unwrap();
    }
}

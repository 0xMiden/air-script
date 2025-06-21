#[cfg(test)]
mod tests {
    use crate::passes::Inlining;
    use air_pass::Pass;
    
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
    fn test_inlining() {
        // fn f0(x0: Felt) -> Felt {
        //   let x1 = x0 + x0;
        //   return x1;
        // }
        //
        // fn f1() -> Felt {
        //   let x1 = f0(Felt(1);
        //   return x1;
        // }
        let mut original = MirGraph::new(vec![
            // double definition
            // x: Variable(Felt, 0, main)
            Node {
                // NodeIndex(0)
                op: Operation::Value(SpannedMirValue {
                    span: Default::default(),
                    value: MirValue::Variable(
                        MirType::Felt,
                        0,            // arg0
                        NodeIndex(3), // double.body
                    ),
                }),
            },
            // double.return: Variable(Felt, 1, double)
            Node {
                // NodeIndex(1)
                op: Operation::Value(SpannedMirValue {
                    span: Default::default(),
                    value: MirValue::Variable(
                        MirType::Felt,
                        1, // return because double only has one argument
                        // TODO: Define a special type for return values or add it as the last argument
                        NodeIndex(3), // double.body
                    ),
                }),
            },
            // double.body:
            Node {
                // NodeIndex(2)
                op: Operation::Add(NodeIndex(0), NodeIndex(0)),
            },
            // double.y:
            //   y = x + x
            //   return y
            Node {
                // NodeIndex(3)
                op: Operation::Definition(
                    vec![NodeIndex(0)], // x
                    Some(NodeIndex(1)), // return variable
                    vec![NodeIndex(2)], // y = x + x
                ),
            },
            // fn main() -> Felt:
            //   x = 1
            //   y = double(x)
            //   return y
            // main.return: Variable(Felt, 0, main)
            Node {
                // NodeIndex(4)
                op: Operation::Value(SpannedMirValue {
                    span: Default::default(),
                    value: MirValue::Variable(
                        MirType::Felt,
                        0,            // arg0
                        NodeIndex(0), // main.body
                    ),
                }),
            },
            // main.body:
            //   x = 1
            //   y = double(x)
            //   return y

            // x = 1
            Node {
                // NodeIndex(5)
                op: Operation::Value(SpannedMirValue {
                    span: Default::default(),
                    value: MirValue::Constant(ConstantValue::Felt(1)),
                }),
            },
            // y = double(x)
            Node {
                // NodeIndex(6)
                op: Operation::Call(NodeIndex(3), vec![NodeIndex(5)]),
            },
            //main.definition
            Node {
                // NodeIndex(7)
                op: Operation::Definition(
                    vec![],
                    Some(NodeIndex(4)),               // return variable
                    vec![NodeIndex(5), NodeIndex(6)], // y = double(x)
                ),
            },
        ]);
        let double = NodeIndex(3);
        let main = NodeIndex(7);
        original.integrity_constraints_roots.insert(main);
        original.integrity_constraints_roots.insert(double);
        println!("ORIGINAL:\n{}", pretty(&original, &[double, main]));
        println!("ORIGINAL raw:\n{:?}", original);
        println!("============= Inlining pass =============");
        let mut inliner = Inlining::new();

        let mut mir_original = Mir::default();
        *mir_original.constraint_graph_mut() = original.clone();

        let result = inliner.run(mir_original).unwrap();
        println!("=========================================");
        println!("INLINED raw:\n{:?}", result);
        println!("INLINED:\n{}", pretty(&result.constraint_graph(), &[double, main]));
    }
}

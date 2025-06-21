use std::collections::BTreeMap;

use miden_core::Felt;

use crate::{QuadFelt, layout::Layout};

/// One of the 3 arithmetic operations supported by the ACE chiplet.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ArithmeticOp {
    Sub = 0,
    Mul = 1,
    Add = 2,
}

/// One of the 3 types of nodes contained in the ACE graph
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Node {
    /// Index of a lead node representing a variable at which the circuit is evaluated.
    Input(usize),
    /// Index of a leaf node stored in the circuit description.
    Constant(usize),
    /// Index of a non-leaf node representing the result of an [`ArithmeticOp`] applied
    /// to two other [`Node`]s.
    Operation(usize),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct OperationNode {
    pub(crate) op: ArithmeticOp,
    pub(crate) node_l: Node,
    pub(crate) node_r: Node,
}

/// A circuit that can be consumed by the ACE chiplet.
/// The only way to build a circuit is through the CircuitBuilder, it can then be obtained
/// from [`CircuitBuilder::normalize`] and serialized to felts with [`Circuit::to_felts`].
#[derive(Clone, Debug, PartialEq)]
pub struct Circuit {
    pub layout: Layout,
    pub(crate) constants: Vec<Felt>,
    pub(crate) operations: Vec<OperationNode>,
}

impl Circuit {
    /// Evaluates to a [`Quad`] the index `root`, given a vector of inputs to the circuit.
    pub fn eval(&self, node: Node, inputs: &[QuadFelt]) -> QuadFelt {
        let mut evals: BTreeMap<Node, QuadFelt> = BTreeMap::new();
        // Insert inputs nodes with given values
        for (idx, input) in inputs.iter().enumerate() {
            evals.insert(Node::Input(idx), *input);
        }
        // Insert constant nodes with existing values
        for (idx, c) in self.constants.iter().enumerate() {
            evals.insert(Node::Constant(idx), QuadFelt::from(*c));
        }
        // Evaluate operations by querying the values of the input nodes, inserting
        // the result in the graph.
        for (idx, op) in self.operations.iter().enumerate() {
            let OperationNode { op, node_l, node_r } = op;
            let eval_l = evals[node_l];
            let eval_r = evals[node_r];
            let eval = match op {
                ArithmeticOp::Sub => eval_l - eval_r,
                ArithmeticOp::Mul => eval_l * eval_r,
                ArithmeticOp::Add => eval_l + eval_r,
            };
            evals.insert(Node::Operation(idx), eval);
        }
        evals[&node]
    }

    /// Returns the total number of nodes in the circuit's graph.
    pub fn num_nodes(&self) -> usize {
        self.layout.num_inputs + self.constants.len() + self.operations.len()
    }
}

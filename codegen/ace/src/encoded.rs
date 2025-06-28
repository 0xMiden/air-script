use miden_core::{
    Felt,
    crypto::hash::{Rpo256, RpoDigest},
};
use winter_math::FieldElement;

use crate::{
    QuadFelt,
    circuit::{ArithmeticOp, Circuit, Node, OperationNode},
};

/// An encoded [`Circuit`] matching the required format for the ACE chiplet.
/// The chiplet performs an evaluation by sequentially reading a region in memory with the following
/// layout, where each region must be word-aligned.
/// - *Variables* correspond to all leaf nodes, stored two-by-two as extension field elements. It is
///   subdivided into the two following consecutive regions, which is achieved by padding with
///   zeros.
///   - *Inputs*: List of all inputs to the circuit. The internal layout is described in
///     [`crate::AceVars::to_memory_vec`].
///   - *Constants*: Fixed values that can be referenced by instructions.
/// - *Instructions*: List of arithmetic gates to be evaluated. Each instruction is encoded as a
///   single field element. It is padded with instructions which square the output, as this still
///   ensures the final evaluation is still evaluates to zero.
pub struct EncodedCircuit {
    num_vars: usize,
    num_ops: usize,
    instructions: Vec<Felt>,
}

impl EncodedCircuit {
    /// Number of `READ` rows in the ACE chiplet, where:
    /// - Each row contains either two inputs or two constants encoded as extension field elements,
    /// - The list of inputs is followed by the list of constants.
    /// - Each list is padded with unused zero variables to satisfy the memory alignment.
    pub fn num_read_rows(&self) -> usize {
        self.num_vars() / 2
    }

    /// Number of `EVAL` rows in the ACE chiplet, where each row contains four instructions.
    /// If the number of instructions is not a multiple of the memory alignment, the list is padded
    /// with squaring instructions which have no effect when we expect the result to be zero.
    pub fn num_eval_rows(&self) -> usize {
        self.num_ops
    }

    /// Number of variable nodes (inputs and constants).
    pub fn num_vars(&self) -> usize {
        self.num_vars
    }

    /// Number of input nodes.
    pub fn num_inputs(&self) -> usize {
        self.num_vars - self.num_constants()
    }

    /// Number of constant nodes.
    pub fn num_constants(&self) -> usize {
        self.num_nodes() - self.num_ops
    }

    /// Number of nodes (variables and operations).
    pub fn num_nodes(&self) -> usize {
        self.num_vars + self.num_ops
    }

    /// List of encoded instructions (constants and operations).
    pub fn instructions(&self) -> &[Felt] {
        &self.instructions
    }

    /// Returns the digest of the circuit, represented by the constants and instructions.
    pub fn circuit_hash(&self) -> RpoDigest {
        Rpo256::hash_elements(self.instructions())
    }
}

impl Circuit {
    /// Serializes a [`Circuit`] to a list of field elements in the format expected by the
    /// ACE chiplet.
    ///
    /// In the case of a gate, the 63 bits available in a field element are used in
    /// the following order, from least significant:
    /// - 30 bits for the right index,
    /// - 30 bits for the left index,
    /// - 2 bits for the OPCODE, with SUB 00, MUL 01, ADD 10.
    ///
    /// The indices of the nodes in the circuit are mapped in reverse order from
    /// `max_node_index = n_inputs + n_constants + n_gates - 1` down to zero, but encoded in the
    /// same order as the circuit. That is, inputs are followed by constants, which are followed by
    /// instructions.
    ///
    /// For example, the circuit `(i0 + 14) * i0` encodes to
    /// ```text
    /// [
    /// 14, // value of the constant, which has index 1, given that there is one input at index 0
    /// (2 << 60) + (0 << 30) + (1 << 0), // addition gate opcode with left input 0 and right input 1. Output index 2.
    /// (1 << 60) + (2 << 30) + (0 << 0), // multiplication gate opcode with left input 2 and right input 0. Output index 3.
    /// ]
    /// ```
    ///
    /// The encoded circuit is padded according to [`Self::is_padded`].
    pub fn to_ace(&self) -> EncodedCircuit {
        const MAX_NODE_ID: u64 = (1 << 30) - 1;

        assert!(self.num_nodes() as u64 <= MAX_NODE_ID, "more than 2^30 nodes");

        // Constants are encoded two-by-two as extension field elements, followed by operations.
        let num_const = self.constants.len().next_multiple_of(2);
        let num_ops = self.operations.len().next_multiple_of(4);
        let len_const = num_const * 2;
        let len_circuit = len_const + num_ops;
        let mut instructions = Vec::with_capacity(len_circuit);

        // Add constants
        instructions
            .extend(self.constants.iter().flat_map(|c| QuadFelt::from(*c).to_base_elements()));
        // Since constants are treated as extension field elements, we pad this section with zeros
        // to ensure it is aligned in memory.
        instructions.resize(len_const, Felt::ZERO);

        let num_inputs = self.layout.num_inputs;
        let num_constants = self.constants.len();
        let num_nodes = num_inputs + num_constants + num_ops;
        let node_id = |node: Node| -> u64 {
            let input_start = num_nodes - 1;
            let constants_start = input_start - num_inputs;
            let ops_start = constants_start - num_constants;

            match node {
                Node::Input(idx) => input_start.checked_sub(idx),
                Node::Constant(idx) => constants_start.checked_sub(idx),
                Node::Operation(idx) => ops_start.checked_sub(idx),
            }
            .expect("invalid node index") as u64
        };

        let operation_to_instruction = |operation: &OperationNode| {
            let id_0 = node_id(operation.node_l);
            let id_1 = node_id(operation.node_r);
            let op_tag = operation.op as u64;

            // TODO(adr1anh): Make these public for use by the VM when deserializing instructions.
            const OP_OFFSET: u64 = 1 << 60;
            const ID_1_OFFSET: u64 = 1 << 30;
            let instruction = (id_0) + (id_1 * ID_1_OFFSET) + (op_tag * OP_OFFSET);
            Felt::new(instruction)
        };
        for operation in &self.operations {
            let encoded = operation_to_instruction(operation);
            instructions.push(encoded);
        }

        // Since an ACE circuit's last node must evaluate to 0, we pad it with
        // operations which square the last node.
        // Each operation is encoded as a single field element, so the total number
        // of operations must be a multiple of 4.
        let mut last_node_index = self.operations.len() - 1;
        while instructions.len() % 4 != 0 {
            let last_node = Node::Operation(last_node_index);
            let dummy_op = OperationNode {
                op: ArithmeticOp::Mul,
                node_l: last_node,
                node_r: last_node,
            };
            let encoded = operation_to_instruction(&dummy_op);
            instructions.push(encoded);
            last_node_index += 1;
        }

        let num_vars = num_inputs + num_constants;
        EncodedCircuit { num_vars, num_ops, instructions }
    }

    /// Returns `true` when the circuit is properly padded, and each region is word-aligned. It
    /// ensures the following circuit layout:
    /// - Inputs and constants lie two-by-two in memory, treated as extension field elements,
    /// - Operations are encoded as single field elements.
    pub fn is_padded(&self) -> bool {
        (self.layout.num_inputs % 2 == 0)
            && (self.constants.len() % 2 == 0)
            && (self.operations.len() % 4 == 0)
    }
}

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use super::*;
    use crate::{
        circuit::{ArithmeticOp, OperationNode},
        layout::{InputRegion, Layout},
    };

    /// Circuit evaluating `{[(i0 + 1) * i0] - 1}^2`, ensuring
    /// - All arithmetic operations are used
    /// - Padding is tested by taking
    ///   - a single input (encoding adds one)
    ///   - a single constant (encoding adds one)
    ///   - 3 operations (encoding adds a squaring)
    #[test]
    fn test_circuit_encoding() {
        // Manually construct a circuit since Layout::default will add regions for the quotient
        // and stark variables.
        let layout = Layout {
            public_inputs: Default::default(),
            reduced_tables_region: InputRegion::default(),
            reduced_tables: Default::default(),
            random_values: Default::default(),
            trace_segments: [
                [
                    // Main
                    InputRegion { offset: 0, width: 1 },
                    // Aux
                    InputRegion { offset: 1, width: 0 },
                    // Quotient
                    InputRegion { offset: 1, width: 0 },
                ],
                [
                    InputRegion { offset: 1, width: 1 },
                    // Aux
                    InputRegion { offset: 2, width: 0 },
                    // Quotient
                    InputRegion { offset: 2, width: 0 },
                ],
            ],
            stark_vars: Default::default(),
            num_inputs: 2,
        };

        // id = 7
        let input = Node::Input(0);
        // id = 6
        let _dummy_input = Node::Input(1);
        // id = 5
        let one = Node::Constant(0);
        let circuit = Circuit {
            layout,
            constants: vec![
                // id = 5
                Felt::new(1),
                // id = 4, padding
                Felt::new(0),
            ],
            operations: vec![
                // id = 3, op = 0, input + 1
                OperationNode {
                    op: ArithmeticOp::Add, // op = 2
                    node_l: input,         // id = 7
                    node_r: one,           // id = 5
                },
                // id = 2, op = 1, (input + 1) * input
                OperationNode {
                    op: ArithmeticOp::Mul,      // op = 1
                    node_l: Node::Operation(0), // id = 3
                    node_r: input,              // id = 7
                },
                // id = 1, op = 2, [(input + 1) * input] - 1
                OperationNode {
                    op: ArithmeticOp::Sub,      // op = 0
                    node_l: Node::Operation(1), // id = 2
                    node_r: one,                // id = 5
                },
                // Padding with squaring of last operation.
                // // id = 0, {[(input + 1) * input] - 1}^2
                // OperationNode {
                //     op: ArithmeticOp::Mul,      // op = 1
                //     node_l: Node::Operation(1), // id = 1
                //     node_r: Node::Operation(1), // id = 1
                // },
            ],
        };
        let encoded = circuit.to_ace();
        assert_eq!(encoded.num_ops, 4);
        assert_eq!(encoded.num_vars, 4);
        #[allow(clippy::identity_op)]
        let expected = [
            1u64,                      // constant 1
            0,                         // quad padding
            0,                         // alignment padding
            0,                         // alignment padding
            (2 << 60) + (5 << 30) + 7, // id = 3
            (1 << 60) + (7 << 30) + 3, // id = 2
            (0 << 60) + (5 << 30) + 2, // id = 1
            (1 << 60) + (1 << 30) + 1, // id = 0
        ]
        .map(Felt::new);

        for (i, (op, expected)) in zip(encoded.instructions, expected).enumerate() {
            assert_eq!(op, expected, "op {i} is different");
        }
    }
}

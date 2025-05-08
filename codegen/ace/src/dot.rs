use crate::circuit::{ArithmeticOp, Circuit, Node, OperationNode};
use crate::layout::StarkVar;
use std::fmt::{Display, Write};
use winter_math::StarkField;

impl Circuit {
    /// Serialization to Graphviz Dot format for debugging purposes. Display on
    /// https://dreampuf.github.io/GraphvizOnline or using `dot -Tsvg tests/regressions/0.dot > 0.svg`
    pub fn to_dot(&self) -> Result<String, std::fmt::Error> {
        let mut f = String::new();
        writeln!(f, "digraph G {{")?;

        // Constants
        for (i, _c) in self.constants.iter().enumerate() {
            let node = Node::Constant(i);
            let value = self.constants[i].as_int();
            writeln!(f, "{node} [label=\"{value}\"]")?;
        }

        // Public inputs
        for (pi, region) in self.layout.public_inputs.iter() {
            let name = pi.name().as_str();
            for (idx, node) in region.iter_nodes().enumerate() {
                writeln!(f, "{node} [label=\"PI[{name}][{idx}]\"]")?;
            }
        }

        // Random values
        for (idx, node) in self.layout.random_values.iter_nodes().enumerate() {
            writeln!(f, "{node} [label=\"R[{idx}]\"]")?;
        }

        // Main
        for (idx, node) in self.layout.trace_segments[0][0].iter_nodes().enumerate() {
            writeln!(f, "{node} [label=\"M[{idx}]\"]")?;
        }
        for (idx, node) in self.layout.trace_segments[1][0].iter_nodes().enumerate() {
            writeln!(f, "{node} [label=\"M'[{idx}]\"]")?;
        }

        // Aux
        for (idx, node) in self.layout.trace_segments[0][1].iter_nodes().enumerate() {
            writeln!(f, "{node} [label=\"A[{idx}]\"]")?;
        }
        for (idx, node) in self.layout.trace_segments[1][1].iter_nodes().enumerate() {
            writeln!(f, "{node} [label=\"A'[{idx}]\"]")?;
        }

        // Quotient
        for (idx, node) in self.layout.trace_segments[0][1].iter_nodes().enumerate() {
            writeln!(f, "{node} [label=\"Q[{idx}]\"]")?;
        }

        // Air vars
        for var_idx in 0..StarkVar::num_vars() {
            let var = StarkVar::try_from(var_idx).unwrap();
            let node = self.layout.stark_node(var);
            writeln!(f, "{node} [label=\"{var}\"]")?;
        }

        // Operations
        for (op_idx, op_node) in self.operations.iter().enumerate() {
            let OperationNode { op, node_l, node_r } = op_node;
            let op_node = Node::Operation(op_idx);
            writeln!(
                f,
                "{op_node} [label=\"{op_node}\\n{node_l} {op} {node_r}\"]"
            )?;
            writeln!(f, "{node_l} -> {op_node}")?;
            writeln!(f, "{node_r} -> {op_node}")?;
        }

        writeln!(f, "}}")?;
        Ok(f)
    }
}

impl Display for StarkVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::GenLast => "g⁻¹",
            Self::GenPenultimate => "g⁻²",
            Self::Z => "z",
            Self::ZPowN => "zⁿ",
            Self::ZMaxCycle => "zᵐᵃˣ",
            Self::Alpha => "⍺",
        };
        write!(f, "{}", str)
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Input(idx) => {
                write!(f, "input{idx}")
            }
            Node::Constant(idx) => {
                write!(f, "const{idx}")
            }
            Node::Operation(idx) => {
                write!(f, "op{idx}")
            }
        }
    }
}

impl Display for ArithmeticOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ArithmeticOp::Sub => "-",
            ArithmeticOp::Mul => "×",
            ArithmeticOp::Add => "+",
        };
        write!(f, "{}", str)
    }
}

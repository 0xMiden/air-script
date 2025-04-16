use air_ir::{NodeIndex, Operation, Value};
use miden_core::{Felt, QuadExtension};
use std::fmt::Write;

pub type Quad = QuadExtension<Felt>;

/// Serialization to Graphviz Dot format for debugging purposes. Display on
/// https://dreampuf.github.io/GraphvizOnline or using `dot -Tsvg tests/regressions/0.dot > 0.svg`
pub fn operations_to_dot(ops: &[Operation]) -> Result<String, std::fmt::Error> {
    let mut f = String::new();
    writeln!(f, "digraph G {{")?;
    for (out, op) in ops.iter().enumerate() {
        let s = operation_to_dot(out.into(), op)?;
        write!(f, "{}", s)?;
    }
    writeln!(f, "}}")?;
    Ok(f)
}

/// Converts a single operations, together with its output index, to Dot format.
fn operation_to_dot(out: NodeIndex, op: &Operation) -> Result<String, std::fmt::Error> {
    let mut f = String::new();
    let out: usize = out.into();
    match op {
        Operation::Value(v) => {
            writeln!(f, "{} [label=\"{}\"]", out, value_to_dot(v))
        }
        Operation::Add(l, r) => {
            let l: usize = NodeIndex::into(*l);
            let r: usize = NodeIndex::into(*r);
            writeln!(f, "{} [label=\"add\"]", out)?;
            writeln!(f, "{} -> {} [label=\"{}\"]", out, l, l)?;
            writeln!(f, "{} -> {} [label=\"{}\"]", out, r, r)
        }
        Operation::Sub(l, r) => {
            let l: usize = NodeIndex::into(*l);
            let r: usize = NodeIndex::into(*r);
            writeln!(f, "{} [label=\"sub\"]", out)?;
            writeln!(f, "{} -> {} [label=\"{}\"]", out, l, l)?;
            writeln!(f, "{} -> {} [label=\"{}\"]", out, r, r)
        }
        Operation::Mul(l, r) => {
            let l: usize = NodeIndex::into(*l);
            let r: usize = NodeIndex::into(*r);
            writeln!(f, "{} [label=\"mul\"]", out)?;
            writeln!(f, "{} -> {} [label=\"{}\"]", out, l, l)?;
            writeln!(f, "{} -> {} [label=\"{}\"]", out, r, r)
        }
    }?;
    Ok(f)
}

/// Converts a Value to Dot format.
fn value_to_dot(v: &Value) -> String {
    match v {
        Value::Constant(c) => format!("C {}", c),
        // Value::PublicInput(acc) => format!("PI {} {}", acc.name, acc.index),
        // Value::TraceAccess(acc) => {
        //     format!("Trace {} {} {}", acc.segment, acc.column, acc.row_offset)
        // }
        // Value::RandomValue(v) => format!("Random {}", v),
        // Value::PeriodicColumn(_) => unreachable!("Periodic"),
        _ => unreachable!("not a constant"),
    }
}

/// Serialization to field elements.
/// Note that some information is lost, so the original circuit cannot be
/// recovered from its felt serialization.
/// The format starts with a header of 3 field elements containing:
/// - the number of inputs,
/// - the number of constants and
/// - the number of evaluation gates.
///
/// In the case of a gate, the 63 bits available in a field element are used in
/// the following order, from least significant:
/// - 30 bits for right index
/// - 30 bits for left index
/// - 2 bits for the OPCODE, with SUB 00, MUL 01, ADD 10
///
/// The output indexes of inputs, constants and gates are not serialized as they
/// are simply successive values starting from `n_inputs` to
/// `n_inputs + n_constants + n_gates - 1`.
///
/// For example the circuit `(i0 + 14) * i0` encodes to
/// ```text
/// [
/// // begin header
/// 1, // one input
/// 1, // one constant
/// 2, // two gates
/// // end header
/// 14, // value of the constant, which has index 1, given that there is one input at index 0
/// (2 << 60) + (0 << 30) + (1 << 0), // addition gate opcode with left intput 0 and right input 1. Output index 2.
/// (1 << 60) + (2 << 30) + (0 << 0), // multiplication gate opcode with left intput 2 and right input 0. Output index 3.
/// ]
/// ```
pub fn operations_to_felts(n_inputs: usize, ops: &Vec<Operation>) -> Vec<Quad> {
    let mut n_constants = 0u64;
    let mut n_nodes = 0u64;
    for op in ops {
        match op {
            Operation::Value(Value::Constant(_)) => n_constants += 1,
            _ => n_nodes += 1,
        }
    }
    let header: Vec<Quad> = [n_inputs as u64, n_constants, n_nodes]
        .into_iter()
        .map(Quad::from)
        .collect();
    let body: Vec<Quad> = ops.iter().flat_map(|op| operation_to_felt(*op)).collect();
    header.into_iter().chain(body).collect()
}

fn operation_to_felt(op: Operation) -> Vec<Quad> {
    match op {
        Operation::Value(Value::PublicInput(_)) | Operation::Value(Value::TraceAccess(_)) => {
            vec![]
        }
        Operation::Value(Value::Constant(c)) => {
            vec![c.into()]
        }
        Operation::Add(l, r) => {
            let l: usize = NodeIndex::into(l);
            let r: usize = NodeIndex::into(r);
            let res = (2u64 << 60) + ((l as u64) << 30) + (r as u64);
            vec![res.into()]
        }
        Operation::Mul(l, r) => {
            let l: usize = NodeIndex::into(l);
            let r: usize = NodeIndex::into(r);
            let res = (1u64 << 60) + ((l as u64) << 30) + (r as u64);
            vec![res.into()]
        }
        Operation::Sub(l, r) => {
            let l: usize = NodeIndex::into(l);
            let r: usize = NodeIndex::into(r);
            let res = ((l as u64) << 30) + (r as u64);
            vec![res.into()]
        }
        Operation::Value(Value::RandomValue(_)) => unreachable!("randomValue"),
        Operation::Value(Value::PeriodicColumn(_)) => unreachable!("periodic"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_serialization() {
        let vectors = [
            (vec![0], Operation::Value(Value::Constant(0))),
            (
                vec![0b010_000000000000000000000000000001_000000000000000000000000000000u64],
                Operation::Add(1.into(), 0.into()),
            ),
            (
                vec![0b001_000000000000000000000000000001_000000000000000000000000000000],
                Operation::Mul(1.into(), 0.into()),
            ),
            (
                vec![0b000_000000000000000000000000000100_000000000000000000000000000010],
                Operation::Sub(4.into(), 2.into()),
            ),
        ];
        for (expected, op) in vectors.into_iter() {
            let bin: Vec<Quad> = operation_to_felt(op);
            expected
                .iter()
                .zip(bin.iter())
                .for_each(|(expected, bin)| assert_eq!(Quad::from(*expected), *bin));
        }
    }
}

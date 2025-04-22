use crate::comparison::compare_operation;
use crate::serialization::{operations_to_dot, operations_to_felts, Quad};
use air_ir::{NodeIndex, Operation, Value};
use std::cmp::Ordering;
use std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};

/// A circuit that can be consumed by the ACE chiplet.
/// The only way to build a circuit is through the CircuitBuilder, it can then obtained
/// from [`CircuitBuilder::normalize`] and serialized to felts with [`Circuit::to_felts`].
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Circuit {
    n_inputs: usize,
    ops: Vec<Operation>,
}
impl Circuit {
    fn new(n_inputs: usize) -> Self {
        Circuit {
            n_inputs,
            ops: Vec::new(),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.ops.len() + self.n_inputs
    }

    fn push(&mut self, op: Operation) -> NodeIndex {
        self.ops.push(op);
        (self.len() - 1).into()
    }

    fn get(&self, i: NodeIndex) -> Option<Operation> {
        if i < self.n_inputs.into() {
            None
        } else {
            let i: usize = NodeIndex::into(i);
            Some(self.ops[i - self.n_inputs])
        }
    }

    /// Serialization to Graphviz Dot format for debugging purposes. Display on
    /// <https://dreampuf.github.io/GraphvizOnline> or using `dot -Tsvg tests/regressions/0.dot > 0.svg`
    pub fn to_dot(&self) -> Result<String, std::fmt::Error> {
        operations_to_dot(&self.ops)
    }

    /// Serialization to a vector of field elements, as expected by the
    /// ACE chiplet.
    pub fn to_felts(&self) -> Vec<Quad> {
        operations_to_felts(self.n_inputs, &self.ops)
    }

    /// Evaluates to a Felt the index `root`, given a vector of inputs to
    /// the circuit.
    pub fn eval(&self, root: NodeIndex, inputs: &[Quad]) -> Quad {
        assert_eq!(inputs.len(), self.n_inputs);
        let mut res: Vec<Quad> = inputs.to_vec();
        res.reserve(self.len());
        for op in self.ops.iter() {
            match op {
                Operation::Value(v) => {
                    let o = match v {
                        Value::Constant(c) => Quad::from(*c),
                        Value::TraceAccess(_)
                        | Value::PublicInput(_)
                        | Value::PeriodicColumn(_)
                        | Value::RandomValue(_) => {
                            unreachable!("not constant")
                        }
                    };
                    res.push(o);
                }
                Operation::Add(l, r) => {
                    let l: usize = NodeIndex::into(*l);
                    let r: usize = NodeIndex::into(*r);
                    res.push(res[l] + res[r]);
                }
                Operation::Sub(l, r) => {
                    let l: usize = NodeIndex::into(*l);
                    let r: usize = NodeIndex::into(*r);
                    res.push(res[l] - res[r]);
                }
                Operation::Mul(l, r) => {
                    let l: usize = NodeIndex::into(*l);
                    let r: usize = NodeIndex::into(*r);
                    res.push(res[l] * res[r]);
                }
            }
        }
        let root: usize = root.into();
        res[root]
    }
}

/// Iterates over the nodes in the DAG depth-first.
/// Properties of the result:
/// - only dependencies of the given roots are returned
/// - parents come after their children
/// - no duplicates (e.g. when two contraints share a subexpression)
/// - inputs are preserved even if unused
pub struct UniqueOperationIterator<'a> {
    // circuit being iterated on
    circuit: &'a Circuit,
    // nodes visited but not necessarily processed yet
    visited: BTreeSet<NodeIndex>,
    // nodes already processed, meaning their children are visited
    processed: BTreeSet<NodeIndex>,
    // nodes to be processed
    to_process: Vec<NodeIndex>,
}
impl<'a> UniqueOperationIterator<'a> {
    pub fn new(circuit: &'a Circuit, process: Vec<NodeIndex>) -> Self {
        let visited = BTreeSet::new();
        let processed = BTreeSet::new();
        UniqueOperationIterator {
            circuit,
            visited,
            processed,
            to_process: process,
        }
    }
}
impl Iterator for UniqueOperationIterator<'_> {
    type Item = (NodeIndex, Operation);
    fn next(&mut self) -> Option<Self::Item> {
        use Operation::*;

        while let Some(id) = self.to_process.pop() {
            match self.circuit.get(id) {
                None => continue, // input
                Some(op) => {
                    if self.processed.contains(&id) {
                        return Some((id, op));
                    } else {
                        self.to_process.push(id);
                        self.visited.insert(id);
                        self.processed.insert(id);
                        match op {
                            Value(_) => {}
                            Add(l, r) | Sub(l, r) | Mul(l, r) => {
                                if !self.visited.contains(&r) {
                                    self.visited.insert(r);
                                    self.to_process.push(r)
                                };
                                if !self.visited.contains(&l) {
                                    self.visited.insert(l);
                                    self.to_process.push(l)
                                };
                            }
                        };
                    }
                }
            }
        }
        None
    }
}

/// Remaps the children of an operation.
/// When a node is migrated from Air to Circuit in [`Codegen::build_ace_circuit`],
/// or Circuit to Circuit in CircuitBuilder::normalize, its index changes and so
/// do the indexes of its children. During the migration a mapping is kept between
/// old and new indexes and this function can be used to apply it to the children of a node.
/// Any index within the inputs is left untouched. Constants are unaffected.
pub fn remap_op(
    n_inputs: usize,
    op: Operation,
    mapping: &BTreeMap<NodeIndex, NodeIndex>,
) -> Operation {
    let remap = |i: NodeIndex| {
        if i < n_inputs.into() {
            i
        } else {
            mapping[&i]
        }
    };
    match op {
        Operation::Add(l, r) => Operation::Add(remap(l), remap(r)),
        Operation::Sub(l, r) => Operation::Sub(remap(l), remap(r)),
        Operation::Mul(l, r) => Operation::Mul(remap(l), remap(r)),
        Operation::Value(Value::Constant(_)) => op,
        _ => unreachable!(""),
    }
}

/// CircuitBuilder is the only way to build a Circuit. It guarantees the following properties:
/// - no dangling references (e.g. a node pointing to a child that does not exist)
/// - no cycles (the circuit is a DAG)
/// - no disconnected nodes, except possibly for inputs
/// - no duplicated nodes
///
/// The circuit can be obtained using [`normalize`].
#[derive(Default)]
pub struct CircuitBuilder {
    // The ACE circuit being built
    circuit: Circuit,
    // A cache of nodes already inserted in the circuit, used to avoid duplicates
    cache: BTreeMap<Operation, NodeIndex>,
    // Constants zero and one are inserted first so that they can be used as
    // identity and absorbing element in addition and multiplication, respectively.
    pub zero: NodeIndex,
    pub one: NodeIndex,
}

impl CircuitBuilder {
    pub fn new(n_inputs: usize) -> (Self, Vec<NodeIndex>) {
        let mut cache: BTreeMap<Operation, NodeIndex> = BTreeMap::new();
        let mut circuit = Circuit::new(n_inputs);
        let [zero, one] = [0, 1].map(|c| {
            let op = Operation::Value(Value::Constant(c as u64));
            let out = circuit.push(op);
            cache.insert(op, out);
            out
        });
        (
            Self {
                cache,
                circuit,
                zero,
                one,
            },
            (0..n_inputs).map(NodeIndex::from).collect(),
        )
    }

    // Appends an operation to the circuit. A new operation is added only if
    // syntactically different from any operations already added to the circuit
    // so far.
    fn push_cached(&mut self, op: Operation) -> NodeIndex {
        match self.cache.get(&op) {
            Some(out) => *out,
            None => {
                let out = self.circuit.push(op);
                self.cache.insert(op, out);
                out
            }
        }
    }

    /// Appends an operation to the circuit after applying some optimizations
    /// from the algebraic laws of addition, multiplication and subtraction.
    pub fn push(&mut self, op: Operation) -> NodeIndex {
        match op {
            Operation::Value(Value::Constant(_)) => self.push_cached(op),
            Operation::Add(l, r) => {
                if l == self.zero {
                    r
                } else if r == self.zero {
                    l
                } else {
                    // indexes are sorted so that semantically equivalent nodes
                    // (because of commutativity) are syntactically equivalent
                    // and can be cached
                    let (l, r) = if l < r { (l, r) } else { (r, l) };
                    self.push_cached(Operation::Add(l, r))
                }
            }
            Operation::Mul(l, r) => {
                if l == self.one {
                    r
                } else if r == self.one {
                    l
                } else if l == self.zero || r == self.zero {
                    self.zero
                } else {
                    // indexes are sorted so that semantically equivalent nodes
                    // (because of commutativity) are syntactically equivalent
                    // and can be cached
                    let (l, r) = if l < r { (l, r) } else { (r, l) };
                    self.push_cached(Operation::Mul(l, r))
                }
            }
            Operation::Sub(l, r) => {
                if r == self.zero {
                    l
                } else if l == r {
                    self.zero
                } else {
                    self.push_cached(Operation::Sub(l, r))
                }
            }
            _ => unreachable!("this op kind should not be here"),
        }
    }

    pub fn constant(&mut self, c: u64) -> NodeIndex {
        self.push(Operation::Value(Value::Constant(c)))
    }

    pub fn add(&mut self, l: NodeIndex, r: NodeIndex) -> NodeIndex {
        self.push(Operation::Add(l, r))
    }

    pub fn mul(&mut self, l: NodeIndex, r: NodeIndex) -> NodeIndex {
        self.push(Operation::Mul(l, r))
    }

    pub fn sub(&mut self, l: NodeIndex, r: NodeIndex) -> NodeIndex {
        self.push(Operation::Sub(l, r))
    }

    /// `\sum_i vec[i]`
    pub fn sum(&mut self, vec: &[NodeIndex]) -> NodeIndex {
        assert!(!vec.is_empty());
        vec.iter().fold(self.zero, |acc, r| self.add(acc, *r))
    }

    /// `\prod_i vec[i]`
    pub fn prod(&mut self, vec: &[NodeIndex]) -> NodeIndex {
        assert!(!vec.is_empty());
        vec.iter().fold(self.one, |acc, r| self.mul(acc, *r))
    }

    /// `pow(point, n) = point^n`
    pub fn pow(&mut self, point: NodeIndex, n: u32) -> NodeIndex {
        let mut tmp = point;
        let mut res = self.one;
        let last = 32 - n.leading_zeros();
        for i in 0..last {
            if n & (1 << i) != 0 {
                res = self.mul(res, tmp);
            }
            if i != last - 1 {
                // we skip the last tmp
                tmp = self.mul(tmp, tmp);
            }
        }
        res
    }

    /// Evaluates a polynomial with coefficients `coeffs` on a point `point`,
    /// using Horner's method.
    /// `\sum_i coeffs[i] * x^i = coeff[0] + x * (... (coeff[n-1] + x * coeff[n]) ...)`
    pub fn horners_method(&mut self, point: NodeIndex, coeffs: &[NodeIndex]) -> NodeIndex {
        assert!(!coeffs.is_empty());
        coeffs.iter().rev().fold(self.zero, |acc, coeff| {
            let mul = self.mul(point, acc);
            self.add(*coeff, mul)
        })
    }

    /// Builds a new circuit that respects the order of the operations required by
    /// the ACE chiplet. Starting from a root, only the used nodes are extracted
    /// w/o duplications, sorted, remapped and added to a new circuit.
    pub fn normalize(&mut self, root: NodeIndex) -> (NodeIndex, Circuit) {
        let mut mapping: BTreeMap<NodeIndex, NodeIndex> = BTreeMap::new();
        // While in a circuit the output of each operation is simply its position
        // in the vector, here each operation is associated with its output index
        // so that they can be sorted w/o loosing this information.
        let mut ops: Vec<(NodeIndex, Operation)> =
            UniqueOperationIterator::new(&self.circuit, vec![root]).collect();
        ops.sort_by(|(ia, a), (ib, b)| {
            let c = compare_operation(a, b);
            if c == Ordering::Equal {
                ia.cmp(ib)
            } else {
                c
            }
        });
        let mut res = Circuit::new(self.circuit.n_inputs);
        for (out, op) in ops.iter() {
            let op = remap_op(self.circuit.n_inputs, *op, &mapping);
            let new_out = res.push(op);
            mapping.insert(*out, new_out);
        }
        (mapping[&root], res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_iterator() {
        let (mut cb, _unused_input) = CircuitBuilder::new(1);
        let five = cb.constant(5);
        let root = cb.add(five, five);
        // the iterator drops two unused constants 0 and 1, but keeps the unused input
        let c_before: Vec<Operation> = vec![
            (Operation::Value(Value::Constant(0))),
            (Operation::Value(Value::Constant(1))),
            (Operation::Value(Value::Constant(5))),
            (Operation::Add(3.into(), 3.into())),
        ];
        assert_eq!(c_before, cb.circuit.ops);
        let c_after: Vec<Operation> = vec![
            (Operation::Value(Value::Constant(5))),
            (Operation::Add(3.into(), 3.into())),
        ];
        let c: Vec<Operation> = UniqueOperationIterator::new(&cb.circuit, vec![root])
            .map(|(_out, op)| op)
            .collect();
        assert_eq!(c_after, c)
    }

    #[test]
    fn test_sum() {
        let (mut cb, _) = CircuitBuilder::new(0);
        let constants: Vec<NodeIndex> = (0..5).map(|i| cb.constant(i)).collect();
        let res = cb.sum(&constants);
        assert_eq!(cb.circuit.eval(res, &[]), Quad::from((0..5).sum::<u64>()))
    }

    #[test]
    fn test_prod() {
        let (mut cb, _) = CircuitBuilder::new(0);
        let constants: Vec<NodeIndex> = (1..5).map(|i| cb.constant(i)).collect();
        let res = cb.prod(&constants);
        assert_eq!(
            cb.circuit.eval(res, &[]),
            Quad::from((1..5).product::<u64>())
        )
    }

    #[test]
    fn test_pow() {
        let (mut cb, _) = CircuitBuilder::new(0);
        let base = cb.constant(2);
        let res = cb.pow(base, 5u32);
        assert_eq!(cb.circuit.eval(res, &[]), 2u64.pow(5).into())
    }

    #[test]
    fn test_horner() {
        let (mut cb, _) = CircuitBuilder::new(0);
        let coeffs: Vec<NodeIndex> = (1..5).map(|i| cb.constant(i)).collect();
        let point = cb.constant(2);
        let res = cb.horners_method(point, &coeffs);
        assert_eq!(
            cb.circuit.eval(res, &[]),
            (1 + 2 * 2 + 3 * 2u64.pow(2) + 4 * 2u64.pow(3)).into()
        )
    }

    /// circuit `(i0 + 14) * i0`
    #[test]
    fn test_to_felts() {
        let (mut cb, inputs) = CircuitBuilder::new(1);
        let i0 = inputs[0];
        let cst = cb.constant(14);
        let add = cb.add(i0, cst);
        let mul = cb.mul(i0, add);
        let (_, circuit) = cb.normalize(mul);
        let res = circuit.to_felts();
        let expected: Vec<Quad> = vec![
            1u64,                                                                // n_inputs
            1,                                                                   // n_constants
            2,                                                                   // n_nodes
            14,                                                                  // constant 15
            0b010_000000000000000000000000000000_000000000000000000000000000001, // addition of input at index 0 and constant 5 at index 1
            0b001_000000000000000000000000000000_000000000000000000000000000010, // multiplication of input at index 0 and addition at index 2
        ]
        .into_iter()
        .map(Quad::from)
        .collect();
        assert_eq!(res, expected)
    }
}

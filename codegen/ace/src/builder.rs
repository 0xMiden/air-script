use crate::circuit::{ArithmeticOp, Circuit, Node, OperationNode};
use crate::layout::{Layout, StarkVar};
use air_ir::{
    Air, NodeIndex, Operation as AirOperation, PeriodicColumnAccess, QualifiedIdentifier, Value,
};
use miden_core::Felt;
use std::collections::BTreeMap;

/// [`CircuitBuilder`] is the only way to build a [`Circuit`]. It guarantees the following
/// properties:
/// - no dangling references (e.g., a node pointing to a child that does not exist),
/// - no cycles (the circuit is a DAG),
/// - no disconnected nodes, except possibly for inputs,
/// - no duplicated nodes,
/// - operation nodes can only reference previously constructed nodes.
///
/// The circuit can be obtained using [`CircuitBuilder::into_ace_circuit`].
#[derive(Clone, Debug, Default)]
pub struct CircuitBuilder {
    pub(crate) layout: Layout,
    // Circuit constants and a cache for avoiding duplicate values.
    constants: Vec<Felt>,
    constants_cache: BTreeMap<u64, Node>,
    // Operations and a cache mapping ace operations to already constructed nodes.
    operations: Vec<OperationNode>,
    ops_cache: BTreeMap<OperationNode, Node>,
    // A cache of nodes already inserted in the circuit, used to avoid duplicates.
    air_node_cache: BTreeMap<AirOperation, Node>,
    // Cache mapping a periodic column identifier to the evaluation of a column at `z`.
    periodic_columns_cache: BTreeMap<QualifiedIdentifier, Node>,
}

impl CircuitBuilder {
    /// Initializes a [`CircuitBuilder`] for a given [`Air`].
    pub fn new(air: &Air) -> Self {
        let layout = Layout::new(air);
        Self {
            layout,
            constants: vec![],
            constants_cache: BTreeMap::default(),
            operations: vec![],
            ops_cache: BTreeMap::default(),
            air_node_cache: BTreeMap::default(),
            periodic_columns_cache: BTreeMap::default(),
        }
    }

    /// Returns the built [`Circuit`].
    pub fn into_ace_circuit(self) -> Circuit {
        Circuit {
            layout: self.layout,
            constants: self.constants,
            operations: self.operations,
        }
    }

    /// Returns a [`Node`] representing the evaluation of the arithmetic operation.
    ///
    /// # Details
    /// - Before inserting the node in the graph, the function checks if it has previously been
    ///   evaluated, in which case no new node is inserted in the operations cache.
    /// - If the expression involves two constant nodes, the resulting constant value is evaluated
    ///   before being added as a constant.
    fn push_op(&mut self, op: ArithmeticOp, node_l: Node, node_r: Node) -> Node {
        // If the operation has already been evaluated, return it from the cache
        let operation = OperationNode { op, node_l, node_r };
        if let Some(node) = self.ops_cache.get(&operation) {
            return *node;
        }

        // Otherwise, create a new node for the result expression
        let node = match (node_l, node_r) {
            // Evaluate and cache constant expression
            (Node::Constant(idx_l), Node::Constant(idx_r)) => {
                let c_l = self.constants[idx_l];
                let c_r = self.constants[idx_r];
                let c = match op {
                    ArithmeticOp::Sub => c_l - c_r,
                    ArithmeticOp::Mul => c_l * c_r,
                    ArithmeticOp::Add => c_l + c_r,
                };
                self.constant(c.as_int())
            }
            // Store new `Operation` node
            _ => {
                let index = self.operations.len();
                self.operations.push(operation);
                Node::Operation(index)
            }
        };

        // Cache the operation node for future use.
        self.ops_cache.insert(operation, node);
        node
    }

    /// Returns a [`Node`] corresponding to a circuit constant. The mapping is cached to avoid
    /// duplicating constants.
    pub fn constant(&mut self, c: u64) -> Node {
        // Return the node from the cache
        if let Some(node) = self.constants_cache.get(&c) {
            return *node;
        }

        // Insert the new unique constant and create a node for it.
        let index = self.constants.len();
        self.constants.push(Felt::new(c));
        let node = Node::Constant(index);
        self.constants_cache.insert(c, node);
        node
    }

    /// Recursively maps a [`NodeIndex`] from the [`Air`]'s node graph to an ACE [`Node`]. Results
    /// are cached to avoid recomputing previously visited branches.
    ///
    /// # Panic
    /// Panics if the `node_index` references an invalid node in the [`Air`] from which the
    /// [`CircuitBuilder`] was constructed.
    pub fn node_from_index(&mut self, air: &Air, node_index: &NodeIndex) -> Node {
        let air_op = air.constraint_graph().node(node_index).op();
        if let Some(node) = self.air_node_cache.get(air_op) {
            return *node;
        };

        let node = match air_op {
            AirOperation::Value(v) => match v {
                Value::Constant(c) => self.constant(*c),
                Value::TraceAccess(access) => self
                    .layout
                    .trace_access_node(access)
                    .expect("invalid trace access"),
                Value::PeriodicColumn(access) => self
                    .periodic_column(air, access)
                    .expect("invalid periodic column access"),
                Value::PublicInput(pi) => self.layout.public_inputs[&pi.name]
                    .as_node(pi.index)
                    .expect("invalid public input access"),
                Value::RandomValue(idx) => self
                    .layout
                    .random_values
                    .as_node(*idx)
                    .expect("invalid random value index"),
            },
            AirOperation::Add(l_idx, r_idx) => {
                let node_l = self.node_from_index(air, l_idx);
                let node_r = self.node_from_index(air, r_idx);
                self.add(node_l, node_r)
            }
            AirOperation::Sub(l_idx, r_idx) => {
                let node_l = self.node_from_index(air, l_idx);
                let node_r = self.node_from_index(air, r_idx);
                self.sub(node_l, node_r)
            }
            AirOperation::Mul(l_idx, r_idx) => {
                let node_l = self.node_from_index(air, l_idx);
                let node_r = self.node_from_index(air, r_idx);
                self.mul(node_l, node_r)
            }
        };
        self.air_node_cache.insert(*air_op, node);
        node
    }

    /// Returns the [`Node`] resulting from the addition of two existing [`Node`]s.
    pub fn add(&mut self, mut node_l: Node, mut node_r: Node) -> Node {
        // Since addition is commutative, sorting ensures the operation is not duplicated.
        if node_r < node_l {
            (node_l, node_r) = (node_r, node_l);
        }

        // Check if either node is zero
        let zero = self.constant(0);
        if node_l == zero {
            return node_r;
        }
        if node_r == zero {
            return node_l;
        }

        self.push_op(ArithmeticOp::Add, node_l, node_r)
    }

    /// Returns the [`Node`] resulting from the multiplication of two existing [`Node`]s.
    pub fn mul(&mut self, mut node_l: Node, mut node_r: Node) -> Node {
        // Since multiplication is commutative, sorting ensures the operation is not duplicated
        if node_r < node_l {
            (node_l, node_r) = (node_r, node_l);
        }

        // Return zero when either node is zero
        let zero = self.constant(0);
        if node_l == zero || node_r == zero {
            return zero;
        }

        // Multiplication by 1 is the same as identity
        let one = self.constant(1);
        if node_l == one {
            return node_r;
        }
        if node_r == one {
            return node_l;
        }

        self.push_op(ArithmeticOp::Mul, node_l, node_r)
    }

    /// Returns the [`Node`] resulting from the subtraction of two existing [`Node`]s.
    pub fn sub(&mut self, node_l: Node, node_r: Node) -> Node {
        // Subtracting by zero is the identity.
        let zero = self.constant(0);
        if node_r == zero {
            return node_l;
        }
        if node_l == node_r {
            return zero;
        }

        self.push_op(ArithmeticOp::Sub, node_l, node_r)
    }

    /// Returns the sum of [`Node`]s returned by the `els` `Iterator`.
    #[allow(unused)]
    pub fn sum(&mut self, els: impl IntoIterator<Item = Node>) -> Node {
        els.into_iter()
            .reduce(|acc, r| self.add(acc, r))
            .unwrap_or_else(|| self.constant(0))
    }

    /// Returns the product of [`Node`]s returned by the `els` `Iteration`.
    pub fn prod(&mut self, els: impl IntoIterator<Item = Node>) -> Node {
        els.into_iter()
            .reduce(|acc, r| self.mul(acc, r))
            .unwrap_or_else(|| self.constant(1))
    }

    /// Evaluates a polynomial with coefficients `coeffs` at the given `point`.
    /// `∑ᵢ coeffs[i]⋅xⁱ = coeffs[0] + coeffs[1]⋅x + ⋯ + coeffs[n−1]⋅xⁿ⁻¹`
    pub fn poly_eval(&mut self, point: Node, coeffs: &[Node]) -> Node {
        self.horner_eval(point, coeffs.iter().copied().rev())
    }

    /// Evaluates a polynomial with coefficients `coeffs` at the given `point`
    /// using Horner's method.
    /// `∑ᵢ coeffs[n-i-1]⋅xⁱ = coeffs[n-1] + ⋯ + coeffs[1]⋅xⁿ⁻²  coeffs[0]⋅xⁿ⁻¹`
    pub fn horner_eval(&mut self, point: Node, els: impl IntoIterator<Item = Node>) -> Node {
        els.into_iter()
            .reduce(|acc, coeff| {
                let mul = self.mul(point, acc);
                self.add(coeff, mul)
            })
            .unwrap_or_else(|| self.constant(0))
    }

    /// Returns a [`Node`] corresponding to the evaluation of the `periodic_column` at the appropriate
    /// power of `z`. The evaluation is cached to avoid unnecessary computation.
    fn periodic_column(
        &mut self,
        air: &Air,
        periodic_column: &PeriodicColumnAccess,
    ) -> Option<Node> {
        let ident = periodic_column.name;

        // Check if we have already computed this column's value
        if let Some(node) = self.periodic_columns_cache.get(&ident) {
            return Some(*node);
        }

        let periodic_column = &air.periodic_columns.get(&ident)?;

        // Maximum length of all periodic columns in the air.
        let max_col_len = air
            .periodic_columns
            .values()
            .map(|col| col.values.len())
            .max()
            .unwrap();

        // The power of `z` for the longest column. Let `k` such that `z_max_col = z^k`,
        // where `k = trace_len / max_cycle_len`
        let z_max_col = self.layout.stark_node(StarkVar::ZMaxCycle);

        let col_len = periodic_column.values.len();
        assert!(max_col_len.is_power_of_two());
        assert!(col_len.is_power_of_two());
        // The evaluation point is z^k where
        //   l = trace_len / col_len = k * max_col_len / col_len = k * pow_col.
        // Computed by squaring z^k log(pow_col) times, since pow_col is a power of 2.
        // For different columns, squares of `z_max_col` are cached, avoiding duplicate operations.
        let log_pow_col = (max_col_len / col_len).ilog2();
        let z_col = (0..log_pow_col).fold(z_max_col, |acc, _| self.mul(acc, acc));

        // Interpolate the values of the column, converting the resulting coefficients
        // to constant nodes
        let poly_nodes: Vec<_> = {
            let mut column: Vec<_> = periodic_column
                .values
                .iter()
                .map(|val| Felt::new(*val))
                .collect();
            let inv_twiddles = winter_math::fft::get_inv_twiddles::<Felt>(column.len());
            winter_math::fft::interpolate_poly(&mut column, &inv_twiddles);
            column
                .into_iter()
                .map(|coeff| self.constant(coeff.as_int()))
                .collect()
        };

        // Evaluate the polynomial at z_col
        let result = self.poly_eval(z_col, &poly_nodes);

        // Cache evaluation
        self.periodic_columns_cache.insert(ident, result);
        Some(result)
    }
}

/// Computes a linear combination with the powers of a random challenge alpha
/// `\sum_i alpha^(offset+i) * coeffs[i]`
/// When called multiple times, the alpha keeps being increased with
/// `alpha^(offset-1)` being the last power used in the last call.
pub struct LinearCombination {
    alpha: Node,
    prev_alpha: Option<Node>,
}

impl LinearCombination {
    pub fn new(alpha: Node) -> Self {
        Self {
            alpha,
            prev_alpha: None,
        }
    }

    /// Returns the linear combination of the [`Node`]s returned by the `els` [`Iterator`] with the
    /// coefficients `[alpha^{offset}, ..., alpha^{offset + n - 1}]`, where `n` is the number of
    /// elements in `els`.
    pub fn next_linear_combination(
        &mut self,
        cb: &mut CircuitBuilder,
        els: impl IntoIterator<Item = Node>,
    ) -> Node {
        els.into_iter().fold(cb.constant(0), |acc, node| {
            // Get the next coefficient and save it for the next iteration
            let alpha = match self.prev_alpha {
                None => cb.constant(1),
                Some(prev_alpha) => cb.mul(prev_alpha, self.alpha),
            };
            self.prev_alpha = Some(alpha);

            let next_term = cb.mul(alpha, node);
            cb.add(acc, next_term)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QuadFelt;

    #[test]
    fn test_sum() {
        let mut cb = CircuitBuilder::default();
        let constants: Vec<Node> = (0..5).map(|i| cb.constant(i)).collect();
        let result_node = cb.sum(constants);
        let circuit = cb.into_ace_circuit();
        let result = circuit.eval(result_node, &[]);
        let result_expected: QuadFelt = Felt::new((0..5).sum::<u64>()).into();
        assert_eq!(result, result_expected)
    }

    #[test]
    fn test_prod() {
        let mut cb = CircuitBuilder::default();
        let constants: Vec<Node> = (0..5).map(Node::Input).collect();
        let result_node = cb.prod(constants);
        let circuit = cb.into_ace_circuit();
        let inputs: Vec<_> = (0..5u64).map(Felt::new).map(QuadFelt::from).collect();
        let result = circuit.eval(result_node, &inputs);
        let result_expected: QuadFelt = Felt::new((0..5).product::<u64>()).into();
        assert_eq!(result, result_expected)
    }

    #[test]
    fn test_horner() {
        let mut cb = CircuitBuilder::default();
        let coeff_nodes: Vec<Node> = (0..5).map(Node::Input).collect();
        let point = Felt::new(2u64);
        let point_node = cb.constant(2);
        let result_node = cb.horner_eval(point_node, coeff_nodes.clone());
        let circuit = cb.into_ace_circuit();

        let inputs: Vec<_> = (0..5u64).map(Felt::new).map(QuadFelt::from).collect();

        let result = circuit.eval(result_node, &inputs);
        let result_expected: QuadFelt = inputs
            .into_iter()
            .reduce(|acc, coeff| acc * QuadFelt::from(point) + coeff)
            .unwrap();
        assert_eq!(result, result_expected)
    }

    #[test]
    fn test_poly_eval() {
        let mut cb = CircuitBuilder::default();
        let coeff_nodes: Vec<Node> = (0..5).map(Node::Input).collect();
        let point = Felt::new(2u64);
        let point_node = cb.constant(2);
        let result_node = cb.poly_eval(point_node, &coeff_nodes);
        let circuit = cb.into_ace_circuit();

        let inputs: Vec<_> = (0..5u64).map(Felt::new).map(QuadFelt::from).collect();

        let result = circuit.eval(result_node, &inputs);
        let result_expected: QuadFelt = inputs
            .into_iter()
            .rev()
            .reduce(|acc, coeff| acc * QuadFelt::from(point) + coeff)
            .unwrap();
        assert_eq!(result, result_expected)
    }

    #[test]
    fn test_linear_combination_alpha() {
        let mut cb = CircuitBuilder::default();
        let alpha = Node::Input(0);
        let coeffs_1 = [1, 2, 3].map(Node::Input);
        let coeffs_2 = [4, 5, 6].map(Node::Input);

        let mut lc = LinearCombination::new(alpha);
        let res_1 = lc.next_linear_combination(&mut cb, coeffs_1);
        let res_2 = lc.next_linear_combination(&mut cb, coeffs_2);
        let res = cb.add(res_1, res_2);

        let alpha = QuadFelt::from(Felt::new(5u64));
        let coeffs: Vec<_> = (0..6)
            .map(|i| QuadFelt::from(Felt::new(1 + i as u64)))
            .collect();

        let result_expected = coeffs
            .iter()
            .rev()
            .copied()
            .reduce(|acc, coeff| acc * alpha + coeff)
            .unwrap();

        let circuit = cb.into_ace_circuit();
        let inputs: Vec<_> = [alpha].into_iter().chain(coeffs).collect();
        let result = circuit.eval(res, &inputs);
        assert_eq!(result, result_expected)
    }
}

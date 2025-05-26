use std::collections::BTreeMap;

use air_ir::{
    Air, Bus, BusOpKind, BusType, Identifier, IntegrityConstraintDegree, Operation, TraceAccess,
    TraceSegmentId,
};

use super::{graph::Codegen, ElemType};

// Degree:
// 1 for p / p'
// for each column/latch:
//   accumulate_degree
pub(crate) fn compute_multiset_degree(ir: &Air, bus: &Bus) -> IntegrityConstraintDegree {
    let mut cycles = BTreeMap::default();

    let mut p_degree = 0;
    let mut p_prime_degree = 0;

    for bus_ops in bus.bus_ops.iter() {
        let latch_degree = ir
            .constraint_graph()
            .accumulate_degree(&mut cycles, &bus_ops.latch);
        let mut columns_degree = 0;
        for column in bus_ops.columns.iter() {
            let column_degree = ir.constraint_graph().accumulate_degree(&mut cycles, column);
            columns_degree = columns_degree.max(column_degree);
        }
        let bus_op_degree = latch_degree + columns_degree;
        match bus_ops.op_kind {
            BusOpKind::Insert => p_degree += bus_op_degree,
            BusOpKind::Remove => p_prime_degree += bus_op_degree,
        }
    }
    let degree = p_degree.max(p_prime_degree) + 1; // +1 for the multiplication with p / p'

    if cycles.is_empty() {
        IntegrityConstraintDegree::new(degree)
    } else {
        IntegrityConstraintDegree::with_cycles(degree, cycles.values().copied().collect())
    }
}

pub(crate) fn compute_logup_degree(ir: &Air, bus: &Bus) -> IntegrityConstraintDegree {
    let mut cycles = BTreeMap::default();

    let mut base_columns_degrees = vec![];
    let mut base_latch_degrees = vec![];

    for bus_ops in bus.bus_ops.iter() {
        let latch_degree = ir
            .constraint_graph()
            .accumulate_degree(&mut cycles, &bus_ops.latch);
        let mut columns_degree = 0;

        for column in bus_ops.columns.iter() {
            let column_degree = ir.constraint_graph().accumulate_degree(&mut cycles, column);
            columns_degree = columns_degree.max(column_degree);
        }
        base_columns_degrees.push(columns_degree);
        base_latch_degrees.push(latch_degree);
    }

    let mut degrees = base_columns_degrees.iter().sum::<usize>() + 1;
    for (i, base_latch_degree) in base_latch_degrees.iter().enumerate() {
        let mut cur_degree = *base_latch_degree;

        for (j, base_columns_degree) in base_columns_degrees.iter().enumerate() {
            if i != j {
                cur_degree += *base_columns_degree;
            }
        }
        degrees = degrees.max(cur_degree);
    }

    if cycles.is_empty() {
        IntegrityConstraintDegree::new(degrees)
    } else {
        IntegrityConstraintDegree::with_cycles(degrees, cycles.values().copied().collect())
    }
}

fn random_value_string(index: usize) -> OperationStr {
    OperationStr::Value(format!("aux_rand_elements.rand_elements()[{index}]"))
}

#[derive(Clone)]
pub enum OperationStr {
    Value(String),
    /// Evaluates by addition over two operands (given as nodes in the graph)
    Add(String),
    /// Evaluates by subtraction over two operands (given as nodes in the graph)
    Sub(String),
    /// Evaluates by multiplication over two operands (given as nodes in the graph)
    Mul(String),
}
impl OperationStr {
    pub fn precedence(&self) -> usize {
        match self {
            Self::Add(_) => 1,
            Self::Sub(_) => 2,
            Self::Mul(_) => 3,
            _ => 4,
        }
    }
    pub fn inner(&self) -> String {
        match self {
            Self::Value(value) => value.clone(),
            Self::Add(value) => value.clone(),
            Self::Sub(value) => value.clone(),
            Self::Mul(value) => value.clone(),
        }
    }
    pub fn from_operation(
        ir: &Air,
        op: &Operation,
        elem_type: ElemType,
        trace_segment: TraceSegmentId,
    ) -> OperationStr {
        let inner_str = op.to_string(ir, elem_type, trace_segment);
        match op {
            Operation::Value(_) => OperationStr::Value(inner_str),
            Operation::Add(_, _) => OperationStr::Add(inner_str),
            Operation::Sub(_, _) => OperationStr::Sub(inner_str),
            Operation::Mul(_, _) => OperationStr::Mul(inner_str),
        }
    }
    pub fn create_add(lhs: OperationStr, rhs: OperationStr) -> OperationStr {
        let lhs = lhs.inner();
        let rhs = rhs.inner();
        let res_str = format!("{lhs} + {rhs}");
        OperationStr::Add(res_str)
    }
    pub fn create_sub(lhs: OperationStr, rhs: OperationStr) -> OperationStr {
        let lhs = lhs.inner();
        let rhs = if rhs.precedence() <= OperationStr::Sub("".to_string()).precedence() {
            format!("({})", rhs.inner())
        } else {
            rhs.inner()
        };
        let res_str = format!("{lhs} - {rhs}");
        OperationStr::Sub(res_str)
    }
    pub fn create_mul(lhs: OperationStr, rhs: OperationStr) -> OperationStr {
        let lhs = if lhs.precedence() < OperationStr::Mul("".to_string()).precedence() {
            format!("({})", lhs.inner())
        } else {
            lhs.inner()
        };
        let rhs = if rhs.precedence() < OperationStr::Mul("".to_string()).precedence() {
            format!("({})", rhs.inner())
        } else {
            rhs.inner()
        };
        let res_str = format!("{lhs} * {rhs}");
        OperationStr::Mul(res_str)
    }
}

/// Example:
/// p.insert(a, b) when s
/// p.remove(c, d) when (1 - s)
/// => p' * (( A0 + A1 c + A2 d ) ( 1 - s ) + s) = p * ( A0 + A1 a + A2 b ) s + 1 - s
/// p' * ( columns removed combined with alphas ) = p * ( columns inserted combined with alphas )
pub(crate) fn expand_multiset_constraints(ir: &Air, bus: &Bus, index: usize) -> String {
    let mut p_factor = None;
    let mut p_prime_factor = None;

    let bus_access = TraceAccess::new(1, index, 0);
    let bus_access_with_offset = TraceAccess::new(1, index, 1);

    let bus_access = OperationStr::Value(bus_access.to_string(ir, ElemType::Ext, 1));
    let bus_access_with_offset =
        OperationStr::Value(bus_access_with_offset.to_string(ir, ElemType::Ext, 1));

    for bus_op in bus.bus_ops.iter() {
        let latch = bus_op.latch;
        let bus_op_kind = bus_op.op_kind;

        // 1. Combine args with alphas
        // 1.1 Start with the first alpha
        let mut args_combined = random_value_string(0);

        for (index, column) in bus_op.columns.iter().enumerate() {
            let column = OperationStr::from_operation(
                ir,
                ir.constraint_graph().node(column).op(),
                ElemType::Ext,
                1,
            );

            // 1.2 Create corresponding alpha
            let alpha = random_value_string(index + 1);

            // 1.3 Multiply arg with alpha
            let arg_times_alpha = OperationStr::create_mul(column, alpha);

            // 1.4 Combine with other args
            args_combined = OperationStr::create_add(args_combined, arg_times_alpha);
        }

        let latch = OperationStr::from_operation(
            ir,
            ir.constraint_graph().node(&latch).op(),
            ElemType::Ext,
            1,
        );

        // 2. Multiply by latch
        let args_combined_with_latch = OperationStr::create_mul(args_combined, latch.clone());

        // 3. add inverse of latch
        let one = OperationStr::Value("E::ONE".to_string());
        let inverse_latch = OperationStr::create_sub(one, latch);
        let args_combined_with_latch_and_latch_inverse =
            OperationStr::create_add(args_combined_with_latch, inverse_latch);

        // 4. Multiply them to p_factor or p_prime_factor (depending on bus_op_kind: insert: p, remove: p_prime)
        match bus_op_kind {
            BusOpKind::Insert => {
                p_factor = match p_factor {
                    Some(p_factor) => Some(OperationStr::create_mul(
                        p_factor,
                        args_combined_with_latch_and_latch_inverse,
                    )),
                    None => Some(args_combined_with_latch_and_latch_inverse),
                };
            }
            BusOpKind::Remove => {
                p_prime_factor = match p_prime_factor {
                    Some(p_prime_factor) => Some(OperationStr::create_mul(
                        p_prime_factor,
                        args_combined_with_latch_and_latch_inverse,
                    )),
                    None => Some(args_combined_with_latch_and_latch_inverse),
                };
            }
        }
    }

    // 5. Multiply the factors with the bus column (with and without offset for p' and p respectively)
    let p_prod = match p_factor {
        Some(p_factor) => OperationStr::create_mul(p_factor, bus_access),
        None => bus_access,
    };
    let p_prime_prod = match p_prime_factor {
        Some(p_prime_factor) => OperationStr::create_mul(p_prime_factor, bus_access_with_offset),
        None => bus_access_with_offset,
    };

    // 6. Create the resulting constraint
    let resulting_constraint = OperationStr::create_sub(p_prod, p_prime_prod);
    resulting_constraint.inner()
}

/// Example:
/// q.insert(a, b, c) with d
/// q.remove(e, f, g) when s
/// => q' + s / ( A0 + A1 e + A2 f + A3 g ) = q + d / ( A0 + A1 a + A2 b + A3 c )
///
///  q' + s / ( columns removed combined with alphas ) = q + d / ( columns inserted combined with alphas )
/// PROD * q' + s * ( columns inserted combined with alphas ) = PROD * q + d * ( columns removed combined with alphas )
pub(crate) fn expand_logup_constraints(ir: &Air, bus: &Bus, index: usize) -> String {
    let bus_access = TraceAccess::new(1, index, 0);
    let bus_access_with_offset = TraceAccess::new(1, index, 1);

    let bus_access = OperationStr::Value(bus_access.to_string(ir, ElemType::Ext, 1));
    let bus_access_with_offset =
        OperationStr::Value(bus_access_with_offset.to_string(ir, ElemType::Ext, 1));

    // 1. Compute all the factors
    let mut factors = vec![];
    for bus_op in bus.bus_ops.iter() {
        // 1. Combine args with alphas
        // 1.1 Start with the first alpha
        let mut args_combined = random_value_string(0);
        for (index, column) in bus_op.columns.iter().enumerate() {
            // 1.2 Create corresponding alpha
            let alpha = random_value_string(index + 1);

            let column = OperationStr::from_operation(
                ir,
                ir.constraint_graph().node(column).op(),
                ElemType::Ext,
                1,
            );

            // 1.3 Multiply arg with alpha
            let arg_times_alpha = OperationStr::create_mul(column, alpha);

            // 1.4 Combine with other args
            args_combined = OperationStr::create_add(args_combined, arg_times_alpha);
        }

        factors.push(args_combined);
    }

    // 2. Compute the product of all factors (will be used to multiply q and q')
    let mut total_factors = None;
    for factor in factors.iter() {
        total_factors = match total_factors {
            Some(total_factors) => Some(OperationStr::create_mul(total_factors, factor.clone())),
            None => Some(factor.clone()),
        };
    }

    // 3. For each column, compute the product of all factors except the one of the current column, and multiply it with the latch
    let mut terms_added_to_bus = None;
    let mut terms_removed_from_bus = None;

    for (index, bus_op) in bus.bus_ops.iter().enumerate() {
        let latch = bus_op.latch;
        let bus_op_kind = bus_op.op_kind;

        // 3.1 Compute the product of all factors except the one of the current column
        let mut factors_without_current = None;
        for (i, factor) in factors.iter().enumerate() {
            if i != index {
                factors_without_current = match factors_without_current {
                    Some(factors_without_current) => Some(OperationStr::create_mul(
                        factors_without_current,
                        factor.clone(),
                    )),
                    None => Some(factor.clone()),
                };
            }
        }

        let latch = OperationStr::from_operation(
            ir,
            ir.constraint_graph().node(&latch).op(),
            ElemType::Ext,
            1,
        );

        // 3.2 Multiply by latch
        let factors_without_current_with_latch = match factors_without_current {
            Some(factors_without_current) => {
                OperationStr::create_mul(factors_without_current, latch.clone())
            }
            None => latch.clone(),
        };

        // 3.3 Depending on the bus_op_kind, add to q_factor or q_prime_factor
        match bus_op_kind {
            BusOpKind::Insert => {
                terms_added_to_bus = match terms_added_to_bus {
                    Some(terms_added_to_bus) => Some(OperationStr::create_add(
                        terms_added_to_bus,
                        factors_without_current_with_latch,
                    )),
                    None => Some(factors_without_current_with_latch),
                };
            }
            BusOpKind::Remove => {
                terms_removed_from_bus = match terms_removed_from_bus {
                    Some(terms_removed_from_bus) => Some(OperationStr::create_add(
                        terms_removed_from_bus,
                        factors_without_current_with_latch,
                    )),
                    None => Some(factors_without_current_with_latch),
                };
            }
        }
    }

    // 4. Add all the terms together
    let q_prod = match total_factors.clone() {
        Some(total_factors) => OperationStr::create_mul(total_factors, bus_access),
        None => bus_access,
    };
    let q_prime_prod = match total_factors {
        Some(total_factors) => OperationStr::create_mul(total_factors, bus_access_with_offset),
        None => bus_access_with_offset,
    };
    let q_term = match terms_added_to_bus {
        Some(terms_added_to_bus) => OperationStr::create_add(q_prod, terms_added_to_bus.clone()),
        None => q_prod,
    };
    let q_prime_term = match terms_removed_from_bus {
        Some(terms_removed_from_bus) => {
            OperationStr::create_add(q_prime_prod, terms_removed_from_bus.clone())
        }
        None => q_prime_prod,
    };

    // 5. Create the resulting constraint
    let resulting_constraint = OperationStr::create_sub(q_term, q_prime_term);
    resulting_constraint.inner()
}

pub(crate) fn call_bus_boundary_varlen_pubinput(
    ir: &Air,
    bus_name: Identifier,
    table_name: Identifier,
) -> String {
    let bus = ir.buses.get(&bus_name).expect("bus not found");
    match bus.bus_type {
        BusType::Multiset => format!(
            "Self::bus_multiset_boundary_varlen(aux_rand_elements, &self.{}.iter())",
            table_name
        ),
        BusType::Logup => format!(
            "Self::bus_logup_boundary_varlen(aux_rand_elements, &self.{}.iter())",
            table_name
        ),
    }
}

pub(crate) fn num_bus_boundary_constraints(ir: &Air) -> usize {
    ir.buses.len() * 2
}

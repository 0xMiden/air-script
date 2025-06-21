use air_parser::ast::{Boundary, BusType};
use air_pass::Pass;
use miden_diagnostics::DiagnosticsHandler;
use mir::ir::BusOpKind;

use crate::{
    AUX_SEGMENT, Air, BusBoundary, BusOp, CompileError, ConstraintDomain, NodeIndex, Operation,
    TraceAccess,
};

pub struct BusOpExpand<'a> {
    #[allow(unused)]
    diagnostics: &'a DiagnosticsHandler,
}

impl Pass for BusOpExpand<'_> {
    type Input<'a> = Air;
    type Output<'a> = Air;
    type Error = CompileError;

    fn run<'a>(&mut self, mut ir: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        let buses = ir.buses.clone();

        for (bus_index, (_ident, bus)) in buses.iter().enumerate() {
            let bus_type = bus.bus_type;

            // Expand bus boundary constraints first
            self.handle_boundary_constraint(
                &mut ir,
                bus_type,
                &bus.first,
                Boundary::First,
                bus_index,
            );
            self.handle_boundary_constraint(
                &mut ir,
                bus_type,
                &bus.last,
                Boundary::Last,
                bus_index,
            );

            let bus_ops = bus.bus_ops.clone();

            let bus_trace_access = TraceAccess::new(AUX_SEGMENT, bus_index, 0);
            let bus_trace_access_with_offset = TraceAccess::new(AUX_SEGMENT, bus_index, 1);

            let bus_access =
                ir.constraint_graph_mut()
                    .insert_node(Operation::Value(crate::Value::TraceAccess(
                        bus_trace_access,
                    )));
            let bus_access_with_offset =
                ir.constraint_graph_mut()
                    .insert_node(Operation::Value(crate::Value::TraceAccess(
                        bus_trace_access_with_offset,
                    )));

            // Then, depending on the bus type, expand the integrity constraint
            match bus_type {
                BusType::Multiset => {
                    self.expand_multiset_constraint(
                        &mut ir,
                        bus_ops,
                        bus_access,
                        bus_access_with_offset,
                    );
                }
                BusType::Logup => {
                    self.expand_logup_constraint(
                        &mut ir,
                        bus_ops,
                        bus_access,
                        bus_access_with_offset,
                    );
                }
            }
        }

        ir.num_random_values = buses
            .values()
            .map(|bus| {
                bus.bus_ops
                    .iter()
                    .map(|a| a.columns.len() + 1)
                    .max()
                    .unwrap_or_default()
            })
            .max()
            .unwrap_or_default() as u16;

        Ok(ir)
    }
}

impl<'a> BusOpExpand<'a> {
    #[allow(unused)]
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self { diagnostics }
    }

    /// Helper function to handle and insert into the graph a bus boundary constraint if possible
    fn handle_boundary_constraint(
        &self,
        ir: &mut Air,
        bus_type: BusType,
        bus_boundary: &BusBoundary,
        boundary: Boundary,
        bus_index: usize,
    ) {
        match bus_boundary {
            // Boundaries to PublicInputTable should be handled later during codegen, as we cannot
            // know at this point the length of the table, so we cannot generate the resulting constraint
            BusBoundary::PublicInputTable(_public_input_table_access) => {}
            // Unconstrained boundaries do not require any constraints
            BusBoundary::Unconstrained => {}
            BusBoundary::Null => {
                // The value of the constraint for an empty bus depends on the bus types (1 for multiset, 0 for logup)
                let value = match bus_type {
                    BusType::Multiset => ir
                        .constraint_graph_mut()
                        .insert_node(Operation::Value(crate::Value::Constant(1))),
                    BusType::Logup => ir
                        .constraint_graph_mut()
                        .insert_node(Operation::Value(crate::Value::Constant(0))),
                };

                let bus_trace_access = TraceAccess::new(AUX_SEGMENT, bus_index, 0);
                let bus_access = ir.constraint_graph_mut().insert_node(Operation::Value(
                    crate::Value::TraceAccess(bus_trace_access),
                ));

                // Then, we enforce for instance the constraint `p.first = 1` or `q.first = 0` to have an empty bus initially
                let root = ir
                    .constraint_graph_mut()
                    .insert_node(Operation::Sub(bus_access, value));
                let domain = match boundary {
                    Boundary::First => ConstraintDomain::FirstRow,
                    Boundary::Last => ConstraintDomain::LastRow,
                };
                // Store the generated constraint
                ir.constraints.insert_constraint(AUX_SEGMENT, root, domain);
            }
        }
    }

    /// Helper function to expand the integrity constraint of a multiset bus
    fn expand_multiset_constraint(
        &self,
        ir: &mut Air,
        bus_ops: Vec<BusOp>,
        bus_access: NodeIndex,
        bus_access_with_offset: NodeIndex,
    ) {
        let graph = ir.constraint_graph_mut();

        let mut p_factor = None;
        let mut p_prime_factor = None;

        for bus_op in bus_ops {
            // Expand bus operations
            let columns = bus_op.columns.clone(); // columns are the bus_operations (insert or remove of a Vec of arguments)
            let latch = bus_op.latch; // latch is the selector
            let bus_op_kind = bus_op.op_kind; // kind is either insert or remove

            // Example:
            // p.insert(a, b) when s
            // p.remove(c, d) when (1 - s)
            // => p' * (( A0 + A1 c + A2 d ) ( 1 - s ) + s) = p * ( A0 + A1 a + A2 b ) s + 1 - s

            // p' * ( columns removed combined with alphas ) = p * ( columns inserted combined with alphas )
            let mut args_combined =
                graph.insert_node(Operation::Value(crate::Value::RandomValue(0)));

            for (col_index, column) in columns.iter().enumerate() {
                // 1. Combine args with alphas
                // 1.1 Start with the first alpha

                // 1.2 Create corresponding alpha
                let alpha =
                    graph.insert_node(Operation::Value(crate::Value::RandomValue(col_index + 1)));

                // 1.3 Multiply arg with alpha
                let arg_times_alpha = graph.insert_node(Operation::Mul(*column, alpha));

                // 1.4 Combine with other args
                args_combined = graph.insert_node(Operation::Add(args_combined, arg_times_alpha));
            }

            // 2. Multiply by latch
            let args_combined_with_latch = graph.insert_node(Operation::Mul(args_combined, latch));

            // 3. add inverse of latch
            let one = graph.insert_node(Operation::Value(crate::Value::Constant(1)));
            let inverse_latch = graph.insert_node(Operation::Sub(one, latch));
            let args_combined_with_latch_and_latch_inverse =
                graph.insert_node(Operation::Add(args_combined_with_latch, inverse_latch));

            // 4. Multiply them to p_factor or p_prime_factor (depending on bus_op_kind: insert: p, remove: p_prime)
            match bus_op_kind {
                BusOpKind::Insert => {
                    p_factor = match p_factor {
                        Some(p_factor) => Some(graph.insert_node(Operation::Mul(
                            p_factor,
                            args_combined_with_latch_and_latch_inverse,
                        ))),
                        None => Some(args_combined_with_latch_and_latch_inverse),
                    };
                }
                BusOpKind::Remove => {
                    p_prime_factor = match p_prime_factor {
                        Some(p_prime_factor) => Some(graph.insert_node(Operation::Mul(
                            p_prime_factor,
                            args_combined_with_latch_and_latch_inverse,
                        ))),
                        None => Some(args_combined_with_latch_and_latch_inverse),
                    };
                }
            }
        }

        // 5. Multiply the factors with the bus column (with and without offset for p' and p respectively)
        let p_prod = match p_factor {
            Some(p_factor) => graph.insert_node(Operation::Mul(p_factor, bus_access)),
            None => bus_access,
        };
        let p_prime_prod = match p_prime_factor {
            Some(p_prime_factor) => {
                graph.insert_node(Operation::Mul(p_prime_factor, bus_access_with_offset))
            }
            None => bus_access_with_offset,
        };

        // 6. Create the resulting constraint and insert it into the graph
        let root = graph.insert_node(Operation::Sub(p_prod, p_prime_prod));

        ir.constraints
            .insert_constraint(AUX_SEGMENT, root, ConstraintDomain::EveryRow);
    }

    /// Helper function to expand the integrity constraint of a logup bus
    fn expand_logup_constraint(
        &self,
        ir: &mut Air,
        bus_ops: Vec<BusOp>,
        bus_access: NodeIndex,
        bus_access_with_offset: NodeIndex,
    ) {
        let graph = ir.constraint_graph_mut();
        // Example:
        // q.insert(a, b, c) with d
        // q.remove(e, f, g) when s
        // => q' + s / ( A0 + A1 e + A2 f + A3 g ) = q + d / ( A0 + A1 a + A2 b + A3 c )

        //  q' + s / ( columns removed combined with alphas ) = q + d / ( columns inserted combined with alphas )
        // PROD * q' + s * ( columns inserted combined with alphas ) = PROD * q + d * ( columns removed combined with alphas )

        // 1. Compute all the factors

        let mut factors = vec![];
        for bus_op in bus_ops.iter() {
            // Expand bus operations
            let columns = bus_op.columns.clone(); // columns are the bus_operations (insert or remove of a Vec of arguments)

            // 1. Combine args with alphas
            // 1.1 Start with the first alpha
            let mut args_combined =
                graph.insert_node(Operation::Value(crate::Value::RandomValue(0)));

            for (col_index, column) in columns.iter().enumerate() {
                // 1.2 Create corresponding alpha
                let alpha =
                    graph.insert_node(Operation::Value(crate::Value::RandomValue(col_index + 1)));

                // 1.3 Multiply arg with alpha
                let arg_times_alpha = graph.insert_node(Operation::Mul(*column, alpha));

                // 1.4 Combine with other args
                args_combined = graph.insert_node(Operation::Add(args_combined, arg_times_alpha));
            }
            factors.push(args_combined);
        }

        // 2. Compute the product of all factors (will be used to multiply q and q')
        let mut total_factors = None;
        for factor in factors.iter() {
            total_factors = match total_factors {
                Some(total_factors) => {
                    Some(graph.insert_node(Operation::Mul(total_factors, *factor)))
                }
                None => Some(*factor),
            };
        }

        // 3. For each column, compute the product of all factors except the one of the current column, and multiply it with the latch
        let mut terms_added_to_bus = None;
        let mut terms_removed_from_bus = None;

        for (bus_index, bus_op) in bus_ops.iter().enumerate() {
            let latch = bus_op.latch;
            let bus_op_kind = bus_op.op_kind;

            // 3.1 Compute the product of all factors except the one of the current columns
            let mut factors_without_current = None;
            for (i, factor) in factors.iter().enumerate() {
                if i != bus_index {
                    factors_without_current = match factors_without_current {
                        Some(factors_without_current) => Some(
                            graph.insert_node(Operation::Mul(factors_without_current, *factor)),
                        ),
                        None => Some(*factor),
                    };
                }
            }

            // 3.2 Multiply by latch
            let factors_without_current_with_latch = match factors_without_current {
                Some(factors_without_current) => {
                    graph.insert_node(Operation::Mul(factors_without_current, latch))
                }
                None => latch,
            };

            // 3.3 Depending on the bus_op_kind, add to q_factor or q_prime_factor
            match bus_op_kind {
                BusOpKind::Insert => {
                    terms_added_to_bus = match terms_added_to_bus {
                        Some(terms_added_to_bus) => Some(graph.insert_node(Operation::Add(
                            terms_added_to_bus,
                            factors_without_current_with_latch,
                        ))),
                        None => Some(factors_without_current_with_latch),
                    };
                }
                BusOpKind::Remove => {
                    terms_removed_from_bus = match terms_removed_from_bus {
                        Some(terms_removed_from_bus) => Some(graph.insert_node(Operation::Add(
                            terms_removed_from_bus,
                            factors_without_current_with_latch,
                        ))),
                        None => Some(factors_without_current_with_latch),
                    };
                }
            }
        }

        // 4. Add all the terms together
        let q_prod = match total_factors {
            Some(total_factors) => graph.insert_node(Operation::Mul(total_factors, bus_access)),
            None => bus_access,
        };
        let q_prime_prod = match total_factors {
            Some(total_factors) => {
                graph.insert_node(Operation::Mul(total_factors, bus_access_with_offset))
            }
            None => bus_access_with_offset,
        };
        let q_term = match terms_added_to_bus {
            Some(terms_added_to_bus) => {
                graph.insert_node(Operation::Add(q_prod, terms_added_to_bus))
            }
            None => q_prod,
        };
        let q_prime_term = match terms_removed_from_bus {
            Some(terms_removed_from_bus) => {
                graph.insert_node(Operation::Add(q_prime_prod, terms_removed_from_bus))
            }
            None => q_prime_prod,
        };

        // 5. Create the resulting constraint
        let root = graph.insert_node(Operation::Sub(q_term, q_prime_term));
        ir.constraints
            .insert_constraint(AUX_SEGMENT, root, ConstraintDomain::EveryRow);
    }
}

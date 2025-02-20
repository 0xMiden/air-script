use std::ops::Deref;

use air_parser::ast::{AccessType, BusType};
use air_pass::Pass;
use miden_diagnostics::{DiagnosticsHandler, SourceSpan, Spanned};

use super::duplicate_node;
use crate::{
    ir::{
        Accessor, Add, BusOpKind, ConstantValue, Enf, Link, Mir, MirValue, Mul, Op,
        SpannedMirValue, Sub, Value,
    },
    CompileError,
};

/// TODO MIR:
/// If needed, implement bus operation expand pass on MIR
/// See https://github.com/0xPolygonMiden/air-script/issues/183
///   
pub struct BusOpExpand<'a> {
    #[allow(unused)]
    diagnostics: &'a DiagnosticsHandler,
}

impl Pass for BusOpExpand<'_> {
    type Input<'a> = Mir;
    type Output<'a> = Mir;
    type Error = CompileError;

    fn run<'a>(&mut self, mut ir: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        let graph = ir.constraint_graph_mut();

        let buses = graph.buses.clone();

        for (_ident, bus) in buses {
            let bus_type = bus.borrow().bus_type.clone();
            let columns = bus.borrow().columns.clone(); // columns are the bus_operations (add or remove of a Vec of arguments)
            let latches = bus.borrow().latches.clone(); // latches are the selectors
            let first = bus.borrow().get_first().clone();
            let last = bus.borrow().get_last().clone();

            println!("first: {:?}", &first);
            println!("last: {:?}", &last);

            let bus_access = Value::create(SpannedMirValue {
                span: bus.borrow().span(),
                value: MirValue::BusAccess(bus.clone()),
            });
            let bus_access_with_offset = Accessor::create(
                duplicate_node(bus_access.clone(), &mut Default::default()),
                AccessType::Default,
                1,
                bus.borrow().span(),
            );

            // Expand bus boundary constraints first
            self.handle_boundary_constraint(bus_type.clone(), first/*, air_parser::ast::Boundary::First, bus_access.clone(), bus.borrow().span()*/);
            self.handle_boundary_constraint(
                bus_type.clone(),
                last, /*, air_parser::ast::Boundary::Last, bus_access.clone(), bus.borrow().span()*/
            );

            // Then, expend bus integrity constraints
            match bus_type {
                BusType::Unit => {
                    // Example:
                    // p.add(a, b) when s
                    // p.rem(c, d) when (1 - s)
                    // => p' * (( A0 + A1 c + A2 d ) ( 1 - s ) + s) = p * ( A0 + A1 a + A2 b ) s + 1 - s

                    // p' * ( columns removed combined with alphas ) = p * ( columns added combined with alphas )

                    let mut p_factor = Value::create(SpannedMirValue {
                        span: SourceSpan::default(),
                        value: crate::ir::MirValue::Constant(crate::ir::ConstantValue::Felt(1)),
                    });
                    let mut p_prime_factor = Value::create(SpannedMirValue {
                        span: SourceSpan::default(),
                        value: MirValue::Constant(crate::ir::ConstantValue::Felt(1)),
                    });

                    for (column, latch) in columns.iter().zip(latches.iter()) {
                        let bus_op = column.as_bus_op().unwrap();
                        let bus_op_kind = bus_op.kind.clone();
                        let bus_op_args = bus_op.args.clone();

                        // 1. Combine args with alphas
                        // 1.1 Start with the first alpha
                        let mut args_combined = Value::create(SpannedMirValue {
                            span: SourceSpan::default(),
                            value: MirValue::RandomValue(0),
                        });
                        for (index, arg) in bus_op_args.iter().enumerate() {
                            // 1.2 Create corresponding alpha
                            let alpha = Value::create(SpannedMirValue {
                                span: SourceSpan::default(),
                                value: MirValue::RandomValue(index + 1),
                            });

                            // 1.3 Multiply arg with alpha
                            let arg_times_alpha =
                                Mul::create(arg.clone(), alpha, SourceSpan::default());

                            // 1.4 Combine with other args
                            args_combined =
                                Add::create(args_combined, arg_times_alpha, SourceSpan::default());
                        }

                        // 2. Multiply by latch
                        let args_combined_with_latch =
                            Mul::create(args_combined, latch.clone(), SourceSpan::default());

                        // 3. add inverse of latch
                        let args_combined_with_latch_and_latch_inverse = Add::create(
                            args_combined_with_latch,
                            Sub::create(
                                Value::create(SpannedMirValue {
                                    span: SourceSpan::default(),
                                    value: MirValue::Constant(crate::ir::ConstantValue::Felt(1)),
                                }),
                                duplicate_node(latch.clone(), &mut Default::default()),
                                SourceSpan::default(),
                            ),
                            SourceSpan::default(),
                        );

                        // 4. add to p_factor or p_prime_factor (depending on bus_op_kind: add: p, rem: p_prime)
                        match bus_op_kind {
                            BusOpKind::Add => {
                                p_factor = Add::create(
                                    p_factor,
                                    args_combined_with_latch_and_latch_inverse,
                                    SourceSpan::default(),
                                );
                            }
                            BusOpKind::Rem => {
                                p_prime_factor = Add::create(
                                    p_prime_factor,
                                    args_combined_with_latch_and_latch_inverse,
                                    SourceSpan::default(),
                                );
                            }
                        }
                    }

                    // 5. Multiply the factors with the bus column (with and without offset for p' and p respectively)
                    let p_prod = Mul::create(p_factor, bus_access, SourceSpan::default());
                    let p_prime_prod = Mul::create(
                        p_prime_factor,
                        bus_access_with_offset,
                        SourceSpan::default(),
                    );

                    // 6. Create the resulting constraint and insert it into the graph
                    let resulting_constraint = Enf::create(
                        Sub::create(p_prod, p_prime_prod, SourceSpan::default()),
                        SourceSpan::default(),
                    );

                    graph.insert_integrity_constraints_root(resulting_constraint);
                }
                BusType::Mult => {
                    // Example:
                    // q.add(a, b, c) for d
                    // q.rem(e, f, g) when s
                    // => q' + s / ( A0 + A1 e + A2 f + A3 g ) = q + d / ( A0 + A1 a + A2 b + A3 c )

                    //  q' + s / ( columns removed combined with alphas ) = q + d / ( columns added combined with alphas )
                    // PROD * q' + s * ( columns added combined with alphas ) = PROD * q + d * ( columns removed combined with alphas )

                    // 1. Compute all the factors
                    let mut factors = vec![];
                    for column in columns.iter() {
                        let bus_op = column.as_bus_op().unwrap();
                        let bus_op_args = bus_op.args.clone();

                        // 1. Combine args with alphas
                        // 1.1 Start with the first alpha
                        let mut args_combined = Value::create(SpannedMirValue {
                            span: SourceSpan::default(),
                            value: MirValue::RandomValue(0),
                        });
                        for (index, arg) in bus_op_args.iter().enumerate() {
                            // 1.2 Create corresponding alpha
                            let alpha = Value::create(SpannedMirValue {
                                span: SourceSpan::default(),
                                value: MirValue::RandomValue(index + 1),
                            });

                            // 1.3 Multiply arg with alpha
                            let arg_times_alpha =
                                Mul::create(arg.clone(), alpha, SourceSpan::default());

                            // 1.4 Combine with other args
                            args_combined =
                                Add::create(args_combined, arg_times_alpha, SourceSpan::default());
                        }

                        factors.push(args_combined);
                    }

                    // 2. Compute the product of all factors (will be used to mult q and q')
                    let mut total_factors = Value::create(SpannedMirValue {
                        span: SourceSpan::default(),
                        value: MirValue::Constant(crate::ir::ConstantValue::Felt(1)),
                    });
                    for factor in factors.iter() {
                        total_factors =
                            Mul::create(total_factors, factor.clone(), SourceSpan::default());
                    }

                    // 3. For each column, compute the product of all factors except the one of the current column, and multiply it with the latch
                    let mut terms_added_to_bus = Value::create(SpannedMirValue {
                        span: SourceSpan::default(),
                        value: MirValue::Constant(crate::ir::ConstantValue::Felt(0)),
                    });
                    let mut terms_removed_from_bus = Value::create(SpannedMirValue {
                        span: SourceSpan::default(),
                        value: MirValue::Constant(crate::ir::ConstantValue::Felt(0)),
                    });
                    for (index, (column, latch)) in columns.iter().zip(latches.iter()).enumerate() {
                        let bus_op = column.as_bus_op().unwrap();
                        let bus_op_kind = bus_op.kind.clone();

                        // 3.1 Compute the product of all factors except the one of the current column
                        let mut factors_without_current = Value::create(SpannedMirValue {
                            span: SourceSpan::default(),
                            value: MirValue::Constant(crate::ir::ConstantValue::Felt(1)),
                        });
                        for (i, factor) in factors.iter().enumerate() {
                            if i != index {
                                factors_without_current = Mul::create(
                                    factors_without_current,
                                    factor.clone(),
                                    SourceSpan::default(),
                                );
                            }
                        }

                        // 3.2 Multiply by latch
                        let factors_without_current_with_latch = Mul::create(
                            factors_without_current,
                            latch.clone(),
                            SourceSpan::default(),
                        );

                        // 3.3 Depending on the bus_op_kind, add to q_factor or q_prime_factor
                        match bus_op_kind {
                            BusOpKind::Add => {
                                terms_added_to_bus = Add::create(
                                    terms_added_to_bus,
                                    factors_without_current_with_latch,
                                    SourceSpan::default(),
                                );
                            }
                            BusOpKind::Rem => {
                                terms_removed_from_bus = Add::create(
                                    terms_removed_from_bus,
                                    factors_without_current_with_latch,
                                    SourceSpan::default(),
                                );
                            }
                        }
                    }

                    // 4. Add all the terms together
                    let q_prod =
                        Mul::create(total_factors.clone(), bus_access, SourceSpan::default());
                    let q_prime_prod = Mul::create(
                        total_factors.clone(),
                        bus_access_with_offset,
                        SourceSpan::default(),
                    );
                    let q_term =
                        Add::create(q_prod, terms_added_to_bus.clone(), SourceSpan::default());
                    let q_prime_term = Add::create(
                        q_prime_prod,
                        terms_removed_from_bus.clone(),
                        SourceSpan::default(),
                    );

                    // 5. Create the resulting constraint and insert it into the graph
                    let resulting_constraint = Enf::create(
                        Sub::create(q_term, q_prime_term, SourceSpan::default()),
                        SourceSpan::default(),
                    );

                    graph.insert_integrity_constraints_root(resulting_constraint);
                }
            }
        }

        Ok(ir)
    }
}

impl<'a> BusOpExpand<'a> {
    #[allow(unused)]
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self { diagnostics }
    }

    fn handle_boundary_constraint(
        &self,
        bus_type: BusType,
        link: Link<Op>, /*, boundary: air_parser::ast::Boundary, bus_access: Link<Op>, bus_span: SourceSpan*/
    ) {
        let mut to_update = None;

        match link.borrow().deref() {
            Op::Value(value) => {
                match value.value.value {
                    // TODO: Will be used when handling variable-length public inputs
                    /*MirValue::PublicInputBinding(public_input_binding) => {

                    },*/
                    MirValue::Null => {
                        // Empty bus

                        let unit_constant = match bus_type {
                            BusType::Unit => 1, // Product, unit for product is 1
                            BusType::Mult => 0, // Sum of inverses, unit for sum is 0
                        };
                        let unit_val = Value::create(SpannedMirValue {
                            span: SourceSpan::default(),
                            value: MirValue::Constant(ConstantValue::Felt(unit_constant)),
                        });

                        to_update = Some(unit_val);

                        /*let bus_boundary = Boundary::create(
                            duplicate_node(bus_access.clone(), &mut Default::default()),
                            boundary,
                            bus_span,
                        );

                        let resulting_constraint = Enf::create(
                            Sub::create(bus_boundary, unit_val, SourceSpan::default()),
                            SourceSpan::default(),
                        );

                        //graph.insert_boundary_constraints_root(resulting_constraint);*/
                    }
                    _ => unreachable!(),
                }
            }
            Op::None(_) => {}
            _ => unreachable!(),
        }

        if let Some(to_update) = to_update {
            link.set(&to_update);
        }
    }
}

/*impl Visitor for BusOpExpand<'_> {
    fn work_stack(&mut self) -> &mut Vec<Link<Node>> {
        &mut self.work_stack
    }
    fn root_nodes_to_visit(
        &self,
        graph: &crate::ir::Graph,
    ) -> Vec<crate::ir::Link<crate::ir::Node>> {
        let boundary_constraints_roots_ref = graph.boundary_constraints_roots.borrow();
        let integrity_constraints_roots_ref = graph.integrity_constraints_roots.borrow();
        let combined_roots = boundary_constraints_roots_ref
            .clone()
            .into_iter()
            .map(|bc| bc.as_node())
            .chain(
                integrity_constraints_roots_ref
                    .clone()
                    .into_iter()
                    .map(|ic| ic.as_node()),
            );
        combined_roots.collect()
    }

    fn visit_node(&mut self, graph: &mut Graph, node: Link<Node>) -> Result<(), CompileError> {
        let updated_op: Result<Option<Link<Op>>, CompileError> = match node.borrow().deref() {
            Node::BusOp(bus_op) => {
                let bus_op_link: Link<Op> = bus_op.clone().into();
                let mut updated_node = None;

                {
                    // safe to unwrap because we just dispatched on it
                    let bus_op_ref = bus_op_link.as_bus_op().unwrap();
                    let bus = bus_op_ref.bus.clone();
                    let bus_kind = bus.borrow().bus_type.clone();
                    let bus_operator = bus_op_ref.kind.clone();
                    let args = bus_op_ref.args.clone();
                }

                Ok(updated_node)
            }
            _ => Ok(None),
        };

        // We update the node if needed
        if let Some(updated_op) = updated_op? {
            node.as_op().unwrap().set(&updated_op);
        }

        Ok(())
    }
}*/

use std::{collections::HashMap, ops::Deref};

use air_parser::{
    ast::{self, TraceSegment},
    SemanticAnalysisError,
};
use air_pass::Pass;

use miden_diagnostics::{DiagnosticsHandler, Severity, SourceSpan, Span, Spanned};
use mir::ir::{ConstantValue, Link, Mir, MirValue, Op, Parent, SpannedMirValue};

use crate::{graph::NodeIndex, ir::*, CompileError};

/// This pass creates the [Air] from the [Mir].
///  
/// We mainly directly transform Mir operations to Air operations,
/// as after the Inlining and Unrolling the nodes correspond 1 to 1.
pub struct MirToAir<'a> {
    diagnostics: &'a DiagnosticsHandler,
}
impl<'a> MirToAir<'a> {
    /// Create a new instance of this pass
    #[inline]
    pub fn new(diagnostics: &'a DiagnosticsHandler) -> Self {
        Self { diagnostics }
    }
}
impl Pass for MirToAir<'_> {
    type Input<'a> = Mir;
    type Output<'a> = Air;
    type Error = CompileError;

    fn run<'a>(&mut self, mir: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        let mut air = Air::new(mir.name);

        let buses = mir.constraint_graph().buses.clone();

        let mut trace_columns = mir.trace_columns.clone();

        let mut bus_bindings_map = HashMap::new();
        if !buses.is_empty() {
            let bus_raw_bindings: Vec<_> = buses
                .keys()
                .map(|k| Span::new(k.span(), (Identifier::new(k.span(), k.name()), AUX_SEGMENT)))
                .collect();

            // Add buses as `aux` trace columns
            let aux_trace_segment = TraceSegment::new(
                SourceSpan::default(),
                AUX_SEGMENT,
                Identifier::new(SourceSpan::default(), Symbol::new(AUX_SEGMENT as u32)),
                bus_raw_bindings,
            );
            for binding in aux_trace_segment.bindings.iter() {
                bus_bindings_map.insert(binding.name.unwrap(), binding.offset);
            }
            if trace_columns.len() == 1 {
                trace_columns.push(aux_trace_segment);
            } else {
                panic!(
                    "Expected only one trace segment, but found multiple: {:?}",
                    trace_columns
                );
            }
        }

        air.trace_segment_widths = trace_columns.iter().map(|ts| ts.size as u16).collect();
        air.num_random_values = mir.num_random_values;
        air.periodic_columns = mir.periodic_columns.clone();
        air.public_inputs = mir.public_inputs.clone();

        let mut builder = AirBuilder {
            diagnostics: self.diagnostics,
            air: &mut air,
            trace_columns: trace_columns.clone(),
            bus_bindings_map,
            buses: HashMap::new(),
        };

        let graph = mir.constraint_graph();

        for bc in graph.boundary_constraints_roots.borrow().deref().iter() {
            builder.build_boundary_constraint(bc)?;
        }

        for ic in graph.integrity_constraints_roots.borrow().deref().iter() {
            builder.build_integrity_constraint(ic)?;
        }

        for bus in buses.values() {
            builder.build_bus(bus)?;
        }

        Ok(air)
    }
}

struct AirBuilder<'a> {
    diagnostics: &'a DiagnosticsHandler,
    air: &'a mut Air,
    trace_columns: Vec<TraceSegment>,
    bus_bindings_map: HashMap<Identifier, usize>,
    buses: HashMap<Identifier, Bus>,
}

/// Helper function to remove the vector wrapper from a scalar operation
/// Will panic if the node is a vector of size > 1 (should not happen after unrolling)
fn vec_to_scalar(mir_node: &Link<Op>) -> Link<Op> {
    if let Some(vector) = mir_node.as_vector() {
        let size = vector.size;
        let children = vector.elements.borrow().deref().clone();
        if size != 1 {
            panic!("Vector of len >1 after unrolling");
        }
        let child = children.first().unwrap();
        let child = vec_to_scalar(child);
        child.clone()
    } else {
        mir_node.clone()
    }
}

/// Helper function to remove the enf wrapper from a scalar operation
fn enf_to_scalar(mir_node: &Link<Op>) -> Link<Op> {
    if let Some(enf) = mir_node.as_enf() {
        let child = enf.expr.clone();
        let child = enf_to_scalar(&child);
        child.clone()
    } else {
        mir_node.clone()
    }
}

impl AirBuilder<'_> {
    // Uses square and multiply algorithm to expand the exp into a series of multiplications
    fn expand_exp(&mut self, lhs: NodeIndex, rhs: u64) -> NodeIndex {
        match rhs {
            0 => self.insert_op(Operation::Value(Value::Constant(1))),
            1 => lhs,
            n if n % 2 == 0 => {
                let square = self.insert_op(Operation::Mul(lhs, lhs));
                self.expand_exp(square, n / 2)
            }
            n => {
                let square = self.insert_op(Operation::Mul(lhs, lhs));
                let rec = self.expand_exp(square, (n - 1) / 2);
                self.insert_op(Operation::Mul(lhs, rec))
            }
        }
    }

    /// Recursively insert the MIR operations into the AIR graph
    /// Will panic when encountering an unexpected operation
    /// (i.e. that is not a binary operation, a value, enf node or an accessor)
    fn insert_mir_operation(&mut self, mir_node: &Link<Op>) -> Result<NodeIndex, CompileError> {
        let mir_node = vec_to_scalar(mir_node);
        let mir_node_ref = mir_node.borrow();
        match mir_node_ref.deref() {
            Op::Add(add) => {
                let lhs = add.lhs.clone();
                let rhs = add.rhs.clone();
                let lhs_node_index = self.insert_mir_operation(&lhs)?;
                let rhs_node_index = self.insert_mir_operation(&rhs)?;
                Ok(self.insert_op(Operation::Add(lhs_node_index, rhs_node_index)))
            }
            Op::Sub(sub) => {
                let lhs = sub.lhs.clone();
                let rhs = sub.rhs.clone();
                let lhs_node_index = self.insert_mir_operation(&lhs)?;
                let rhs_node_index = self.insert_mir_operation(&rhs)?;
                Ok(self.insert_op(Operation::Sub(lhs_node_index, rhs_node_index)))
            }
            Op::Mul(mul) => {
                let lhs = mul.lhs.clone();
                let rhs = mul.rhs.clone();
                let lhs_node_index = self.insert_mir_operation(&lhs)?;
                let rhs_node_index = self.insert_mir_operation(&rhs)?;
                Ok(self.insert_op(Operation::Mul(lhs_node_index, rhs_node_index)))
            }
            Op::Exp(exp) => {
                let lhs = exp.lhs.clone();
                let rhs = exp.rhs.clone();

                let lhs_node_index = self.insert_mir_operation(&lhs)?;

                // Remove the accessor for rhs if it exists
                let rhs = match rhs.borrow().deref() {
                    Op::Accessor(accessor) => accessor.indexable.clone(),
                    _ => rhs.clone(),
                };

                let Some(value_ref) = rhs.as_value() else {
                    return Err(CompileError::SemanticAnalysis(
                        SemanticAnalysisError::InvalidExpr(
                            ast::InvalidExprError::NonConstantExponent(rhs.span()),
                        ),
                    ));
                };

                let mir_value = value_ref.value.value.clone();

                let MirValue::Constant(constant_value) = mir_value else {
                    return Err(CompileError::SemanticAnalysis(
                        SemanticAnalysisError::InvalidExpr(
                            ast::InvalidExprError::NonConstantExponent(rhs.span()),
                        ),
                    ));
                };

                let ConstantValue::Felt(rhs_value) = constant_value else {
                    return Err(CompileError::SemanticAnalysis(
                        SemanticAnalysisError::InvalidExpr(
                            ast::InvalidExprError::NonConstantExponent(rhs.span()),
                        ),
                    ));
                };

                Ok(self.expand_exp(lhs_node_index, rhs_value))
            }
            Op::Value(value) => {
                let mir_value = &value.value.value;

                let value = match mir_value {
                    MirValue::Constant(constant_value) => {
                        if let ConstantValue::Felt(felt) = constant_value {
                            crate::ir::Value::Constant(*felt)
                        } else {
                            unreachable!()
                        }
                    }
                    MirValue::TraceAccess(trace_access) => {
                        crate::ir::Value::TraceAccess(crate::ir::TraceAccess {
                            segment: trace_access.segment,
                            column: trace_access.column,
                            row_offset: trace_access.row_offset,
                        })
                    }
                    MirValue::BusAccess(bus_access) => {
                        let name = bus_access.bus.borrow().deref().name.unwrap();
                        let column = self.bus_bindings_map.get(&name).unwrap();
                        crate::ir::Value::TraceAccess(crate::ir::TraceAccess {
                            segment: AUX_SEGMENT,
                            column: *column,
                            row_offset: bus_access.row_offset,
                        })
                    }
                    MirValue::PeriodicColumn(periodic_column_access) => {
                        crate::ir::Value::PeriodicColumn(crate::ir::PeriodicColumnAccess {
                            name: periodic_column_access.name,
                            cycle: periodic_column_access.cycle,
                        })
                    }
                    MirValue::PublicInput(public_input_access) => {
                        crate::ir::Value::PublicInput(crate::ir::PublicInputAccess {
                            name: public_input_access.name,
                            index: public_input_access.index,
                        })
                    }
                    // TODO: Add support for PublicInputBinding of [ast::Table]
                    MirValue::PublicInputBinding(public_input_binding) => {
                        crate::ir::Value::PublicInputBinding(crate::ir::BusVariableBoundary {
                            name: public_input_binding.name,
                            num_cols: public_input_binding.num_cols,
                        })
                    }
                    MirValue::RandomValue(rv) => crate::ir::Value::RandomValue(*rv),
                    _ => unreachable!("Unexpected MirValue: {:#?}", mir_value),
                };

                Ok(self.insert_op(Operation::Value(value)))
            }
            Op::Enf(enf) => {
                let child = enf.expr.clone();
                self.insert_mir_operation(&child)
            }
            Op::Accessor(accessor) => {
                let offset = accessor.offset;
                let child = accessor.indexable.clone();

                let Some(value) = child.as_value() else {
                    unreachable!();
                };

                let mir_value = &value.value.value;

                let value = match mir_value {
                    MirValue::Constant(constant_value) => {
                        if let ConstantValue::Felt(felt) = constant_value {
                            crate::ir::Value::Constant(*felt)
                        } else {
                            unreachable!()
                        }
                    }
                    MirValue::TraceAccess(trace_access) => {
                        crate::ir::Value::TraceAccess(crate::ir::TraceAccess {
                            segment: trace_access.segment,
                            column: trace_access.column,
                            row_offset: offset,
                        })
                    }
                    MirValue::BusAccess(bus_access) => {
                        let name = bus_access.bus.borrow().deref().name.unwrap();
                        let column = self.bus_bindings_map.get(&name).unwrap();
                        crate::ir::Value::TraceAccess(crate::ir::TraceAccess {
                            segment: AUX_SEGMENT,
                            column: *column,
                            row_offset: offset,
                        })
                    }
                    MirValue::PeriodicColumn(periodic_column_access) => {
                        crate::ir::Value::PeriodicColumn(crate::ir::PeriodicColumnAccess {
                            name: periodic_column_access.name,
                            cycle: periodic_column_access.cycle,
                        })
                    }
                    MirValue::PublicInput(public_input_access) => {
                        crate::ir::Value::PublicInput(crate::ir::PublicInputAccess {
                            name: public_input_access.name,
                            index: public_input_access.index,
                        })
                    }
                    MirValue::RandomValue(rv) => crate::ir::Value::RandomValue(*rv),
                    _ => unreachable!(),
                };

                Ok(self.insert_op(Operation::Value(value)))
            }
            _ => panic!("Should not have Mir op in graph: {:?}", mir_node),
        }
    }

    fn build_boundary_constraint(&mut self, bc: &Link<Op>) -> Result<(), CompileError> {
        match bc.borrow().deref() {
            Op::Vector(vector) => {
                let vec = vector.elements.borrow().deref().clone();
                for node in vec.iter() {
                    self.build_boundary_constraint(node)?;
                }
                Ok(())
            }
            Op::Matrix(matrix) => {
                let rows = matrix.elements.borrow().deref().clone();
                for row in rows.iter() {
                    let vec = row.borrow().deref().children().borrow().deref().clone();
                    for node in vec.iter() {
                        self.build_boundary_constraint(node)?;
                    }
                }
                Ok(())
            }
            Op::Enf(enf) => {
                let child_op = enf.expr.clone();
                let child_op = vec_to_scalar(&child_op);

                self.build_boundary_constraint(&child_op)?;
                Ok(())
            }
            Op::Sub(sub) => {
                // Check that lhs is a Bounded trace access
                let lhs = sub.lhs.clone();
                let lhs = vec_to_scalar(&lhs);
                let rhs = sub.rhs.clone();
                let rhs = vec_to_scalar(&rhs);
                let lhs_span = lhs.span();
                let rhs_span = rhs.span();

                let boundary = lhs.as_boundary().unwrap().clone();

                let expected_trace_access_expr = boundary.expr.clone();
                let Op::Value(value) = expected_trace_access_expr.borrow().deref().clone() else {
                    unreachable!(); // Raise diag
                };

                let (trace_access, _) = match value.value.clone() {
                    SpannedMirValue {
                        value: MirValue::TraceAccess(trace_access),
                        span: lhs_span,
                    } => (trace_access, lhs_span),
                    SpannedMirValue {
                        value: MirValue::TraceAccessBinding(trace_access_binding),
                        span: lhs_span,
                    } => {
                        if trace_access_binding.size != 1 {
                            self.diagnostics.diagnostic(Severity::Error)
                                        .with_message("invalid boundary constraint")
                                        .with_primary_label(lhs_span, "this has a trace access binding with a size greater than 1")
                                        .with_note("Boundary constraints require both sides of the constraint to be single columns.")
                                        .emit();
                            return Err(CompileError::Failed);
                        }
                        let trace_access = mir::ir::TraceAccess {
                            segment: trace_access_binding.segment,
                            column: trace_access_binding.offset,
                            row_offset: 0,
                        };
                        (trace_access, lhs_span)
                    }
                    SpannedMirValue {
                        value: MirValue::BusAccess(bus_access),
                        span: lhs_span,
                    } => {
                        let bus = bus_access.bus;
                        let name = bus.borrow().deref().name.unwrap();
                        let column = self.bus_bindings_map.get(&name).unwrap();
                        let trace_access =
                            mir::ir::TraceAccess::new(AUX_SEGMENT, *column, bus_access.row_offset);
                        (trace_access, lhs_span)
                    }
                    _ => unreachable!(
                        "Expected TraceAccess or BusAccess, received {:?}",
                        value.value
                    ), // Raise diag
                };

                if let Some(prev) = self.trace_columns[trace_access.segment].mark_constrained(
                    lhs_span,
                    trace_access.column,
                    boundary.kind,
                ) {
                    self.diagnostics
                        .diagnostic(Severity::Error)
                        .with_message("overlapping boundary constraints")
                        .with_primary_label(
                            lhs_span,
                            "this constrains a column and boundary that has already been constrained",
                        )
                        .with_secondary_label(prev, "previous constraint occurs here")
                        .emit();
                    return Err(CompileError::Failed);
                }

                let lhs = self
                    .air
                    .constraint_graph_mut()
                    .insert_node(Operation::Value(crate::ir::Value::TraceAccess(
                        crate::ir::TraceAccess {
                            segment: trace_access.segment,
                            column: trace_access.column,
                            row_offset: trace_access.row_offset,
                        },
                    )));
                let rhs = self.insert_mir_operation(&rhs)?;

                // Compare the inferred trace segment and domain of the operands
                let domain = boundary.kind.into();
                {
                    let graph = self.air.constraint_graph();
                    let (lhs_segment, lhs_domain) = graph.node_details(&lhs, domain)?;
                    let (rhs_segment, rhs_domain) = graph.node_details(&rhs, domain)?;
                    if lhs_segment < rhs_segment {
                        // trace segment inference defaults to the lowest segment (the main trace) and is
                        // adjusted according to the use of random values and trace columns.
                        let lhs_segment_name = self.trace_columns[lhs_segment].name;
                        let rhs_segment_name = self.trace_columns[rhs_segment].name;
                        self.diagnostics.diagnostic(Severity::Error)
                                    .with_message("invalid boundary constraint")
                                    .with_primary_label(lhs_span, format!("this constrains a column in the '{lhs_segment_name}' trace segment"))
                                    .with_secondary_label(rhs_span, format!("but this expression implies the '{rhs_segment_name}' trace segment"))
                                    .with_note("Boundary constraints require both sides of the constraint to apply to the same trace segment.")
                                    .emit();
                        return Err(CompileError::Failed);
                    }
                    if lhs_domain != rhs_domain {
                        self.diagnostics.diagnostic(Severity::Error)
                                    .with_message("invalid boundary constraint")
                                    .with_primary_label(lhs_span, format!("this has a constraint domain of {lhs_domain}"))
                                    .with_secondary_label(rhs_span, format!("this has a constraint domain of {rhs_domain}"))
                                    .with_note("Boundary constraints require both sides of the constraint to be in the same domain.")
                                    .emit();
                        return Err(CompileError::Failed);
                    }
                }

                // Merge the expressions into a single constraint
                let root = self.insert_op(Operation::Sub(lhs, rhs));

                // Store the generated constraint
                self.air
                    .constraints
                    .insert_constraint(trace_access.segment, root, domain);
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn build_integrity_constraint(&mut self, ic: &Link<Op>) -> Result<(), CompileError> {
        match ic.borrow().deref() {
            Op::Vector(vector) => {
                let vec = vector.children().borrow().deref().clone();
                for node in vec.iter() {
                    self.build_integrity_constraint(node)?;
                }
            }
            Op::Matrix(matrix) => {
                let rows = matrix.elements.borrow().deref().clone();
                for row in rows.iter() {
                    let vec = row.borrow().deref().children().borrow().deref().clone();
                    for node in vec.iter() {
                        self.build_integrity_constraint(node)?;
                    }
                }
            }
            Op::Enf(enf) => {
                let child_op = enf.expr.clone();
                let child_op = vec_to_scalar(&child_op);
                let child_op = enf_to_scalar(&child_op);
                match child_op.clone().borrow().deref() {
                    Op::Sub(_sub) => {
                        self.build_integrity_constraint(&child_op)?;
                    }
                    _ => unreachable!("Enforced with unexpected operation: {:?}", child_op),
                }
            }
            Op::Sub(sub) => {
                let lhs = sub.lhs.clone();
                let rhs = sub.rhs.clone();
                let lhs_node_index = self.insert_mir_operation(&lhs)?;
                let rhs_node_index = self.insert_mir_operation(&rhs)?;
                let root = self.insert_op(Operation::Sub(lhs_node_index, rhs_node_index));
                let (trace_segment, domain) = self
                    .air
                    .constraint_graph()
                    .node_details(&root, ConstraintDomain::EveryRow)?;
                self.air
                    .constraints
                    .insert_constraint(trace_segment, root, domain);
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn build_bus(&mut self, mir_bus: &Link<mir::ir::Bus>) -> Result<(), CompileError> {
        let mir_bus = mir_bus.borrow();
        eprintln!("bus: {:#?}", mir_bus);
        let first = self.insert_mir_operation(&mir_bus.get_first())?;
        eprintln!("first: {:#?}", first);
        let last = self.insert_mir_operation(&mir_bus.get_last())?;
        eprintln!("last: {:#?}", last);
        self.buses.insert(
            mir_bus.name.unwrap(),
            Bus::new(mir_bus.name, mir_bus.bus_type, first, last),
        );
        Ok(())
    }

    /// Adds the specified operation to the graph and returns the index of its node.
    #[inline]
    fn insert_op(&mut self, op: Operation) -> NodeIndex {
        self.air.constraint_graph_mut().insert_node(op)
    }
}

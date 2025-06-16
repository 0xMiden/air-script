use std::{collections::BTreeMap, ops::Deref};

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

        let mut bus_bindings_map = BTreeMap::new();
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
        };

        let graph = mir.constraint_graph();

        for bus in buses.values() {
            builder.build_bus(bus)?;
        }

        for bc in graph.boundary_constraints_roots.borrow().deref().iter() {
            builder.build_boundary_constraint(bc)?;
        }

        for ic in graph.integrity_constraints_roots.borrow().deref().iter() {
            builder.build_integrity_constraint(ic)?;
        }
        Ok(air)
    }
}

struct AirBuilder<'a> {
    diagnostics: &'a DiagnosticsHandler,
    air: &'a mut Air,
    trace_columns: Vec<TraceSegment>,
    bus_bindings_map: BTreeMap<Identifier, usize>,
}

/// In case of nested list comprehension, we may not have entirely unrolled outer loops iterators
/// so we need to ensure these cases are properly indexed.
fn indexed_accessor(mir_node: &Link<Op>) -> Link<Op> {
    if let Some(accessor) = mir_node.as_accessor() {
        if let AccessType::Index(index) = accessor.access_type {
            if let Some(vec) = accessor.indexable.as_vector() {
                let children = vec.elements.borrow().deref().clone();
                if index >= children.len() {
                    panic!("Index out of bounds during indexed accessor translation from MIR to AIR: {}", index);
                }
                children[index].clone()
            } else {
                mir_node.clone()
            }
        } else {
            mir_node.clone()
        }
    } else {
        mir_node.clone()
    }
}

/// Helper function to remove the vector wrapper from a scalar operation
/// Will panic if the node is a vector of size > 1 (should not happen after unrolling)
fn vec_to_scalar(mir_node: &Link<Op>) -> Link<Op> {
    if let Some(vector) = mir_node.as_vector() {
        let size = vector.size;
        let children = vector.elements.borrow().deref().clone();
        if size != 1 {
            panic!("Vector of len >1 after unrolling: {:?}", mir_node);
        }
        let child = children.first().unwrap();
        let child = indexed_accessor(child);
        let child = vec_to_scalar(&child);
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
        let mir_node = indexed_accessor(mir_node);
        let mir_node = vec_to_scalar(&mir_node);
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
                        let name = bus_access.bus.borrow().deref().name();
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
                let child = indexed_accessor(&child);

                let Some(value) = child.as_value() else {
                    unreachable!("Expected value in accessor, found: {:?}", child);
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
                        let name = bus_access.bus.borrow().deref().name();
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
                let child_op = indexed_accessor(&child_op);
                let child_op = vec_to_scalar(&child_op);

                self.build_boundary_constraint(&child_op)?;
                Ok(())
            }
            Op::Sub(sub) => {
                // Check that lhs is a Bounded trace access
                let lhs = sub.lhs.clone();
                let lhs = indexed_accessor(&lhs);
                let lhs = vec_to_scalar(&lhs);
                let rhs = sub.rhs.clone();
                let rhs = indexed_accessor(&rhs);
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
                        let name = bus.borrow().deref().name();
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
                let child_op = indexed_accessor(&child_op);
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

    /// Builds the bus struct, containing the bus operations and boundaries.
    fn build_bus(&mut self, mir_bus: &Link<mir::ir::Bus>) -> Result<(), CompileError> {
        let mir_bus = mir_bus.borrow();

        let first = build_bus_boundary(&mir_bus.get_first())?;
        let last = build_bus_boundary(&mir_bus.get_last())?;

        let mut bus_ops = vec![];
        for (mir_column, mir_latch) in mir_bus.columns.iter().zip(mir_bus.latches.iter()) {
            let mut column = vec![];

            // Note: we have checked this will not panic in the MIR pass
            let mir_bus_op = mir_column
                .as_bus_op()
                .expect("Bus column should be a bus operation");
            let mir_bus_op_args = mir_bus_op.args.clone();
            for arg in mir_bus_op_args.iter() {
                let arg = self.insert_mir_operation(arg)?;
                column.push(arg);
            }
            let latch = self.insert_mir_operation(mir_latch)?;

            let bus_op = BusOp::new(column, latch, mir_bus_op.kind);
            bus_ops.push(bus_op);
        }
        self.air.buses.insert(
            mir_bus.name(),
            Bus::new(mir_bus.name(), mir_bus.bus_type, first, last, bus_ops),
        );
        Ok(())
    }

    /// Adds the specified operation to the graph and returns the index of its node.
    #[inline]
    fn insert_op(&mut self, op: Operation) -> NodeIndex {
        self.air.constraint_graph_mut().insert_node(op)
    }
}

// HELPERS FUNCTIONS
// ================================================================================================

/// Helper function to convert a MIR bus boundary node into an AIR bus boundary.
fn build_bus_boundary(mir_bus_boundary_node: &Link<Op>) -> Result<BusBoundary, CompileError> {
    let mir_node = vec_to_scalar(mir_bus_boundary_node);
    let mir_node_ref = mir_node.borrow();
    match mir_node_ref.deref() {
        Op::Value(value) => match &value.value.value {
            // This represents public input table boundary
            MirValue::PublicInputTable(public_input_table) => Ok(
                crate::ir::BusBoundary::PublicInputTable(crate::ir::PublicInputTableAccess::new(
                    public_input_table.table_name,
                    public_input_table.bus_name(),
                    public_input_table.num_cols,
                )),
            ),
            // This represents an empty bus
            MirValue::Null => Ok(crate::ir::BusBoundary::Null),
            _ => Err(CompileError::Failed),
        },
        Op::None(_) => {
            // This is an unconstrained bus boundary
            Ok(crate::ir::BusBoundary::Unconstrained)
        }
        _ => unreachable!("Unexpected Mir Op in bus boundary: {:#?}", mir_node_ref),
    }
}

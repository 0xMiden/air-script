use air_ir::{
    Air, BusType, Identifier, IntegrityConstraintDegree, NodeIndex, Operation, TraceAccess,
    TraceSegmentId, Value,
};

use super::ElemType;

// RUST STRING GENERATION FOR THE CONSTRAINT GRAPH
// ================================================================================================

/// Code generation trait for generating Rust code strings from IR types related to constraints and
/// the [AlgebraicGraph].
/// TODO: replace panics with errors
pub trait Codegen {
    fn to_string(&self, ir: &Air, elem_type: ElemType, trace_segment: TraceSegmentId) -> String;
}

impl Codegen for IntegrityConstraintDegree {
    fn to_string(&self, _ir: &Air, _elem_type: ElemType, _trace_segment: TraceSegmentId) -> String {
        if self.cycles().is_empty() {
            format!("TransitionConstraintDegree::new({})", self.base())
        } else {
            let cycles = self
                .cycles()
                .iter()
                .map(|cycle_len| cycle_len.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            format!(
                "TransitionConstraintDegree::with_cycles({}, vec![{}])",
                self.base(),
                cycles
            )
        }
    }
}

impl Codegen for TraceAccess {
    fn to_string(&self, _ir: &Air, _elem_type: ElemType, trace_segment: TraceSegmentId) -> String {
        let frame = if self.segment == 0 { "main" } else { "aux" };
        let row_offset = match self.row_offset {
            0 => {
                format!("current[{}]", self.column)
            }
            1 => {
                format!("next[{}]", self.column)
            }
            _ => panic!("Winterfell doesn't support row offsets greater than 1."),
        };
        if self.segment == 0 && self.segment != trace_segment {
            format!("E::from({frame}_{row_offset})")
        } else {
            format!("{frame}_{row_offset}")
        }
    }
}

impl Codegen for NodeIndex {
    fn to_string(&self, ir: &Air, elem_type: ElemType, trace_segment: TraceSegmentId) -> String {
        let op = ir.constraint_graph().node(self).op();
        op.to_string(ir, elem_type, trace_segment)
    }
}

impl Codegen for Operation {
    fn to_string(&self, ir: &Air, elem_type: ElemType, trace_segment: TraceSegmentId) -> String {
        match self {
            Operation::Value(value) => value.to_string(ir, elem_type, trace_segment),
            Operation::Add(_, _) => binary_op_to_string(ir, self, elem_type, trace_segment),
            Operation::Sub(_, _) => binary_op_to_string(ir, self, elem_type, trace_segment),
            Operation::Mul(_, _) => binary_op_to_string(ir, self, elem_type, trace_segment),
        }
    }
}

impl Codegen for Value {
    fn to_string(&self, ir: &Air, elem_type: ElemType, trace_segment: TraceSegmentId) -> String {
        match self {
            // TODO: move constant handling to a helper function
            Value::Constant(0) => match elem_type {
                ElemType::Base => "Felt::ZERO".to_string(),
                ElemType::Ext => "E::ZERO".to_string(),
            },
            Value::Constant(1) => match elem_type {
                ElemType::Base => "Felt::ONE".to_string(),
                ElemType::Ext => "E::ONE".to_string(),
            },
            Value::Constant(value) => match elem_type {
                ElemType::Base => format!("Felt::new({value})"),
                ElemType::Ext => format!("E::from({value}_u64)"),
            },
            Value::TraceAccess(trace_access) => {
                trace_access.to_string(ir, elem_type, trace_segment)
            }
            Value::PeriodicColumn(pc) => {
                let index = ir
                    .periodic_columns
                    .iter()
                    .position(|(qid, _)| qid == &pc.name)
                    .unwrap();
                format!("periodic_values[{index}]")
            }
            Value::PublicInput(air_ir::PublicInputAccess { name, index }) => {
                format!("self.{name}[{index}]")
            }
            Value::PublicInputTable(air_ir::PublicInputTableAccess {
                bus_name,
                table_name,
                ..
            }) => call_bus_boundary_varlen_pubinput(ir, *bus_name, *table_name),
            Value::RandomValue(idx) => {
                format!("aux_rand_elements.get_segment_elements(0)[{idx}]")
            }
        }
    }
}

/// Returns a string representation of a binary operation.
fn binary_op_to_string(
    ir: &Air,
    op: &Operation,
    elem_type: ElemType,
    trace_segment: TraceSegmentId,
) -> String {
    match op {
        Operation::Add(l_idx, r_idx) => {
            let lhs = l_idx.to_string(ir, elem_type, trace_segment);
            let rhs = r_idx.to_string(ir, elem_type, trace_segment);
            format!("{lhs} + {rhs}")
        }
        Operation::Sub(l_idx, r_idx) => {
            let lhs = l_idx.to_string(ir, elem_type, trace_segment);
            let rhs = if ir.constraint_graph().node(r_idx).op().precedence() <= op.precedence() {
                format!("({})", r_idx.to_string(ir, elem_type, trace_segment))
            } else {
                r_idx.to_string(ir, elem_type, trace_segment)
            };
            format!("{lhs} - {rhs}")
        }
        Operation::Mul(l_idx, r_idx) => {
            let lhs = if ir.constraint_graph().node(l_idx).op().precedence() < op.precedence() {
                format!("({})", l_idx.to_string(ir, elem_type, trace_segment))
            } else {
                l_idx.to_string(ir, elem_type, trace_segment)
            };
            let rhs = if ir.constraint_graph().node(r_idx).op().precedence() < op.precedence() {
                format!("({})", r_idx.to_string(ir, elem_type, trace_segment))
            } else {
                r_idx.to_string(ir, elem_type, trace_segment)
            };
            format!("{lhs} * {rhs}")
        }
        _ => panic!("unsupported operation"),
    }
}

fn call_bus_boundary_varlen_pubinput(
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

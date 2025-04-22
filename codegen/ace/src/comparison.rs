use air_ir::{Operation, Value};
use std::cmp::Ordering;

fn compare_value(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Constant(a), Value::Constant(b)) => a.cmp(b),
        _ => unreachable!(""),
    }
}

/// Values, which can only be constants, should be first.
fn value_operation(a: &Operation) -> usize {
    match a {
        Operation::Value(_) => 0,
        Operation::Add(_, _) | Operation::Sub(_, _) | Operation::Mul(_, _) => 1,
    }
}

/// Compare function used to sort the operations as needed by the ace chiplet.
pub fn compare_operation(a: &Operation, b: &Operation) -> Ordering {
    let cmp = value_operation(a).cmp(&value_operation(b));
    if cmp == Ordering::Equal {
        match (a, b) {
            (Operation::Value(a), Operation::Value(b)) => compare_value(a, b),
            _ => Ordering::Equal,
        }
    } else {
        cmp
    }
}

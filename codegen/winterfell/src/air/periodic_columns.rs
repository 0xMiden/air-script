use std::collections::BTreeMap;

use air_ir::{Air, PeriodicColumn, QualifiedIdentifier};

use super::Impl;

pub(super) fn add_fn_get_periodic_column_values(impl_ref: &mut Impl, ir: &Air) {
    // define the function.
    let get_periodic_column_values = impl_ref
        .new_fn("get_periodic_column_values")
        .arg_ref_self()
        .ret("Vec<Vec<Felt>>");

    // output the periodic columns.
    let periodic_columns = &ir.periodic_columns;
    get_periodic_column_values.line(periodic_columns.codegen());
}

/// Code generation trait for generating Rust code strings from Periodic Columns.
trait Codegen {
    fn codegen(&self) -> String;
}

impl Codegen for &BTreeMap<QualifiedIdentifier, PeriodicColumn> {
    fn codegen(&self) -> String {
        let mut columns = vec![];
        for column in self.values() {
            let mut rows = vec![];
            for row in column.values.iter().copied() {
                match row {
                    0 => {
                        rows.push("Felt::ZERO".to_string());
                    },
                    1 => {
                        rows.push("Felt::ONE".to_string());
                    },
                    row => {
                        rows.push(format!("Felt::new({row})"));
                    },
                }
            }
            columns.push(format!("vec![{}]", rows.join(", ")));
        }
        format!("vec![{}]", columns.join(", "))
    }
}

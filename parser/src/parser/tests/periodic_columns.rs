use miden_diagnostics::SourceSpan;

use crate::ast::*;

use super::ParseTest;

#[test]
fn periodic_columns() {
    let source = "
    mod test

    periodic_columns {
        k0: [1, 0, 0, 0],
        k1: [0, 0, 0, 0, 0, 0, 0, 1],
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.periodic_columns.insert(
        ident!(k0),
        PeriodicColumn::new(SourceSpan::UNKNOWN, ident!(k0), vec![1, 0, 0, 0]),
    );
    expected.periodic_columns.insert(
        ident!(k1),
        PeriodicColumn::new(
            SourceSpan::UNKNOWN,
            ident!(k1),
            vec![0, 0, 0, 0, 0, 0, 0, 1],
        ),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn empty_periodic_columns() {
    let source = "
    mod test

    periodic_columns{}";

    let expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn err_periodic_columns_length() {
    let source = "
    mod test

    periodic_columns {
        k0: [1, 0, 0],
    }";

    ParseTest::new().expect_module_diagnostic(
        source,
        "periodic columns must have a non-zero cycle length which is a power of two",
    );
}

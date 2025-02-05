use miden_diagnostics::SourceSpan;

use crate::ast::*;

use super::ParseTest;

#[test]
fn buses() {
    let source = "
    mod test

    buses {
        unit p,
        mult q,
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.buses.insert(
        ident!(p),
        Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Unit),
    );
    expected.buses.insert(
        ident!(q),
        Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Mult),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn empty_buses() {
    let source = "
    mod test

    buses{}";

    let expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    ParseTest::new().expect_module_ast(source, expected);
}

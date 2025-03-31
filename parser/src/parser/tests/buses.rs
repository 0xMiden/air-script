use miden_diagnostics::SourceSpan;

use crate::ast::*;

use super::ParseTest;

#[test]
fn buses() {
    let source = "
    mod test

    buses {
        multiset p,
        logup q,
    }";

    let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.buses.insert(
        ident!(p),
        Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset),
    );
    expected.buses.insert(
        ident!(q),
        Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup),
    );
    ParseTest::new().expect_module_ast(source, expected);
}

#[test]
fn boundary_constraints_buses() {
    let _source = "
    mod test

    buses {
        multiset p,
        logup q,
    }
    
    boundary_constraints {
        p.first = null;
        q.last = null;
    }";

    /*let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.buses.insert(
        ident!(p),
        Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset),
    );
    expected.buses.insert(
        ident!(q),
        Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup),
    );
    ParseTest::new().expect_module_ast(source, expected);*/
}

#[test]
fn integrity_constraints_buses() {
    let _source = "
    mod test

    buses {
        multiset p,
        logup q,
    }
    
    integrity_constraints {
        p.add(1) when 1;
        p.rem(1) when 1;
        q.add(1, 2) when 1;
        q.add(1, 2) when 1;
        q.rem(1, 2) for 2;
    }";

    /*let mut expected = Module::new(ModuleType::Library, SourceSpan::UNKNOWN, ident!(test));
    expected.buses.insert(
        ident!(p),
        Bus::new(SourceSpan::UNKNOWN, ident!(p), BusType::Multiset),
    );
    expected.buses.insert(
        ident!(q),
        Bus::new(SourceSpan::UNKNOWN, ident!(q), BusType::Logup),
    );
    ParseTest::new().expect_module_ast(source, expected);*/
}

#[test]
fn err_empty_buses() {
    let source = "
    mod test

    buses{}";

    ParseTest::new().expect_module_diagnostic(source, "expected one of: '\"logup\"', '\"multiset\"'");
}

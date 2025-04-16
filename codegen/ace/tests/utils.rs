use air_codegen_ace::Circuit;
use air_ir::NodeIndex;
use miden_diagnostics::{
    term::termcolor::ColorChoice, CodeMap, DefaultEmitter, DiagnosticsHandler,
};
use std::sync::Arc;

/// Generate an ACE circuit and its root index from an AirScript program
pub fn codegen(source: &str) -> (NodeIndex, Circuit, String) {
    use air_pass::Pass;

    let codemap = Arc::new(CodeMap::new());
    let emitter = Arc::new(DefaultEmitter::new(ColorChoice::Auto));
    let diagnostics = DiagnosticsHandler::new(Default::default(), codemap.clone(), emitter);

    let air = air_parser::parse(&diagnostics, codemap, source)
        .map_err(air_ir::CompileError::Parse)
        .and_then(|ast| {
            let mut pipeline = air_parser::transforms::ConstantPropagation::new(&diagnostics)
                .chain(air_parser::transforms::Inlining::new(&diagnostics))
                .chain(air_ir::passes::AstToAir::new(&diagnostics));
            pipeline.run(ast)
        })
        .expect("lowering failed");
    let (root, circuit) = air_codegen_ace::build_ace_circuit(&air).expect("codegen failed");
    let name = air.name().to_string();
    (root, circuit, name)
}

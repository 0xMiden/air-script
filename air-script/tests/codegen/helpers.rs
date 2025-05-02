use std::sync::Arc;

use air_ir::{CodeGenerator, CompileError};
use air_pass::Pass;
use miden_diagnostics::{
    term::termcolor::ColorChoice, CodeMap, DefaultEmitter, DiagnosticsHandler,
};

pub enum Target {
    Winterfell,
}
pub enum Pipeline {
    WithMIR,
    WithoutMIR,
}

pub struct Test {
    input_path: String,
}
impl Test {
    pub fn new(input_path: String) -> Self {
        Test { input_path }
    }

    pub fn transpile(&self, target: Target, pipeline: Pipeline) -> Result<String, CompileError> {
        let codemap = Arc::new(CodeMap::new());
        let emitter = Arc::new(DefaultEmitter::new(ColorChoice::Auto));
        let diagnostics = DiagnosticsHandler::new(Default::default(), codemap.clone(), emitter);

        // Parse from file to internal representation
        let air = match pipeline {
            Pipeline::WithMIR => air_parser::parse_file(&diagnostics, codemap, &self.input_path)
                .map_err(CompileError::Parse)
                .and_then(|ast| {
                    let mut pipeline =
                        air_parser::transforms::ConstantPropagation::new(&diagnostics)
                            .chain(mir::passes::AstToMir::new(&diagnostics))
                            .chain(mir::passes::Inlining::new(&diagnostics))
                            .chain(mir::passes::Unrolling::new(&diagnostics))
                            .chain(mir::passes::BusOpExpand::new(&diagnostics))
                            .chain(air_ir::passes::MirToAir::new(&diagnostics));
                    pipeline.run(ast)
                })?,
            Pipeline::WithoutMIR => air_parser::parse_file(&diagnostics, codemap, &self.input_path)
                .map_err(CompileError::Parse)
                .and_then(|ast| {
                    let mut pipeline =
                        air_parser::transforms::ConstantPropagation::new(&diagnostics)
                            .chain(air_parser::transforms::Inlining::new(&diagnostics))
                            .chain(air_ir::passes::AstToAir::new(&diagnostics));
                    pipeline.run(ast)
                })?,
        };

        let backend: Box<dyn CodeGenerator<Output = String>> = match target {
            Target::Winterfell => Box::new(air_codegen_winter::CodeGenerator),
        };

        // generate Rust code targeting Winterfell
        Ok(backend.generate(&air).expect("code generation failed"))
    }
}

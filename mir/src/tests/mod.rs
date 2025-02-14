mod access;
mod boundary_constraints;
mod buses;
mod constant;
mod evaluators;
mod functions;
mod integrity_constraints;
mod ir;
mod list_folding;
mod pub_inputs;
mod random_values;
mod selectors;
mod source_sections;
mod trace;
mod variables;

/// Note: Tests on this module are currently redundant with the tests in the `air-ir` crate.
///
/// Indeed, these tests ensure that we can compile and translate AirScript code into AIR, with both pipelines (with and without MIR),
/// so if these tests pass, we can produce a Mir.
///
/// However, instead of removing the following tests, we should ensure the resulting Mir graph is consistent with what is expected, as well as test each pass.
pub use crate::CompileError;

use std::sync::Arc;

use crate::ir::Mir;
use air_pass::Pass;
use miden_diagnostics::{CodeMap, DiagnosticsConfig, DiagnosticsHandler, Verbosity};

pub fn compile(source: &str) -> Result<Mir, ()> {
    let compiler = Compiler::default();
    match compiler.compile(source) {
        Ok(mir) => Ok(mir),
        Err(err) => {
            compiler.diagnostics.emit(err);
            compiler.emitter.print_captured_to_stderr();
            Err(())
        }
    }
}

pub fn translate(source: &str) -> Result<Mir, ()> {
    let compiler = Compiler::default();
    match compiler.translate(source) {
        Ok(mir) => Ok(mir),
        Err(err) => {
            compiler.diagnostics.emit(err);
            compiler.emitter.print_captured_to_stderr();
            Err(())
        }
    }
}

#[allow(dead_code)]
pub fn parse(source: &str) -> Result<air_parser::ast::Program, ()> {
    let compiler = Compiler::default();
    match compiler.parse(source) {
        Ok(ast) => Ok(ast),
        Err(err) => {
            compiler.diagnostics.emit(err);
            compiler.emitter.print_captured_to_stderr();
            Err(())
        }
    }
}

#[track_caller]
pub fn expect_diagnostic(source: &str, expected: &str) {
    let compiler = Compiler::default();
    let err = match compiler.compile(source) {
        Ok(ref ast) => {
            panic!("expected compilation to fail, got {:#?}", ast);
        }
        Err(err) => err,
    };
    compiler.diagnostics.emit(err);
    let found = compiler.emitter.captured().contains(expected);
    if !found {
        compiler.emitter.print_captured_to_stderr();
    }
    assert!(
        found,
        "expected diagnostic output to contain the string: '{}'",
        expected
    );
}

struct Compiler {
    codemap: Arc<CodeMap>,
    emitter: Arc<SplitEmitter>,
    diagnostics: Arc<DiagnosticsHandler>,
}
impl Default for Compiler {
    fn default() -> Self {
        Self::new(DiagnosticsConfig {
            verbosity: Verbosity::Warning,
            warnings_as_errors: true,
            no_warn: false,
            display: Default::default(),
        })
    }
}
impl Compiler {
    pub fn new(config: DiagnosticsConfig) -> Self {
        let codemap = Arc::new(CodeMap::new());
        let emitter = Arc::new(SplitEmitter::new());
        let diagnostics = Arc::new(DiagnosticsHandler::new(
            config,
            codemap.clone(),
            emitter.clone(),
        ));

        Self {
            codemap,
            emitter,
            diagnostics,
        }
    }

    pub fn compile(&self, source: &str) -> Result<Mir, CompileError> {
        air_parser::parse(&self.diagnostics, self.codemap.clone(), source)
            .map_err(CompileError::Parse)
            .and_then(|ast| {
                let mut pipeline =
                    air_parser::transforms::ConstantPropagation::new(&self.diagnostics)
                        .chain(crate::passes::AstToMir::new(&self.diagnostics))
                        .chain(crate::passes::Inlining::new(&self.diagnostics))
                        .chain(crate::passes::Unrolling::new(&self.diagnostics));
                pipeline.run(ast)
            })
    }
    pub fn translate(&self, source: &str) -> Result<Mir, CompileError> {
        air_parser::parse(&self.diagnostics, self.codemap.clone(), source)
            .map_err(CompileError::Parse)
            .and_then(|ast| {
                let mut pipeline =
                    air_parser::transforms::ConstantPropagation::new(&self.diagnostics)
                        .chain(crate::passes::AstToMir::new(&self.diagnostics));
                pipeline.run(ast)
            })
    }
    #[allow(dead_code)]
    pub fn parse(&self, source: &str) -> Result<air_parser::ast::Program, CompileError> {
        air_parser::parse(&self.diagnostics, self.codemap.clone(), source)
            .map_err(CompileError::Parse)
    }
}

struct SplitEmitter {
    capture: miden_diagnostics::CaptureEmitter,
    default: miden_diagnostics::DefaultEmitter,
}
impl SplitEmitter {
    #[inline]
    pub fn new() -> Self {
        use miden_diagnostics::term::termcolor::ColorChoice;

        Self {
            capture: Default::default(),
            default: miden_diagnostics::DefaultEmitter::new(ColorChoice::Auto),
        }
    }

    pub fn captured(&self) -> String {
        self.capture.captured()
    }

    pub fn print_captured_to_stderr(&self) {
        use miden_diagnostics::Emitter;
        use std::io::Write;

        let mut copy = self.default.buffer();
        let captured = self.capture.captured();
        copy.write_all(captured.as_bytes()).unwrap();
        self.default.print(copy).unwrap();
    }
}
impl miden_diagnostics::Emitter for SplitEmitter {
    #[inline]
    fn buffer(&self) -> miden_diagnostics::term::termcolor::Buffer {
        self.capture.buffer()
    }

    #[inline]
    fn print(&self, buffer: miden_diagnostics::term::termcolor::Buffer) -> std::io::Result<()> {
        self.capture.print(buffer)
    }
}

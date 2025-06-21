use std::sync::Arc;

use miden_diagnostics::{CodeMap, DiagnosticsConfig, DiagnosticsHandler, Emitter, Verbosity};
use pretty_assertions::assert_eq;

use crate::{
    ast::{Module, Program},
    parser::{ParseError, Parser},
};

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
}
impl Emitter for SplitEmitter {
    #[inline]
    fn buffer(&self) -> miden_diagnostics::term::termcolor::Buffer {
        self.capture.buffer()
    }

    #[inline]
    fn print(&self, buffer: miden_diagnostics::term::termcolor::Buffer) -> std::io::Result<()> {
        use std::io::Write;

        let mut copy = self.capture.buffer();
        copy.write_all(buffer.as_slice())?;
        self.capture.print(buffer)?;
        self.default.print(copy)
    }
}

// TEST HANDLER
// ================================================================================================

/// [ParseTest] is a container for the data required to run parser tests. Used to build an AST from
/// the given source string and asserts that executing the test will result in the expected AST.
///
/// # Errors:
/// - ScanError test: check that the source provided contains valid characters and keywords.
/// - ParseError test: check that the parsed values are valid.
///   * InvalidInt: This error is returned if the parsed number is not a valid u64.
pub struct ParseTest {
    pub diagnostics: Arc<DiagnosticsHandler>,
    emitter: Arc<SplitEmitter>,
    parser: Parser,
}

impl ParseTest {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Creates a new test, from the source string.
    pub fn new() -> Self {
        let codemap = Arc::new(CodeMap::new());
        let emitter = Arc::new(SplitEmitter::new());
        let config = DiagnosticsConfig {
            verbosity: Verbosity::Warning,
            warnings_as_errors: true,
            no_warn: false,
            display: Default::default(),
        };
        let diagnostics =
            Arc::new(DiagnosticsHandler::new(config, codemap.clone(), emitter.clone()));
        let parser = Parser::new((), codemap);
        Self { diagnostics, emitter, parser }
    }

    /// This adds a new in-memory file to the [CodeMap] for this test.
    ///
    /// This is used when we want to write a test with imports, without having to place files on
    /// disk
    pub fn add_virtual_file<P: AsRef<std::path::Path>>(&self, name: P, content: String) {
        self.parser.codemap.add(name.as_ref(), content);
    }

    pub fn parse_module_from_file(&self, path: &str) -> Result<Module, ParseError> {
        self.parser.parse_file::<Module, _, _>(&self.diagnostics, path)
    }

    pub fn parse_program_from_file(&self, path: &str) -> Result<Program, ParseError> {
        self.parser.parse_file::<Program, _, _>(&self.diagnostics, path)
    }

    pub fn parse_module(&self, source: &str) -> Result<Module, ParseError> {
        self.parser.parse_string::<Module, _, _>(&self.diagnostics, source)
    }

    pub fn parse_program(&self, source: &str) -> Result<Program, ParseError> {
        self.parser.parse_string::<Program, _, _>(&self.diagnostics, source)
    }

    // TEST METHODS
    // --------------------------------------------------------------------------------------------

    #[track_caller]
    pub fn expect_program_diagnostic(&self, source: &str, expected: &str) {
        if let Err(err) = self.parse_program(source) {
            self.diagnostics.emit(err);
            assert!(
                self.emitter.captured().contains(expected),
                "expected diagnostic output to contain the string: '{expected}'",
            );
        } else {
            panic!("expected parsing to fail, but it succeeded");
        }
    }

    #[track_caller]
    pub fn expect_module_diagnostic(&self, source: &str, expected: &str) {
        if let Err(err) = self.parse_module(source) {
            self.diagnostics.emit(err);
            assert!(
                self.emitter.captured().contains(expected),
                "expected diagnostic output to contain the string: '{expected}'",
            );
        } else {
            panic!("expected parsing to fail, but it succeeded");
        }
    }

    /// If an unrecognized token is present in the source string, return UnrecognizedToken error.
    #[track_caller]
    pub fn expect_unrecognized_token(&self, source: &str) {
        if let Err(err) = self.parse_program(source) {
            assert_matches!(err, ParseError::UnrecognizedToken { .. })
        } else {
            panic!("expected unrecognized token error, but parsing succeeded");
        }
    }

    /// Parses a [Program] from the given source string and asserts that executing the test will
    /// result in the expected AST.
    #[allow(unused)]
    #[track_caller]
    pub fn expect_program_ast(&self, source: &str, expected: Program) {
        match self.parse_program(source) {
            Err(err) => {
                self.diagnostics.emit(err);
                panic!("expected parsing to succeed, see diagnostics for details");
            },
            Ok(ast) => assert_eq!(ast, expected),
        }
    }

    /// Parses a [Module] from the given source string and asserts that executing the test will
    /// result in the expected AST.
    #[track_caller]
    pub fn expect_module_ast(&self, source: &str, expected: Module) {
        match self.parse_module(source) {
            Err(err) => {
                self.diagnostics.emit(err);
                panic!("expected parsing to succeed, see diagnostics for details");
            },
            Ok(ast) => assert_eq!(ast, expected),
        }
    }

    /// Parses a [Program] from the given source path and asserts that executing the test will
    /// result in the expected AST.
    #[track_caller]
    pub fn expect_program_ast_from_file(&self, path: &str, expected: Program) {
        match self.parse_program_from_file(path) {
            Err(err) => {
                self.diagnostics.emit(err);
                panic!("expected parsing to succeed, see diagnostics for details");
            },
            Ok(ast) => assert_eq!(ast, expected),
        }
    }

    /// Parses a [Module] from the given source path and asserts that executing the test will result
    /// in the expected AST.
    #[allow(unused)]
    #[track_caller]
    pub fn expect_module_ast_from_file(&self, path: &str, expected: Module) {
        match self.parse_module_from_file(path) {
            Err(err) => {
                self.diagnostics.emit(err);
                panic!("expected parsing to succeed, see diagnostics for details");
            },
            Ok(ast) => assert_eq!(ast, expected),
        }
    }
}

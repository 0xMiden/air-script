mod codegen;

pub mod ir;
pub mod passes;
#[cfg(test)]
mod tests;

use miden_diagnostics::{Diagnostic, ToDiagnostic};

pub use self::codegen::CodeGenerator;

/// Error type that can be returned during the Mir passes
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error(transparent)]
    Parse(#[from] air_parser::ParseError),
    #[error(transparent)]
    SemanticAnalysis(#[from] air_parser::SemanticAnalysisError),
    #[error("compilation failed, see diagnostics for more information")]
    Failed,
}

impl ToDiagnostic for CompileError {
    /// Helper to convert a [CompileError] into a [Diagnostic]
    fn to_diagnostic(self) -> Diagnostic {
        match self {
            Self::Parse(err) => err.to_diagnostic(),
            Self::SemanticAnalysis(err) => err.to_diagnostic(),
            Self::Failed => Diagnostic::error().with_message(self.to_string()),
        }
    }
}

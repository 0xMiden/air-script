use miden_diagnostics::{Diagnostic, Label, SourceSpan, Spanned, ToDiagnostic};

use crate::ast::{Identifier, InvalidExprError, InvalidTypeError, ModuleId};

/// Represents the various module validation errors we might encounter during semantic analysis.
#[derive(Clone, Debug, thiserror::Error)]
pub enum SemanticAnalysisError {
    #[error("root module is missing")]
    MissingRoot,
    #[error(
        "root module must contain at both boundary_constraints and integrity_constraints sections"
    )]
    MissingConstraints,
    #[error("root module must contain a public_inputs section")]
    MissingPublicInputs,
    #[error("reference to unknown module '{0}'")]
    MissingModule(ModuleId),
    #[error("invalid use of restricted section type in library module")]
    RootSectionInLibrary(SourceSpan),
    #[error("invalid import of root module")]
    RootImport(SourceSpan),
    #[error("name already in use")]
    NameConflict(SourceSpan),
    #[error("import refers to undefined item in '{0}'")]
    ImportUndefined(ModuleId),
    #[error("cannot import from self")]
    ImportSelf(SourceSpan),
    #[error("import conflict")]
    ImportConflict { item: Identifier, prev: SourceSpan },
    #[error("import failed")]
    ImportFailed(SourceSpan),
    #[error(transparent)]
    InvalidExpr(#[from] InvalidExprError),
    #[error(transparent)]
    InvalidType(#[from] InvalidTypeError),
    #[error("module is invalid, see diagnostics for details")]
    Invalid,
}
impl Eq for SemanticAnalysisError {}
impl PartialEq for SemanticAnalysisError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MissingModule(lm), Self::MissingModule(rm)) => lm == rm,
            (Self::ImportUndefined(lm), Self::ImportUndefined(rm)) => lm == rm,
            (Self::ImportConflict { item: li, .. }, Self::ImportConflict { item: ri, .. }) => {
                li == ri
            },
            (Self::InvalidExpr(l), Self::InvalidExpr(r)) => l == r,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl ToDiagnostic for SemanticAnalysisError {
    fn to_diagnostic(self) -> Diagnostic {
        match self {
            Self::MissingRoot => Diagnostic::error().with_message("no root module found"),
            Self::MissingConstraints => Diagnostic::error().with_message("root module must contain both boundary_constraints and integrity_constraints sections"),
            Self::MissingPublicInputs => Diagnostic::error().with_message("root module must contain a public_inputs section"),
            Self::MissingModule(id) => Diagnostic::error()
                .with_message("found reference to module which does not exist")
                .with_labels(vec![Label::primary(id.span().source_id(), id.span()).with_message("this module could not be found")]),
            Self::RootSectionInLibrary(span) => Diagnostic::error()
                .with_message("invalid use of restricted section type in library module")
                .with_labels(vec![Label::primary(span.source_id(), span)
                    .with_message("invalid declaration occurs here")]),
            Self::RootImport(span) => Diagnostic::error()
                .with_message("invalid import of root module")
                .with_labels(vec![Label::primary(span.source_id(), span)
                    .with_message("invalid declaration occurs here")])
                .with_notes(vec!["The root module may not be imported. Try extracting the items you wish to import into a library module".to_string()]),
            Self::NameConflict(span) => Diagnostic::error()
                .with_message("name already in use")
                .with_labels(vec![Label::primary(span.source_id(), span)
                    .with_message("conflicting definition occurs here")]),
            Self::ImportUndefined(from) => Diagnostic::error()
                .with_message("invalid import")
                .with_labels(vec![Label::primary(from.span().source_id(), from.span())
                    .with_message(format!("no such item in '{from}'"))]),
            Self::ImportSelf(span) => Diagnostic::error()
                .with_message("invalid import")
                .with_labels(vec![Label::primary(span.source_id(), span)
                    .with_message("cannot import a module from within itself")]),
            Self::ImportConflict { item, prev } => Diagnostic::error()
                .with_message("conflicting import")
                .with_labels(vec![Label::primary(item.span().source_id(), item.span())
                    .with_message(format!("the item '{item}' is imported here")),
                                  Label::secondary(prev.source_id(), prev)
                    .with_message("but it conflicts with an item of the same name here")]),
            Self::ImportFailed(span) => Diagnostic::error()
                .with_message("error occurred while resolving an import")
                .with_labels(vec![Label::primary(span.source_id(), span)
                    .with_message("failed import occurred here")]),
            Self::InvalidExpr(err) => err.to_diagnostic(),
            Self::InvalidType(err) => err.to_diagnostic(),
            Self::Invalid => Diagnostic::error().with_message("module is invalid, see diagnostics for details"),
        }
    }
}

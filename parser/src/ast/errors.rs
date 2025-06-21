use miden_diagnostics::{Diagnostic, Label, SourceSpan, ToDiagnostic};

/// Represents an invalid expression for use in an `Expr` context
#[derive(Clone, Debug, thiserror::Error)]
pub enum InvalidExprError {
    #[error("this value is too large for an exponent")]
    InvalidExponent(SourceSpan),
    #[error("expected exponent to be a constant")]
    NonConstantExponent(SourceSpan),
    #[error("expected constant range expression")]
    NonConstantRangeExpr(SourceSpan),
    #[error("accessing column boundaries is not allowed here")]
    BoundedSymbolAccess(SourceSpan),
    #[error("expected scalar expression")]
    InvalidScalarExpr(SourceSpan),
    #[error(
        "invalid let in expression position: body produces no value, or the type of that value is unknown"
    )]
    InvalidLetExpr(SourceSpan),
    #[error("syntax does not represent a valid expression")]
    NotAnExpr(SourceSpan),
}
impl Eq for InvalidExprError {}
impl PartialEq for InvalidExprError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}
impl ToDiagnostic for InvalidExprError {
    fn to_diagnostic(self) -> Diagnostic {
        let message = format!("{}", &self);
        match self {
            Self::NonConstantExponent(span) => Diagnostic::error()
                .with_message("invalid expression")
                .with_labels(vec![Label::primary(span.source_id(), span).with_message(message)])
                .with_notes(vec![
                    "Only constant powers are supported with the exponentiation operator currently"
                        .to_string(),
                ]),
            Self::NonConstantRangeExpr(span) => Diagnostic::error()
                .with_message("invalid expression")
                .with_labels(vec![Label::primary(span.source_id(), span).with_message(message)])
                .with_notes(vec![
                    "Range expression must be a constant to do this operation".to_string(),
                ]),
            Self::InvalidExponent(span)
            | Self::BoundedSymbolAccess(span)
            | Self::InvalidScalarExpr(span)
            | Self::InvalidLetExpr(span)
            | Self::NotAnExpr(span) => Diagnostic::error()
                .with_message("invalid expression")
                .with_labels(vec![Label::primary(span.source_id(), span).with_message(message)]),
        }
    }
}

/// Represents an invalid type for use in a `BindingType` context
#[derive(Clone, Debug, thiserror::Error)]
pub enum InvalidTypeError {
    #[error("expected iterable to be a vector")]
    NonVectorIterable(SourceSpan),
}
impl Eq for InvalidTypeError {}
impl PartialEq for InvalidTypeError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}
impl ToDiagnostic for InvalidTypeError {
    fn to_diagnostic(self) -> Diagnostic {
        let message = format!("{}", &self);
        match self {
            Self::NonVectorIterable(span) => Diagnostic::error()
                .with_message("invalid type")
                .with_labels(vec![Label::primary(span.source_id(), span).with_message(message)])
                .with_notes(vec!["Only vectors can be used as iterables".to_string()]),
        }
    }
}

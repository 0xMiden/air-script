use crate::lexer::{Lexer, LexicalError, Token};
use crate::parser::ParseError;
use crate::Symbol;

mod arithmetic_ops;
mod boundary_constraints;
mod constants;
mod evaluator_functions;
mod functions;
mod identifiers;
mod list_comprehension;
mod modules;
mod periodic_columns;
mod pub_inputs;
mod variables;

// TEST HELPERS
// ================================================================================================

use std::sync::Arc;

use miden_diagnostics::CodeMap;
use miden_parsing::{FileMapSource, Scanner, Source};

fn expect_valid_tokenization(source: &str, expected_tokens: Vec<Token>) {
    let codemap = Arc::new(CodeMap::new());
    let id = codemap.add("nofile", source.to_string());
    let file = codemap.get(id).unwrap();
    let scanner = Scanner::new(FileMapSource::new(file));
    let lexer = Lexer::new(scanner);

    let tokens: Vec<Token> = lexer.map(|res| res.unwrap().1).collect();
    assert_eq!(tokens, expected_tokens);
}

/// Asserts that lexing fails with a specific error, that occurs at the given line and column.
///
/// NOTE: Provided line/column numbers should be zero-indexed
fn expect_error_at_location(source: &str, expected: LexicalError, line: u32, col: u32) {
    use miden_diagnostics::{ColumnIndex, LineIndex};

    let codemap = Arc::new(CodeMap::new());
    let mut tokens = lex(codemap.clone(), source);
    let err = tokens
        .find_map(|res| match res {
            Ok(_) => None,
            Err(ParseError::Lexer(err)) => Some(err),
            Err(_) => unreachable!(),
        })
        .expect("expected lexical error, but lexing completed successfully");

    let loc = match &err {
        LexicalError::InvalidInt { span, .. } => codemap.location(span).unwrap(),
        LexicalError::UnexpectedCharacter { start, .. } => {
            let span = miden_diagnostics::SourceSpan::new(*start, *start);
            codemap.location(&span).unwrap()
        }
    };
    assert_eq!(err, expected);
    assert_eq!(loc.line, LineIndex(line));
    assert_eq!(loc.column, ColumnIndex(col));
}

/// Asserts that lexing fails for any reason, and returns the first error
fn expect_any_error(source: &str) -> LexicalError {
    let codemap = Arc::new(CodeMap::new());
    let mut tokens = lex(codemap, source);
    tokens
        .find_map(|res| match res {
            Ok(_) => None,
            Err(ParseError::Lexer(err)) => Some(err),
            Err(_) => unreachable!(),
        })
        .expect("expected lexical error, but lexing completed successfully")
}

fn lex(codemap: Arc<CodeMap>, source: &str) -> Lexer<FileMapSource> {
    let id = codemap.add("nofile", source.to_string());
    let file = codemap.get(id).unwrap();
    let scanner = Scanner::new(FileMapSource::new(file));
    Lexer::new(scanner)
}

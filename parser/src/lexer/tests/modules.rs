use super::{Symbol, Token, expect_valid_tokenization};

#[test]
fn root_module_tokenization() {
    let source = r#"
    def hello

    trace_columns {
        main: [a]
    }

    boundary_constraints {
        enf a.first = 0
    }"#;
    let tokens = vec![
        Token::Def,
        Token::Ident(Symbol::intern("hello")),
        Token::TraceColumns,
        Token::LBrace,
        Token::Main,
        Token::Colon,
        Token::LBracket,
        Token::Ident(Symbol::intern("a")),
        Token::RBracket,
        Token::RBrace,
        Token::BoundaryConstraints,
        Token::LBrace,
        Token::Enf,
        Token::Ident(Symbol::intern("a")),
        Token::Dot,
        Token::First,
        Token::Equal,
        Token::Num(0),
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn root_module_with_imports_tokenization() {
    let source = r#"
    def hello

    use std::*
    use my_constraints::first_is_zero

    trace_columns {
        main: [a]
    }

    integrity_constraints {
        enf first_is_zero(a)
    }"#;
    let tokens = vec![
        Token::Def,
        Token::Ident(Symbol::intern("hello")),
        Token::Use,
        Token::Ident(Symbol::intern("std")),
        Token::ColonColon,
        Token::Star,
        Token::Use,
        Token::Ident(Symbol::intern("my_constraints")),
        Token::ColonColon,
        Token::Ident(Symbol::intern("first_is_zero")),
        Token::TraceColumns,
        Token::LBrace,
        Token::Main,
        Token::Colon,
        Token::LBracket,
        Token::Ident(Symbol::intern("a")),
        Token::RBracket,
        Token::RBrace,
        Token::IntegrityConstraints,
        Token::LBrace,
        Token::Enf,
        Token::FunctionIdent(Symbol::intern("first_is_zero")),
        Token::LParen,
        Token::Ident(Symbol::intern("a")),
        Token::RParen,
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn library_module_tokenization() {
    let source = r#"
    mod my_constraints

    ev first_is_zero(state[a]) {
        enf a = 0
    }"#;
    let tokens = vec![
        Token::Mod,
        Token::Ident(Symbol::intern("my_constraints")),
        Token::Ev,
        Token::FunctionIdent(Symbol::intern("first_is_zero")),
        Token::LParen,
        Token::Ident(Symbol::intern("state")),
        Token::LBracket,
        Token::Ident(Symbol::intern("a")),
        Token::RBracket,
        Token::RParen,
        Token::LBrace,
        Token::Enf,
        Token::Ident(Symbol::intern("a")),
        Token::Equal,
        Token::Num(0),
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

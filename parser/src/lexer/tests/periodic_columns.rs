use super::{Symbol, Token, expect_valid_tokenization};

#[test]
fn periodic_columns_kw() {
    let source = "periodic_columns";
    let tokens = vec![Token::PeriodicColumns];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn periodic_columns() {
    let source = "
periodic_columns {
    k0: [1, 0, 0, 0]
    k1: [0, 0, 0, 0, 0, 0, 0, 1]
}";

    let tokens = vec![
        Token::PeriodicColumns,
        Token::LBrace,
        Token::Ident(Symbol::intern("k0")),
        Token::Colon,
        Token::LBracket,
        Token::Num(1),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::RBracket,
        Token::Ident(Symbol::intern("k1")),
        Token::Colon,
        Token::LBracket,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(0),
        Token::Comma,
        Token::Num(1),
        Token::RBracket,
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

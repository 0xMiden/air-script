use super::{Symbol, Token, expect_valid_tokenization};

// FUNCTION VALID TOKENIZATION
// ================================================================================================

#[test]
fn fn_with_scalars() {
    let source = "fn fn_name(a: felt, b: felt) -> felt {
        return a + b
    }";

    let tokens = [
        Token::Fn,
        Token::FunctionIdent(Symbol::intern("fn_name")),
        Token::LParen,
        Token::Ident(Symbol::intern("a")),
        Token::Colon,
        Token::Felt,
        Token::Comma,
        Token::Ident(Symbol::intern("b")),
        Token::Colon,
        Token::Felt,
        Token::RParen,
        Token::Arrow,
        Token::Felt,
        Token::LBrace,
        Token::Return,
        Token::Ident(Symbol::intern("a")),
        Token::Plus,
        Token::Ident(Symbol::intern("b")),
        Token::RBrace,
    ];

    expect_valid_tokenization(source, tokens.to_vec());
}

#[test]
fn fn_with_vectors() {
    let source = "fn fn_name(a: felt[12], b: felt[12]) -> felt[12] {
        return [x + y for x, y in (a, b)]
    }";

    let tokens = [
        Token::Fn,
        Token::FunctionIdent(Symbol::intern("fn_name")),
        Token::LParen,
        Token::Ident(Symbol::intern("a")),
        Token::Colon,
        Token::Felt,
        Token::LBracket,
        Token::Num(12),
        Token::RBracket,
        Token::Comma,
        Token::Ident(Symbol::intern("b")),
        Token::Colon,
        Token::Felt,
        Token::LBracket,
        Token::Num(12),
        Token::RBracket,
        Token::RParen,
        Token::Arrow,
        Token::Felt,
        Token::LBracket,
        Token::Num(12),
        Token::RBracket,
        Token::LBrace,
        Token::Return,
        Token::LBracket,
        Token::Ident(Symbol::intern("x")),
        Token::Plus,
        Token::Ident(Symbol::intern("y")),
        Token::For,
        Token::Ident(Symbol::intern("x")),
        Token::Comma,
        Token::Ident(Symbol::intern("y")),
        Token::In,
        Token::LParen,
        Token::Ident(Symbol::intern("a")),
        Token::Comma,
        Token::Ident(Symbol::intern("b")),
        Token::RParen,
        Token::RBracket,
        Token::RBrace,
    ];

    expect_valid_tokenization(source, tokens.to_vec());
}

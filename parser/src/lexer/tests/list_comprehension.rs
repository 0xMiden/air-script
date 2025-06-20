use super::{Symbol, Token, expect_valid_tokenization};

// LIST COMPREHENSION VALID TOKENIZATION
// ================================================================================================

#[test]
fn one_iterable_comprehension() {
    let source = "let y = [x for x in x]";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("y")),
        Token::Equal,
        Token::LBracket,
        Token::Ident(Symbol::intern("x")),
        Token::For,
        Token::Ident(Symbol::intern("x")),
        Token::In,
        Token::Ident(Symbol::intern("x")),
        Token::RBracket,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn multiple_iterables_comprehension() {
    let source = "let a = [w + x - y - z for (w, x, y, z) in (0..3, x, y[0..3], z[0..3])]";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("a")),
        Token::Equal,
        Token::LBracket,
        Token::Ident(Symbol::intern("w")),
        Token::Plus,
        Token::Ident(Symbol::intern("x")),
        Token::Minus,
        Token::Ident(Symbol::intern("y")),
        Token::Minus,
        Token::Ident(Symbol::intern("z")),
        Token::For,
        Token::LParen,
        Token::Ident(Symbol::intern("w")),
        Token::Comma,
        Token::Ident(Symbol::intern("x")),
        Token::Comma,
        Token::Ident(Symbol::intern("y")),
        Token::Comma,
        Token::Ident(Symbol::intern("z")),
        Token::RParen,
        Token::In,
        Token::LParen,
        Token::Num(0),
        Token::DotDot,
        Token::Num(3),
        Token::Comma,
        Token::Ident(Symbol::intern("x")),
        Token::Comma,
        Token::Ident(Symbol::intern("y")),
        Token::LBracket,
        Token::Num(0),
        Token::DotDot,
        Token::Num(3),
        Token::RBracket,
        Token::Comma,
        Token::Ident(Symbol::intern("z")),
        Token::LBracket,
        Token::Num(0),
        Token::DotDot,
        Token::Num(3),
        Token::RBracket,
        Token::RParen,
        Token::RBracket,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn one_iterable_folding() {
    let source = "let y = sum([x for x in x])";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("y")),
        Token::Equal,
        Token::FunctionIdent(Symbol::intern("sum")),
        Token::LParen,
        Token::LBracket,
        Token::Ident(Symbol::intern("x")),
        Token::For,
        Token::Ident(Symbol::intern("x")),
        Token::In,
        Token::Ident(Symbol::intern("x")),
        Token::RBracket,
        Token::RParen,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn multiple_iterables_list_folding() {
    let source = "let a = sum([w + x - y - z for (w, x, y, z) in (0..3, x, y[0..3], z[0..3])])";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("a")),
        Token::Equal,
        Token::FunctionIdent(Symbol::intern("sum")),
        Token::LParen,
        Token::LBracket,
        Token::Ident(Symbol::intern("w")),
        Token::Plus,
        Token::Ident(Symbol::intern("x")),
        Token::Minus,
        Token::Ident(Symbol::intern("y")),
        Token::Minus,
        Token::Ident(Symbol::intern("z")),
        Token::For,
        Token::LParen,
        Token::Ident(Symbol::intern("w")),
        Token::Comma,
        Token::Ident(Symbol::intern("x")),
        Token::Comma,
        Token::Ident(Symbol::intern("y")),
        Token::Comma,
        Token::Ident(Symbol::intern("z")),
        Token::RParen,
        Token::In,
        Token::LParen,
        Token::Num(0),
        Token::DotDot,
        Token::Num(3),
        Token::Comma,
        Token::Ident(Symbol::intern("x")),
        Token::Comma,
        Token::Ident(Symbol::intern("y")),
        Token::LBracket,
        Token::Num(0),
        Token::DotDot,
        Token::Num(3),
        Token::RBracket,
        Token::Comma,
        Token::Ident(Symbol::intern("z")),
        Token::LBracket,
        Token::Num(0),
        Token::DotDot,
        Token::Num(3),
        Token::RBracket,
        Token::RParen,
        Token::RBracket,
        Token::RParen,
    ];
    expect_valid_tokenization(source, tokens);
}

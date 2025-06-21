use super::expect_valid_tokenization;
use crate::{Symbol, lexer::Token};

// EXPRESSIONS VALID TOKENIZATION
// ================================================================================================

#[test]
fn chained_add_ops() {
    let source = "enf clk' + clk + 1 = 0";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Plus,
        Token::Ident(Symbol::intern("clk")),
        Token::Plus,
        Token::Num(1),
        Token::Equal,
        Token::Num(0),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn chained_sub_ops() {
    let source = "enf clk' - clk - 1 = 0";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Minus,
        Token::Ident(Symbol::intern("clk")),
        Token::Minus,
        Token::Num(1),
        Token::Equal,
        Token::Num(0),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn chained_mul_ops() {
    let source = "enf clk' * clk * 1 = 0";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Star,
        Token::Ident(Symbol::intern("clk")),
        Token::Star,
        Token::Num(1),
        Token::Equal,
        Token::Num(0),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn exp_op() {
    let source = "enf clk'^2 - clk - 1 = 0";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Caret,
        Token::Num(2),
        Token::Minus,
        Token::Ident(Symbol::intern("clk")),
        Token::Minus,
        Token::Num(1),
        Token::Equal,
        Token::Num(0),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn ops_with_parens() {
    let source = "enf clk' - (clk + 1) = 0";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Minus,
        Token::LParen,
        Token::Ident(Symbol::intern("clk")),
        Token::Plus,
        Token::Num(1),
        Token::RParen,
        Token::Equal,
        Token::Num(0),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn ops_without_matching_closing_parens() {
    // This doesn't throw an error while scanning but while parsing.
    let source = "enf (clk' - (clk + 1) = 0";
    let tokens = vec![
        Token::Enf,
        Token::LParen,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Minus,
        Token::LParen,
        Token::Ident(Symbol::intern("clk")),
        Token::Plus,
        Token::Num(1),
        Token::RParen,
        Token::Equal,
        Token::Num(0),
    ];
    expect_valid_tokenization(source, tokens);
}

use super::{Symbol, Token, expect_valid_tokenization};

// VARIABLES VALID TOKENIZATION
// ================================================================================================

#[test]
fn boundary_constraint_with_scalar_variables() {
    let source = "
    let first_value = 0
    enf clk.first = first_value";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("first_value")),
        Token::Equal,
        Token::Num(0),
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::First,
        Token::Equal,
        Token::Ident(Symbol::intern("first_value")),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn boundary_constraint_with_vector_variables() {
    let source = "
    let boundary_values = [0, 1]
    enf clk.first = boundary_values[0]
    enf clk.last = boundary_values[1]";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("boundary_values")),
        Token::Equal,
        Token::LBracket,
        Token::Num(0),
        Token::Comma,
        Token::Num(1),
        Token::RBracket,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::First,
        Token::Equal,
        Token::Ident(Symbol::intern("boundary_values")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::Last,
        Token::Equal,
        Token::Ident(Symbol::intern("boundary_values")),
        Token::LBracket,
        Token::Num(1),
        Token::RBracket,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn integrity_constraint_with_scalar_variables() {
    let source = "
    let a = 0
    enf clk' = clk - a";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("a")),
        Token::Equal,
        Token::Num(0),
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Equal,
        Token::Ident(Symbol::intern("clk")),
        Token::Minus,
        Token::Ident(Symbol::intern("a")),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn integrity_constraint_with_vector_variables() {
    let source = "
    let a = [0, 1]
    enf clk' = clk - a[0] + a[1]";
    let tokens = vec![
        Token::Let,
        Token::Ident(Symbol::intern("a")),
        Token::Equal,
        Token::LBracket,
        Token::Num(0),
        Token::Comma,
        Token::Num(1),
        Token::RBracket,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Equal,
        Token::Ident(Symbol::intern("clk")),
        Token::Minus,
        Token::Ident(Symbol::intern("a")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::Plus,
        Token::Ident(Symbol::intern("a")),
        Token::LBracket,
        Token::Num(1),
        Token::RBracket,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn variables_with_or_operators() {
    let source = "
    integrity_constraints {
        let flag = s[0] | !s[1]'
        enf clk' = clk + 1 when flag
    }";
    let tokens = vec![
        Token::IntegrityConstraints,
        Token::LBrace,
        Token::Let,
        Token::Ident(Symbol::intern("flag")),
        Token::Equal,
        Token::Ident(Symbol::intern("s")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::Bar,
        Token::Bang,
        Token::Ident(Symbol::intern("s")),
        Token::LBracket,
        Token::Num(1),
        Token::RBracket,
        Token::Quote,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Quote,
        Token::Equal,
        Token::Ident(Symbol::intern("clk")),
        Token::Plus,
        Token::Num(1),
        Token::When,
        Token::Ident(Symbol::intern("flag")),
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

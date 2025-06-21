use super::{Symbol, Token, expect_valid_tokenization};

// BOUNDARY STATEMENTS VALID TOKENIZATION
// ================================================================================================

#[test]
fn first_boundary_constant() {
    let source = "enf clk.first = 0";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::First,
        Token::Equal,
        Token::Num(0),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn last_boundary_constant() {
    let source = "enf clk.last = 15";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::Last,
        Token::Equal,
        Token::Num(15),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn boundary_with_pub_input() {
    let source = "enf clk.first = stack_inputs[0]";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::First,
        Token::Equal,
        Token::Ident(Symbol::intern("stack_inputs")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn boundary_expression() {
    let source = "enf clk.first = 5 + stack_inputs[3] + 6";
    let tokens = vec![
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::First,
        Token::Equal,
        Token::Num(5),
        Token::Plus,
        Token::Ident(Symbol::intern("stack_inputs")),
        Token::LBracket,
        Token::Num(3),
        Token::RBracket,
        Token::Plus,
        Token::Num(6),
    ];
    expect_valid_tokenization(source, tokens);
}

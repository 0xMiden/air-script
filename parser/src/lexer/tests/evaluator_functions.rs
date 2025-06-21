use super::{Symbol, Token, expect_valid_tokenization};

// EVALUATOR FUNCTION VALID TOKENIZATION
// ================================================================================================

#[test]
fn ev_fn_with_main_cols() {
    let source = "
    ev ev_fn([state[12]]) {
        let s1 = [x^7 for x in state]
        let s2 = [x^7 for x in s1]
  
        enf s1[0] = s2[0]
    }";

    let tokens = [
        Token::Ev,
        Token::FunctionIdent(Symbol::intern("ev_fn")),
        Token::LParen,
        Token::LBracket,
        Token::Ident(Symbol::intern("state")),
        Token::LBracket,
        Token::Num(12),
        Token::RBracket,
        Token::RBracket,
        Token::RParen,
        Token::LBrace,
        Token::Let,
        Token::Ident(Symbol::intern("s1")),
        Token::Equal,
        Token::LBracket,
        Token::Ident(Symbol::intern("x")),
        Token::Caret,
        Token::Num(7),
        Token::For,
        Token::Ident(Symbol::intern("x")),
        Token::In,
        Token::Ident(Symbol::intern("state")),
        Token::RBracket,
        Token::Let,
        Token::Ident(Symbol::intern("s2")),
        Token::Equal,
        Token::LBracket,
        Token::Ident(Symbol::intern("x")),
        Token::Caret,
        Token::Num(7),
        Token::For,
        Token::Ident(Symbol::intern("x")),
        Token::In,
        Token::Ident(Symbol::intern("s1")),
        Token::RBracket,
        Token::Enf,
        Token::Ident(Symbol::intern("s1")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::Equal,
        Token::Ident(Symbol::intern("s2")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::RBrace,
    ];

    expect_valid_tokenization(source, tokens.to_vec());
}

#[test]
fn ev_fn_call() {
    let source = "
        integrity_constraints {
            enf ev_fn([state[12]])
        }";

    let tokens = [
        Token::IntegrityConstraints,
        Token::LBrace,
        Token::Enf,
        Token::FunctionIdent(Symbol::intern("ev_fn")),
        Token::LParen,
        Token::LBracket,
        Token::Ident(Symbol::intern("state")),
        Token::LBracket,
        Token::Num(12),
        Token::RBracket,
        Token::RBracket,
        Token::RParen,
        Token::RBrace,
    ];

    expect_valid_tokenization(source, tokens.to_vec());
}

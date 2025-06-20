use super::{Symbol, Token, expect_valid_tokenization};

#[test]
fn pub_inputs_kw() {
    let source = "public_inputs";
    let tokens = vec![Token::PublicInputs];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn pub_inputs_sized_arrays() {
    let source = "
public_inputs {
    program_hash: [4]
    stack_inputs: [12]
}";

    let tokens = vec![
        Token::PublicInputs,
        Token::LBrace,
        Token::Ident(Symbol::intern("program_hash")),
        Token::Colon,
        Token::LBracket,
        Token::Num(4),
        Token::RBracket,
        Token::Ident(Symbol::intern("stack_inputs")),
        Token::Colon,
        Token::LBracket,
        Token::Num(12),
        Token::RBracket,
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

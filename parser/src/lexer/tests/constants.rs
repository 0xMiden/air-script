use super::{Symbol, Token, expect_valid_tokenization};

#[test]
fn constants_scalar() {
    let source = "
    const A = 1
    const B = 2";

    let tokens = vec![
        Token::Const,
        Token::Ident(Symbol::intern("A")),
        Token::Equal,
        Token::Num(1),
        Token::Const,
        Token::Ident(Symbol::intern("B")),
        Token::Equal,
        Token::Num(2),
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn constants_vector() {
    let source = "
    const A = [1, 2, 3, 4]
    const B = [5, 6, 7, 8]";

    let tokens = vec![
        Token::Const,
        Token::Ident(Symbol::intern("A")),
        Token::Equal,
        Token::LBracket,
        Token::Num(1),
        Token::Comma,
        Token::Num(2),
        Token::Comma,
        Token::Num(3),
        Token::Comma,
        Token::Num(4),
        Token::RBracket,
        Token::Const,
        Token::Ident(Symbol::intern("B")),
        Token::Equal,
        Token::LBracket,
        Token::Num(5),
        Token::Comma,
        Token::Num(6),
        Token::Comma,
        Token::Num(7),
        Token::Comma,
        Token::Num(8),
        Token::RBracket,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn constants_matrix() {
    let source = "
        const A = [[1, 2], [3, 4]]
        const B = [[5, 6], [7, 8]]";

    let tokens = vec![
        Token::Const,
        Token::Ident(Symbol::intern("A")),
        Token::Equal,
        Token::LBracket,
        Token::LBracket,
        Token::Num(1),
        Token::Comma,
        Token::Num(2),
        Token::RBracket,
        Token::Comma,
        Token::LBracket,
        Token::Num(3),
        Token::Comma,
        Token::Num(4),
        Token::RBracket,
        Token::RBracket,
        Token::Const,
        Token::Ident(Symbol::intern("B")),
        Token::Equal,
        Token::LBracket,
        Token::LBracket,
        Token::Num(5),
        Token::Comma,
        Token::Num(6),
        Token::RBracket,
        Token::Comma,
        Token::LBracket,
        Token::Num(7),
        Token::Comma,
        Token::Num(8),
        Token::RBracket,
        Token::RBracket,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn constants_access_inside_boundary_expr() {
    // This is invalid since the constants are not declared but this error will be thrown at the
    // IR level.
    let source = "
    boundary_constraints {
        enf clk.first = A + B[0]
        enf clk.last = C[0][1]
    }";

    let tokens = vec![
        Token::BoundaryConstraints,
        Token::LBrace,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::First,
        Token::Equal,
        Token::Ident(Symbol::intern("A")),
        Token::Plus,
        Token::Ident(Symbol::intern("B")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Dot,
        Token::Last,
        Token::Equal,
        Token::Ident(Symbol::intern("C")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::LBracket,
        Token::Num(1),
        Token::RBracket,
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn constants_access_inside_integrity_expr() {
    let source = "
        const A = 1
        const B = [1, 0]
        const C = [[1, 0], [0, 1]]
        integrity_constraints {
            enf clk * 2^A = B[0] + C[0][1]
        }
    ";
    let tokens = vec![
        Token::Const,
        Token::Ident(Symbol::intern("A")),
        Token::Equal,
        Token::Num(1),
        Token::Const,
        Token::Ident(Symbol::intern("B")),
        Token::Equal,
        Token::LBracket,
        Token::Num(1),
        Token::Comma,
        Token::Num(0),
        Token::RBracket,
        Token::Const,
        Token::Ident(Symbol::intern("C")),
        Token::Equal,
        Token::LBracket,
        Token::LBracket,
        Token::Num(1),
        Token::Comma,
        Token::Num(0),
        Token::RBracket,
        Token::Comma,
        Token::LBracket,
        Token::Num(0),
        Token::Comma,
        Token::Num(1),
        Token::RBracket,
        Token::RBracket,
        Token::IntegrityConstraints,
        Token::LBrace,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Star,
        Token::Num(2),
        Token::Caret,
        Token::Ident(Symbol::intern("A")),
        Token::Equal,
        Token::Ident(Symbol::intern("B")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::Plus,
        Token::Ident(Symbol::intern("C")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::LBracket,
        Token::Num(1),
        Token::RBracket,
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

#[test]
fn constants_access_inside_integrity_expr_invalid() {
    // This is invalid since the constants are not declared and the constant names should be
    // capitalized but these errors will be thrown at the IR level and parsing level respectively.
    let source = "
        integrity_constraints {
            enf clk * 2^a = b[0] + c[0][1]
        }
    ";
    let tokens = vec![
        Token::IntegrityConstraints,
        Token::LBrace,
        Token::Enf,
        Token::Ident(Symbol::intern("clk")),
        Token::Star,
        Token::Num(2),
        Token::Caret,
        Token::Ident(Symbol::intern("a")),
        Token::Equal,
        Token::Ident(Symbol::intern("b")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::Plus,
        Token::Ident(Symbol::intern("c")),
        Token::LBracket,
        Token::Num(0),
        Token::RBracket,
        Token::LBracket,
        Token::Num(1),
        Token::RBracket,
        Token::RBrace,
    ];
    expect_valid_tokenization(source, tokens);
}

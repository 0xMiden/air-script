#[cfg(test)]
mod tests;

use core::{fmt, mem, num::IntErrorKind};

use miden_diagnostics::{Diagnostic, SourceIndex, SourceSpan, ToDiagnostic};
use miden_parsing::{Scanner, Source};

use crate::{parser::ParseError, Symbol};

/// The value produced by the Lexer when iterated
pub type Lexed = Result<(SourceIndex, Token, SourceIndex), ParseError>;

/// Errors that may occur during lexing of the source
#[derive(Clone, Debug, thiserror::Error)]
pub enum LexicalError {
    #[error("invalid integer value: {}", DisplayIntErrorKind(reason))]
    InvalidInt {
        span: SourceSpan,
        reason: IntErrorKind,
    },
    #[error("encountered unexpected character '{found}'")]
    UnexpectedCharacter { start: SourceIndex, found: char },
}
impl PartialEq for LexicalError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::InvalidInt { reason: lhs, .. }, Self::InvalidInt { reason: rhs, .. }) => {
                lhs == rhs
            }
            (
                Self::UnexpectedCharacter { found: lhs, .. },
                Self::UnexpectedCharacter { found: rhs, .. },
            ) => lhs == rhs,
            _ => false,
        }
    }
}
impl ToDiagnostic for LexicalError {
    fn to_diagnostic(self) -> Diagnostic {
        use miden_diagnostics::Label;

        match self {
            Self::InvalidInt { span, ref reason } => Diagnostic::error()
                .with_message("invalid integer literal")
                .with_labels(vec![Label::primary(span.source_id(), span)
                    .with_message(format!("{}", DisplayIntErrorKind(reason)))]),
            Self::UnexpectedCharacter { start, .. } => Diagnostic::error()
                .with_message("unexpected character")
                .with_labels(vec![Label::primary(
                    start.source_id(),
                    SourceSpan::new(start, start),
                )]),
        }
    }
}

struct DisplayIntErrorKind<'a>(&'a IntErrorKind);
impl fmt::Display for DisplayIntErrorKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            IntErrorKind::Empty => write!(f, "unable to parse empty string as integer"),
            IntErrorKind::InvalidDigit => write!(f, "invalid digit"),
            IntErrorKind::PosOverflow => write!(f, "value is too big"),
            IntErrorKind::NegOverflow => write!(f, "value is too big"),
            IntErrorKind::Zero => write!(f, "zero is not a valid value here"),
            other => write!(f, "unable to parse integer value: {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Token {
    Eof,
    Error(LexicalError),
    Comment,
    // PRIMITIVES
    // --------------------------------------------------------------------------------------------
    /// Identifiers should start with alphabet followed by one or more alpha numeric characters
    /// or an underscore.
    Ident(Symbol),
    /// A reference to an identifier used for a section declaration, such as the random values
    /// array or a trace segment like "main" or "aux".
    DeclIdentRef(Symbol),
    /// A function identifier
    FunctionIdent(Symbol),
    /// Integers should only contain numeric characters.
    Num(u64),

    // DECLARATION KEYWORDS
    // --------------------------------------------------------------------------------------------
    /// Used to declare AIR program.
    Def,
    /// Used to declare AIR module.
    Mod,
    /// Used to import items from an AIR module.
    Use,
    /// Used to declare intermediate variables in the AIR constraints module.
    Let,
    /// Used to declare constants in the AIR constraints module.
    Const,
    /// Used to declare trace columns section in the AIR constraints module.
    TraceColumns,
    /// Used to declare main trace columns.
    Main,
    /// Used to declare aux trace columns.
    Aux,
    /// Keyword to declare the public inputs declaration section for the AIR.
    PublicInputs,
    /// Keyword to declare the periodic columns declaration section for the AIR.
    PeriodicColumns,
    /// Keyword to declare random values section in the AIR constraints module.
    RandomValues,
    /// Keyword to declare the evaluator function section in the AIR constraints module.
    Ev,
    /// Keyword to declare the function section in the AIR constraints module.
    Fn,

    // BUSES KEYWORDS
    // --------------------------------------------------------------------------------------------
    /// Marks the beginning of buses section in the constraints file.
    Buses,
    /// Used to represent a multiset bus declaration.
    Unit,
    /// Used to represent a logup bus declaration.
    Mult,
    /// Used to represent an empty bus
    Null,
    /// Used to represent the addition of a given tuple to a bus
    Add,
    /// Used to represent the removal of a given tuple to a bus
    Rem,

    // BOUNDARY CONSTRAINT KEYWORDS
    // --------------------------------------------------------------------------------------------
    /// Marks the beginning of boundary constraints section in the constraints file.
    BoundaryConstraints,
    /// Used to represent the first row of the column to which a boundary constraint is applied.
    First,
    /// Used to represent the last row of the column to which a boundary constraint is applied.
    Last,

    // INTEGRITY CONSTRAINT KEYWORDS
    // --------------------------------------------------------------------------------------------
    /// Marks the beginning of integrity constraints section in the constraints file.
    IntegrityConstraints,

    // LIST COMPREHENSION KEYWORDS
    // --------------------------------------------------------------------------------------------
    For,
    In,

    // GENERAL KEYWORDS
    // --------------------------------------------------------------------------------------------
    /// Keyword to signify that a constraint needs to be enforced
    Enf,
    Return,
    Match,
    Case,
    When,
    Felt,

    // PUNCTUATION
    // --------------------------------------------------------------------------------------------
    Quote,
    Colon,
    ColonColon,
    Comma,
    Dot,
    DotDot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Equal,
    Plus,
    Minus,
    Star,
    Caret,
    Ampersand,
    Bar,
    Bang,
    Arrow,
    SemiColon,
}
impl Token {
    pub fn from_keyword_or_ident(s: &str) -> Self {
        match s {
            "def" => Self::Def,
            "mod" => Self::Mod,
            "use" => Self::Use,
            "let" => Self::Let,
            "const" => Self::Const,
            "trace_columns" => Self::TraceColumns,
            "main" => Self::Main,
            "aux" => Self::Aux,
            "public_inputs" => Self::PublicInputs,
            "periodic_columns" => Self::PeriodicColumns,
            "random_values" => Self::RandomValues,
            "ev" => Self::Ev,
            "fn" => Self::Fn,
            "felt" => Self::Felt,
            "buses" => Self::Buses,
            "unit" => Self::Unit,
            "mult" => Self::Mult,
            "null" => Self::Null,
            "add" => Self::Add,
            "rem" => Self::Rem,
            "boundary_constraints" => Self::BoundaryConstraints,
            "integrity_constraints" => Self::IntegrityConstraints,
            "first" => Self::First,
            "last" => Self::Last,
            "for" => Self::For,
            "in" => Self::In,
            "enf" => Self::Enf,
            "return" => Self::Return,
            "match" => Self::Match,
            "case" => Self::Case,
            "when" => Self::When,
            other => Self::Ident(Symbol::intern(other)),
        }
    }
}
impl Eq for Token {}
impl PartialEq for Token {
    fn eq(&self, other: &Token) -> bool {
        match self {
            Self::Num(i) => {
                if let Self::Num(i2) = other {
                    return *i == *i2;
                }
            }
            Self::Error(_) => {
                if let Self::Error(_) = other {
                    return true;
                }
            }
            Self::Ident(i) => {
                if let Self::Ident(i2) = other {
                    return i == i2;
                }
            }
            Self::DeclIdentRef(i) => {
                if let Self::DeclIdentRef(i2) = other {
                    return i == i2;
                }
            }
            Self::FunctionIdent(i) => {
                if let Self::FunctionIdent(i2) = other {
                    return i == i2;
                }
            }
            _ => return mem::discriminant(self) == mem::discriminant(other),
        }
        false
    }
}
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eof => write!(f, "EOF"),
            Self::Error(_) => write!(f, "ERROR"),
            Self::Comment => write!(f, "COMMENT"),
            Self::Ident(ref id) => write!(f, "{}", id),
            Self::DeclIdentRef(ref id) => write!(f, "{}", id),
            Self::FunctionIdent(ref id) => write!(f, "{}", id),
            Self::Num(ref i) => write!(f, "{}", i),
            Self::Def => write!(f, "def"),
            Self::Mod => write!(f, "mod"),
            Self::Use => write!(f, "use"),
            Self::Let => write!(f, "let"),
            Self::Const => write!(f, "const"),
            Self::TraceColumns => write!(f, "trace_columns"),
            Self::Main => write!(f, "main"),
            Self::Aux => write!(f, "aux"),
            Self::PublicInputs => write!(f, "public_inputs"),
            Self::PeriodicColumns => write!(f, "periodic_columns"),
            Self::RandomValues => write!(f, "random_values"),
            Self::Ev => write!(f, "ev"),
            Self::Fn => write!(f, "fn"),
            Self::Felt => write!(f, "felt"),
            Self::Buses => write!(f, "buses"),
            Self::Unit => write!(f, "unit"),
            Self::Mult => write!(f, "mult"),
            Self::Null => write!(f, "null"),
            Self::Add => write!(f, "add"),
            Self::Rem => write!(f, "rem"),
            Self::BoundaryConstraints => write!(f, "boundary_constraints"),
            Self::First => write!(f, "first"),
            Self::Last => write!(f, "last"),
            Self::IntegrityConstraints => write!(f, "integrity_constraints"),
            Self::For => write!(f, "for"),
            Self::In => write!(f, "in"),
            Self::Enf => write!(f, "enf"),
            Self::Return => write!(f, "return"),
            Self::Match => write!(f, "match"),
            Self::Case => write!(f, "case"),
            Self::When => write!(f, "when"),
            Self::Quote => write!(f, "'"),
            Self::Colon => write!(f, ":"),
            Self::ColonColon => write!(f, "::"),
            Self::Comma => write!(f, ","),
            Self::Dot => write!(f, "."),
            Self::DotDot => write!(f, ".."),
            Self::LParen => write!(f, "("),
            Self::RParen => write!(f, ")"),
            Self::LBracket => write!(f, "["),
            Self::RBracket => write!(f, "]"),
            Self::LBrace => write!(f, "{{"),
            Self::RBrace => write!(f, "}}"),
            Self::Equal => write!(f, "="),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Caret => write!(f, "^"),
            Self::Ampersand => write!(f, "&"),
            Self::Bar => write!(f, "|"),
            Self::Bang => write!(f, "!"),
            Self::Arrow => write!(f, "->"),
            Self::SemiColon => write!(f, ";"),
        }
    }
}

macro_rules! pop {
    ($lex:ident) => {{
        $lex.skip();
    }};
    ($lex:ident, $code:expr) => {{
        $lex.skip();
        $code
    }};
}

macro_rules! pop2 {
    ($lex:ident) => {{
        $lex.skip();
        $lex.skip();
    }};
    ($lex:ident, $code:expr) => {{
        $lex.skip();
        $lex.skip();
        $code
    }};
}

/// The lexer that is used to perform lexical analysis on the AirScript grammar. The lexer implements
/// the `Iterator` trait, so in order to retrieve the tokens, you simply have to iterate over it.
///
/// # Errors
///
/// Because the lexer is implemented as an iterator over tokens, this means that you can continue
/// to get tokens even if a lexical error occurs. The lexer will attempt to recover from an error
/// by injecting tokens it expects.
///
/// If an error is unrecoverable, the lexer will continue to produce tokens, but there is no
/// guarantee that parsing them will produce meaningful results, it is primarily to assist in
/// gathering as many errors as possible.
pub struct Lexer<S> {
    /// The scanner produces a sequence of chars + location, and can be controlled
    /// The location type is SourceIndex
    scanner: Scanner<S>,

    /// The most recent token to be lexed.
    /// At the start and end, this should be Token::Eof
    token: Token,

    /// The position in the input where the current token starts
    /// At the start this will be the byte index of the beginning of the input
    token_start: SourceIndex,

    /// The position in the input where the current token ends
    /// At the start this will be the byte index of the beginning of the input
    token_end: SourceIndex,

    /// When we have reached true Eof, this gets set to true, and the only token
    /// produced after that point is Token::Eof, or None, depending on how you are
    /// consuming the lexer
    eof: bool,
}
impl<S> Lexer<S>
where
    S: Source,
{
    /// Produces an instance of the lexer with the lexical analysis to be performed on the `input`
    /// string. Note that no lexical analysis occurs until the lexer has been iterated over.
    pub fn new(scanner: Scanner<S>) -> Self {
        use miden_diagnostics::ByteOffset;

        let start = scanner.start();
        let mut lexer = Lexer {
            scanner,
            token: Token::Eof,
            token_start: start + ByteOffset(0),
            token_end: start + ByteOffset(0),
            eof: false,
        };
        lexer.advance();
        lexer
    }

    pub fn lex(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.eof && self.token == Token::Eof {
            return None;
        }

        let token = std::mem::replace(&mut self.token, Token::Eof);
        let start = self.token_start;
        let end = self.token_end;
        self.advance();
        match token {
            Token::Error(err) => Some(Err(err.into())),
            token => Some(Ok((start, token, end))),
        }
    }

    fn advance(&mut self) {
        self.advance_start();
        self.token = self.tokenize();
    }

    #[inline]
    fn advance_start(&mut self) {
        let mut position: SourceIndex;
        loop {
            let (pos, c) = self.scanner.read();

            position = pos;

            if c == '\0' {
                self.eof = true;
                return;
            }

            if c.is_whitespace() {
                self.scanner.advance();
                continue;
            }

            break;
        }

        self.token_start = position;
    }

    #[inline]
    fn pop(&mut self) -> char {
        use miden_diagnostics::ByteOffset;

        let (pos, c) = self.scanner.pop();
        self.token_end = pos + ByteOffset::from_char_len(c);
        c
    }

    #[inline]
    fn peek(&mut self) -> char {
        let (_, c) = self.scanner.peek();
        c
    }

    #[inline]
    fn read(&mut self) -> char {
        let (_, c) = self.scanner.read();
        c
    }

    #[inline]
    fn skip(&mut self) {
        self.pop();
    }

    /// Get the span for the current token in `Source`.
    #[inline]
    fn span(&self) -> SourceSpan {
        SourceSpan::new(self.token_start, self.token_end)
    }

    /// Get a string slice of the current token.
    #[inline]
    fn slice(&self) -> &str {
        self.scanner.slice(self.span())
    }

    #[inline]
    fn skip_whitespace(&mut self) {
        let mut c: char;
        loop {
            c = self.read();

            if !c.is_whitespace() {
                break;
            }

            self.skip();
        }
    }

    fn tokenize(&mut self) -> Token {
        let c = self.read();

        if c == '#' {
            self.skip();
            return self.lex_comment();
        }

        if c == '\0' {
            self.eof = true;
            return Token::Eof;
        }

        if c.is_whitespace() {
            self.skip_whitespace();
        }

        match self.read() {
            ',' => pop!(self, Token::Comma),
            '.' => match self.peek() {
                '.' => pop2!(self, Token::DotDot),
                _ => pop!(self, Token::Dot),
            },
            ':' => match self.peek() {
                ':' => pop2!(self, Token::ColonColon),
                _ => pop!(self, Token::Colon),
            },
            '\'' => pop!(self, Token::Quote),
            '(' => pop!(self, Token::LParen),
            ')' => pop!(self, Token::RParen),
            '[' => pop!(self, Token::LBracket),
            ']' => pop!(self, Token::RBracket),
            '{' => pop!(self, Token::LBrace),
            '}' => pop!(self, Token::RBrace),
            '=' => pop!(self, Token::Equal),
            '+' => pop!(self, Token::Plus),
            '-' => match self.peek() {
                '>' => pop2!(self, Token::Arrow),
                _ => pop!(self, Token::Minus),
            },
            '*' => pop!(self, Token::Star),
            '^' => pop!(self, Token::Caret),
            '&' => pop!(self, Token::Ampersand),
            '|' => pop!(self, Token::Bar),
            '!' => pop!(self, Token::Bang),
            ';' => pop!(self, Token::SemiColon),
            '$' => self.lex_special_identifier(),
            '0'..='9' => self.lex_number(),
            'a'..='z' => self.lex_keyword_or_ident(),
            'A'..='Z' => self.lex_identifier(),
            c => Token::Error(LexicalError::UnexpectedCharacter {
                start: self.span().start(),
                found: c,
            }),
        }
    }

    fn lex_comment(&mut self) -> Token {
        let mut c;
        loop {
            c = self.read();

            if c == '\n' {
                break;
            }

            if c == '\0' {
                self.eof = true;
                break;
            }

            self.skip();
        }

        Token::Comment
    }

    #[inline]
    fn lex_special_identifier(&mut self) -> Token {
        let c = self.pop();
        debug_assert!(c == '$');

        // Must start with an alphabetic character
        match self.read() {
            c if c.is_ascii_alphabetic() => (),
            c => {
                return Token::Error(LexicalError::UnexpectedCharacter {
                    start: self.span().start(),
                    found: c,
                })
            }
        }

        self.skip_ident();

        Token::DeclIdentRef(Symbol::intern(self.slice()))
    }

    #[inline]
    fn lex_keyword_or_ident(&mut self) -> Token {
        let c = self.pop();
        debug_assert!(c.is_ascii_alphabetic() && c.is_lowercase());

        self.skip_ident();

        let next = self.read();
        match Token::from_keyword_or_ident(self.slice()) {
            Token::Ident(id) if next == '(' => Token::FunctionIdent(id),
            //Token::Ident(id) if next == '.' => Token::BusIdent(id),
            token => token,
        }
    }

    #[inline]
    fn lex_identifier(&mut self) -> Token {
        let c = self.pop();
        debug_assert!(c.is_ascii_alphabetic());

        self.skip_ident();

        if self.read() == '(' {
            Token::FunctionIdent(Symbol::intern(self.slice()))
        } else {
            Token::Ident(Symbol::intern(self.slice()))
        }
    }

    fn skip_ident(&mut self) {
        loop {
            match self.read() {
                '_' => self.skip(),
                '0'..='9' => self.skip(),
                c if c.is_ascii_alphabetic() => self.skip(),
                _ => break,
            }
        }
    }

    #[inline]
    fn lex_number(&mut self) -> Token {
        let mut num = String::new();

        // Expect the first character to be a digit
        debug_assert!(self.read().is_ascii_digit());

        while let '0'..='9' = self.read() {
            num.push(self.pop());
        }

        match num.parse::<u64>() {
            Ok(i) => Token::Num(i),
            Err(err) => Token::Error(LexicalError::InvalidInt {
                span: self.span(),
                reason: err.kind().clone(),
            }),
        }
    }
}

impl<S> Iterator for Lexer<S>
where
    S: Source,
{
    type Item = Lexed;

    fn next(&mut self) -> Option<Self::Item> {
        let mut res = self.lex();
        while let Some(Ok((_, Token::Comment, _))) = res {
            res = self.lex();
        }
        res
    }
}

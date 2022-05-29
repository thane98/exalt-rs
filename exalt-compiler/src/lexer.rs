use logos::{Lexer, Logos};
use std::fmt::Display;
use std::ops::Range;

/// Tokens recognized by the Exalt lexer
#[derive(Logos, Debug, PartialEq, Copy, Clone)]
pub enum Token {
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("+f")]
    FloatPlus,
    #[token("-f")]
    FloatMinus,
    #[token("*")]
    Times,
    #[token("/")]
    Divide,
    #[token("*f")]
    FloatTimes,
    #[token("/f")]
    FloatDivide,
    #[token("%")]
    Modulo,
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token("==f")]
    FloatEqual,
    #[token("!=f")]
    FloatNotEqual,
    #[token("<")]
    LessThan,
    #[token("<=")]
    LessThanOrEqualTo,
    #[token(">")]
    GreaterThan,
    #[token(">=")]
    GreaterThanOrEqualTo,
    #[token("<f")]
    FloatLessThan,
    #[token("<=f")]
    FloatLessThanOrEqualTo,
    #[token(">f")]
    FloatGreaterThan,
    #[token(">=f")]
    FloatGreaterThanOrEqualTo,
    #[token(">>")]
    RightShift,
    #[token("<<")]
    LeftShift,
    #[token("&")]
    Ampersand,
    #[token("|")]
    BinaryOr,
    #[token("^")]
    Xor,
    #[token("&&")]
    LogicalAnd,
    #[token("||")]
    LogicalOr,
    #[token("~")]
    BinaryNot,
    #[token("!")]
    LogicalNot,
    #[token("++")]
    Increment,
    #[token("--")]
    Decrement,
    #[token("=")]
    Assign,
    #[token("+=")]
    AssignAdd,
    #[token("-=")]
    AssignSubtract,
    #[token("*=")]
    AssignMultiply,
    #[token("/=")]
    AssignDivide,
    #[token("%=")]
    AssignModulo,
    #[token("|=")]
    AssignBinaryOr,
    #[token("&=")]
    AssignBinaryAnd,
    #[token("^=")]
    AssignXor,
    #[token(">>=")]
    AssignRightShift,
    #[token("<<=")]
    AssignLeftShift,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token(";")]
    Semicolon,
    #[token("->")]
    Arrow,
    #[token("@")]
    AtSign,
    #[token(".")]
    Dot,
    #[token("array")]
    Array,
    #[token("break")]
    Break,
    #[token("callback")]
    Callback,
    #[token("const")]
    Const,
    #[token("continue")]
    Continue,
    #[token("def")]
    Def,
    #[token("else")]
    Else,
    #[token("enum")]
    Enum,
    #[token("for")]
    For,
    #[token("goto")]
    Goto,
    #[token("if")]
    If,
    #[token("label")]
    Label,
    #[token("let")]
    Let,
    #[token("match")]
    Match,
    #[token("printf")]
    Printf,
    #[token("return")]
    Return,
    #[token("static")]
    Static,
    #[token("struct")]
    Struct,
    // This is reserved since it's used internally as a data type
    #[token("Void")]
    Void,
    #[token("while")]
    While,
    #[token("yield")]
    Yield,
    #[regex(r"([0-9]+)|(0x[0-9a-fA-F]+)|(0b[01]+)|(0o[0-7]+)")]
    Int,
    #[regex(r"([0-9]*[.])[0-9]+")]
    Float,
    #[regex(r"[^\W0-9](::|\w|・|？)*")]
    Identifier,
    #[regex("\"[^\"]*\"")]
    Str,
    #[error]
    #[regex(r"([ \t\n\r\f]+)|(//[^\n]*)", logos::skip)]
    Error,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Token::Plus => "+",
                Token::Minus => "-",
                Token::FloatPlus => "+f",
                Token::FloatMinus => "-f",
                Token::Times => "*",
                Token::Divide => "/",
                Token::FloatTimes => "*f",
                Token::FloatDivide => "/f",
                Token::Modulo => "%",
                Token::Equal => "==",
                Token::NotEqual => "!=",
                Token::FloatEqual => "==f",
                Token::FloatNotEqual => "!=f",
                Token::LessThan => "<",
                Token::LessThanOrEqualTo => "<=",
                Token::GreaterThan => ">",
                Token::GreaterThanOrEqualTo => ">=",
                Token::FloatLessThan => "<f",
                Token::FloatLessThanOrEqualTo => "<=f",
                Token::FloatGreaterThan => ">f",
                Token::FloatGreaterThanOrEqualTo => ">=f",
                Token::RightShift => ">>",
                Token::LeftShift => "<<",
                Token::Ampersand => "&",
                Token::BinaryOr => "|",
                Token::Xor => "^",
                Token::LogicalAnd => "&&",
                Token::LogicalOr => "||",
                Token::BinaryNot => "~",
                Token::LogicalNot => "!",
                Token::Increment => "++",
                Token::Decrement => "--",
                Token::Assign => "=",
                Token::AssignAdd => "+=",
                Token::AssignSubtract => "-=",
                Token::AssignMultiply => "*=",
                Token::AssignDivide => "/=",
                Token::AssignModulo => "%=",
                Token::AssignBinaryOr => "|=",
                Token::AssignBinaryAnd => "&=",
                Token::AssignXor => "^=",
                Token::AssignRightShift => ">>=",
                Token::AssignLeftShift => "<<=",
                Token::Comma => ",",
                Token::Colon => ":",
                Token::LeftParen => "(",
                Token::RightParen => ")",
                Token::LeftBracket => "[",
                Token::RightBracket => "]",
                Token::LeftBrace => "{",
                Token::RightBrace => "}",
                Token::Semicolon => ";",
                Token::Arrow => "->",
                Token::AtSign => "@",
                Token::Dot => ".",
                Token::Array => "array",
                Token::Break => "break",
                Token::Callback => "callback",
                Token::Const => "const",
                Token::Continue => "continue",
                Token::Else => "else",
                Token::Enum => "enum",
                Token::For => "for",
                Token::Def => "func",
                Token::Goto => "goto",
                Token::If => "if",
                Token::Label => "label",
                Token::Let => "let",
                Token::Match => "match",
                Token::Static => "static",
                Token::Struct => "struct",
                Token::Printf => "printf",
                Token::Return => "return",
                Token::Void => "void",
                Token::While => "while",
                Token::Yield => "yield",
                Token::Int => "int",
                Token::Float => "float",
                Token::Identifier => "identifier",
                Token::Str => "string",
                Token::Error => "<error>",
            }
        )
    }
}

/// Wrapper for logos's lexer that supports peeking/lookahead
pub struct Peekable<'source> {
    lexer: Lexer<'source, Token>,
    peeked: Option<Option<Token>>,
}

impl<'source> Peekable<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            lexer: Token::lexer(source),
            peeked: None,
        }
    }

    pub fn peek(&mut self) -> Option<Token> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next());
        }
        self.peeked.unwrap()
    }

    pub fn slice(&self) -> &'source str {
        self.lexer.slice()
    }

    pub fn span(&self) -> Range<usize> {
        self.lexer.span()
    }
}

impl<'source> Iterator for Peekable<'source> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if let Some(peeked) = self.peeked.take() {
            peeked
        } else {
            self.lexer.next()
        }
    }
}

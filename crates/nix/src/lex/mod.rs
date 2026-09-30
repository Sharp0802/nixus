use std::fmt::Display;
use std::num::{IntErrorKind, ParseFloatError, ParseIntError};
use std::ops::Range;

use crate::macros::Span;

#[derive(Clone, Debug)]
enum ErrorKind {
    InvalidDigit,
    IntegerEmpty,
    IntegerOverflow,
    IntegerUnderflow,
    NotFloat,
    FloatEmpty,
    NotString,
    StringUnclosed,
    IdentEmpty,
    NotQuote,
    Complex(Vec<Self>),
}

impl ErrorKind {
    fn merge(mut self, mut other: Self) -> Self {
        if let Self::Complex(vec) = &mut self {
            vec.push(other);
            self
        } else if let Self::Complex(vec) = &mut other {
            vec.push(self);
            other
        } else {
            let mut errors = Vec::with_capacity(5);
            errors.push(self);
            errors.push(other);
            Self::Complex(errors)
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDigit => write!(f, "invalid digit"),
            Self::IntegerEmpty => write!(f, "integer cannot be empty"),
            Self::IntegerOverflow => write!(f, "integer too big"),
            Self::IntegerUnderflow => write!(f, "integer too small"),
            Self::NotFloat => write!(f, "it is not float"),
            Self::FloatEmpty => write!(f, "float cannot be empty"),
            Self::NotString => write!(f, "it is not string"),
            Self::StringUnclosed => write!(f, "string unclosed"),
            Self::IdentEmpty => write!(f, "identifier cannot be empty"),
            Self::NotQuote => write!(f, "it is not quote"),
            Self::Complex(errors) => {
                for (i, error) in errors.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", and ")?;
                    }

                    write!(f, "{error}")?;
                }

                Ok(())
            }
        }
    }
}

impl From<ParseIntError> for ErrorKind {
    fn from(value: ParseIntError) -> Self {
        match value.kind() {
            IntErrorKind::Empty => Self::IntegerEmpty,
            IntErrorKind::InvalidDigit => Self::InvalidDigit,
            IntErrorKind::PosOverflow => Self::IntegerOverflow,
            IntErrorKind::NegOverflow => Self::IntegerUnderflow,
            _ => unreachable!(),
        }
    }
}

impl From<ParseFloatError> for ErrorKind {
    fn from(_value: ParseFloatError) -> Self {
        Self::InvalidDigit
    }
}

pub trait Span {
    fn range(&self) -> Range<usize>;
    fn offset(&mut self, offset: usize);

    fn len(&self) -> usize {
        self.range().len()
    }
}

trait Lex<'a>: Sized {
    fn lex(src: &'a str) -> Result<Self, ErrorKind>;
}

#[derive(Clone, Debug, PartialEq, Eq, Span)]
pub struct IntegerLiteral {
    pub range: Range<usize>,
    pub value: i64,
}

impl PartialEq<i64> for IntegerLiteral {
    fn eq(&self, other: &i64) -> bool {
        self.value == *other
    }
}

impl Lex<'_> for IntegerLiteral {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        let mut i = 0;
        while i < src.len() && src.as_bytes()[i].is_ascii_digit() {
            i += 1;
        }

        let value: i64 = src[..i].parse()?;

        Ok(Self { range: 0..i, value })
    }
}

#[derive(Clone, Debug, PartialEq, Span)]
pub struct FloatLiteral {
    pub range: Range<usize>,
    pub value: f64,
}

impl PartialEq<f64> for FloatLiteral {
    fn eq(&self, other: &f64) -> bool {
        self.value == *other
    }
}

impl Lex<'_> for FloatLiteral {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        let mut dot = 1;
        let mut i = usize::from(src.starts_with('+') || src.starts_with('-'));
        while i < src.len() {
            let ch = src.as_bytes()[i];
            if ch == b'.' {
                if dot <= 0 {
                    break;
                }

                dot -= 1;
            } else if !ch.is_ascii_digit() {
                break;
            }

            i += 1;
        }
        if dot == 1 || src.as_bytes()[..i].last() == Some(&b'.') {
            return Err(ErrorKind::NotFloat);
        }
        if i == 0 {
            return Err(ErrorKind::FloatEmpty);
        }

        let value: f64 = src[..i].parse()?;

        Ok(Self { range: 0..i, value })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Span)]
pub struct StringLiteral<'a> {
    pub range: Range<usize>,
    pub value: &'a str,
}

impl PartialEq<&str> for StringLiteral<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.value == *other
    }
}

impl<'a> Lex<'a> for StringLiteral<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        if src.as_bytes().first() != Some(&b'"') {
            return Err(ErrorKind::NotString);
        }

        let mut i = 1;
        while i < src.len() && src.as_bytes()[i] != b'\"' {
            if src.as_bytes()[i..].starts_with(b"\\\"") {
                i += 2;
            } else {
                i += 1;
            }
        }
        if i >= src.len() {
            return Err(ErrorKind::StringUnclosed);
        }

        let value = &src[1..i];
        i += 1;

        Ok(Self { range: 0..i, value })
    }
}

#[derive(Clone, Debug, PartialEq, Span)]
pub enum Literal<'a> {
    Integer(IntegerLiteral),
    Float(FloatLiteral),
    String(StringLiteral<'a>),
}

impl PartialEq<i64> for Literal<'_> {
    fn eq(&self, other: &i64) -> bool {
        matches!(self, Self::Integer(int) if int == other)
    }
}

impl PartialEq<f64> for Literal<'_> {
    fn eq(&self, other: &f64) -> bool {
        matches!(self, Self::Float(f) if f == other)
    }
}

impl PartialEq<&str> for Literal<'_> {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, Self::String(str) if str == other)
    }
}

impl<'a> Lex<'a> for Literal<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        FloatLiteral::lex(src)
            .map(Self::Float)
            .or_else(|e| {
                IntegerLiteral::lex(src)
                    .map(Self::Integer)
                    .map_err(|e2| e.merge(e2))
            })
            .or_else(|e| {
                StringLiteral::lex(src)
                    .map(Self::String)
                    .map_err(|e2| e.merge(e2))
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Span)]
pub struct Ident<'a> {
    pub range: Range<usize>,
    pub value: &'a str,
}

impl PartialEq<&str> for Ident<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.value == *other
    }
}

impl<'a> Lex<'a> for Ident<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        let mut len = 0;
        for ch in src.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                break;
            }

            len += ch.len_utf8();
        }
        if len == 0 {
            return Err(ErrorKind::IdentEmpty);
        }

        Ok(Self {
            range: 0..len,
            value: &src[0..len],
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Span)]
pub struct Quote {
    pub range: Range<usize>,
    pub value: &'static str,
}

impl PartialEq<&str> for Quote {
    fn eq(&self, other: &&str) -> bool {
        self.value == *other
    }
}

impl Lex<'_> for Quote {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        macro_rules! entries {
            ($($lit:literal),* $(,)?) => {
                $(if src.starts_with($lit) {
                    return Ok(Self { range: 0..$lit.len(), value: $lit });
                })*
            };
        }

        #[rustfmt::skip]
        entries![
            "::", ":",
            "..", ".",
            "==", "=", "!=",
            "${", "{", "}",
            "[", "]",
            "<", ">",
            "+", "-", "*", "/", "%",
            "||", "|",
            "&&", "&",
            "!", "^",
            ",",
        ];

        Err(ErrorKind::NotQuote)
    }
}

#[derive(Clone, Debug, PartialEq, Span)]
pub enum Token<'a> {
    Literal(Literal<'a>),
    Ident(Ident<'a>),
    Quote(Quote),
}

impl<'a> Token<'a> {
    pub const fn as_literal(&self) -> Option<&Literal<'a>> {
        match self {
            Self::Literal(lit) => Some(lit),
            _ => None,
        }
    }

    pub const fn as_ident(&self) -> Option<&Ident<'a>> {
        match self {
            Self::Ident(id) => Some(id),
            _ => None,
        }
    }

    pub const fn as_quote(&self) -> Option<&Quote> {
        match self {
            Self::Quote(quote) => Some(quote),
            _ => None,
        }
    }
}

impl<'a> Lex<'a> for Token<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        Literal::lex(src)
            .map(Self::Literal)
            .or_else(|e| Ident::lex(src).map(Self::Ident).map_err(|e2| e.merge(e2)))
            .or_else(|e| Quote::lex(src).map(Self::Quote).map_err(|e2| e.merge(e2)))
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    line: usize,
    column: usize,
    kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.kind)
    }
}

pub fn lex(str: &str) -> Result<Vec<Token<'_>>, Error> {
    let mut vec = Vec::new();

    let mut i = 0;
    while {
        while i < str.len() && str.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }

        i < str.len()
    } {
        let mut tk = Token::lex(&str[i..]).map_err(|e| {
            let mut line = 0;
            let mut column = 0;

            let mut j = 0;
            while j < i {
                match str.as_bytes()[i..] {
                    [b'\r', b'\n', ..] => {
                        j += 2;
                        line += 1;
                        column = 0;
                    }
                    [b'\r' | b'\n', ..] => {
                        j += 1;
                        line += 1;
                        column = 0;
                    }
                    _ => {
                        j += 1;
                        column += 1;
                    }
                }
            }

            Error {
                line,
                column,
                kind: e,
            }
        })?;
        tk.offset(i);
        i = tk.range().end;
        vec.push(tk);
    }

    Ok(vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_ident() {
        assert_eq!(Token::lex("hello").unwrap().as_ident().unwrap(), &"hello");
        assert_eq!(Token::lex("h3ll0").unwrap().as_ident().unwrap(), &"h3ll0");
        assert_eq!(Token::lex("ha wo").unwrap().as_ident().unwrap(), &"ha");
        assert!(Token::lex("4ell0").unwrap().as_ident().is_none());
    }

    #[test]
    fn lex_literal() {
        assert_eq!(Token::lex("1.25").unwrap().as_literal().unwrap(), &1.25);
        assert_eq!(Token::lex("125").unwrap().as_literal().unwrap(), &125);
        assert_eq!(Token::lex("4ell0").unwrap().as_literal().unwrap(), &4);
    }

    #[test]
    fn lex_quote() {
        assert_eq!(Token::lex("..").unwrap().as_quote().unwrap(), &"..");
        assert_eq!(Token::lex("!=").unwrap().as_quote().unwrap(), &"!=");
        assert_eq!(Token::lex("!.").unwrap().as_quote().unwrap(), &"!");
    }
}

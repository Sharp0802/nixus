use std::ops::Range;

use crate::macros::Span;

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

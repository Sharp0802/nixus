use std::fmt::Display;
use std::ops::Range;

use crate::lex::QuoteId;

#[derive(Clone, Debug)]
pub struct Error {
    pub range: Range<usize>,
    pub kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}: {}", self.range.start, self.range.end, self.kind)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub enum ErrorKind {
    Empty,
    NotExpr,
    NotSubst,
    NotPath,
    NotPathSegment,
    NotParameter,
    DuplicateParameter,
    PathUnclosed,
    Missing(QuoteId),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "expression cannot be empty"),
            Self::NotExpr => write!(f, "it is not expression"),
            Self::NotSubst => write!(f, "it is not substitution"),
            Self::NotPath => write!(f, "it is not path"),
            Self::NotPathSegment => write!(f, "it is not path segment"),
            Self::NotParameter => write!(f, "invalid function parameter"),
            Self::DuplicateParameter => write!(f, "duplicate function parameter"),
            Self::PathUnclosed => write!(f, "path not closed"),
            Self::Missing(id) => write!(f, "missing '{}'", id.to_str()),
        }
    }
}

impl std::error::Error for ErrorKind {}

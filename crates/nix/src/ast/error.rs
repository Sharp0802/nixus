use std::fmt::Display;
use std::ops::Range;

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
    NotSubst,
    NotPath,
    NotPathSegment,
    PathUnclosed,
    Missing(&'static str),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "expression cannot be empty"),
            Self::NotSubst => write!(f, "it is not substitution"),
            Self::NotPath => write!(f, "it is not path"),
            Self::NotPathSegment => write!(f, "it is not path segment"),
            Self::PathUnclosed => write!(f, "path not closed"),
            Self::Missing(str) => write!(f, "missing '{str}'"),
        }
    }
}

impl std::error::Error for ErrorKind {}

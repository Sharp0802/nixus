use std::fmt::Display;
use std::num::{IntErrorKind, ParseFloatError, ParseIntError};

#[derive(Clone, Debug)]
pub struct Error {
    pub pos: usize,
    pub kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}: {}", self.pos, self.kind)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

#[derive(Clone, Debug)]
pub enum ErrorKind {
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
    pub fn merge(mut self, mut other: Self) -> Self {
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

impl std::error::Error for ErrorKind {}

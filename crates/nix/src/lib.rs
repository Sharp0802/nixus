#![doc = include_str!("../README.md")]

use std::{fmt::Display, ops::Range};

mod ast;
mod lex;

use ast::{Expr, Parse};
use lex::Span;
use nixus_macros as macros;

#[derive(Clone, Debug)]
pub enum Error {
    Ast(ast::Error),
    Lex(lex::Error),
    ExtraToken(Range<usize>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ast(v) => write!(f, "{v}"),
            Self::Lex(v) => write!(f, "{v}"),
            Self::ExtraToken(Range { start, end }) => {
                write!(f, "{start}..{end}: extraneous tokens")
            }
        }
    }
}

impl From<ast::Error> for Error {
    fn from(value: ast::Error) -> Self {
        Self::Ast(value)
    }
}

impl From<lex::Error> for Error {
    fn from(value: lex::Error) -> Self {
        Self::Lex(value)
    }
}

pub fn parse(str: &str) -> Result<Expr<'_>, Error> {
    let tokens = lex::lex(str)?;
    let mut iter = tokens.into_iter().peekable();

    let expr = Expr::parse(&mut iter)?;

    if let Some(extra) = iter.next() {
        let end = iter
            .last()
            .map_or_else(|| extra.range().end, |tk| tk.range().end);
        return Err(Error::ExtraToken(extra.range().start..end));
    }

    Ok(expr)
}

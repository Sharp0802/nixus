use std::ops::Range;

use crate::lex::Ident;
use crate::lex::Literal;
use crate::lex::Quote;
use crate::macros::Span;

#[derive(Clone, Debug, PartialEq, Span)]
pub enum Expr<'a> {
    Literal(Literal<'a>),
    Path(Path<'a>),
    Array(Array<'a>),
    Set(Set<'a>),
    Subst(Subst<'a>),
}

#[derive(Clone, Debug, PartialEq, Span)]
pub enum PathSegment<'a> {
    Ident(Ident<'a>),
    Subst(Subst<'a>),
}

#[derive(Clone, Debug, PartialEq, Span)]
pub struct Path<'a> {
    pub range: Range<usize>,
    pub segments: Vec<(PathSegment<'a>, Option<Quote>)>,
}

#[derive(Clone, Debug, PartialEq, Span)]
pub struct Field<'a> {
    pub range: Range<usize>,
    pub key: Path<'a>,
    pub eq: Quote,
    pub value: Expr<'a>,
    pub semicolon: Quote,
}

#[derive(Clone, Debug, PartialEq, Span)]
pub struct Subst<'a> {
    pub range: Range<usize>,
    pub open: Quote,
    pub close: Quote,
    pub value: Box<Expr<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Span)]
pub struct Group<T, const P: u8, const Q: u8> {
    pub range: Range<usize>,
    pub open: Quote,
    pub close: Quote,
    pub items: Vec<T>,
}

pub type Array<'a> = Group<Expr<'a>, b'[', b']'>;
pub type Set<'a> = Group<Field<'a>, b'{', b'}'>;

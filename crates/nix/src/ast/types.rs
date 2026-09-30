use std::ops::Range;

use crate::lex::Ident;
use crate::lex::Literal;
use crate::lex::Quote;
use crate::lex::QuoteId;
use crate::macros::Span;

#[derive(Clone, Debug, PartialEq, Span)]
pub enum Expr<'a> {
    Literal(Literal<'a>),
    Path(Path<'a>),
    Array(Array<'a>),
    Set(Set<'a>),
    Subst(Subst<'a>),
    Operation(Operation<'a>),
    Call(Call<'a>),
    Group(Group<'a>),
}

#[derive(Clone, Debug, PartialEq, Span)]
pub struct Call<'a> {
    pub range: Range<usize>,
    pub callee: Box<Expr<'a>>,
    pub argument: Box<Expr<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand<'a> {
    Unary(Expr<'a>),
    Binary(Expr<'a>, Expr<'a>),
}

#[derive(Clone, Debug, PartialEq, Span)]
pub struct Operation<'a> {
    pub range: Range<usize>,
    pub operator: Quote,
    pub operand: Box<Operand<'a>>,
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
pub struct Boxed<'a, const P: u8, const Q: u8> {
    pub range: Range<usize>,
    pub open: Quote,
    pub close: Quote,
    pub value: Box<Expr<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Span)]
pub struct Punctuated<T, const P: u8, const Q: u8> {
    pub range: Range<usize>,
    pub open: Quote,
    pub close: Quote,
    pub items: Vec<T>,
}

pub type Subst<'a> = Boxed<'a, { QuoteId::SubstL as u8 }, { QuoteId::BraceR as u8 }>;
pub type Group<'a> = Boxed<'a, { QuoteId::ParenL as u8 }, { QuoteId::ParenR as u8 }>;
pub type Array<'a> = Punctuated<Expr<'a>, { QuoteId::BracketL as u8 }, { QuoteId::BracketR as u8 }>;
pub type Set<'a> = Punctuated<Field<'a>, { QuoteId::BraceL as u8 }, { QuoteId::BraceR as u8 }>;

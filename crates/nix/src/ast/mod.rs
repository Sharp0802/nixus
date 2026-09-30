use std::iter::Peekable;
use std::vec;

use crate::lex::{Quote, QuoteId, Span, Token};

mod error;
mod stack;
mod types;

pub use error::*;
use stack::*;
pub use types::*;

type Cx<'a> = Peekable<vec::IntoIter<Token<'a>>>;

pub trait Parse<'a>: Sized {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error>;
}

impl<'a> Parse<'a> for Expr<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let mut stack = Stack::default();
        loop {
            let Some(tk) = cx.peek() else {
                let end = stack.end();
                return Err(Error {
                    range: end..end,
                    kind: ErrorKind::Empty,
                });
            };

            let expr = match tk {
                Token::Literal(_literal) => {
                    let literal = cx.next().unwrap().into_literal().unwrap();
                    Self::Literal(literal)
                }

                Token::Ident(..)
                | Token::Quote(Quote {
                    value: QuoteId::SubstL,
                    ..
                }) => {
                    let path = Path::parse(cx)?;
                    Self::Path(path)
                }

                Token::Quote(Quote {
                    value: QuoteId::ParenL,
                    ..
                }) => {
                    let group = Group::parse(cx)?;
                    Self::Group(group)
                }
                Token::Quote(Quote {
                    value: QuoteId::BraceL,
                    ..
                }) => {
                    let set = Set::parse(cx)?;
                    Self::Set(set)
                }
                Token::Quote(Quote {
                    value: QuoteId::BracketL,
                    ..
                }) => {
                    let array = Array::parse(cx)?;
                    Self::Array(array)
                }

                Token::Quote(Quote {
                    value: QuoteId::Sub,
                    ..
                }) => {
                    let operator = cx.next().unwrap().into_quote().unwrap();
                    stack.push_unary(operator);
                    continue;
                }

                Token::Quote(Quote { .. }) => {
                    return Err(Error {
                        range: tk.range(),
                        kind: ErrorKind::NotExpr,
                    });
                }
            };

            if let Some(expr) = stack.reduce(expr, cx) {
                return Ok(expr);
            }
        }
    }
}

impl<'a, T: Parse<'a> + Span, const P: u8, const Q: u8> Parse<'a> for Punctuated<T, P, Q> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let open_id = const { QuoteId::transmute(P) };
        let close_id = const { QuoteId::transmute(Q) };

        let open = cx
            .next()
            .ok_or(0..0)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::Missing(open_id),
            })?;
        if open != open_id {
            return Err(Error {
                range: open.range,
                kind: ErrorKind::Missing(open_id),
            });
        }

        let mut vec = Vec::new();
        while let Some(tk) = cx.peek() {
            if tk.as_quote().is_some_and(|quote| quote == &close_id) {
                break;
            }

            let item = T::parse(cx)?;
            vec.push(item);
        }

        let last = vec.last().map_or(open.range.end, |f| f.range().end);
        let close = cx
            .next()
            .ok_or(last..last)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::Missing(close_id),
            })?;
        if close != close_id {
            return Err(Error {
                range: close.range,
                kind: ErrorKind::Missing(close_id),
            });
        }

        Ok(Self {
            range: open.range().start..close.range().end,
            open,
            close,
            items: vec,
        })
    }
}

impl<'a> Parse<'a> for Field<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let path = Path::parse(cx)?;

        let eq = cx
            .next()
            .ok_or_else(|| path.range().end..path.range().end)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::Missing(QuoteId::Eq),
            })?;
        if eq != QuoteId::Eq {
            return Err(Error {
                range: eq.range,
                kind: ErrorKind::Missing(QuoteId::Eq),
            });
        }

        let expr = Expr::parse(cx)?;

        let semicolon = cx
            .next()
            .ok_or_else(|| expr.range().end..expr.range().end)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::Missing(QuoteId::Semicolon),
            })?;
        if semicolon != QuoteId::Semicolon {
            return Err(Error {
                range: semicolon.range,
                kind: ErrorKind::Missing(QuoteId::Semicolon),
            });
        }

        Ok(Self {
            range: path.range().start..semicolon.range().end,
            key: path,
            eq,
            value: expr,
            semicolon,
        })
    }
}

impl<'a, const P: u8, const Q: u8> Parse<'a> for Boxed<'a, P, Q> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let open_id = const { QuoteId::transmute(P) };
        let close_id = const { QuoteId::transmute(Q) };

        let open = cx
            .next()
            .ok_or(0..0)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::Missing(open_id),
            })?;
        if open != open_id {
            return Err(Error {
                range: open.range(),
                kind: ErrorKind::Missing(open_id),
            });
        }

        let expr = Expr::parse(cx)?;

        let end = expr.range().end;
        let close = cx
            .next()
            .ok_or(end..end)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::Missing(close_id),
            })?;
        if close != close_id {
            return Err(Error {
                range: close.range(),
                kind: ErrorKind::Missing(close_id),
            });
        }

        Ok(Self {
            range: open.range.start..close.range.end,
            open,
            close,
            value: Box::new(expr),
        })
    }
}

impl<'a> Parse<'a> for PathSegment<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let tk = cx.peek().ok_or(Error {
            range: 0..0,
            kind: ErrorKind::NotPathSegment,
        })?;

        match tk {
            Token::Ident(_ident) => {
                let ident = cx.next().unwrap().into_ident().unwrap();
                Ok(Self::Ident(ident))
            }
            Token::Quote(Quote {
                value: QuoteId::SubstL,
                ..
            }) => {
                let subst = Subst::parse(cx)?;
                Ok(Self::Subst(subst))
            }
            _ => Err(Error {
                range: tk.range(),
                kind: ErrorKind::NotPathSegment,
            }),
        }
    }
}

impl<'a> Parse<'a> for Path<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let mut tmp = PathSegment::parse(cx)?;
        let start = tmp.range().start;

        let mut vec = Vec::new();
        loop {
            if !matches!(
                cx.peek().and_then(|tk| tk.as_quote()),
                Some(Quote {
                    value: QuoteId::Dot,
                    ..
                })
            ) {
                break;
            }

            let dot = cx.next().unwrap().into_quote().unwrap();
            vec.push((tmp, Some(dot)));

            tmp = PathSegment::parse(cx)?;
        }

        let end = tmp.range().end;
        vec.push((tmp, None));

        Ok(Path {
            range: start..end,
            segments: vec,
        })
    }
}

use std::vec;

use crate::lex::{Literal, PathLiteral, Quote, QuoteId, Span, StringLiteral, Token};

mod error;
mod parameter;
mod stack;
mod types;

pub use error::*;
use stack::*;
pub use types::*;

pub struct Cx<'a> {
    tokens: vec::IntoIter<Token<'a>>,
    end: usize,
}

impl<'a> Cx<'a> {
    pub fn new(tokens: Vec<Token<'a>>) -> Self {
        Self {
            tokens: tokens.into_iter(),
            end: 0,
        }
    }

    fn peek(&self) -> Option<&Token<'a>> {
        self.peek_nth(0)
    }

    fn peek_nth(&self, index: usize) -> Option<&Token<'a>> {
        self.tokens.as_slice().get(index)
    }

    fn next_if(&mut self, predicate: impl FnOnce(&Token<'a>) -> bool) -> Option<Token<'a>> {
        if self.peek().is_some_and(predicate) {
            self.next()
        } else {
            None
        }
    }
}

impl<'a> Iterator for Cx<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.tokens.next()?;
        self.end = token.range().end;
        Some(token)
    }

    fn last(self) -> Option<Self::Item> {
        self.tokens.last()
    }
}

pub trait Parse<'a>: Sized {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error>;

    fn parse_item(cx: &mut Cx<'a>) -> Result<Self, Error> {
        Self::parse(cx)
    }
}

impl<'a> Parse<'a> for Expr<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        if parameter::starts_parameter(cx) {
            return Lambda::parse(cx).map(Self::Lambda);
        }

        let expr = Self::parse_inner(cx, false)?;
        if cx
            .peek()
            .is_some_and(|tk| tk.as_quote().is_some_and(|quote| quote == &QuoteId::Colon))
        {
            return Err(Error {
                range: expr.range(),
                kind: ErrorKind::NotParameter,
            });
        }

        Ok(expr)
    }

    fn parse_item(cx: &mut Cx<'a>) -> Result<Self, Error> {
        Self::parse_inner(cx, true)
    }
}

impl<'a> Expr<'a> {
    fn parse_inner(cx: &mut Cx<'a>, list_item: bool) -> Result<Self, Error> {
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
                    match literal {
                        Literal::Path(path) => Self::parse_file_path(cx, path)?,
                        literal => Self::Literal(literal),
                    }
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
                    value: QuoteId::DoubleQuote,
                    ..
                }) => {
                    let string = StringExpr::parse(cx)?;
                    Self::String(string)
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
                }) if !list_item => {
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

            if list_item {
                return Ok(expr);
            }

            if let Some(expr) = stack.reduce(expr, cx) {
                return Ok(expr);
            }
        }
    }

    fn parse_file_path(cx: &mut Cx<'a>, path: PathLiteral<'a>) -> Result<Self, Error> {
        if !cx.peek().is_some_and(|tk| {
            tk.range().start == path.range.end
                && tk.as_quote().is_some_and(|quote| quote == &QuoteId::SubstL)
        }) {
            return Ok(Self::Literal(Literal::Path(path)));
        }

        let mut range = path.range.clone();
        let mut parts = vec![StringPart::Text(StringLiteral {
            range: path.range,
            value: path.value,
        })];
        while cx.peek().is_some_and(|tk| {
            tk.range().start == range.end
                && matches!(
                    tk,
                    Token::Literal(Literal::String(_))
                        | Token::Quote(Quote {
                            value: QuoteId::SubstL,
                            ..
                        })
                )
        }) {
            let part = StringPart::parse(cx)?;
            range.end = part.range().end;
            parts.push(part);
        }

        Ok(Self::FilePath(FilePathExpr { range, parts }))
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

            let item = T::parse_item(cx)?;
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
        let colon = cx
            .next_if(|tk| tk.as_quote().is_some_and(|quote| quote == &QuoteId::Colon))
            .and_then(Token::into_quote);
        let ty = if colon.is_some() {
            Some(Path::parse(cx)?)
        } else {
            None
        };

        let eq = cx
            .next_if(|tk| tk.as_quote().is_some_and(|quote| quote == &QuoteId::Eq))
            .and_then(Token::into_quote);
        let expr = if eq.is_some() {
            Some(Expr::parse(cx)?)
        } else {
            None
        };

        let semicolon = cx
            .next()
            .ok_or(cx.end..cx.end)
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
            colon,
            ty,
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

impl<'a> Parse<'a> for StringExpr<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let string = Punctuated::<
            StringPart<'a>,
            { QuoteId::DoubleQuote as u8 },
            { QuoteId::DoubleQuote as u8 },
        >::parse(cx)?;

        Ok(Self {
            range: string.range,
            parts: string.items,
        })
    }
}

impl<'a> Parse<'a> for StringPart<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        if let Some(Token::Literal(Literal::String(text))) =
            cx.next_if(|tk| matches!(tk, Token::Literal(Literal::String(_))))
        {
            return Ok(Self::Text(text));
        }

        Subst::parse(cx).map(Self::Subst)
    }
}

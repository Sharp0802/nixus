use std::iter::Peekable;
use std::vec;

use crate::lex::{Quote, Span, Token};

mod error;
mod types;

pub use error::*;
pub use types::*;

type Cx<'a> = Peekable<vec::IntoIter<Token<'a>>>;

pub trait Parse<'a>: Sized {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error>;
}

impl<'a> Parse<'a> for Expr<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let Some(tk) = cx.peek() else {
            return Err(Error {
                range: 0..0,
                kind: ErrorKind::Empty,
            });
        };

        match tk {
            Token::Literal(_literal) => {
                let literal = cx.next().unwrap().into_literal().unwrap();
                Ok(Self::Literal(literal))
            }

            Token::Ident(..) | Token::Quote(Quote { value: "${", .. }) => {
                let path = Path::parse(cx)?;
                Ok(Self::Path(path))
            }

            Token::Quote(Quote { value: "{", .. }) => {
                let set = Set::parse(cx)?;
                Ok(Self::Set(set))
            }
            Token::Quote(Quote { value: "[", .. }) => {
                let array = Array::parse(cx)?;
                Ok(Self::Array(array))
            }

            Token::Quote(Quote { .. }) => {
                todo!();
            }
        }
    }
}

impl<'a, T: Parse<'a> + Span, const P: u8, const Q: u8> Parse<'a> for Group<T, P, Q> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let open_str: &'static str = const { str::from_utf8(&[P]) }.unwrap();
        let close_str: &'static str = const { str::from_utf8(&[Q]) }.unwrap();

        let open = cx
            .next()
            .ok_or(0..0)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::Missing(open_str),
            })?;
        if open != open_str {
            return Err(Error {
                range: open.range,
                kind: ErrorKind::Missing(open_str),
            });
        }

        let mut vec = Vec::new();
        while cx
            .peek()
            .and_then(|tk| tk.as_quote())
            .is_none_or(|quote| quote != &close_str)
        {
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
                kind: ErrorKind::Missing(close_str),
            })?;
        if close != close_str {
            return Err(Error {
                range: close.range,
                kind: ErrorKind::Missing(close_str),
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
                kind: ErrorKind::Missing("="),
            })?;
        if eq != "=" {
            return Err(Error {
                range: eq.range,
                kind: ErrorKind::Missing("="),
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
                kind: ErrorKind::Missing(";"),
            })?;
        if semicolon != ";" {
            return Err(Error {
                range: semicolon.range,
                kind: ErrorKind::Missing(";"),
            });
        }

        Ok(Self {
            range: path.range().start..expr.range().end,
            key: path,
            eq,
            value: expr,
            semicolon,
        })
    }
}

impl<'a> Parse<'a> for Subst<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let open = cx
            .next()
            .ok_or(0..0)
            .and_then(|tk| {
                let range = tk.range();
                tk.into_quote().ok_or(range)
            })
            .map_err(|e| Error {
                range: e,
                kind: ErrorKind::NotSubst,
            })?;
        if open != "${" {
            return Err(Error {
                range: open.range(),
                kind: ErrorKind::NotSubst,
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
                kind: ErrorKind::PathUnclosed,
            })?;

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
            Token::Quote(Quote { value: "${", .. }) => {
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
                Some(Quote { value: ".", .. })
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

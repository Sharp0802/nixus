use crate::lex::{Ident, Quote, QuoteId, Span, Token};

use super::{Cx, Error, ErrorKind, Expr, Formal, Lambda, Parameter, Parse, SetPattern};

pub(super) fn starts_parameter(cx: &Cx<'_>) -> bool {
    let third = cx
        .peek_nth(2)
        .and_then(Token::as_quote)
        .map(|quote| quote.value);

    match (cx.peek(), cx.peek_nth(1)) {
        (Some(Token::Ident(_)), Some(Token::Quote(quote))) => quote == &QuoteId::Colon,
        (Some(Token::Quote(open)), Some(next)) if open == &QuoteId::BraceL => match next {
            Token::Ident(_) => matches!(third, Some(QuoteId::Comma | QuoteId::BraceR)),
            Token::Quote(quote) if quote == &QuoteId::Ellipsis => true,
            Token::Quote(quote) if quote == &QuoteId::BraceR => third == Some(QuoteId::Colon),
            _ => false,
        },
        _ => false,
    }
}

impl<'a> Parse<'a> for Lambda<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let parameter = Parameter::parse(cx)?;
        let colon = expect_quote(cx, QuoteId::Colon)?;
        if cx.peek().is_none() {
            return Err(Error {
                range: colon.range.end..colon.range.end,
                kind: ErrorKind::Empty,
            });
        }

        let body = Expr::parse(cx)?;
        Ok(Self {
            range: parameter.range().start..body.range().end,
            parameter,
            colon,
            body: Box::new(body),
        })
    }
}

impl<'a> Parse<'a> for Parameter<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        if matches!(cx.peek(), Some(Token::Ident(_))) {
            return expect_ident(cx).map(Self::Ident);
        }

        SetPattern::parse(cx).map(|pattern| Self::Set(Box::new(pattern)))
    }
}

impl<'a> Parse<'a> for SetPattern<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let open = expect_quote(cx, QuoteId::BraceL)?;
        let mut formals = Vec::new();
        let mut ellipsis = None;

        while cx
            .peek()
            .is_some_and(|tk| !tk.as_quote().is_some_and(|quote| quote == &QuoteId::BraceR))
        {
            if let Some(quote) = take_quote(cx, QuoteId::Ellipsis) {
                ellipsis = Some(quote);
                break;
            }

            let formal = Formal::parse(cx)?;
            let has_comma = formal.comma.is_some();
            formals.push(formal);
            if !has_comma {
                break;
            }
        }

        let close = expect_quote(cx, QuoteId::BraceR)?;
        Ok(Self {
            range: open.range.start..close.range.end,
            open,
            close,
            formals,
            ellipsis,
        })
    }
}

impl<'a> Parse<'a> for Formal<'a> {
    fn parse(cx: &mut Cx<'a>) -> Result<Self, Error> {
        let name = expect_ident(cx)?;
        let comma = take_quote(cx, QuoteId::Comma);

        Ok(Self {
            range: name.range.start..cx.end,
            name,
            comma,
        })
    }
}

fn take_quote(cx: &mut Cx<'_>, id: QuoteId) -> Option<Quote> {
    cx.next_if(|tk| tk.as_quote().is_some_and(|quote| quote == &id))
        .and_then(Token::into_quote)
}

fn expect_quote(cx: &mut Cx<'_>, id: QuoteId) -> Result<Quote, Error> {
    let token = cx.next().ok_or(Error {
        range: cx.end..cx.end,
        kind: ErrorKind::Missing(id),
    })?;
    let range = token.range();
    match token {
        Token::Quote(quote) if quote == id => Ok(quote),
        _ => Err(Error {
            range,
            kind: ErrorKind::Missing(id),
        }),
    }
}

fn expect_ident<'a>(cx: &mut Cx<'a>) -> Result<Ident<'a>, Error> {
    let token = cx.next().ok_or(Error {
        range: cx.end..cx.end,
        kind: ErrorKind::NotParameter,
    })?;
    let range = token.range();
    token.into_ident().ok_or(Error {
        range,
        kind: ErrorKind::NotParameter,
    })
}

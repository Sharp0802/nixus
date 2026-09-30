use std::{mem::transmute, ops::Range};

mod error;
mod number;
mod types;

pub use error::*;
pub use types::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuoteId {
    Add,
    Sub,
    Mul,
    SubstL,
    BraceL,
    BraceR,
    BracketL,
    BracketR,
    ParenL,
    ParenR,
    Eq,
    Ellipsis,
    Dot,
    Semicolon,
    DoubleQuote,
    Colon,
    Comma,
    __Count,
}

impl QuoteId {
    #[rustfmt::skip]
    const TABLE: [&'static str; Self::__Count as usize] = [
        "+", "-", "*",
        "${", "{", "}",
        "[", "]",
        "(", ")",
        "=",
        "...", ".",
        ";", "\"",
        ":", ",",
    ];

    pub const fn transmute(val: u8) -> Self {
        assert!((val as usize) < Self::TABLE.len());
        unsafe { transmute(val) }
    }

    pub fn from_str(str: &str) -> Option<Self> {
        for (i, ch) in Self::TABLE.iter().enumerate() {
            if str.starts_with(ch) {
                let en = Self::transmute(i.try_into().unwrap());
                return Some(en);
            }
        }

        None
    }

    pub const fn to_str(self) -> &'static str {
        Self::TABLE[self as usize]
    }

    pub const fn is_op(self) -> bool {
        matches!(self, Self::Add | Self::Sub | Self::Mul)
    }
}

pub trait Span {
    fn range(&self) -> Range<usize>;
    fn offset(&mut self, offset: usize);
}

trait Lex<'a>: Sized {
    fn lex(src: &'a str) -> Result<Self, ErrorKind>;
}

impl<'a> Lex<'a> for StringLiteral<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        let mut i = 0;
        while i < src.len() {
            let bstr = &src.as_bytes()[i..];
            if bstr[0] == b'"' || bstr.starts_with(b"${") {
                break;
            }

            if (bstr[0] == b'\\' && bstr.len() > 1) || bstr.starts_with(b"$$") {
                i += 2;
            } else {
                i += 1;
            }
        }
        if i == 0 {
            return Err(ErrorKind::NotString);
        }

        Ok(Self {
            range: 0..i,
            value: &src[..i],
        })
    }
}

impl<'a> Lex<'a> for Ident<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        let mut len = 0;
        for ch in src.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                break;
            }

            len += ch.len_utf8();
        }
        if len == 0 {
            return Err(ErrorKind::IdentEmpty);
        }

        Ok(Self {
            range: 0..len,
            value: &src[0..len],
        })
    }
}

impl Lex<'_> for Quote {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        let id = QuoteId::from_str(src).ok_or(ErrorKind::NotQuote)?;

        Ok(Self {
            range: 0..id.to_str().len(),
            value: id,
        })
    }
}

impl<'a> Lex<'a> for Token<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        match PathLiteral::lex(src) {
            Ok(path) => return Ok(Self::Literal(Literal::Path(path))),
            Err(ErrorKind::NotPath) => {}
            Err(error) => return Err(error),
        }

        if src.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            return Literal::lex(src).map(Self::Literal);
        }

        Ident::lex(src)
            .map(|ident| match ident.value {
                "true" | "false" => Self::Literal(Literal::Bool(BoolLiteral {
                    range: ident.range,
                    value: ident.value == "true",
                })),
                _ => Self::Ident(ident),
            })
            .or_else(|e| Quote::lex(src).map(Self::Quote).map_err(|e2| e.merge(e2)))
    }
}

impl<'a> Lex<'a> for PathLiteral<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        let start = usize::from(src.starts_with("~/"));
        let part = Self::lex_part(&src[start..])?;
        if !part.value.contains('/') {
            return Err(ErrorKind::NotPath);
        }
        let end = start + part.range.end;

        Ok(Self {
            range: 0..end,
            value: &src[..end],
        })
    }
}

impl<'a> PathLiteral<'a> {
    fn lex_part(src: &'a str) -> Result<Self, ErrorKind> {
        let end = src
            .bytes()
            .take_while(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+' | b'/')
            })
            .count();
        let value = &src[..end];
        if value.is_empty() {
            return Err(ErrorKind::NotPath);
        }
        if value.contains("//") || (value.ends_with('/') && !src[end..].starts_with("${")) {
            return Err(ErrorKind::InvalidPath);
        }

        Ok(Self {
            range: 0..end,
            value,
        })
    }
}

enum Mode {
    Expression,
    String(usize),
    Path,
}

pub fn lex(str: &str) -> Result<Vec<Token<'_>>, Error> {
    let mut vec = Vec::new();
    let mut modes = Vec::new();
    let mut i = 0;
    while i < str.len() {
        let in_string = matches!(modes.last(), Some(Mode::String(_)));
        let in_path = matches!(modes.last(), Some(Mode::Path));
        if !in_string && !in_path {
            while i < str.len() && str.as_bytes()[i].is_ascii_whitespace() {
                i += 1;
            }
            if i == str.len() {
                break;
            }
        }

        let mut tk = if in_path && !str[i..].starts_with("${") {
            match PathLiteral::lex_part(&str[i..]) {
                Ok(part) => Ok(Token::Literal(Literal::String(StringLiteral {
                    range: part.range,
                    value: part.value,
                }))),
                Err(ErrorKind::NotPath) => {
                    modes.pop();
                    continue;
                }
                Err(error) => Err(error),
            }
        } else if in_string {
            match StringLiteral::lex(&str[i..]) {
                Ok(literal) => Ok(Token::Literal(Literal::String(literal))),
                Err(ErrorKind::NotString) => Quote::lex(&str[i..]).map(Token::Quote),
                Err(error) => Err(error),
            }
        } else {
            Token::lex(&str[i..])
        }
        .map_err(|e| Error { pos: i, kind: e })?;

        if matches!(tk, Token::Literal(Literal::Path(_))) {
            modes.push(Mode::Path);
        }

        match tk.as_quote().map(|quote| quote.value) {
            Some(QuoteId::DoubleQuote) if in_string => {
                modes.pop();
            }
            Some(QuoteId::DoubleQuote) => modes.push(Mode::String(i)),
            Some(QuoteId::SubstL | QuoteId::BraceL) => modes.push(Mode::Expression),
            Some(QuoteId::BraceR) => {
                modes.pop();
            }
            _ => {}
        }

        tk.offset(i);
        i = tk.range().end;
        vec.push(tk);
    }

    if let Some(Mode::String(start)) = modes.last() {
        return Err(Error {
            pos: *start,
            kind: ErrorKind::StringUnclosed,
        });
    }

    Ok(vec)
}

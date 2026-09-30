use std::ops::Range;

mod error;
mod types;

pub use error::*;
pub use types::*;

pub trait Span {
    fn range(&self) -> Range<usize>;
    fn offset(&mut self, offset: usize);

    fn len(&self) -> usize {
        self.range().len()
    }
}

trait Lex<'a>: Sized {
    fn lex(src: &'a str) -> Result<Self, ErrorKind>;
}

impl Lex<'_> for IntegerLiteral {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        let mut i = 0;
        while i < src.len() && src.as_bytes()[i].is_ascii_digit() {
            i += 1;
        }

        let value: i64 = src[..i].parse()?;

        Ok(Self { range: 0..i, value })
    }
}

impl Lex<'_> for FloatLiteral {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        let mut dot = 1;
        let mut i = usize::from(src.starts_with('+') || src.starts_with('-'));
        while i < src.len() {
            let ch = src.as_bytes()[i];
            if ch == b'.' {
                if dot <= 0 {
                    break;
                }

                dot -= 1;
            } else if !ch.is_ascii_digit() {
                break;
            }

            i += 1;
        }
        if dot == 1 || src.as_bytes()[..i].last() == Some(&b'.') {
            return Err(ErrorKind::NotFloat);
        }
        if i == 0 {
            return Err(ErrorKind::FloatEmpty);
        }

        let value: f64 = src[..i].parse()?;

        Ok(Self { range: 0..i, value })
    }
}

impl<'a> Lex<'a> for StringLiteral<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        if src.as_bytes().first() != Some(&b'"') {
            return Err(ErrorKind::NotString);
        }

        let mut i = 1;
        while i < src.len() && src.as_bytes()[i] != b'\"' {
            if src.as_bytes()[i..].starts_with(b"\\\"") {
                i += 2;
            } else {
                i += 1;
            }
        }
        if i >= src.len() {
            return Err(ErrorKind::StringUnclosed);
        }

        let value = &src[1..i];
        i += 1;

        Ok(Self { range: 0..i, value })
    }
}

impl<'a> Lex<'a> for Literal<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        FloatLiteral::lex(src)
            .map(Self::Float)
            .or_else(|e| {
                IntegerLiteral::lex(src)
                    .map(Self::Integer)
                    .map_err(|e2| e.merge(e2))
            })
            .or_else(|e| {
                StringLiteral::lex(src)
                    .map(Self::String)
                    .map_err(|e2| e.merge(e2))
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
        macro_rules! entries {
            ($($lit:literal),* $(,)?) => {
                $(if src.starts_with($lit) {
                    return Ok(Self { range: 0..$lit.len(), value: $lit });
                })*
            };
        }

        #[rustfmt::skip]
        entries![
            "::", ":",
            "..", ".",
            "==", "=", "!=",
            "${", "{", "}",
            "[", "]",
            "<", ">",
            "+", "-", "*", "/", "%",
            "||", "|",
            "&&", "&",
            "!", "^",
            ",",
        ];

        Err(ErrorKind::NotQuote)
    }
}

impl<'a> Lex<'a> for Token<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        Literal::lex(src)
            .map(Self::Literal)
            .or_else(|e| Ident::lex(src).map(Self::Ident).map_err(|e2| e.merge(e2)))
            .or_else(|e| Quote::lex(src).map(Self::Quote).map_err(|e2| e.merge(e2)))
    }
}

pub fn lex(str: &str) -> Result<Vec<Token<'_>>, Error> {
    let mut vec = Vec::new();

    let mut i = 0;
    while {
        while i < str.len() && str.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }

        i < str.len()
    } {
        let mut tk = Token::lex(&str[i..]).map_err(|e| Error { pos: i, kind: e })?;
        tk.offset(i);
        i = tk.range().end;
        vec.push(tk);
    }

    Ok(vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_ident() {
        assert_eq!(Token::lex("hello").unwrap().as_ident().unwrap(), &"hello");
        assert_eq!(Token::lex("h3ll0").unwrap().as_ident().unwrap(), &"h3ll0");
        assert_eq!(Token::lex("ha wo").unwrap().as_ident().unwrap(), &"ha");
        assert!(Token::lex("4ell0").unwrap().as_ident().is_none());
    }

    #[test]
    fn lex_literal() {
        assert_eq!(Token::lex("1.25").unwrap().as_literal().unwrap(), &1.25);
        assert_eq!(Token::lex("125").unwrap().as_literal().unwrap(), &125);
        assert_eq!(Token::lex("4ell0").unwrap().as_literal().unwrap(), &4);
    }

    #[test]
    fn lex_quote() {
        assert_eq!(Token::lex("..").unwrap().as_quote().unwrap(), &"..");
        assert_eq!(Token::lex("!=").unwrap().as_quote().unwrap(), &"!=");
        assert_eq!(Token::lex("!.").unwrap().as_quote().unwrap(), &"!");
    }
}

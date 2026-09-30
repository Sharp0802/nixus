use super::{ErrorKind, FloatLiteral, IntegerLiteral, Lex, Literal};

impl Lex<'_> for IntegerLiteral {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        if !src.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            return Err(ErrorKind::IntegerEmpty);
        }

        let (radix, start) = match src.as_bytes().get(..2) {
            Some(b"0b") => (2, 2),
            Some(b"0o") => (8, 2),
            Some(b"0x") => (16, 2),
            _ => (10, 0),
        };
        let mut end = start;
        while src.as_bytes().get(end).is_some_and(|byte| {
            *byte == b'_' || byte.is_ascii_digit() || (radix == 16 && byte.is_ascii_hexdigit())
        }) {
            end += 1;
        }

        if radix != 10 && starts_fraction(src, end) {
            return Err(ErrorKind::InvalidDigit);
        }
        reject_suffix(src, end)?;
        let value = i64::from_str_radix(&src[start..end].replace('_', ""), radix)?;

        Ok(Self {
            range: 0..end,
            value,
        })
    }
}

impl Lex<'_> for FloatLiteral {
    fn lex(src: &str) -> Result<Self, ErrorKind> {
        if !src.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            return Err(ErrorKind::NotFloat);
        }

        let mut end = decimal_end(src, 0);
        let fraction = starts_fraction(src, end);
        if fraction {
            end = decimal_end(src, end + 1);
        }

        let exponent = matches!(src.as_bytes().get(end), Some(b'e' | b'E'));
        if exponent {
            end += 1;
            if matches!(src.as_bytes().get(end), Some(b'+' | b'-')) {
                end += 1;
            }
            let start = end;
            end = decimal_end(src, end);
            if !src.as_bytes()[start..end].iter().any(u8::is_ascii_digit) {
                return Err(ErrorKind::InvalidExponent);
            }
        }

        if !fraction && !exponent {
            return Err(ErrorKind::NotFloat);
        }
        reject_suffix(src, end)?;
        let value: f64 = src[..end].replace('_', "").parse()?;
        if !value.is_finite() {
            return Err(ErrorKind::FloatOverflow);
        }

        Ok(Self {
            range: 0..end,
            value,
        })
    }
}

impl<'a> Lex<'a> for Literal<'a> {
    fn lex(src: &'a str) -> Result<Self, ErrorKind> {
        match FloatLiteral::lex(src) {
            Ok(literal) => Ok(Self::Float(literal)),
            Err(ErrorKind::NotFloat) => IntegerLiteral::lex(src).map(Self::Integer),
            Err(error) => Err(error),
        }
    }
}

fn decimal_end(src: &str, mut end: usize) -> usize {
    while src
        .as_bytes()
        .get(end)
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'_')
    {
        end += 1;
    }
    end
}

fn starts_fraction(src: &str, end: usize) -> bool {
    src.as_bytes().get(end) == Some(&b'.')
        && !src
            .as_bytes()
            .get(end + 1)
            .is_some_and(|byte| *byte == b'.' || *byte == b'_' || byte.is_ascii_alphabetic())
}

fn reject_suffix(src: &str, end: usize) -> Result<(), ErrorKind> {
    if src
        .as_bytes()
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        Err(ErrorKind::LiteralSuffix)
    } else {
        Ok(())
    }
}

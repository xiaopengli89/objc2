//! Parsing encodings from their string representation.
#![deny(unsafe_code)]
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::helper::{ContainerKind, Helper, NestingLevel};
use crate::{Encoding, EncodingBox};

/// Check whether a struct or union name is a valid identifier
pub(crate) const fn verify_name(name: &str) -> bool {
    let bytes = name.as_bytes();

    if let b"?" = bytes {
        return true;
    }

    if bytes.is_empty() {
        return false;
    }

    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return false;
        }
        i += 1;
    }
    true
}

/// The error that was encountered while parsing the encoding.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ParseError {
    kind: ErrorKind,
    data: String,
    split_point: usize,
}

impl ParseError {
    pub(crate) fn new(parser: Parser<'_>, kind: ErrorKind) -> Self {
        Self {
            kind,
            data: parser.data.to_string(),
            split_point: parser.split_point,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed parsing encoding: {} at byte-index {} in {:?}",
            self.kind, self.split_point, self.data,
        )
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum ErrorKind {
    UnexpectedEnd,
    Unknown(u8),
    UnknownAfterComplex(u8),
    ExpectedInteger,
    IntegerTooLarge,
    WrongEndArray,
    WrongEndContainer(ContainerKind),
    InvalidIdentifier(ContainerKind),
    NotAllConsumed,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd => write!(f, "unexpected end"),
            Self::Unknown(b) => {
                write!(f, "unknown encoding character {}", *b as char)
            }
            Self::UnknownAfterComplex(b) => {
                write!(f, "unknown encoding character {} after complex", *b as char,)
            }
            Self::ExpectedInteger => write!(f, "expected integer"),
            Self::IntegerTooLarge => write!(f, "integer too large"),
            Self::WrongEndArray => write!(f, "expected array to be closed"),
            Self::WrongEndContainer(kind) => {
                write!(f, "expected {} to be closed", kind)
            }
            Self::InvalidIdentifier(kind) => {
                write!(f, "got invalid identifier in {}", kind)
            }
            Self::NotAllConsumed => {
                write!(f, "remaining contents after parsing")
            }
        }
    }
}

type Result<T, E = ErrorKind> = core::result::Result<T, E>;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct Parser<'a> {
    data: &'a str,
    // Always "behind"/"at" the current character
    split_point: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(data: &'a str) -> Self {
        Self {
            split_point: 0,
            data,
        }
    }

    fn peek(&self) -> Result<u8> {
        self.try_peek().ok_or(ErrorKind::UnexpectedEnd)
    }

    fn try_peek(&self) -> Option<u8> {
        self.data.as_bytes().get(self.split_point).copied()
    }

    fn advance(&mut self) {
        self.split_point += 1;
    }

    fn consume_while(&mut self, mut condition: impl FnMut(u8) -> bool) {
        while let Some(b) = self.try_peek() {
            if condition(b) {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.try_peek().is_none()
    }

    pub(crate) fn expect_empty(&self) -> Result<()> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(ErrorKind::NotAllConsumed)
        }
    }
}

impl Parser<'_> {
    /// Strip leading qualifiers, if any.
    pub(crate) fn strip_leading_qualifiers(&mut self) {
        const QUALIFIERS: &[u8] = &[
            b'r', // const
            b'n', // in
            b'N', // inout
            b'o', // out
            b'O', // bycopy
            b'R', // byref
            b'V', // oneway
                  // b'!', // GCINVISIBLE
        ];

        self.consume_while(|b| QUALIFIERS.contains(&b));
    }

    /// Chomp until we hit a non-digit.
    ///
    /// + and - prefixes are not supported.
    fn chomp_digits(&mut self) -> Result<&str> {
        let old_split_point = self.split_point;

        // Parse first digit (which must be present).
        if !self.peek()?.is_ascii_digit() {
            return Err(ErrorKind::ExpectedInteger);
        }

        // Parse the rest, stopping if we hit a non-digit.
        self.consume_while(|b| b.is_ascii_digit());

        Ok(&self.data[old_split_point..self.split_point])
    }

    fn parse_usize(&mut self) -> Result<usize> {
        self.chomp_digits()?
            .parse()
            .map_err(|_| ErrorKind::IntegerTooLarge)
    }

    fn parse_u8(&mut self) -> Result<u8> {
        self.chomp_digits()?
            .parse()
            .map_err(|_| ErrorKind::IntegerTooLarge)
    }
}

/// Check if the data matches an expected value.
///
/// The errors here aren't currently used, so they're hackily set up.
impl Parser<'_> {
    fn expect_byte(&mut self, byte: u8) -> Option<()> {
        if self.try_peek()? == byte {
            self.advance();
            Some(())
        } else {
            None
        }
    }

    fn expect_str(&mut self, s: &str) -> Option<()> {
        for b in s.as_bytes() {
            self.expect_byte(*b)?;
        }
        Some(())
    }

    fn expect_usize(&mut self, int: usize) -> Option<()> {
        if self.parse_usize().ok()? == int {
            Some(())
        } else {
            None
        }
    }

    pub(crate) fn expect_encoding(&mut self, enc: &Encoding, level: NestingLevel) -> Option<()> {
        let helper = Helper::new(enc, level);
        match helper {
            Helper::Primitive(primitive) => self.expect_str(primitive.to_str()),
            Helper::BitField(b, _type, _level) => {
                // TODO: Use the type on GNUStep (nesting level?)
                self.expect_byte(b'b')?;
                self.expect_usize(b as usize)
            }
            Helper::Indirection(kind, t, level) => {
                self.expect_byte(kind.prefix_byte())?;
                self.expect_encoding(t, level)
            }
            Helper::Array(len, item, level) => {
                self.expect_byte(b'[')?;
                self.expect_usize(len)?;
                self.expect_encoding(item, level)?;
                self.expect_byte(b']')
            }
            Helper::Container(kind, name, items, level) => {
                self.expect_byte(kind.start_byte())?;
                self.expect_str(name)?;
                if let Some(items) = items {
                    self.expect_byte(b'=')?;
                    for item in items {
                        self.expect_encoding(item, level)?;
                    }
                }
                self.expect_byte(kind.end_byte())
            }
        }
    }
}

impl Parser<'_> {
    fn parse_container(&mut self, kind: ContainerKind) -> Result<(&str, Option<Vec<EncodingBox>>)> {
        let old_split_point = self.split_point;

        // Parse name until hits `=`
        let has_items = loop {
            let b = self.try_peek().ok_or(ErrorKind::WrongEndContainer(kind))?;
            if b == b'=' {
                break true;
            } else if b == kind.end_byte() {
                break false;
            }
            self.advance();
        };

        let s = &self.data[old_split_point..self.split_point];

        if !verify_name(s) {
            return Err(ErrorKind::InvalidIdentifier(kind));
        }

        self.advance();

        if has_items {
            let mut items = Vec::new();
            // Parse items until hits end
            loop {
                let b = self.try_peek().ok_or(ErrorKind::WrongEndContainer(kind))?;
                if b == kind.end_byte() {
                    self.advance();
                    break;
                } else {
                    // Wasn't the end, so try to extract one more encoding
                    items.push(self.parse_encoding()?);
                }
            }
            Ok((s, Some(items)))
        } else {
            Ok((s, None))
        }
    }

    pub(crate) fn parse_encoding(&mut self) -> Result<EncodingBox> {
        self.try_parse_encoding()
            .and_then(|res| res.ok_or(ErrorKind::UnexpectedEnd))
    }

    fn try_parse_encoding(&mut self) -> Result<Option<EncodingBox>> {
        Ok(if let Some(b) = self.try_peek() {
            self.advance();
            Some(self.parse_encoding_inner(b)?)
        } else {
            None
        })
    }

    fn parse_encoding_inner(&mut self, b: u8) -> Result<EncodingBox> {
        Ok(match b {
            b'c' => EncodingBox::Char,
            b's' => EncodingBox::Short,
            b'i' => EncodingBox::Int,
            b'l' => EncodingBox::Long,
            b'q' => EncodingBox::LongLong,
            b'C' => EncodingBox::UChar,
            b'S' => EncodingBox::UShort,
            b'I' => EncodingBox::UInt,
            b'L' => EncodingBox::ULong,
            b'Q' => EncodingBox::ULongLong,
            b'f' => EncodingBox::Float,
            b'd' => EncodingBox::Double,
            b'D' => EncodingBox::LongDouble,
            b'j' => {
                let res = match self.peek()? {
                    b'f' => EncodingBox::FloatComplex,
                    b'd' => EncodingBox::DoubleComplex,
                    b'D' => EncodingBox::LongDoubleComplex,
                    b => return Err(ErrorKind::UnknownAfterComplex(b)),
                };
                self.advance();
                res
            }
            b'B' => EncodingBox::Bool,
            b'v' => EncodingBox::Void,
            b'*' => EncodingBox::String,
            b'@' => match self.try_peek() {
                // Special handling for blocks
                Some(b'?') => {
                    self.advance();
                    EncodingBox::Block
                }
                _ => EncodingBox::Object,
            },
            b'#' => EncodingBox::Class,
            b':' => EncodingBox::Sel,
            b'?' => EncodingBox::Unknown,

            b'b' => {
                // TODO: Parse type here on GNUStep
                EncodingBox::BitField(self.parse_u8()?, None)
            }
            b'^' => EncodingBox::Pointer(Box::new(self.parse_encoding()?)),
            b'A' => EncodingBox::Atomic(Box::new(self.parse_encoding()?)),
            b'[' => {
                let len = self.parse_usize()?;
                let item = self.parse_encoding()?;
                self.expect_byte(b']').ok_or(ErrorKind::WrongEndArray)?;
                EncodingBox::Array(len, Box::new(item))
            }
            b'{' => {
                let kind = ContainerKind::Struct;
                let (name, items) = self.parse_container(kind)?;
                EncodingBox::Struct(name.to_string(), items)
            }
            b'(' => {
                let kind = ContainerKind::Union;
                let (name, items) = self.parse_container(kind)?;
                EncodingBox::Union(name.to_string(), items)
            }
            b => return Err(ErrorKind::Unknown(b)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parse_container() {
        const KIND: ContainerKind = ContainerKind::Struct;

        fn assert_name(enc: &str, expected: Result<(&str, Option<Vec<EncodingBox>>)>) {
            let mut parser = Parser::new(enc);
            assert_eq!(parser.parse_container(KIND), expected);
        }

        assert_name("abc=}", Ok(("abc", Some(vec![]))));
        assert_name(
            "abc=ii}",
            Ok(("abc", Some(vec![EncodingBox::Int, EncodingBox::Int]))),
        );
        assert_name("_=}.a'", Ok(("_", Some(vec![]))));
        assert_name("abc}def", Ok(("abc", None)));
        assert_name("=def}", Err(ErrorKind::InvalidIdentifier(KIND)));
        assert_name(".=def}", Err(ErrorKind::InvalidIdentifier(KIND)));
        assert_name("}xyz", Err(ErrorKind::InvalidIdentifier(KIND)));
        assert_name("", Err(ErrorKind::WrongEndContainer(KIND)));
        assert_name("abc", Err(ErrorKind::WrongEndContainer(KIND)));
        assert_name("abc)def", Err(ErrorKind::WrongEndContainer(KIND)));
    }
}

//! S-expression input stream. Port of sexp-input.cpp.
//!
//! The parser is a faithful port of the C++ `sexp_input_stream_t`. It maintains
//! a 1-character lookahead (`next_char`) and a `count` of 8-bit characters read
//! so far (starting at -1; after the first `get_char`, count is 0). The
//! `byte_size` field is 8 by default but switches to 4 inside `#...#` hex
//! regions and 6 inside `|...|` base64 regions, allowing inline decoding.
//!
//! To support the GnuPG extended-key parser (which needs continuation-aware
//! character reading), the input source is abstracted via the [`CharSource`]
//! trait.

use crate::chars;
use crate::depth::{DepthManager, DEFAULT_MAX_DEPTH};
use crate::error::{sexp_report, Severity, SexpError};
use crate::{Sexp, SexpList, SexpString};

pub const EOF: i32 = -1;
const U32_MAX: u32 = u32::MAX;

/// Source of bytes for the parser. The C++ implementation uses virtual
/// dispatch (`ext_key_input_stream_t::read_char` overrides the base); we use
/// a trait, with [`SliceSource`] as the default for raw byte slices.
pub trait CharSource {
    fn read_char(&mut self) -> i32;
}

pub struct SliceSource<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> SliceSource<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        SliceSource { bytes, pos: 0 }
    }
}

impl<'a> CharSource for SliceSource<'a> {
    fn read_char(&mut self) -> i32 {
        if self.pos >= self.bytes.len() {
            return EOF;
        }
        let c = self.bytes[self.pos] as i32;
        self.pos += 1;
        c
    }
}

impl<S: CharSource + ?Sized> CharSource for &mut S {
    fn read_char(&mut self) -> i32 {
        (**self).read_char()
    }
}

pub struct SexpInputStream<S: CharSource = SliceSource<'static>> {
    source: S,
    pub count: i32,
    pub next_char: i32,
    pub byte_size: u32,
    bits: u32,
    n_bits: u32,
    depth: DepthManager,
}

impl<'a> SexpInputStream<SliceSource<'a>> {
    pub fn new(input: &'a (impl AsRef<[u8]> + ?Sized)) -> Self {
        Self::new_with_max_depth(input.as_ref(), DEFAULT_MAX_DEPTH)
    }

    pub fn new_with_max_depth(input: &'a [u8], max_depth: usize) -> Self {
        SexpInputStream {
            source: SliceSource::new(input),
            count: -1,
            next_char: b' ' as i32,
            byte_size: 8,
            bits: 0,
            n_bits: 0,
            depth: DepthManager::new(max_depth),
        }
    }
}

impl<S: CharSource> SexpInputStream<S> {
    pub fn from_source(source: S) -> Self {
        Self::from_source_with_max_depth(source, DEFAULT_MAX_DEPTH)
    }

    pub fn from_source_with_max_depth(source: S, max_depth: usize) -> Self {
        SexpInputStream {
            source,
            count: -1,
            next_char: b' ' as i32,
            byte_size: 8,
            bits: 0,
            n_bits: 0,
            depth: DepthManager::new(max_depth),
        }
    }

    pub fn set_byte_size(&mut self, size: u32) -> &mut Self {
        self.byte_size = size;
        self.n_bits = 0;
        self.bits = 0;
        self
    }

    pub fn get_byte_size(&self) -> u32 {
        self.byte_size
    }

    pub fn get_next_char(&self) -> i32 {
        self.next_char
    }

    pub fn set_next_char(&mut self, c: i32) -> &mut Self {
        self.next_char = c;
        self
    }

    fn read_char(&mut self) -> i32 {
        self.count += 1;
        self.source.read_char()
    }

    pub fn get_char(&mut self) -> Result<&mut Self, SexpError> {
        if self.next_char == EOF {
            self.byte_size = 8;
            return Ok(self);
        }

        loop {
            let c = self.read_char();
            self.next_char = c;
            if c == EOF {
                return Ok(self);
            }
            let byte = c as u8;
            if (self.byte_size == 6 && (byte == b'|' || byte == b'}'))
                || (self.byte_size == 4 && byte == b'#')
            {
                // End of region. Warn if there are non-zero leftover bits.
                if self.n_bits > 0 && (((1u32 << self.n_bits) - 1) & self.bits) != 0 {
                    sexp_report(
                        &format!(
                            "{}-bit region ended with {} unused bits left-over",
                            self.byte_size, self.n_bits
                        ),
                        Severity::Warning,
                        self.count,
                    )?;
                }
                self.set_byte_size(8);
                return Ok(self);
            } else if self.byte_size != 8 && chars::is_whitespace(byte) {
                // ignore whitespace in hex/base64 regions
            } else if self.byte_size == 6 && byte == b'=' {
                // ignore '=' in base64 regions
            } else if self.byte_size == 8 {
                return Ok(self);
            } else {
                // byte_size < 8: accumulate bits and emit decoded bytes
                self.bits <<= self.byte_size;
                self.n_bits += self.byte_size;
                if self.byte_size == 6 && chars::is_base64_digit(byte) {
                    self.bits |= chars::base64_value(byte) as u32;
                } else if self.byte_size == 4 && chars::is_hex_digit(byte) {
                    self.bits |= chars::hex_value(byte) as u32;
                } else {
                    return Err(SexpError::sexp(
                        format!(
                            "character '{}' found in {}-bit coding region",
                            byte as char, self.byte_size
                        ),
                        Severity::Error,
                        self.count,
                    ));
                }
                if self.n_bits >= 8 {
                    self.next_char = ((self.bits >> (self.n_bits - 8)) & 0xFF) as i32;
                    self.n_bits -= 8;
                    return Ok(self);
                }
            }
        }
    }

    pub fn skip_white_space(&mut self) -> Result<&mut Self, SexpError> {
        while chars::is_whitespace(self.next_char as u8) {
            self.get_char()?;
        }
        Ok(self)
    }

    pub fn skip_char(&mut self, expected: u8) -> Result<&mut Self, SexpError> {
        if self.next_char != expected as i32 {
            let found = if self.next_char == EOF {
                "EOF".to_string()
            } else {
                format!("'{}'", self.next_char as u8 as char)
            };
            let want = format!("'{}'", expected as char);
            return Err(SexpError::sexp(
                format!("character {} found where {} was expected", found, want),
                Severity::Error,
                self.count,
            ));
        }
        self.get_char()?;
        Ok(self)
    }

    fn scan_token(&mut self) -> Result<Vec<u8>, SexpError> {
        self.skip_white_space()?;
        let mut bytes = Vec::new();
        while chars::is_token_char(self.next_char as u8) {
            bytes.push(self.next_char as u8);
            self.get_char()?;
        }
        Ok(bytes)
    }

    pub fn scan_to_eof(&mut self) -> Result<SexpString, SexpError> {
        let mut data = Vec::new();
        self.skip_white_space()?;
        while self.next_char != EOF {
            data.push(self.next_char as u8);
            self.get_char()?;
        }
        Ok(SexpString {
            data,
            presentation_hint: None,
        })
    }

    fn scan_decimal_string(&mut self) -> Result<u32, SexpError> {
        let mut value: u32 = 0;
        let mut i = 0;
        while chars::is_dec_digit(self.next_char as u8) {
            value = value * 10 + chars::dec_value(self.next_char as u8) as u32;
            self.get_char()?;
            if i > 8 {
                return Err(SexpError::sexp(
                    "Decimal number is too long",
                    Severity::Error,
                    self.count,
                ));
            }
            i += 1;
        }
        Ok(value)
    }

    fn scan_verbatim_string(&mut self, length: u32) -> Result<Vec<u8>, SexpError> {
        self.skip_white_space()?.skip_char(b':')?;
        if length > 1024 * 1024 {
            return Err(SexpError::sexp(
                format!("Verbatim string is too long: {}", length),
                Severity::Error,
                self.count,
            ));
        }
        let mut bytes = Vec::with_capacity(length as usize);
        for _ in 0..length {
            if self.next_char == EOF {
                return Err(SexpError::sexp(
                    "EOF while reading verbatim string",
                    Severity::Error,
                    self.count,
                ));
            }
            bytes.push(self.next_char as u8);
            self.get_char()?;
        }
        Ok(bytes)
    }

    fn scan_quoted_string(&mut self, length: u32) -> Result<Vec<u8>, SexpError> {
        self.skip_char(b'"')?;
        let mut bytes = Vec::new();
        while (bytes.len() as u32) <= length {
            if self.next_char == b'"' as i32 {
                if length == U32_MAX || bytes.len() as u32 == length {
                    self.skip_char(b'"')?;
                    return Ok(bytes);
                } else {
                    return Err(SexpError::sexp(
                        format!(
                            "Declared length was {}, but quoted string ended too early",
                            length
                        ),
                        Severity::Error,
                        self.count,
                    ));
                }
            } else if self.next_char == b'\\' as i32 {
                self.get_char()?;
                let c = self.next_char as u8;
                match c {
                    b'b' => bytes.push(0x08),
                    b't' => bytes.push(b'\t'),
                    b'v' => bytes.push(0x0b),
                    b'n' => bytes.push(b'\n'),
                    b'f' => bytes.push(0x0c),
                    b'r' => bytes.push(b'\r'),
                    b'"' => bytes.push(b'"'),
                    b'\'' => bytes.push(b'\''),
                    b'\\' => bytes.push(b'\\'),
                    b'x' => {
                        let mut val: u8 = 0;
                        self.get_char()?;
                        for j in 0..2u32 {
                            if chars::is_hex_digit(self.next_char as u8) {
                                val = (val << 4) | chars::hex_value(self.next_char as u8);
                                if j < 1 {
                                    self.get_char()?;
                                }
                            } else {
                                return Err(SexpError::sexp(
                                    format!("Hex character \\x{:x}... too short", val),
                                    Severity::Error,
                                    self.count,
                                ));
                            }
                        }
                        bytes.push(val);
                    }
                    b'\n' => {
                        self.get_char()?;
                        if self.next_char != b'\r' as i32 {
                            continue;
                        }
                        // else fall through; trailing get_char consumes the \r
                    }
                    b'\r' => {
                        self.get_char()?;
                        if self.next_char != b'\n' as i32 {
                            continue;
                        }
                        // else fall through; trailing get_char consumes the \n
                    }
                    b'0'..=b'7' => {
                        let mut val: u32 = 0;
                        for j in 0..3u32 {
                            let cur = self.next_char;
                            if cur >= b'0' as i32 && cur <= b'7' as i32 {
                                val = (val << 3) | ((cur - b'0' as i32) as u32);
                                if j < 2 {
                                    self.get_char()?;
                                }
                            } else {
                                return Err(SexpError::sexp(
                                    format!("Octal character \\{:o}... too short", val),
                                    Severity::Error,
                                    self.count,
                                ));
                            }
                        }
                        if val > 255 {
                            return Err(SexpError::sexp(
                                format!("Octal character \\{:o}... too big", val),
                                Severity::Error,
                                self.count,
                            ));
                        }
                        bytes.push(val as u8);
                    }
                    _ => {
                        return Err(SexpError::sexp(
                            format!("Unknown escape sequence \\{}", c as char),
                            Severity::Error,
                            self.count,
                        ));
                    }
                }
            } else if self.next_char == EOF {
                return Err(SexpError::sexp(
                    "unexpected end of file",
                    Severity::Error,
                    self.count,
                ));
            } else {
                bytes.push(self.next_char as u8);
            }
            self.get_char()?;
        }
        Ok(bytes)
    }

    fn scan_hexadecimal_string(&mut self, length: u32) -> Result<Vec<u8>, SexpError> {
        self.set_byte_size(4).skip_char(b'#')?;
        let mut bytes = Vec::new();
        while self.next_char != EOF && (self.next_char != b'#' as i32 || self.byte_size == 4) {
            bytes.push(self.next_char as u8);
            self.get_char()?;
        }
        self.skip_char(b'#')?;
        if bytes.len() as u32 != length && length != U32_MAX {
            sexp_report(
                &format!(
                    "Hex string has length {} different than declared length {}",
                    bytes.len(),
                    length
                ),
                Severity::Warning,
                self.count,
            )?;
        }
        Ok(bytes)
    }

    fn scan_base64_string(&mut self, length: u32) -> Result<Vec<u8>, SexpError> {
        self.set_byte_size(6).skip_char(b'|')?;
        let mut bytes = Vec::new();
        while self.next_char != EOF && (self.next_char != b'|' as i32 || self.byte_size == 6) {
            bytes.push(self.next_char as u8);
            self.get_char()?;
        }
        self.skip_char(b'|')?;
        if bytes.len() as u32 != length && length != U32_MAX {
            sexp_report(
                &format!(
                    "Base64 string has length {} different than declared length {}",
                    bytes.len(),
                    length
                ),
                Severity::Warning,
                self.count,
            )?;
        }
        Ok(bytes)
    }

    fn scan_simple_string(&mut self) -> Result<Vec<u8>, SexpError> {
        self.skip_white_space()?;
        let cur = self.next_char;
        if cur != EOF && chars::is_token_char(cur as u8) && !chars::is_dec_digit(cur as u8) {
            return self.scan_token();
        }
        let length = if cur != EOF && chars::is_dec_digit(cur as u8) {
            self.scan_decimal_string()?
        } else {
            U32_MAX
        };
        match self.next_char as u8 {
            b'"' if self.next_char != EOF => self.scan_quoted_string(length),
            b'#' if self.next_char != EOF => self.scan_hexadecimal_string(length),
            b'|' if self.next_char != EOF => self.scan_base64_string(length),
            b':' if self.next_char != EOF => self.scan_verbatim_string(length),
            _ => {
                let msg = if self.next_char == EOF {
                    "unexpected end of file".to_string()
                } else if (self.next_char as u8).is_ascii_graphic() {
                    format!(
                        "illegal character '{}' (0x{:x})",
                        self.next_char as u8 as char, self.next_char as u8
                    )
                } else {
                    format!("illegal character 0x{:x}", self.next_char as u8)
                };
                Err(SexpError::sexp(msg, Severity::Error, self.count))
            }
        }
    }

    pub fn scan_string(&mut self) -> Result<SexpString, SexpError> {
        let mut s = SexpString::default();
        if self.next_char == b'[' as i32 {
            self.skip_char(b'[')?;
            let hint = self.scan_simple_string()?;
            self.skip_white_space()?
                .skip_char(b']')?
                .skip_white_space()?;
            s.presentation_hint = Some(hint);
        }
        s.data = self.scan_simple_string()?;
        Ok(s)
    }

    pub fn scan_list(&mut self) -> Result<SexpList, SexpError> {
        self.open_list()?;
        self.skip_white_space()?;
        let mut list = SexpList::new();
        if self.next_char == b')' as i32 {
            // empty list
        } else {
            list.push(self.scan_object()?);
        }
        loop {
            self.skip_white_space()?;
            if self.next_char == b')' as i32 {
                self.close_list()?;
                return Ok(list);
            }
            list.push(self.scan_object()?);
        }
    }

    pub fn scan_object(&mut self) -> Result<Sexp, SexpError> {
        self.skip_white_space()?;
        if self.next_char == b'{' as i32 && self.byte_size != 6 {
            self.set_byte_size(6);
            self.skip_char(b'{')?;
            let obj = self.scan_object()?;
            self.skip_char(b'}')?;
            Ok(obj)
        } else if self.next_char == b'(' as i32 {
            Ok(Sexp::List(self.scan_list()?))
        } else {
            Ok(Sexp::String(self.scan_string()?))
        }
    }

    pub fn open_list(&mut self) -> Result<&mut Self, SexpError> {
        self.skip_char(b'(')?;
        self.depth.increase(self.count)?;
        Ok(self)
    }

    pub fn close_list(&mut self) -> Result<&mut Self, SexpError> {
        self.skip_char(b')')?;
        self.depth.decrease();
        Ok(self)
    }
}

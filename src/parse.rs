//! Parser — port of sexp-input.cpp.
//!
//! Supports both canonical (binary-safe `length:bytes`) and advanced
//! (human-readable) S-expression syntax.

use crate::chars;
use crate::error::SexpError;
use crate::{Sexp, SexpList, SexpString};

const DEFAULT_MAX_DEPTH: usize = 1024;

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    depth: usize,
    max_depth: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Parser {
            input,
            pos: 0,
            depth: 0,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if !chars::is_whitespace(c) {
                break;
            }
            self.pos += 1;
        }
    }

    fn expect_char(&mut self, expected: u8) -> Result<(), SexpError> {
        match self.peek() {
            Some(c) if c == expected => {
                self.pos += 1;
                Ok(())
            }
            Some(c) => Err(SexpError::CharMismatch {
                expected: expected as char,
                found: c as char,
                pos: self.pos,
            }),
            None => Err(SexpError::UnexpectedEof { pos: self.pos }),
        }
    }

    fn scan_decimal(&mut self) -> Result<u32, SexpError> {
        let mut value: u32 = 0;
        let mut count = 0;
        while let Some(c) = self.peek() {
            if !chars::is_dec_digit(c) {
                break;
            }
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(chars::dec_value(c) as u32))
                .ok_or(SexpError::DecimalTooLong { pos: self.pos })?;
            self.pos += 1;
            count += 1;
            if count > 9 {
                return Err(SexpError::DecimalTooLong { pos: self.pos });
            }
        }
        Ok(value)
    }

    fn scan_token(&mut self) -> Vec<u8> {
        self.skip_whitespace();
        let mut bytes = Vec::new();
        while let Some(c) = self.peek() {
            if !chars::is_token_char(c) {
                break;
            }
            bytes.push(c);
            self.pos += 1;
        }
        bytes
    }

    fn scan_verbatim(&mut self, length: u32) -> Result<Vec<u8>, SexpError> {
        if length > 1024 * 1024 {
            return Err(SexpError::VerbatimTooLong { len: length });
        }
        self.expect_char(b':')?;
        let mut bytes = Vec::with_capacity(length as usize);
        for _ in 0..length {
            match self.next() {
                Some(c) => bytes.push(c),
                None => return Err(SexpError::UnexpectedEof { pos: self.pos }),
            }
        }
        Ok(bytes)
    }

    fn scan_quoted_string(&mut self, declared_len: Option<u32>) -> Result<Vec<u8>, SexpError> {
        self.expect_char(b'"')?;
        let mut bytes = Vec::new();
        loop {
            match self.next() {
                Some(b'"') => {
                    if let Some(declared) = declared_len {
                        if bytes.len() as u32 != declared {
                            return Err(SexpError::LengthMismatch {
                                declared,
                                actual: bytes.len(),
                                pos: self.pos,
                            });
                        }
                    }
                    return Ok(bytes);
                }
                Some(b'\\') => {
                    let c = self.next().ok_or(SexpError::UnexpectedEof { pos: self.pos })?;
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
                            let mut val: u16 = 0;
                            for _ in 0..2 {
                                let h = self
                                    .next()
                                    .ok_or(SexpError::UnexpectedEof { pos: self.pos })?;
                                if !chars::is_hex_digit(h) {
                                    return Err(SexpError::IllegalChar {
                                        ch: h,
                                        pos: self.pos,
                                    });
                                }
                                val = (val << 4) | chars::hex_value(h) as u16;
                            }
                            bytes.push(val as u8);
                        }
                        b'0'..=b'7' => {
                            let mut val: u32 = (c - b'0') as u32;
                            for _ in 0..2 {
                                let o = self.peek();
                                match o {
                                    Some(oc @ b'0'..=b'7') => {
                                        val = val * 8 + (oc - b'0') as u32;
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            if val > 255 {
                                return Err(SexpError::OctalTooBig { val });
                            }
                            bytes.push(val as u8);
                        }
                        _ => {
                            return Err(SexpError::UnknownEscape {
                                ch: c as char,
                                pos: self.pos,
                            })
                        }
                    }
                }
                Some(c) => bytes.push(c),
                None => return Err(SexpError::UnexpectedEof { pos: self.pos }),
            }
        }
    }

    fn scan_hex_string(&mut self, declared_len: Option<u32>) -> Result<Vec<u8>, SexpError> {
        self.expect_char(b'#')?;
        let mut bytes = Vec::new();
        let mut hex_buf = Vec::new();
        loop {
            match self.peek() {
                Some(b'#') => {
                    self.pos += 1;
                    break;
                }
                Some(c) if chars::is_hex_digit(c) || chars::is_whitespace(c) => {
                    if !chars::is_whitespace(c) {
                        hex_buf.push(c);
                    }
                    self.pos += 1;
                }
                None => return Err(SexpError::UnexpectedEof { pos: self.pos }),
                Some(c) => {
                    return Err(SexpError::IllegalChar {
                        ch: c,
                        pos: self.pos,
                    })
                }
            }
        }
        // Decode hex pairs
        let mut hex_str = String::from_utf8_lossy(&hex_buf).to_string();
        hex_str.retain(|c| !c.is_whitespace());
        if hex_str.len() % 2 != 0 {
            return Err(SexpError::IllegalChar {
                ch: 0,
                pos: self.pos,
            });
        }
        for chunk in hex_str.as_bytes().chunks(2) {
            let hi = chars::hex_value(chunk[0]);
            let lo = chars::hex_value(chunk[1]);
            bytes.push((hi << 4) | lo);
        }
        if let Some(declared) = declared_len {
            if bytes.len() as u32 != declared {
                return Err(SexpError::LengthMismatch {
                    declared,
                    actual: bytes.len(),
                    pos: self.pos,
                });
            }
        }
        Ok(bytes)
    }

    fn scan_base64_string(&mut self, declared_len: Option<u32>) -> Result<Vec<u8>, SexpError> {
        self.expect_char(b'|')?;
        let mut b64 = String::new();
        loop {
            match self.peek() {
                Some(b'|') => {
                    self.pos += 1;
                    break;
                }
                Some(c) if chars::is_base64_digit(c) || c == b'=' || chars::is_whitespace(c) => {
                    if !chars::is_whitespace(c) && c != b'=' {
                        b64.push(c as char);
                    }
                    self.pos += 1;
                }
                None => return Err(SexpError::UnexpectedEof { pos: self.pos }),
                Some(c) => {
                    return Err(SexpError::IllegalChar {
                        ch: c,
                        pos: self.pos,
                    })
                }
            }
        }
        let bytes = base64_decode(&b64)?;
        if let Some(declared) = declared_len {
            if bytes.len() as u32 != declared {
                return Err(SexpError::LengthMismatch {
                    declared,
                    actual: bytes.len(),
                    pos: self.pos,
                });
            }
        }
        Ok(bytes)
    }

    fn scan_simple_string(&mut self) -> Result<Vec<u8>, SexpError> {
        self.skip_whitespace();
        let c = self.peek().ok_or(SexpError::UnexpectedEof { pos: self.pos })?;

        if chars::is_token_char(c) && !chars::is_dec_digit(c) {
            return Ok(self.scan_token());
        }

        let declared_len = if chars::is_dec_digit(c) {
            Some(self.scan_decimal()?)
        } else {
            None
        };

        let next_c = self
            .peek()
            .ok_or(SexpError::UnexpectedEof { pos: self.pos })?;
        match next_c {
            b'"' => self.scan_quoted_string(declared_len),
            b'#' => self.scan_hex_string(declared_len),
            b'|' => self.scan_base64_string(declared_len),
            b':' => self.scan_verbatim(declared_len.unwrap_or(0)),
            _ => Err(SexpError::IllegalChar {
                ch: next_c,
                pos: self.pos,
            }),
        }
    }

    fn scan_string(&mut self) -> Result<SexpString, SexpError> {
        self.skip_whitespace();

        // Check for presentation hint: [hint]string
        let data = if self.peek() == Some(b'[') {
            self.pos += 1; // skip [
            let hint = self.scan_simple_string()?;
            self.expect_char(b']')?;
            let mut s = SexpString::from_bytes(self.scan_simple_string()?);
            s.presentation_hint = Some(hint);
            s
        } else {
            SexpString::from_bytes(self.scan_simple_string()?)
        };

        Ok(data)
    }

    fn scan_list(&mut self) -> Result<SexpList, SexpError> {
        self.expect_char(b'(')?;
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(SexpError::MaxDepthExceeded {
                max: self.max_depth,
                pos: self.pos,
            });
        }

        let mut list = SexpList::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b')') => {
                    self.pos += 1;
                    self.depth -= 1;
                    break;
                }
                None => return Err(SexpError::UnexpectedEof { pos: self.pos }),
                _ => {
                    let elem = self.scan_object()?;
                    list.push(elem);
                }
            }
        }
        Ok(list)
    }

    fn scan_object(&mut self) -> Result<Sexp, SexpError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'(') => Ok(Sexp::List(self.scan_list()?)),
            Some(b'{') => {
                // Base64-wrapped canonical object
                self.pos += 1;
                let inner = self.scan_object()?;
                self.expect_char(b'}')?;
                Ok(inner)
            }
            None => Err(SexpError::UnexpectedEof { pos: self.pos }),
            Some(_) => Ok(Sexp::String(self.scan_string()?)),
        }
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, SexpError> {
    use std::convert::TryFrom;
    let chars: Vec<char> = input.chars().collect();
    let mut result = Vec::with_capacity(chars.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut nbits: u32 = 0;
    for &c in &chars {
        let val = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            'a'..='z' => (c as u32) - ('a' as u32) + 26,
            '0'..='9' => (c as u32) - ('0' as u32) + 52,
            '+' => 62,
            '/' => 63,
            '=' => break,
            _ => continue,
        };
        buf = (buf << 6) | val;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            result.push(u8::try_from((buf >> nbits) & 0xFF).unwrap());
        }
    }
    Ok(result)
}

pub fn parse_advanced(input: &str) -> Result<Sexp, SexpError> {
    let mut p = Parser::new(input.as_bytes());
    let sexp = p.scan_object()?;
    p.skip_whitespace();
    if p.pos < p.input.len() {
        return Err(SexpError::IllegalChar {
            ch: p.input[p.pos],
            pos: p.pos,
        });
    }
    Ok(sexp)
}

pub fn parse_canonical(input: &[u8]) -> Result<Sexp, SexpError> {
    // Canonical form uses the same syntax as advanced, but without
    // whitespace between elements and without pretty-printing.
    // The parser handles both since the grammar is a superset.
    let mut p = Parser::new(input);
    p.scan_object()
}

/// Parse GnuPG extended private key format.
/// Returns the raw fields as a Sexp list.
pub fn parse_extended(input: &[u8]) -> Result<Sexp, SexpError> {
    // The extended format is a sequence of "name: value\n" pairs followed
    // by the actual S-expression key. For now, delegate to the regular
    // parser — the extended format wrapper can be added later.
    parse_canonical(input)
}

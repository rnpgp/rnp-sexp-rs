//! GnuPG 2.3+ extended private key format. Port of ext-key-format.cpp.
//!
//! Format: a sequence of `Name: value` header-like pairs (case-insensitive
//! names, multi-valued), followed by a mandatory single `Key:` field holding
//! the S-expression. Continuation lines start with a single space; an empty
//! continuation line encodes a newline. Windows (CRLF) and Unix (LF) line
//! endings are both accepted.
//!
//! In the C++ port, `ext_key_input_stream_t` inherits from
//! `sexp_input_stream_t` and overrides `read_char` to handle continuations.
//! Here, [`ExtKeyInputStream`] implements [`crate::CharSource`] and is passed
//! to a regular [`crate::SexpInputStream`] for parsing the Key body. The
//! shared `count`/`next_char`/`byte_size` state lives in the SexpInputStream,
//! while the continuation logic stays in the ExtKeyInputStream.

use crate::chars;
use crate::error::{Severity, SexpError};
use crate::input::{CharSource, SexpInputStream, EOF};
use crate::{Sexp, SexpList};

#[derive(Debug, Clone, Default)]
pub struct ExtendedPrivateKey {
    /// The parsed `Key:` field. Empty if parsing succeeded via the bare-list
    /// path (no header pairs, the whole input was just a canonical key).
    pub key: SexpList,
    /// Header fields, case-insensitive multimap.
    pub fields: Vec<(String, String)>,
}

impl ExtendedPrivateKey {
    pub fn fields_count(&self, name: &str) -> usize {
        self.fields
            .iter()
            .filter(|(n, _)| eq_ignore_case(n, name))
            .count()
    }

    pub fn find(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(n, _)| eq_ignore_case(n, name))
            .map(|(_, v)| v.as_str())
    }
}

fn eq_ignore_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Wraps the byte slice with continuation-line awareness required by the
/// extended key format. The C++ implementation maintains an `is_scanning_value`
/// flag that toggles continuation handling; this wrapper exposes it via
/// [`Self::set_scanning_value`].
pub struct ExtKeyInputStream<'a> {
    input: &'a [u8],
    pos: usize,
    count: i32,
    is_scanning_value: bool,
    has_key: bool,
}

impl<'a> ExtKeyInputStream<'a> {
    pub fn new(input: &'a (impl AsRef<[u8]> + ?Sized)) -> Self {
        let bytes = input.as_ref();
        ExtKeyInputStream {
            input: bytes,
            pos: 0,
            count: -1,
            is_scanning_value: false,
            has_key: false,
        }
    }

    pub fn count(&self) -> i32 {
        self.count
    }

    pub fn set_scanning_value(&mut self, v: bool) {
        self.is_scanning_value = v;
    }

    /// Mirrors C++ `ext_key_input_stream_t::skip_line`. Note: unlike
    /// `next_byte`, this does NOT increment `count` — the C++ original consumes
    /// bytes via `input_file->get()` without touching `count`, which is why
    /// comment-line bytes don't appear in error positions.
    fn skip_line(&mut self) -> i32 {
        let mut c;
        loop {
            if self.pos >= self.input.len() {
                c = EOF;
                break;
            }
            c = self.input[self.pos] as i32;
            self.pos += 1;
            if is_newline(c) || c == EOF {
                break;
            }
        }
        c
    }

    fn next_byte(&mut self) -> i32 {
        if self.pos >= self.input.len() {
            self.count += 1;
            return EOF;
        }
        let c = self.input[self.pos] as i32;
        self.pos += 1;
        self.count += 1;
        c
    }

    fn peek_byte(&self) -> i32 {
        if self.pos >= self.input.len() {
            return EOF;
        }
        self.input[self.pos] as i32
    }

    fn consume_byte(&mut self) -> i32 {
        self.next_byte()
    }

    /// Mirrors C++ `ext_key_input_stream_t::read_char` — the override that
    /// applies continuation-line unwinding when `is_scanning_value` is set.
    fn read_char_ext(&mut self) -> i32 {
        let mut lookahead_1 = self.next_byte();
        if self.is_scanning_value && is_newline(lookahead_1) {
            loop {
                let mut lookahead_2 = self.peek_byte();
                if lookahead_1 == b'\r' as i32 && lookahead_2 == b'\n' as i32 {
                    lookahead_1 = self.consume_byte();
                    lookahead_2 = self.peek_byte();
                }
                if lookahead_2 == b' ' as i32 {
                    self.consume_byte();
                    let lookahead_3 = self.peek_byte();
                    if lookahead_3 == b'#' as i32 {
                        lookahead_1 = self.skip_line();
                        continue;
                    }
                    if is_newline(lookahead_3) {
                        // Blank continuation line: encodes a literal newline.
                        // Do NOT consume — the next iteration will peek it
                        // again and return it as the logical char.
                        lookahead_1 = lookahead_3;
                        continue;
                    }
                    lookahead_1 = self.consume_byte();
                }
                return lookahead_1;
            }
        }
        lookahead_1
    }

    fn is_namechar(c: i32) -> bool {
        if !(0..=255).contains(&c) {
            return false;
        }
        let b = c as u8;
        b == b'-' || b.is_ascii_digit() || b.is_ascii_uppercase() || b.is_ascii_lowercase()
    }

    fn is_alpha(c: i32) -> bool {
        c >= 0 && (c as u8).is_ascii_alphabetic()
    }

    fn scan_name(&mut self, initial_c: i32) -> Result<String, SexpError> {
        let mut name = String::new();
        let mut c = initial_c;
        if !Self::is_alpha(c) {
            let msg = if c != EOF && (c as u8).is_ascii_graphic() {
                format!(
                    "unexpected character '{}' (0x{:x}) found starting a name field",
                    c as u8 as char, c as u8
                )
            } else {
                format!("unexpected character 0x{:x} found starting a name field", c)
            };
            return Err(SexpError::ext_key(msg, Severity::Error, self.count));
        }
        name.push(c as u8 as char);
        c = self.read_char_ext();
        while c != b':' as i32 {
            if c == EOF {
                return Err(SexpError::ext_key(
                    "unexpected end of file",
                    Severity::Error,
                    self.count,
                ));
            }
            if is_newline(c) {
                return Err(SexpError::ext_key(
                    "unexpected end of line",
                    Severity::Error,
                    self.count,
                ));
            }
            if !Self::is_namechar(c) {
                let msg = if (c as u8).is_ascii_graphic() {
                    format!(
                        "unexpected character '{}' (0x{:x}) found in a name field",
                        c as u8 as char, c as u8
                    )
                } else {
                    format!("unexpected character 0x{:x} found in a name field", c)
                };
                return Err(SexpError::ext_key(msg, Severity::Error, self.count));
            }
            name.push(c as u8 as char);
            c = self.read_char_ext();
        }
        Ok(name)
    }

    fn scan_value(&mut self) -> Result<String, SexpError> {
        let mut value = String::new();
        let mut c;
        loop {
            c = self.read_char_ext();
            if !chars::is_whitespace(c as u8) {
                break;
            }
        }
        while c != EOF && !is_newline(c) {
            value.push(c as u8 as char);
            c = self.read_char_ext();
        }
        Ok(value)
    }

    /// Parse the entire input as an extended private key.
    pub fn scan(&mut self, res: &mut ExtendedPrivateKey) -> Result<(), SexpError> {
        let first = self.read_char_ext();
        let mut c = first;
        if c == b'(' as i32 {
            // Bare canonical/advanced key with no header pairs.
            res.key = self.parse_key_list(c)?;
            self.has_key = true;
            return Ok(());
        }
        while c != EOF {
            let name = self.scan_name(c)?;
            self.is_scanning_value = true;
            if eq_ignore_case(&name, "key") {
                if self.has_key {
                    return Err(SexpError::ext_key(
                        "'key' field must occur only once",
                        Severity::Error,
                        self.count,
                    ));
                }
                loop {
                    c = self.read_char_ext();
                    if !chars::is_whitespace(c as u8) {
                        break;
                    }
                }
                res.key = self.parse_key_list(c)?;
                self.has_key = true;
            } else {
                let value = self.scan_value()?;
                res.fields.push((name, value));
            }
            c = self.read_char_ext();
            self.is_scanning_value = false;
        }
        if !self.has_key {
            return Err(SexpError::ext_key(
                "missing mandatory 'key' field",
                Severity::Error,
                self.count,
            ));
        }
        Ok(())
    }

    /// Hand off control to a regular [`SexpInputStream`] for parsing the Key
    /// body. We implement [`CharSource`] so the sexp parser reads through us,
    /// applying continuation-line unwinding transparently. `first_char` (the
    /// first non-whitespace byte after `Key:`) is fed to the parser by
    /// priming its `next_char` — the source's `read_char` then provides the
    /// bytes that follow.
    ///
    /// Our own `count` field continues to track the underlying byte position
    /// because `read_char_ext` increments it on each call. The SexpInputStream
    /// has a separate offset count which we deliberately do NOT sync back,
    /// since our count already reflects the true position.
    fn parse_key_list(&mut self, first_char: i32) -> Result<SexpList, SexpError> {
        let obj: Sexp = {
            let mut sexp_is = SexpInputStream::from_source(&mut *self);
            sexp_is.next_char = first_char;
            sexp_is.scan_object()?
        };
        match obj {
            Sexp::List(l) => Ok(l),
            Sexp::String(_) => Err(SexpError::ext_key(
                "expected Key field to be a list",
                Severity::Error,
                self.count,
            )),
        }
    }
}

impl<'a> CharSource for ExtKeyInputStream<'a> {
    fn read_char(&mut self) -> i32 {
        self.read_char_ext()
    }
}

fn is_newline(c: i32) -> bool {
    c == b'\r' as i32 || c == b'\n' as i32
}

/// Convenience: parse `input` as an extended private key.
pub fn parse_extended(input: &[u8]) -> Result<ExtendedPrivateKey, SexpError> {
    let mut scanner = ExtKeyInputStream::new(input);
    let mut key = ExtendedPrivateKey::default();
    scanner.scan(&mut key)?;
    Ok(key)
}

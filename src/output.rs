//! S-expression output stream. Port of sexp-output.cpp, sexp-simple-string.cpp,
//! and sexp-object.cpp.
//!
//! Three print modes:
//! - `Canonical`: `(length:bytes)`, binary-safe, no whitespace.
//! - `Advanced`: human-readable, pretty-printed, line-wrapped at 75 cols by default.
//! - `Base64`: the canonical form base64-encoded and wrapped in `{...}`.
//!
//! Advanced-mode printing of each simple string picks among token, quoted,
//! hexadecimal (length<=4 with non-printable bytes), or base64 forms based on
//! content AND remaining column width. List layout chooses horizontal vs.
//! vertical based on the list's `advanced_length` and the remaining column.

use crate::chars;
use crate::depth::{DepthManager, DEFAULT_MAX_DEPTH};
use crate::error::{Severity, SexpError};
use crate::{Sexp, SexpList, SexpString};

const HEX_DIGITS: &[u8] = b"0123456789ABCDEF";
const BASE64_DIGITS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const EOF_POS: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintMode {
    Canonical,
    Base64,
    Advanced,
}

pub const DEFAULT_LINE_LENGTH: u32 = 75;

#[derive(Debug)]
pub struct SexpOutputStream {
    out: Vec<u8>,
    pub base64_count: u32,
    pub byte_size: u32,
    bits: u32,
    n_bits: u32,
    pub mode: PrintMode,
    pub column: u32,
    pub max_column: u32,
    pub indent: u32,
    depth: DepthManager,
}

impl Default for SexpOutputStream {
    fn default() -> Self {
        Self::new()
    }
}

impl SexpOutputStream {
    pub fn new() -> Self {
        Self::new_with_max_depth(DEFAULT_MAX_DEPTH)
    }

    pub fn new_with_max_depth(max_depth: usize) -> Self {
        SexpOutputStream {
            out: Vec::new(),
            base64_count: 0,
            byte_size: 8,
            bits: 0,
            n_bits: 0,
            mode: PrintMode::Canonical,
            column: 0,
            max_column: DEFAULT_LINE_LENGTH,
            indent: 0,
            depth: DepthManager::new(max_depth),
        }
    }

    pub fn set_max_column(&mut self, mc: u32) -> &mut Self {
        self.max_column = mc;
        self
    }
    pub fn get_max_column(&self) -> u32 {
        self.max_column
    }
    pub fn get_column(&self) -> u32 {
        self.column
    }
    pub fn get_byte_size(&self) -> u32 {
        self.byte_size
    }
    pub fn reset_column(&mut self) -> &mut Self {
        self.column = 0;
        self
    }
    pub fn inc_indent(&mut self) -> &mut Self {
        self.indent += 1;
        self
    }
    pub fn dec_indent(&mut self) -> &mut Self {
        self.indent -= 1;
        self
    }

    pub fn put_char(&mut self, c: u8) -> &mut Self {
        self.out.push(c);
        self.column += 1;
        self
    }

    /// Emit a byte through the current byte-size encoder. In canonical/advanced
    /// mode (byte_size=8) this is a passthrough; in hex (4) or base64 (6) mode
    /// it accumulates bits and emits encoded chars.
    pub fn var_put_char(&mut self, c: u8) -> Result<&mut Self, SexpError> {
        let c_byte = c;
        self.bits = (self.bits << 8) | c as u32;
        self.n_bits += 8;
        while self.n_bits >= self.byte_size {
            let wrap = (self.byte_size == 6
                || self.byte_size == 4
                || c_byte == b'}'
                || c_byte == b'{'
                || c_byte == b'#'
                || c_byte == b'|')
                && self.max_column > 0
                && self.column >= self.max_column;
            if wrap {
                self.new_line(self.mode);
            }
            if self.byte_size == 4 {
                let idx = (self.bits >> (self.n_bits - 4)) & 0x0F;
                self.put_char(HEX_DIGITS[idx as usize]);
            } else if self.byte_size == 6 {
                let idx = (self.bits >> (self.n_bits - 6)) & 0x3F;
                self.put_char(BASE64_DIGITS[idx as usize]);
            } else {
                self.put_char((self.bits & 0xFF) as u8);
            }
            self.n_bits -= self.byte_size;
            self.base64_count += 1;
        }
        Ok(self)
    }

    pub fn new_line(&mut self, mode: PrintMode) -> &mut Self {
        if mode == PrintMode::Advanced || mode == PrintMode::Base64 {
            self.put_char(b'\n');
            self.column = 0;
        }
        if mode == PrintMode::Advanced {
            let mut i = 0;
            while i < self.indent && 4 * i < self.max_column {
                self.put_char(b' ');
                i += 1;
            }
        }
        self
    }

    pub fn change_output_byte_size(
        &mut self,
        new_size: u32,
        mode: PrintMode,
    ) -> Result<&mut Self, SexpError> {
        if new_size != 4 && new_size != 6 && new_size != 8 {
            return Err(SexpError::sexp(
                format!("Illegal output base {}", new_size),
                Severity::Error,
                EOF_POS,
            ));
        }
        if new_size != 8 && self.byte_size != 8 {
            return Err(SexpError::sexp(
                format!(
                    "Illegal change of output byte size from {} to {}",
                    self.byte_size, new_size
                ),
                Severity::Error,
                EOF_POS,
            ));
        }
        self.byte_size = new_size;
        self.n_bits = 0;
        self.bits = 0;
        self.base64_count = 0;
        self.mode = mode;
        Ok(self)
    }

    /// Flush any remaining bits and pad base64 output to a multiple of 4 chars.
    pub fn flush(&mut self) -> Result<&mut Self, SexpError> {
        if self.n_bits > 0 {
            // byte_size must be 6 here (asserted in C++)
            let idx = (self.bits << (6 - self.n_bits)) & 0x3F;
            self.put_char(BASE64_DIGITS[idx as usize]);
            self.n_bits = 0;
            self.base64_count += 1;
        }
        if self.byte_size == 6 {
            while (self.base64_count & 3) != 0 {
                if self.max_column > 0 && self.column >= self.max_column {
                    self.new_line(self.mode);
                }
                self.put_char(b'=');
                self.base64_count += 1;
            }
        }
        Ok(self)
    }

    pub fn print_decimal(&mut self, n: u64) -> Result<&mut Self, SexpError> {
        let s = n.to_string();
        for b in s.bytes() {
            self.var_put_char(b)?;
        }
        Ok(self)
    }

    pub fn open_list(&mut self) -> Result<&mut Self, SexpError> {
        self.put_char(b'(');
        self.increase_depth()?;
        Ok(self)
    }

    pub fn close_list(&mut self) -> Result<&mut Self, SexpError> {
        self.put_char(b')');
        self.decrease_depth();
        Ok(self)
    }

    pub fn var_open_list(&mut self) -> Result<&mut Self, SexpError> {
        self.var_put_char(b'(')?;
        self.increase_depth()?;
        Ok(self)
    }

    pub fn var_close_list(&mut self) -> Result<&mut Self, SexpError> {
        self.var_put_char(b')')?;
        self.decrease_depth();
        Ok(self)
    }

    fn increase_depth(&mut self) -> Result<(), SexpError> {
        // Position is -1 for the serializer (matches C++ default arg)
        self.depth.increase(EOF_POS)
    }

    fn decrease_depth(&mut self) {
        self.depth.decrease();
    }

    // --- Object printers (port of sexp-object.cpp) ---

    pub fn print_canonical(&mut self, obj: &Sexp) -> Result<&mut Self, SexpError> {
        match obj {
            Sexp::String(s) => self.print_canonical_string(s),
            Sexp::List(l) => self.print_canonical_list(l),
        }
    }

    pub fn print_advanced(&mut self, obj: &Sexp) -> Result<&mut Self, SexpError> {
        match obj {
            Sexp::String(s) => self.print_advanced_string(s),
            Sexp::List(l) => self.print_advanced_list(l),
        }
    }

    /// Wrap the canonical form of `obj` in `{...}` and base64-encode it.
    pub fn print_base64(&mut self, obj: &Sexp) -> Result<&mut Self, SexpError> {
        self.change_output_byte_size(8, PrintMode::Base64)?
            .var_put_char(b'{')?
            .change_output_byte_size(6, PrintMode::Base64)?;
        self.print_canonical(obj)?;
        self.flush()?
            .change_output_byte_size(8, PrintMode::Base64)?
            .var_put_char(b'}')?;
        Ok(self)
    }

    fn print_canonical_string(&mut self, s: &SexpString) -> Result<&mut Self, SexpError> {
        if let Some(hint) = &s.presentation_hint {
            self.var_put_char(b'[')?;
            self.print_canonical_verbatim(hint)?;
            self.var_put_char(b']')?;
        }
        self.print_canonical_verbatim(&s.data)?;
        Ok(self)
    }

    fn print_canonical_verbatim(&mut self, data: &[u8]) -> Result<&mut Self, SexpError> {
        self.print_decimal(data.len() as u64)?;
        self.var_put_char(b':')?;
        for &b in data {
            self.var_put_char(b)?;
        }
        Ok(self)
    }

    fn print_canonical_list(&mut self, l: &SexpList) -> Result<&mut Self, SexpError> {
        self.var_open_list()?;
        for elem in &l.elements {
            self.print_canonical(elem)?;
        }
        self.var_close_list()?;
        Ok(self)
    }

    // Mirrors sexp_object_t::print_advanced: prefix new_line if near wrap.
    fn print_advanced_object_prefix(&mut self) -> &mut Self {
        if self.max_column > 0 && self.column > self.max_column.saturating_sub(4) {
            self.new_line(PrintMode::Advanced);
        }
        self
    }

    fn print_advanced_string(&mut self, s: &SexpString) -> Result<&mut Self, SexpError> {
        self.print_advanced_object_prefix();
        if let Some(hint) = &s.presentation_hint {
            self.put_char(b'[');
            self.print_advanced_simple_string(hint)?;
            self.put_char(b']');
        }
        self.print_advanced_simple_string(&s.data)?;
        Ok(self)
    }

    fn print_advanced_list(&mut self, l: &SexpList) -> Result<&mut Self, SexpError> {
        self.print_advanced_object_prefix();
        self.open_list()?;
        self.inc_indent();
        // Vertical if the list's advanced_length won't fit in the remaining column.
        // Use wrapping_sub to match C++ unsigned arithmetic.
        let remaining = self.max_column.wrapping_sub(self.column);
        let vertical = self.advanced_length_list(l) as u32 > remaining;
        let mut first = true;
        for elem in &l.elements {
            if !first {
                if vertical {
                    self.new_line(PrintMode::Advanced);
                } else {
                    self.put_char(b' ');
                }
            }
            self.print_advanced(elem)?;
            first = false;
        }
        if self.max_column > 0 && self.column > self.max_column.saturating_sub(2) {
            self.new_line(PrintMode::Advanced);
        }
        self.dec_indent();
        self.put_char(b')');
        // open_list increased depth; pair it with a decrease here (close_list
        // would also put_char(')') which we've already done).
        self.decrease_depth();
        Ok(self)
    }

    // --- Simple-string printers (port of sexp-simple-string.cpp) ---

    fn print_advanced_simple_string(&mut self, data: &[u8]) -> Result<&mut Self, SexpError> {
        if self.can_print_as_token(data) {
            self.print_token(data)?;
        } else if self.can_print_as_quoted_string(data) {
            self.print_quoted(data);
        } else if data.len() <= 4 && self.byte_size == 8 {
            self.print_hexadecimal(data)?;
        } else if self.byte_size == 8 {
            self.print_base64_simple_string(data)?;
        } else {
            return Err(SexpError::sexp(
                "Can't print in advanced mode with restricted output character set",
                Severity::Error,
                EOF_POS,
            ));
        }
        Ok(self)
    }

    fn print_token(&mut self, data: &[u8]) -> Result<&mut Self, SexpError> {
        if self.max_column > 0 && self.column > self.max_column.saturating_sub(data.len() as u32) {
            self.new_line(PrintMode::Advanced);
        }
        for &b in data {
            self.put_char(b);
        }
        Ok(self)
    }

    fn print_quoted(&mut self, data: &[u8]) -> &mut Self {
        self.put_char(b'"');
        for &b in data {
            if self.max_column > 0 && self.column >= self.max_column.saturating_sub(2) {
                self.put_char(b'\\');
                self.put_char(b'\n');
                self.column = 0;
            }
            self.put_char(b);
        }
        self.put_char(b'"');
        self
    }

    fn print_hexadecimal(&mut self, data: &[u8]) -> Result<&mut Self, SexpError> {
        self.put_char(b'#');
        self.change_output_byte_size(4, PrintMode::Advanced)?;
        for &b in data {
            self.var_put_char(b)?;
        }
        self.flush()?
            .change_output_byte_size(8, PrintMode::Advanced)?
            .put_char(b'#');
        Ok(self)
    }

    fn print_base64_simple_string(&mut self, data: &[u8]) -> Result<&mut Self, SexpError> {
        self.var_put_char(b'|')?;
        self.change_output_byte_size(6, PrintMode::Advanced)?;
        for &b in data {
            self.var_put_char(b)?;
        }
        self.flush()?
            .change_output_byte_size(8, PrintMode::Advanced)?
            .var_put_char(b'|')?;
        Ok(self)
    }

    fn can_print_as_token(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        if chars::is_dec_digit(data[0]) {
            return false;
        }
        if self.max_column > 0 && self.column + data.len() as u32 >= self.max_column {
            return false;
        }
        data.iter().all(|&c| chars::is_token_char(c))
    }

    fn can_print_as_quoted_string(&self, data: &[u8]) -> bool {
        data.iter().all(|&c| chars::is_token_char(c) || c == b' ')
    }

    fn advanced_length_simple(&self, data: &[u8]) -> usize {
        if self.can_print_as_token(data) {
            data.len()
        } else if self.can_print_as_quoted_string(data) {
            data.len() + 2
        } else if data.len() <= 4 && self.byte_size == 8 {
            2 * data.len() + 2
        } else if self.byte_size == 8 {
            4 * ((data.len() + 2) / 3) + 2
        } else {
            0
        }
    }

    fn advanced_length_list(&self, l: &SexpList) -> usize {
        let mut len = 1;
        for elem in &l.elements {
            len += self.advanced_length(elem);
        }
        len + 1
    }

    fn advanced_length_string(&self, s: &SexpString) -> usize {
        let mut len = 0;
        if let Some(hint) = &s.presentation_hint {
            len += 2 + self.advanced_length_simple(hint);
        }
        len += self.advanced_length_simple(&s.data);
        len
    }

    pub fn advanced_length(&self, obj: &Sexp) -> usize {
        match obj {
            Sexp::String(s) => self.advanced_length_string(s),
            Sexp::List(l) => self.advanced_length_list(l),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.out
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.out
    }

    pub fn into_string_lossy(self) -> String {
        String::from_utf8_lossy(&self.out).into_owned()
    }
}

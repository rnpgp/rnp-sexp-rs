//! Pure-Rust S-expression parser and serializer for OpenPGP key formats.
//!
//! A faithful port of librnp's `sexpp` C++ library. Supports canonical,
//! advanced, and base64 printing modes plus the GnuPG 2.3+ extended private key
//! format.
//!
//! ```
//! use rnp_sexp::Sexp;
//!
//! let input = "(8:test-key (1:a 3:bcd))";
//! let sexp = Sexp::parse_advanced(input).unwrap();
//! println!("{}", sexp.to_advanced());
//! ```

mod chars;
mod depth;
pub mod error;
mod ext_key;
mod input;
mod output;

pub use error::{Severity, SexpError};
pub use ext_key::{parse_extended, ExtKeyInputStream, ExtendedPrivateKey};
pub use input::{CharSource, SexpInputStream, SliceSource};
pub use output::{PrintMode, SexpOutputStream};

/// An S-expression value: either a list or a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sexp {
    List(SexpList),
    String(SexpString),
}

/// An S-expression list: `(elem1 elem2 ...)`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SexpList {
    pub elements: Vec<Sexp>,
}

/// An S-expression string: raw bytes with an optional presentation hint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SexpString {
    pub data: Vec<u8>,
    pub presentation_hint: Option<Vec<u8>>,
}

impl Sexp {
    pub fn list(elements: Vec<Sexp>) -> Self {
        Sexp::List(SexpList { elements })
    }

    pub fn string(data: impl Into<Vec<u8>>) -> Self {
        Sexp::String(SexpString {
            data: data.into(),
            presentation_hint: None,
        })
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Sexp::List(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Sexp::String(_))
    }

    pub fn as_list(&self) -> Option<&SexpList> {
        match self {
            Sexp::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&SexpString> {
        match self {
            Sexp::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn parse_advanced(input: &str) -> Result<Self, SexpError> {
        let mut is = SexpInputStream::new(input);
        is.get_char()?;
        is.scan_object()
    }

    pub fn parse_canonical(input: &[u8]) -> Result<Self, SexpError> {
        let mut is = SexpInputStream::new(input);
        is.get_char()?;
        is.scan_object()
    }

    pub fn to_string_with_mode(&self, mode: PrintMode) -> String {
        let mut os = SexpOutputStream::new();
        match mode {
            PrintMode::Canonical => os.print_canonical(self).ok(),
            PrintMode::Advanced => os.print_advanced(self).ok(),
            PrintMode::Base64 => os.print_base64(self).ok(),
        };
        os.into_string_lossy()
    }

    pub fn to_advanced(&self) -> String {
        self.to_string_with_mode(PrintMode::Advanced)
    }

    pub fn to_canonical(&self) -> Vec<u8> {
        let mut os = SexpOutputStream::new();
        os.print_canonical(self).ok();
        os.into_bytes()
    }

    pub fn to_base64(&self) -> String {
        self.to_string_with_mode(PrintMode::Base64)
    }

    pub fn equals_str(&self, other: &str) -> bool {
        match self {
            Sexp::String(s) => s.equals_str(other),
            _ => false,
        }
    }

    /// Mirrors C++ `as_unsigned()`: returns u32::MAX if the data is empty or
    /// not a valid decimal, otherwise the parsed value.
    pub fn as_unsigned(&self) -> u32 {
        match self {
            Sexp::String(s) => s.as_unsigned(),
            _ => u32::MAX,
        }
    }
}

impl SexpString {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_bytes(data: impl Into<Vec<u8>>) -> Self {
        SexpString {
            data: data.into(),
            presentation_hint: None,
        }
    }

    pub fn from_string(s: &str) -> Self {
        Self::from_bytes(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.data)
    }

    pub fn has_presentation_hint(&self) -> bool {
        self.presentation_hint.is_some()
    }

    /// Mirrors C++ `as_unsigned()`: returns u32::MAX on empty or non-numeric
    /// data, otherwise the parsed value.
    pub fn as_unsigned(&self) -> u32 {
        if self.data.is_empty() {
            return u32::MAX;
        }
        let s = match std::str::from_utf8(&self.data) {
            Ok(s) => s,
            Err(_) => return u32::MAX,
        };
        s.parse::<u32>().unwrap_or(u32::MAX)
    }

    pub fn equals_str(&self, other: &str) -> bool {
        self.data == other.as_bytes()
    }
}

impl SexpList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, elem: Sexp) {
        self.elements.push(elem);
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Sexp> {
        self.elements.iter()
    }

    pub fn get(&self, idx: usize) -> Option<&Sexp> {
        self.elements.get(idx)
    }

    pub fn sexp_list_at(&self, idx: usize) -> Option<&SexpList> {
        self.elements.get(idx).and_then(|e| e.as_list())
    }

    pub fn sexp_string_at(&self, idx: usize) -> Option<&SexpString> {
        self.elements.get(idx).and_then(|e| e.as_string())
    }
}

impl std::ops::Index<usize> for SexpList {
    type Output = Sexp;
    fn index(&self, idx: usize) -> &Sexp {
        &self.elements[idx]
    }
}

// --- Top-level convenience functions (mirror upstream free functions) ---

pub fn parse_advanced(input: &str) -> Result<Sexp, SexpError> {
    Sexp::parse_advanced(input)
}

pub fn parse_canonical(input: &[u8]) -> Result<Sexp, SexpError> {
    Sexp::parse_canonical(input)
}

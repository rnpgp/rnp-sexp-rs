//! Pure-Rust S-expression parser and serializer for OpenPGP key formats.
//!
//! A faithful port of librnp's `sexpp` C++ library. Supports canonical,
//! advanced, and base64 printing modes.
//!
//! ```
//! use rnp_sexp::Sexp;
//!
//! let input = "(8:test-key (1:a 3:bcd))";
//! let sexp = Sexp::parse_advanced(input).unwrap();
//! println!("{}", sexp.to_advanced());
//! ```

mod chars;
mod error;
mod parse;
mod ser;

pub use error::SexpError;
pub use parse::{parse_advanced, parse_canonical, parse_extended};
pub use ser::PrintMode;

mod types {
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

        pub fn parse_advanced(input: &str) -> Result<Self, crate::SexpError> {
            crate::parse::parse_advanced(input)
        }

        pub fn parse_canonical(input: &[u8]) -> Result<Self, crate::SexpError> {
            crate::parse::parse_canonical(input)
        }

        pub fn to_string_with_mode(&self, mode: crate::PrintMode) -> String {
            crate::ser::serialize(self, mode)
        }

        pub fn to_advanced(&self) -> String {
            self.to_string_with_mode(crate::PrintMode::Advanced)
        }

        pub fn to_canonical(&self) -> Vec<u8> {
            crate::ser::serialize_canonical_bytes(self)
        }

        pub fn equals_str(&self, other: &str) -> bool {
            match self {
                Sexp::String(s) => s.equals_str(other),
                _ => false,
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

        pub fn as_unsigned(&self) -> Option<u32> {
            if self.data.is_empty() {
                return None;
            }
            let s = std::str::from_utf8(&self.data).ok()?;
            s.parse::<u32>().ok()
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
    }

    impl std::ops::Index<usize> for SexpList {
        type Output = Sexp;
        fn index(&self, idx: usize) -> &Sexp {
            &self.elements[idx]
        }
    }
}

pub use types::*;

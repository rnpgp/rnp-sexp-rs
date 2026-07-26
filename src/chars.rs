//! Character classification tables. Port of sexp-char-defs.cpp.

/// Token characters: alpha, digit, and the punctuation set defined by the
/// S-expression spec. Note that `:` is a token char — this matters for the
/// parser's verbatim-vs-token dispatch.
pub fn is_token_char(c: u8) -> bool {
    matches!(
        c,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'/'
            | b'_'
            | b':'
            | b'!'
            | b'*'
            | b'+'
            | b'&'
            | b'='
            | b'^'
            | b'$'
            | b'@'
            | b'%'
    )
}

pub fn is_dec_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

pub fn is_hex_digit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

pub fn is_base64_digit(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'+' || c == b'/'
}

#[allow(dead_code)]
pub fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

pub fn is_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b)
}

pub fn dec_value(c: u8) -> u8 {
    if c.is_ascii_digit() {
        c - b'0'
    } else {
        0
    }
}

pub fn hex_value(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

pub fn base64_value(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => 0,
    }
}

#[allow(dead_code)]
pub const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

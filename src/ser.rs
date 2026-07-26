//! Serializer — port of sexp-output.cpp.

use crate::chars;
use crate::{Sexp, SexpString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintMode {
    /// Canonical form: `(length:bytes)`, no whitespace, binary-safe.
    Canonical,
    /// Advanced form: human-readable, pretty-printed, line-wrapped.
    Advanced,
}

pub fn serialize(sexp: &Sexp, mode: PrintMode) -> String {
    let mut out = String::new();
    let mut ctx = SerCtx {
        column: 0,
        max_column: 75,
        indent: 0,
        depth: 0,
    };
    match mode {
        PrintMode::Canonical => serialize_canonical(sexp, &mut out),
        PrintMode::Advanced => serialize_advanced(sexp, &mut out, &mut ctx),
    }
    out
}

pub fn serialize_canonical_bytes(sexp: &Sexp) -> Vec<u8> {
    let mut out = Vec::new();
    serialize_canonical_into(sexp, &mut out);
    out
}

struct SerCtx {
    column: usize,
    max_column: usize,
    indent: usize,
    depth: usize,
}

fn serialize_canonical(sexp: &Sexp, out: &mut String) {
    match sexp {
        Sexp::String(s) => serialize_string_canonical(s, out),
        Sexp::List(l) => {
            out.push('(');
            for elem in &l.elements {
                serialize_canonical(elem, out);
            }
            out.push(')');
        }
    }
}

fn serialize_canonical_into(sexp: &Sexp, out: &mut Vec<u8>) {
    match sexp {
        Sexp::String(s) => {
            out.extend_from_slice(format!("{}:", s.data.len()).as_bytes());
            out.extend_from_slice(&s.data);
        }
        Sexp::List(l) => {
            out.push(b'(');
            for elem in &l.elements {
                serialize_canonical_into(elem, out);
            }
            out.push(b')');
        }
    }
}

fn serialize_string_canonical(s: &SexpString, out: &mut String) {
    if let Some(ref hint) = s.presentation_hint {
        out.push('[');
        out.push_str(&format!("{}:", hint.len()));
        // SAFETY: presentation hints are typically ASCII
        out.push_str(&String::from_utf8_lossy(hint));
        out.push(']');
    }
    out.push_str(&format!("{}:", s.data.len()));
    // In canonical form the bytes are raw; this may produce invalid UTF-8
    // but the caller asked for String. Use lossy conversion.
    out.push_str(&String::from_utf8_lossy(&s.data));
}

fn serialize_advanced(sexp: &Sexp, out: &mut String, ctx: &mut SerCtx) {
    match sexp {
        Sexp::String(s) => serialize_string_advanced(s, out, ctx),
        Sexp::List(l) => {
            out.push('(');
            ctx.column += 1;
            ctx.depth += 1;
            ctx.indent += 1;
            for (i, elem) in l.elements.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                    ctx.column += 1;
                }
                if ctx.column >= ctx.max_column {
                    out.push('\n');
                    for _ in 0..ctx.indent {
                        out.push(' ');
                    }
                    ctx.column = ctx.indent;
                }
                serialize_advanced(elem, out, ctx);
            }
            ctx.indent -= 1;
            ctx.depth -= 1;
            out.push(')');
            ctx.column += 1;
        }
    }
}

fn serialize_string_advanced(s: &SexpString, out: &mut String, _ctx: &mut SerCtx) {
    if let Some(ref hint) = s.presentation_hint {
        out.push('[');
        out.push_str(&format!("{}:", hint.len()));
        out.push_str(&String::from_utf8_lossy(hint));
        out.push(']');
    }

    if s.data.is_empty() {
        out.push_str("0:");
        return;
    }

    // Try token form first (bare alphanumeric string)
    if can_print_as_token(&s.data) {
        out.push_str(&String::from_utf8_lossy(&s.data));
        return;
    }

    // Try quoted string form
    if can_print_as_quoted(&s.data) {
        out.push('"');
        for &c in &s.data {
            match c {
                b'\\' => out.push_str("\\\\"),
                b'"' => out.push_str("\\\""),
                b'\t' => out.push_str("\\t"),
                b'\n' => out.push_str("\\n"),
                b'\r' => out.push_str("\\r"),
                0x20..=0x7e => out.push(c as char),
                _ => {
                    out.push_str(&format!("\\x{:02x}", c));
                }
            }
        }
        out.push('"');
        return;
    }

    // Fall back to verbatim canonical form (length:bytes)
    out.push_str(&format!("{}:", s.data.len()));
    out.push_str(&String::from_utf8_lossy(&s.data));
}

fn can_print_as_token(data: &[u8]) -> bool {
    !data.is_empty()
        && data.iter().all(|&c| {
            (chars::is_token_char(c) && !chars::is_dec_digit(c)) || c == b':'
        })
        && !data.iter().all(|&c| chars::is_dec_digit(c))
}

fn can_print_as_quoted(data: &[u8]) -> bool {
    !data.is_empty()
        && data.iter().all(|&c| {
            c == b'\t' || c == b'\n' || c == b'\r' || (0x20..=0x7e).contains(&c)
        })
}

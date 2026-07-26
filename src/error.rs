//! Error type and reporting infrastructure. Port of sexp-error.cpp.
//!
//! Error messages must match upstream format exactly because the ported
//! exception-tests assert on the formatted `what()` string:
//!   `<prefix> ERROR: <msg> at position <N>`
//!   `<prefix> WARNING: <msg> at position <N>`
//! where the position suffix is omitted when position < 0.

use std::cell::Cell;
use std::fmt;

thread_local! {
    static VERBOSITY: Cell<u8> = const { Cell::new(0) }; // 0 = Error, 1 = Warning
    static INTERACTIVE: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SexpError {
    pub message: String,
    pub level: Severity,
    pub position: i32,
}

impl SexpError {
    pub fn new(prefix: &str, msg: impl Into<String>, level: Severity, position: i32) -> Self {
        let level_str = match level {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
        };
        let mut full = format!("{} {}: {}", prefix, level_str, msg.into());
        if position >= 0 {
            full.push_str(&format!(" at position {}", position));
        }
        SexpError {
            message: full,
            level,
            position,
        }
    }

    pub fn sexp(msg: impl Into<String>, level: Severity, position: i32) -> Self {
        Self::new("SEXP", msg, level, position)
    }

    pub fn ext_key(msg: impl Into<String>, level: Severity, position: i32) -> Self {
        Self::new("EXTENDED KEY FORMAT", msg, level, position)
    }
}

impl fmt::Display for SexpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SexpError {}

pub fn set_verbosity(level: Severity) {
    VERBOSITY.with(|v| {
        v.set(match level {
            Severity::Error => 0,
            Severity::Warning => 1,
        })
    });
}

pub fn get_verbosity() -> Severity {
    VERBOSITY.with(|v| match v.get() {
        0 => Severity::Error,
        _ => Severity::Warning,
    })
}

pub fn set_interactive(b: bool) {
    INTERACTIVE.with(|c| c.set(b));
}

pub fn is_interactive() -> bool {
    INTERACTIVE.with(|c| c.get())
}

pub fn shall_throw(level: Severity) -> bool {
    level == Severity::Error || get_verbosity() != Severity::Error
}

/// Report an error or warning. Mirrors C++ `sexp_error(...)`:
/// - If `shall_throw`, returns Err.
/// - Else if interactive mode is on, prints to stdout.
/// - Else, silently drops.
pub fn report(prefix: &str, msg: &str, level: Severity, position: i32) -> Result<(), SexpError> {
    let err = SexpError::new(prefix, msg, level, position);
    if shall_throw(level) {
        Err(err)
    } else {
        if is_interactive() {
            println!();
            println!("*** {} ***", err.message);
        }
        Ok(())
    }
}

pub fn sexp_report(msg: &str, level: Severity, position: i32) -> Result<(), SexpError> {
    report("SEXP", msg, level, position)
}

pub fn ext_key_report(msg: &str, level: Severity, position: i32) -> Result<(), SexpError> {
    report("EXTENDED KEY FORMAT", msg, level, position)
}

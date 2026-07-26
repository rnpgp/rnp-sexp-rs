//! Nesting-depth manager shared by parser and serializer. Port of
//! sexp-depth-manager.cpp.

use crate::error::{sexp_report, Severity, SexpError};

pub const DEFAULT_MAX_DEPTH: usize = 1024;

#[derive(Debug)]
pub struct DepthManager {
    pub depth: usize,
    pub max_depth: usize,
}

impl DepthManager {
    pub fn new(max_depth: usize) -> Self {
        DepthManager {
            depth: 0,
            max_depth,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self, max_depth: usize) {
        self.depth = 0;
        self.max_depth = max_depth;
    }

    /// Increase depth, returning an error if the limit is exceeded.
    /// `position` is the value reported in the error message — pass a
    /// negative value to omit the position suffix (used by the serializer).
    pub fn increase(&mut self, position: i32) -> Result<(), SexpError> {
        self.depth += 1;
        if self.max_depth != 0 && self.depth > self.max_depth {
            sexp_report(
                &format!(
                    "Maximum allowed SEXP list depth ({}) is exceeded",
                    self.max_depth
                ),
                Severity::Error,
                position,
            )
        } else {
            Ok(())
        }
    }

    pub fn decrease(&mut self) {
        self.depth -= 1;
    }
}

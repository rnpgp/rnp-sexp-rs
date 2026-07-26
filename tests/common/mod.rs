//! Test helpers ported from compare-files.cpp.

use std::fs;
use std::path::Path;

pub fn read_bytes(path: impl AsRef<Path>) -> Vec<u8> {
    fs::read(path.as_ref())
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.as_ref().display()))
}

/// Compare two byte streams for exact equality. Mirrors `compare_binary_files`.
#[allow(dead_code)]
pub fn compare_binary(a: &[u8], b: &[u8]) -> bool {
    a == b
}

/// Compare two text files line-by-line, normalizing line endings. Mirrors
/// `compare_text_files` which uses `safe_get_line` (handles LF, CRLF, CR).
#[allow(dead_code)]
pub fn compare_text(a: &[u8], b: &[u8]) -> bool {
    let mut i = 0;
    let mut j = 0;
    loop {
        let (line_a, next_i) = safe_get_line(a, i);
        let (line_b, next_j) = safe_get_line(b, j);
        if line_a != line_b {
            return false;
        }
        let eof_a = next_i > a.len();
        let eof_b = next_j > b.len();
        if eof_a && eof_b {
            return true;
        }
        if eof_a != eof_b {
            // One still has content
            return false;
        }
        i = next_i;
        j = next_j;
    }
}

#[allow(dead_code)]
fn safe_get_line(input: &[u8], mut pos: usize) -> (Vec<u8>, usize) {
    let mut line = Vec::new();
    while pos < input.len() {
        let c = input[pos];
        pos += 1;
        match c {
            b'\n' => return (line, pos),
            b'\r' => {
                if pos < input.len() && input[pos] == b'\n' {
                    pos += 1;
                }
                return (line, pos);
            }
            _ => line.push(c),
        }
    }
    (line, pos + 1) // mark EOF past end
}

pub const SAMPLES_DIR: &str = env!("CARGO_MANIFEST_DIR");
pub fn sample_path(sub: &str) -> String {
    format!("{}/tests/samples/{}", SAMPLES_DIR, sub)
}

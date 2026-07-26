//! Port of g23-exception-tests.cpp.

mod common;

use common::{read_bytes, sample_path};
use rnp_sexp::{error, ExtKeyInputStream, ExtendedPrivateKey, Severity};

fn do_scan_ex(filename: &str, expected: &str) {
    let path = sample_path(&format!("compat/g23/{filename}"));
    let bytes = read_bytes(&path);
    let mut is = ExtKeyInputStream::new(&bytes);
    let mut key = ExtendedPrivateKey::default();
    match is.scan(&mut key) {
        Err(e) => assert_eq!(e.to_string(), expected, "for {filename}"),
        Ok(_) => panic!("expected error for {filename}"),
    }
}

#[test]
fn malformed_name_break() {
    do_scan_ex(
        "malformed_name_break.key",
        "EXTENDED KEY FORMAT ERROR: unexpected end of line at position 5",
    );
}

#[test]
fn malformed_name_eof() {
    do_scan_ex(
        "malformed_name_eof.key",
        "EXTENDED KEY FORMAT ERROR: unexpected end of file at position 2800",
    );
}

#[test]
fn malformed_invalid_name_char() {
    do_scan_ex(
        "malformed_invalid_name_char.key",
        "EXTENDED KEY FORMAT ERROR: unexpected character '@' (0x40) found in a name field at position 28",
    );
}

#[test]
fn malformed_invalid_name_first_char() {
    do_scan_ex(
        "malformed_invalid_name_first_char.key",
        "EXTENDED KEY FORMAT ERROR: unexpected character '1' (0x31) found starting a name field at position 21",
    );
}

#[test]
fn malformed_no_key() {
    do_scan_ex(
        "malformed_no_key.key",
        "EXTENDED KEY FORMAT ERROR: missing mandatory 'key' field at position 2819",
    );
}

#[test]
fn malformed_two_keys() {
    do_scan_ex(
        "malformed_two_keys.key",
        "EXTENDED KEY FORMAT ERROR: 'key' field must occur only once at position 2822",
    );
}

#[test]
fn g23_warning() {
    // Interactive mode prints to stdout in C++; we just verify no panic.
    error::set_interactive(true);
    let _ = error::ext_key_report("Test warning", Severity::Warning, 200);
    error::set_interactive(false);
}

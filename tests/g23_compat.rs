//! Port of g23-compat-tests.cpp.

mod common;

use common::{read_bytes, sample_path};
use rnp_sexp::{ExtKeyInputStream, ExtendedPrivateKey};

fn scan_and_check_correct(filename: &str) {
    let path = sample_path(&format!("compat/g23/{filename}"));
    let bytes = read_bytes(&path);
    let mut is = ExtKeyInputStream::new(&bytes);
    let mut key = ExtendedPrivateKey::default();
    is.scan(&mut key)
        .unwrap_or_else(|e| panic!("scan failed: {e}"));

    assert_eq!(key.fields.len(), 2, "for {filename}");
    assert_eq!(key.fields_count("Created"), 1);
    assert_eq!(key.fields_count("creaTed"), 1, "names are case-insensitive");
    assert_eq!(key.fields_count("something"), 0);

    let created = key.find("Created").expect("Created field present");
    assert_eq!(created, "20221130T160847", "for {filename}");
}

#[test]
fn g10_test() {
    let path = sample_path("compat/g10/canonical.key");
    let bytes = read_bytes(&path);
    let mut is = ExtKeyInputStream::new(&bytes);
    let mut key = ExtendedPrivateKey::default();
    is.scan(&mut key).unwrap();
    assert_eq!(key.fields.len(), 0);
}

#[test]
fn g23_correct() {
    scan_and_check_correct("correct.key");
}

#[test]
fn g23_correct_no_eol() {
    scan_and_check_correct("correct_no_eol.key");
}

#[test]
fn g23_correct_with_comment() {
    scan_and_check_correct("correct_with_comment.key");
}

#[test]
fn g23_correct_with_two_empty_lines() {
    scan_and_check_correct("correct_with_two_empty_lines.key");
}

#[test]
fn g23_correct_with_empty_line() {
    scan_and_check_correct("correct_with_empty_line.key");
}

#[test]
fn g23_correct_with_windows_eol() {
    scan_and_check_correct("correct_with_windows_eol.key");
}

#[test]
fn g23_correct_with_comment_at_eof() {
    scan_and_check_correct("correct_with_comment_at_eof.key");
}

#[test]
fn g23_correct_with_multiple_fields() {
    let path = sample_path("compat/g23/correct_mult_fields.key");
    let bytes = read_bytes(&path);
    let mut is = ExtKeyInputStream::new(&bytes);
    let mut key = ExtendedPrivateKey::default();
    is.scan(&mut key).unwrap();
    assert_eq!(key.fields.len(), 4);
    assert_eq!(key.fields_count("Created"), 1);
    assert_eq!(key.fields_count("Description"), 3);
    assert_eq!(key.fields_count("something"), 0);
    let desc = key.find("Description").expect("Description field present");
    assert_eq!(desc, "RSA/RSA");
}

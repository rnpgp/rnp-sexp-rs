//! Port of exception-tests.cpp.

use rnp_sexp::{error, ExtKeyInputStream, Severity, SexpError, SexpInputStream, SexpOutputStream};

fn do_scan_with_exception(input: &str, expected: &str) {
    let mut is = SexpInputStream::new(input);
    is.set_byte_size(8);
    is.get_char().unwrap();
    let result = is.scan_object();
    match result {
        Err(e) => assert_eq!(e.to_string(), expected, "input: {input:?}, got: {e}"),
        Ok(_) => panic!("expected error for input: {input:?}"),
    }
}

#[test]
fn unexpected_eof() {
    do_scan_with_exception(
        "(4:This2:is1:a4:test",
        "SEXP ERROR: unexpected end of file at position 20",
    );
}

#[test]
fn unexpected_character_4bit() {
    do_scan_with_exception(
        "(4:This2:is1:a4:test #)",
        "SEXP ERROR: character ')' found in 4-bit coding region at position 22",
    );
}

#[test]
fn illegal_character() {
    do_scan_with_exception(
        "(This is a test ?)",
        "SEXP ERROR: illegal character '?' (0x3f) at position 16",
    );
}

#[test]
fn unexpected_eof_after_quote() {
    do_scan_with_exception("(\")\n", "SEXP ERROR: unexpected end of file at position 4");
}

#[test]
fn illegal_character_base64() {
    do_scan_with_exception(
        "(Test {KDQ6VGhpczI6aXMxOmE0OnRlc3Qq})",
        "SEXP ERROR: illegal character '}' (0x7d) at position 35",
    );
}

#[test]
fn invalid_hex() {
    do_scan_with_exception(
        "(\"\\x1U\")",
        "SEXP ERROR: Hex character \\x1... too short at position 5",
    );
}

#[test]
fn invalid_octal() {
    do_scan_with_exception(
        "(\"\\12U\")",
        "SEXP ERROR: Octal character \\12... too short at position 5",
    );
}

#[test]
fn too_big_octal() {
    do_scan_with_exception(
        "(\"\\666U\")",
        "SEXP ERROR: Octal character \\666... too big at position 5",
    );
}

#[test]
fn invalid_escape() {
    do_scan_with_exception(
        "(\"\\?\")",
        "SEXP ERROR: Unknown escape sequence \\? at position 3",
    );
}

#[test]
fn string_too_short_quoted() {
    do_scan_with_exception(
        "(4\"ABC\")",
        "SEXP ERROR: Declared length was 4, but quoted string ended too early at position 6",
    );
}

#[test]
fn string_too_short_base64() {
    error::set_verbosity(Severity::Warning);
    do_scan_with_exception(
        "(8|NDpBQkNE|)",
        "SEXP WARNING: Base64 string has length 6 different than declared length 8 at position 12",
    );
    error::set_verbosity(Severity::Error);
}

#[test]
fn string_too_short_hex() {
    error::set_verbosity(Severity::Warning);
    do_scan_with_exception(
        "(8#AAABFCAD#)",
        "SEXP WARNING: Hex string has length 4 different than declared length 8 at position 12",
    );
    error::set_verbosity(Severity::Error);
}

#[test]
fn string_bad_length() {
    do_scan_with_exception(
        "(1A:AAABFCAD)",
        "SEXP ERROR: illegal character 'A' (0x41) at position 2",
    );
}

#[test]
fn string_too_long_truncated() {
    do_scan_with_exception(
        "(982582599:",
        "SEXP ERROR: Verbatim string is too long: 982582599 at position 11",
    );
}

#[test]
fn string_truncated() {
    do_scan_with_exception(
        "(1024:",
        "SEXP ERROR: EOF while reading verbatim string at position 6",
    );
}

#[test]
fn decimal_too_long() {
    do_scan_with_exception(
        "(1234567890:AAABFCAD)",
        "SEXP ERROR: Decimal number is too long at position 11",
    );
}

#[test]
fn base64_curly_bracket() {
    do_scan_with_exception(
        "({ey})",
        "SEXP ERROR: illegal character '{' (0x7b) at position 3",
    );
}

#[test]
fn unused_bits() {
    error::set_verbosity(Severity::Warning);
    do_scan_with_exception(
        "(Test |AABBCCDD11|)",
        "SEXP WARNING: 6-bit region ended with 4 unused bits left-over at position 17",
    );
    error::set_verbosity(Severity::Error);
}

#[test]
fn not_a_list_when_expected() {
    let mut is =
        SexpInputStream::new("|d738/4ghP9rFZ0gAIYZ5q9y6iskDJwASi5rEQpEQq8ZyMZeIZzIAR2I5iGE=|");
    is.set_byte_size(8);
    is.get_char().unwrap();
    let result = is.scan_list();
    match result {
        Err(e) => assert_eq!(
            e.to_string(),
            "SEXP ERROR: character '|' found where '(' was expected at position 0"
        ),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn invalid_byte_size_and_mode() {
    let mut is = SexpInputStream::new("(3:a\tc)");
    is.set_byte_size(8);
    is.get_char().unwrap();
    let obj = is.scan_object().unwrap();
    let mut os = SexpOutputStream::new();
    os.change_output_byte_size(4, rnp_sexp::PrintMode::Advanced)
        .unwrap();
    let result = os.print_advanced(&obj);
    match result {
        Err(e) => assert_eq!(
            e.to_string(),
            "SEXP ERROR: Can't print in advanced mode with restricted output character set"
        ),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn sexp_warning() {
    // C++ captures stdout when interactive mode is on. We follow the same model.
    error::set_interactive(true);
    // No assertion on stdout — just verify the call doesn't panic.
    let _: Result<(), SexpError> = error::sexp_report("Test warning", Severity::Warning, 200);
    error::set_interactive(false);
}

// --- MaxDepth tests ---

fn parse_list_from_string(input: &str) -> Result<(), SexpError> {
    let mut is = SexpInputStream::new(input);
    is.set_byte_size(8);
    is.get_char()?;
    is.scan_list()?;
    Ok(())
}

fn parse_list_from_string_with_limit(input: &str, max_depth: usize) -> Result<(), SexpError> {
    let mut is = SexpInputStream::new_with_max_depth(input.as_bytes(), max_depth);
    is.set_byte_size(8);
    is.get_char()?;
    is.scan_list()?;
    Ok(())
}

#[test]
fn max_depth_parse() {
    let depth_1 = "(sexp_list_1)";
    let depth_4 = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4))))";
    let depth_4e = "(sexp_list_1 (sexp_list_2 (sexp_list_3 ())))";
    let depth_5 = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4 (sexp_list_5)))))";
    let depth_5e = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4 ()))))";

    parse_list_from_string(depth_1).unwrap();
    parse_list_from_string(depth_4).unwrap();
    parse_list_from_string(depth_4e).unwrap();
    parse_list_from_string(depth_5).unwrap();
    parse_list_from_string(depth_5e).unwrap();

    parse_list_from_string_with_limit(depth_1, 4).unwrap();
    parse_list_from_string_with_limit(depth_4, 4).unwrap();
    parse_list_from_string_with_limit(depth_4e, 4).unwrap();

    let e = parse_list_from_string_with_limit(depth_5, 4).unwrap_err();
    assert_eq!(
        e.to_string(),
        "SEXP ERROR: Maximum allowed SEXP list depth (4) is exceeded at position 53"
    );

    let e = parse_list_from_string_with_limit(depth_5e, 4).unwrap_err();
    assert_eq!(
        e.to_string(),
        "SEXP ERROR: Maximum allowed SEXP list depth (4) is exceeded at position 53"
    );
}

fn print_list_from_string(input: &str, advanced: bool, max_depth: usize) -> Result<(), SexpError> {
    let mut is = SexpInputStream::new(input);
    is.set_byte_size(8);
    is.get_char()?;
    let lst = is.scan_list()?;
    let mut os = SexpOutputStream::new_with_max_depth(max_depth);
    if advanced {
        os.print_advanced(&rnp_sexp::Sexp::List(lst))?;
    } else {
        os.print_canonical(&rnp_sexp::Sexp::List(lst))?;
    }
    Ok(())
}

#[test]
fn max_depth_print_advanced() {
    let depth_1 = "(sexp_list_1)";
    let depth_4 = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4))))";
    let depth_4e = "(sexp_list_1 (sexp_list_2 (sexp_list_3 ())))";
    let depth_5 = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4 (sexp_list_5)))))";
    let depth_5e = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4 ()))))";

    print_list_from_string(depth_1, true, 0).unwrap();
    print_list_from_string(depth_4, true, 0).unwrap();
    print_list_from_string(depth_4e, true, 0).unwrap();
    print_list_from_string(depth_5, true, 0).unwrap();
    print_list_from_string(depth_5e, true, 0).unwrap();

    print_list_from_string(depth_1, true, 4).unwrap();
    print_list_from_string(depth_4, true, 4).unwrap();
    print_list_from_string(depth_4e, true, 4).unwrap();

    let e = print_list_from_string(depth_5, true, 4).unwrap_err();
    assert_eq!(
        e.to_string(),
        "SEXP ERROR: Maximum allowed SEXP list depth (4) is exceeded"
    );

    let e = print_list_from_string(depth_5e, true, 4).unwrap_err();
    assert_eq!(
        e.to_string(),
        "SEXP ERROR: Maximum allowed SEXP list depth (4) is exceeded"
    );
}

#[test]
fn max_depth_print_canonical() {
    let depth_1 = "(sexp_list_1)";
    let depth_4 = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4))))";
    let depth_4e = "(sexp_list_1 (sexp_list_2 (sexp_list_3 ())))";
    let depth_5 = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4 (sexp_list_5)))))";
    let depth_5e = "(sexp_list_1 (sexp_list_2 (sexp_list_3 (sexp_list_4 ()))))";

    print_list_from_string(depth_1, false, 0).unwrap();
    print_list_from_string(depth_4, false, 0).unwrap();
    print_list_from_string(depth_4e, false, 0).unwrap();
    print_list_from_string(depth_5, false, 0).unwrap();
    print_list_from_string(depth_5e, false, 0).unwrap();

    print_list_from_string(depth_1, false, 4).unwrap();
    print_list_from_string(depth_4, false, 4).unwrap();
    print_list_from_string(depth_4e, false, 4).unwrap();

    let e = print_list_from_string(depth_5, false, 4).unwrap_err();
    assert_eq!(
        e.to_string(),
        "SEXP ERROR: Maximum allowed SEXP list depth (4) is exceeded"
    );

    let e = print_list_from_string(depth_5e, false, 4).unwrap_err();
    assert_eq!(
        e.to_string(),
        "SEXP ERROR: Maximum allowed SEXP list depth (4) is exceeded"
    );
}

// Helper to allow tests that pass `&mut ExtKeyInputStream` (unused here, but
// kept to mirror the upstream test class structure).
#[allow(dead_code)]
fn _ext_key_scan(_is: &mut ExtKeyInputStream) {}

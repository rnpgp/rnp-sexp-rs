//! Port of primitives-tests.cpp.

use rnp_sexp::{PrintMode, Sexp, SexpInputStream, SexpOutputStream};

fn parse(input: &str) -> Sexp {
    let mut is = SexpInputStream::new(input);
    is.set_byte_size(8);
    is.get_char().unwrap();
    is.scan_object().expect("scan_object should succeed")
}

fn do_test_advanced(input: &str, expected: &str) {
    let obj = parse(input);
    let mut os = SexpOutputStream::new();
    os.print_advanced(&obj).unwrap();
    let out = os.into_string_lossy();
    assert_eq!(out, expected, "advanced mismatch for input: {input:?}");
}

fn do_test_canonical(input: &str, expected: &str) {
    let obj = parse(input);
    let mut os = SexpOutputStream::new();
    os.print_canonical(&obj).unwrap();
    let out = String::from_utf8_lossy(os.as_bytes()).into_owned();
    assert_eq!(out, expected, "canonical mismatch for input: {input:?}");
}

#[test]
fn empty_list() {
    do_test_canonical("( )", "()");
    do_test_advanced("( )", "()");
}

#[test]
fn empty_string() {
    rnp_sexp::error::set_verbosity(rnp_sexp::Severity::Error);
    do_test_canonical("(\"\")", "(0:)");
    do_test_advanced("(\"\")", "(\"\")");
}

#[test]
fn string() {
    rnp_sexp::error::set_verbosity(rnp_sexp::Severity::Error);
    do_test_canonical("(ab)", "(2:ab)");
    do_test_advanced("(ab)", "(ab)");
}

#[test]
fn quoted_string_with_octal() {
    rnp_sexp::error::set_verbosity(rnp_sexp::Severity::Error);
    do_test_canonical("\"ab\\015\"", "3:ab\r");
    do_test_advanced("\"ab\\015\"", "#61620D#");
}

#[test]
fn quoted_string_with_escape() {
    rnp_sexp::error::set_verbosity(rnp_sexp::Severity::Error);
    do_test_canonical("\"ab\\tc\"", "4:ab\tc");
    do_test_advanced("4:ab\tc", "#61620963#");
}

#[test]
fn hex_string() {
    rnp_sexp::error::set_verbosity(rnp_sexp::Severity::Error);
    do_test_canonical("#616263#", "3:abc");
    do_test_advanced("#616263#", "abc");
}

#[test]
fn list_list() {
    do_test_canonical(
        "(string-level-1 (string-level-2) )",
        "(14:string-level-1(14:string-level-2))",
    );
    do_test_advanced(
        "(string-level-1 (string-level-2) )",
        "(string-level-1 (string-level-2))",
    );
}

#[test]
fn base64_of_octet() {
    do_test_canonical("|YWJj|", "3:abc");
    do_test_advanced("|YWJj|", "abc");
}

#[test]
fn base64_of_verbatim() {
    do_test_canonical("{MzphYmM=}", "3:abc");
    do_test_advanced("{MzphYmM=}", "abc");
}

#[test]
fn multiline_linux() {
    do_test_canonical("\"abcd\\\nef\"", "6:abcdef");
    do_test_advanced("\"abcd\\\nef\"", "abcdef");
}

#[test]
fn multiline_mac() {
    do_test_canonical("\"abcd\\\ref\"", "6:abcdef");
    do_test_advanced("\"abcd\\\ref\"", "abcdef");
}

#[test]
fn multiline_win() {
    do_test_canonical("\"abcd\\\r\nef\"", "6:abcdef");
    do_test_advanced("\"abcd\\\r\nef\"", "abcdef");
}

#[test]
fn multiline_bsd() {
    do_test_canonical("\"abcd\\\n\ref\"", "6:abcdef");
    do_test_advanced("\"abcd\\\n\ref\"", "abcdef");
}

#[test]
fn wrap() {
    let really_long = "(a (b (c ddddddddddddddd\
ddddddddddddddddddddddddddddddddddddddddddddddddd\
ddddddddddddddddddddddddddddddddddddddddddddddddd\
ddddddddddddddddddddddddddddddddddddddddddddddddd\
ddddddd)))";
    let still_long = "(1:a(1:b(1:c169:ddddddddd\
ddddddddddddddddddddddddddddddddddddddddddddddddd\
ddddddddddddddddddddddddddddddddddddddddddddddddd\
ddddddddddddddddddddddddddddddddddddddddddddddddd\
ddddddddddddd)))";
    let broken = "(a\n (b\n  (c\n   \"\
ddddddddddddddddddddddddddddddddddddddddddddddddddddd\
dddddddddddddddd\\\n\
ddddddddddddddddddddddddddddddddddddddddddddddddddddddd\
dddddddddddddddddd\\\n\
ddddddddddddddddddddddddddd\")))";
    do_test_canonical(really_long, still_long);
    do_test_advanced(really_long, broken);
}

#[test]
fn escapes() {
    do_test_canonical(
        "(\"\\b\\t\\v\\n\\f\\r\\\"\\'\\\\\")",
        "(9:\x08\t\x0b\n\x0c\r\"'\\)",
    );
    do_test_advanced("(\"\\b\\t\\v\\n\\f\\r\\\"\\'\\\\\")", "(|CAkLCgwNIidc|)");

    do_test_canonical("(\"\\040\\041\\042\\043\\044\")", "(5: !\"#$)");
    do_test_advanced("(\"\\065\\061\\062\\063\\064\")", "(\"51234\")");

    do_test_canonical("(\"\\x40\\x41\\x42\\x43\\x44\")", "(5:@ABCD)");
    do_test_advanced("(\"\\x65\\x61\\x62\\x63\\x64\")", "(eabcd)");
}

#[test]
fn at4rnp() {
    let obj = parse("(rnp_block (rnp_list1 rnp_list2))");
    let lst = obj.as_list().unwrap();
    assert!(lst.sexp_list_at(0).is_none());
    assert!(lst.sexp_list_at(1).is_some());
    assert!(lst.sexp_string_at(0).is_some());
    assert!(lst.sexp_string_at(1).is_none());

    let inner = lst.sexp_list_at(1).unwrap();
    assert!(inner.sexp_list_at(0).is_none());
    assert!(inner.sexp_list_at(1).is_none());

    let s = lst.sexp_string_at(0).unwrap();
    assert_eq!(s.as_bytes(), b"rnp_block");
}

#[test]
fn eq4rnp() {
    let obj = parse("(rnp_block (rnp_list1 rnp_list2))");
    let lst = obj.as_list().unwrap();
    assert!(lst[0].equals_str("rnp_block"));
    assert!(!lst[0].equals_str("not_rnp_block"));
    assert!(!lst[1].equals_str("rnp_block"));
    assert!(!lst[1].equals_str("not_rnp_block"));

    let inner = lst.sexp_list_at(1).unwrap();
    assert!(inner[0].equals_str("rnp_list1"));
    assert!(inner.sexp_string_at(1).unwrap().equals_str("rnp_list2"));
}

#[test]
fn u4rnp() {
    let obj1 = parse("(unsigned_value \"12345\")");
    let obj2 = parse("(14:unsigned_value5:54321)");
    let lst1 = obj1.as_list().unwrap();
    let lst2 = obj2.as_list().unwrap();
    assert_eq!(lst1.sexp_string_at(1).unwrap().as_unsigned(), 12345);
    assert_eq!(lst2.sexp_string_at(1).unwrap().as_unsigned(), 54321);
}

#[test]
fn pro_inheritance() {
    let lst = rnp_sexp::SexpList::new();
    assert!(!Sexp::List(lst.clone()).is_string());
    assert!(Sexp::List(lst).is_list());
    assert_eq!(Sexp::List(rnp_sexp::SexpList::new()).as_string(), None);
    assert_eq!(
        Sexp::List(rnp_sexp::SexpList::new())
            .as_list()
            .map(|l| l.len()),
        Some(0)
    );
    assert_eq!(
        Sexp::List(rnp_sexp::SexpList::new()).as_unsigned(),
        u32::MAX
    );
    assert!(rnp_sexp::SexpList::new().sexp_list_at(0).is_none());
    assert!(rnp_sexp::SexpList::new().sexp_string_at(0).is_none());

    let s = rnp_sexp::SexpString::new();
    assert!(!Sexp::String(s.clone()).is_list());
    assert!(Sexp::String(s).is_string());
}

#[test]
fn display_hint() {
    do_test_canonical(
        "(URL [URI]www.ribose.com)",
        "(3:URL[3:URI]14:www.ribose.com)",
    );
    do_test_advanced(
        "(3:URL[3:URI]14:www.ribose.com)",
        "(URL [URI]www.ribose.com)",
    );
}

#[test]
fn scan_to_eof() {
    let mut is = SexpInputStream::new("ABCD");
    let obj = is.scan_to_eof().unwrap();
    assert_eq!(obj.as_bytes(), b"ABCD");

    is.set_byte_size(4);
    assert_eq!(is.get_byte_size(), 4);
    is.get_char().unwrap();
    assert_eq!(is.get_byte_size(), 8);
}

#[test]
fn change_output_byte_size_test() {
    let mut os = SexpOutputStream::new();
    os.change_output_byte_size(8, PrintMode::Advanced).unwrap();

    let err = os
        .change_output_byte_size(7, PrintMode::Advanced)
        .unwrap_err();
    assert_eq!(err.to_string(), "SEXP ERROR: Illegal output base 7");

    os.change_output_byte_size(4, PrintMode::Advanced).unwrap();
    let err = os
        .change_output_byte_size(6, PrintMode::Advanced)
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "SEXP ERROR: Illegal change of output byte size from 4 to 6"
    );
}

#[test]
fn flush_test() {
    let mut os = SexpOutputStream::new();
    os.change_output_byte_size(6, PrintMode::Advanced)
        .unwrap()
        .print_decimal(1)
        .unwrap()
        .flush()
        .unwrap();
    assert_eq!(os.as_bytes(), b"MQ==");

    let mut os = SexpOutputStream::new();
    os.change_output_byte_size(6, PrintMode::Advanced)
        .unwrap()
        .set_max_column(2)
        .print_decimal(2)
        .unwrap()
        .flush()
        .unwrap();
    assert_eq!(os.as_bytes(), b"Mg\n==");
}

#[test]
fn list_wrap_test() {
    let obj = parse("(abc)");
    let mut os = SexpOutputStream::new();
    os.set_max_column(5);
    os.print_advanced(&obj).unwrap();
    assert_eq!(os.as_bytes(), b"(abc\n )");
}

#[test]
fn ensure_hex_test() {
    let obj = parse("(3:a\tc)");
    let mut os = SexpOutputStream::new();
    os.print_advanced(&obj).unwrap();
    assert_eq!(os.as_bytes(), b"(#610963#)");
}

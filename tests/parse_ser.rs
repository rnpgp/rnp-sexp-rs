//! Tests for the rnp-sexp crate.

use rnp_sexp::{parse_advanced, parse_canonical, PrintMode, Sexp};

#[test]
fn parse_simple_token() {
    let sexp = parse_advanced("(test)").unwrap();
    let list = sexp.as_list().unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].equals_str("test"));
}

#[test]
fn parse_length_prefixed() {
    let sexp = parse_advanced("(4:test)").unwrap();
    let list = sexp.as_list().unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].equals_str("test"));
}

#[test]
fn parse_nested_lists() {
    let sexp = parse_advanced("((3:abc)(3:def))").unwrap();
    let list = sexp.as_list().unwrap();
    assert_eq!(list.len(), 2);
    let inner0 = list[0].as_list().unwrap();
    assert!(inner0[0].equals_str("abc"));
    let inner1 = list[1].as_list().unwrap();
    assert!(inner1[0].equals_str("def"));
}

#[test]
fn parse_quoted_string() {
    let sexp = parse_advanced("(\"hello world\")").unwrap();
    let list = sexp.as_list().unwrap();
    assert!(list[0].equals_str("hello world"));
}

#[test]
fn parse_quoted_with_escape() {
    let sexp = parse_advanced(r#"("hello\nworld")"#).unwrap();
    let list = sexp.as_list().unwrap();
    assert_eq!(list[0].as_string().unwrap().as_bytes(), b"hello\nworld");
}

#[test]
fn parse_hex_string() {
    // #68656c6c6f# = "hello" in hex
    let sexp = parse_advanced("(#68656c6c6f#)").unwrap();
    let list = sexp.as_list().unwrap();
    assert!(list[0].equals_str("hello"));
}

#[test]
fn parse_base64_string() {
    // |aGVsbG8=| = "hello" in base64
    let sexp = parse_advanced("(|aGVsbG8=|)").unwrap();
    let list = sexp.as_list().unwrap();
    assert!(list[0].equals_str("hello"));
}

#[test]
fn parse_multiple_elements() {
    let sexp = parse_advanced("(3:abc 3:def 3:ghi)").unwrap();
    let list = sexp.as_list().unwrap();
    assert_eq!(list.len(), 3);
    assert!(list[0].equals_str("abc"));
    assert!(list[1].equals_str("def"));
    assert!(list[2].equals_str("ghi"));
}

#[test]
fn parse_openpgp_like_key() {
    // Simplified OpenPGP-style key structure
    let input = "(11:private-key (5:ecdsa (5:curve 7:NIST-P-) (1:q 4:abcd)))";
    let result = parse_advanced(input);
    assert!(result.is_ok(), "should parse: {result:?}");
    let sexp = result.unwrap();
    let list = sexp.as_list().unwrap();
    assert!(list[0].equals_str("private-key"));
}

#[test]
fn parse_canonical_round_trip() {
    let canonical = b"(4:test(3:abc))";
    let sexp = parse_canonical(canonical).unwrap();
    let round = sexp.to_canonical();
    assert_eq!(round, canonical);
}

#[test]
fn advanced_round_trip() {
    let input = "(3:abc(3:def))";
    let sexp = parse_advanced(input).unwrap();
    let output = sexp.to_advanced();
    let reparsed = parse_advanced(&output).unwrap();
    assert_eq!(sexp, reparsed);
}

#[test]
fn serialize_canonical_basic() {
    let sexp = Sexp::list(vec![
        Sexp::string(b"test".to_vec()),
        Sexp::list(vec![Sexp::string(b"abc".to_vec())]),
    ]);
    let out = sexp.to_string_with_mode(PrintMode::Canonical);
    assert_eq!(out, "(4:test(3:abc))");
}

#[test]
fn serialize_advanced_token() {
    let sexp = Sexp::list(vec![Sexp::string(b"token".to_vec())]);
    let out = sexp.to_advanced();
    assert!(out.contains("token") || out.contains("5:token"));
}

#[test]
fn serialize_advanced_quoted() {
    let sexp = Sexp::list(vec![Sexp::string(b"hello world".to_vec())]);
    let out = sexp.to_advanced();
    assert!(
        out.contains("\"hello world\"") || out.contains("11:hello world"),
        "got: {out}"
    );
}

#[test]
fn empty_list() {
    let sexp = parse_advanced("()").unwrap();
    let list = sexp.as_list().unwrap();
    assert!(list.is_empty());
}

#[test]
fn empty_string() {
    let sexp = parse_advanced("(0:)").unwrap();
    let list = sexp.as_list().unwrap();
    assert_eq!(list[0].as_string().unwrap().as_bytes(), b"");
}

#[test]
fn deep_nesting() {
    let input: String = "(".repeat(100)
        + "1:x"
        + &")".repeat(100);
    let result = parse_advanced(&input);
    assert!(result.is_ok(), "100-deep nesting should parse: {result:?}");
}

#[test]
fn max_depth_protection() {
    let input: String = "(".repeat(2000)
        + "1:x"
        + &")".repeat(2000);
    let result = parse_advanced(&input);
    assert!(result.is_err(), "2000-deep nesting should fail");
}

#[test]
fn error_on_trailing_garbage() {
    let result = parse_advanced("(3:abc) garbage");
    assert!(result.is_err());
}

#[test]
fn error_on_unbalanced_parens() {
    let result = parse_advanced("(3:abc");
    assert!(result.is_err());
}

#[test]
fn presentation_hint() {
    let input = "([4:hint]4:data)";
    let result = parse_advanced(input);
    assert!(result.is_ok(), "presentation hint should parse: {result:?}");
    if let Ok(sexp) = result {
        let list = sexp.as_list().unwrap();
        let s = list[0].as_string().unwrap();
        assert!(s.has_presentation_hint());
        assert_eq!(s.presentation_hint.as_ref().unwrap(), b"hint");
        assert_eq!(s.as_bytes(), b"data");
    }
}

#[test]
fn binary_data_canonical() {
    // Canonical form handles binary: 5:\x00\x01\x02\x03\x04
    let input: Vec<u8> = vec![b'(', b'5', b':', 0x00, 0x01, 0x02, 0x03, 0x04, b')'];
    let sexp = parse_canonical(&input).unwrap();
    let list = sexp.as_list().unwrap();
    assert_eq!(list[0].as_string().unwrap().as_bytes(), &[0x00, 0x01, 0x02, 0x03, 0x04]);
}

//! Port of baseline-tests.cpp.
//!
//! Reads each of the three baseline sample files (advanced, base64, canonical),
//! parses each one, re-serializes to all three modes, and verifies byte-for-byte
//! equivalence against the canonical sample.

mod common;

use common::{compare_binary, compare_text, read_bytes, sample_path};
use rnp_sexp::{SexpInputStream, SexpOutputStream};

fn parse_sample(name: &str) -> rnp_sexp::Sexp {
    let bytes = read_bytes(name);
    let mut is = SexpInputStream::new(&bytes);
    is.set_byte_size(8);
    is.get_char().expect("get_char");
    is.scan_object().expect("scan_object")
}

#[test]
fn scan_to_canonical() {
    let samples = [
        sample_path("baseline/sexp-sample-a"),
        sample_path("baseline/sexp-sample-b"),
        sample_path("baseline/sexp-sample-c"),
    ];
    let canonical_sample = sample_path("baseline/sexp-sample-c");
    let canonical_bytes = read_bytes(&canonical_sample);
    for s in samples {
        let obj = parse_sample(&s);
        let mut os = SexpOutputStream::new();
        os.print_canonical(&obj).unwrap();
        assert!(
            compare_binary(os.as_bytes(), &canonical_bytes),
            "canonical mismatch when re-serializing {s}"
        );
    }
}

#[test]
fn scan_to_base64() {
    let samples = [
        sample_path("baseline/sexp-sample-a"),
        sample_path("baseline/sexp-sample-b"),
        sample_path("baseline/sexp-sample-c"),
    ];
    let base64_sample = sample_path("baseline/sexp-sample-b");
    let base64_bytes = read_bytes(&base64_sample);
    for s in samples {
        let obj = parse_sample(&s);
        let mut os = SexpOutputStream::new();
        os.set_max_column(0);
        os.print_base64(&obj).unwrap();
        // C++ appends std::endl after the base64 output.
        let mut out = os.into_bytes();
        out.push(b'\n');
        assert!(
            compare_text(&out, &base64_bytes),
            "base64 mismatch when re-serializing {s}"
        );
    }
}

#[test]
fn scan_to_advanced() {
    let samples = [
        sample_path("baseline/sexp-sample-a"),
        sample_path("baseline/sexp-sample-b"),
        sample_path("baseline/sexp-sample-c"),
    ];
    let advanced_sample = sample_path("baseline/sexp-sample-a");
    let advanced_bytes = read_bytes(&advanced_sample);
    for s in samples {
        let obj = parse_sample(&s);
        let mut os = SexpOutputStream::new();
        os.print_advanced(&obj).unwrap();
        assert!(
            compare_text(os.as_bytes(), &advanced_bytes),
            "advanced mismatch when re-serializing {s}\ngot: {:?}\nexpected: {:?}",
            String::from_utf8_lossy(os.as_bytes()),
            String::from_utf8_lossy(&advanced_bytes)
        );
    }
}

//! Port of g10-compat-tests.cpp.

mod common;

use common::{compare_binary, read_bytes, sample_path};
use rnp_sexp::{SexpInputStream, SexpOutputStream};

#[test]
fn canonical() {
    let keyfile = sample_path("compat/g10/canonical.key");
    let bytes = read_bytes(&keyfile);
    let mut is = SexpInputStream::new(&bytes);
    is.set_byte_size(8);
    is.get_char().unwrap();
    let obj = is.scan_object().unwrap();
    let mut os = SexpOutputStream::new();
    os.print_canonical(&obj).unwrap();
    assert!(
        compare_binary(os.as_bytes(), &bytes),
        "canonical round-trip mismatch for {keyfile}"
    );
}

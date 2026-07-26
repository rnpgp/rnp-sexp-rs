# rnp-sexp

Pure-Rust S-expression parser and serializer for OpenPGP key formats.

A faithful port of [librnp's `sexpp`](https://github.com/rnpgp/sexpp) C++ library,
which implements the S-expression syntax from
[RFC 2693 (SPKI/SDSI)](https://tools.ietf.org/html/rfc2693) and the
[GnuPG extended key format](https://github.com/gpg/gnupg/blob/master/agent/keyformat.txt).

## Features

- Three printing modes:
  - **Canonical** — binary-safe `length:bytes` form for hashing/transmission
  - **Advanced** — human-readable, pretty-printed, line-wrapped at 75 columns
  - **Base64** — canonical form base64-encoded and wrapped in `{...}`
- Five simple-string encodings on input: verbatim (`5:bytes`), token (`abc`),
  quoted (`"hi"`), hex (`#6162#`), and base64 (`|YWJj|`)
- Full GnuPG 2.3+ extended private key format: `Name: value` header pairs with
  case-insensitive names, multi-valued fields, single-line continuation via a
  leading space, blank-continuation encodes a newline, `#`-prefixed comment lines
- Configurable nesting-depth limit (default 1024) enforced on both parser and
  serializer
- Position-precise error messages matching the upstream library, so test fixtures
  and diagnostics from the C++ version port directly

## Quick start

```rust
use rnp_sexp::Sexp;

// Parse advanced (human-readable) form
let input = "(11:test-key (1:a 3:bcd))";
let sexp = Sexp::parse_advanced(input).unwrap();

// Round-trip back to advanced form
assert_eq!(sexp.to_advanced(), input);

// Canonical bytes for hashing/transmission
let canonical: Vec<u8> = sexp.to_canonical();
```

For the GnuPG extended private key format:

```rust
use rnp_sexp::parse_extended;

let key = parse_extended(b"Created: 20221130T160847\n\
                          Key: (private-key (rsa (n #00AB...#) (e #010001#)))\n")?;

assert_eq!(key.find("Created").unwrap(), "20221130T160847");
let key_sexp = &key.key;  // SexpList
```

For finer-grained control over parsing or serialization (custom max-depth,
incremental reads, base64 print mode, custom column width), use
[`SexpInputStream`](https://docs.rs/rnp-sexp/latest/rnp_sexp/struct.SexpInputStream.html)
and
[`SexpOutputStream`](https://docs.rs/rnp-sexp/latest/rnp_sexp/struct.SexpOutputStream.html)
directly.

## Compatibility with upstream

The module layout mirrors `../sexp/src/` file-by-file (`chars.rs`,
`error.rs`, `depth.rs`, `input.rs`, `output.rs`, `ext_key.rs`), and behavior is
byte-for-byte identical. Error message text matches the C++ version exactly
because the ported test fixtures assert on the formatted `what()` strings.

The `tests/samples/` directory is a verbatim copy of `sexp/samples/` from the
upstream repository.

## License

MIT, matching the upstream sexpp project.

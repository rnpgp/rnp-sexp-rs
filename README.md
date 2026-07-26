# rnp-sexp

Pure-Rust S-expression parser and serializer for OpenPGP key formats.

A faithful port of [librnp's `sexpp`](https://github.com/rnpgp/sexpp) C++ library,
which implements the S-expression syntax from
[RFC 2693 (SPKI/SDSI)](https://tools.ietf.org/html/rfc2693) and the
[GnuPG extended key format](https://github.com/gpg/gnupg/blob/master/agent/keyformat.txt).

Supports all three printing modes:
- **Canonical** — binary-safe, used for hashing/transmission
- **Advanced** — human-readable, pretty-printed
- **Base64** — base64-encoded canonical form

## Quick start

```rust
use rnp_sexp::Sexp;

let input = "(11:test-key (1:a 3:bcd))";
let sexp = Sexp::parse_advanced(input).unwrap();
println!("{}", sexp.to_advanced());
```

## License

MIT, matching the upstream sexpp project.

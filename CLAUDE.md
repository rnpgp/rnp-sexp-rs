# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`rnp-sexp` is a pure-Rust port of [librnp's `sexpp` C++ library](https://github.com/rnpgp/sexp),
located as a sibling directory at `../sexp/`. It implements S-expression parsing and serialization
for OpenPGP key formats (RFC 2693 SPKI/SDSI syntax plus the GnuPG extended private key format).

The port mirrors the upstream file layout one-to-one. When changing behavior, consult the
corresponding `../sexp/src/*.cpp` file and keep semantics byte-for-byte identical — the ported
test suite checks exact error message text and exact serialization output (including line-wrap
positions).

## Build & test

```bash
cargo build
cargo test                       # all unit + integration tests
cargo test --test primitives      # one integration-test file
cargo test test_name              # one test by name
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo clippy -D warnings`,
`cargo test` with `RUSTFLAGS=-D warnings`.

## Architecture

Module layout mirrors `../sexp/src/` so reviewers can diff side-by-side:

- `src/chars.rs` ← `sexp-char-defs.cpp` — character classification (`is_token_char`, `is_dec_digit`, `is_hex_digit`, `is_base64_digit`, `dec_value`/`hex_value`/`base64_value`).
- `src/error.rs` ← `sexp-error.cpp` — `SexpError` enum. Error message strings must match upstream format `SEXP ERROR: <msg> at position <N>` because the ported exception tests assert on `e.what()` text.
- `src/depth.rs` ← `sexp-depth-manager.cpp` — depth tracking shared by parser and serializer (default max 1024). Both parser AND serializer must enforce the limit.
- `src/input.rs` ← `sexp-input.cpp` — `SexpInputStream` struct with `byte_size` (4/6/8), `bits`/`n_bits` accumulators for hex/base64 regions, `next_char`, `count` (chars read, starts at -1). Public entry points: `scan_object`, `scan_string`, `scan_list`, `scan_simple_string`, `scan_to_eof`. Hex (`#...#`) and base64 (`|...|`) regions switch `byte_size` mid-scan.
- `src/output.rs` ← `sexp-output.cpp` — `SexpOutputStream` struct with `byte_size`, `bits`/`n_bits`, `column`, `max_column` (default 75), `indent`, `base64_count`, `mode`. `var_put_char` is the core: emits through the current byte-size encoder. `flush` handles base64 padding. Three print modes: `canonical`, `advanced`, `base64` (whole object wrapped in `{...}` base64-encoded canonical).
- `src/simple_string.rs` ← `sexp-simple-string.cpp` — `SexpSimpleString` (raw bytes, no presentation hint). Advanced-mode serialization picks among token/quoted/hex/base64 forms based on contents AND remaining column width: `length <= 4 && byte_size == 8` → hex; otherwise → base64. `can_print_as_token` rejects if the token would not fit in the remaining line.
- `src/object.rs` ← `sexp-object.cpp` — `Sexp` enum (`List` | `String`) and `SexpList`/`SexpString` structs. Advanced-mode list printing computes `advanced_length` first to choose horizontal vs. vertical layout.
- `src/ext_key.rs` ← `ext-key-format.cpp` — GnuPG 2.3+ extended private key format. Header-like `Name: value` pairs (case-insensitive names, multi-valued via `multimap` semantics) followed by a mandatory single `Key:` field holding the actual S-expression. Continuation lines start with a single space; ` CR` ` LF` sequences are comment lines.
- `src/lib.rs` — re-exports the public API.

`tests/samples/` is a verbatim copy of `../sexp/samples/` (baseline + g10 + g23 fixtures). Tests in `tests/*.rs` read these files; if a sample is added upstream, copy it here.

## Porting conventions

- **Behavior parity over Rust idiom.** When upstream and idiomatic Rust disagree, upstream wins (e.g., `as_unsigned()` returns `u32::MAX` on empty input, not `Option`, to match C++'s `numeric_limits<uint32_t>::max()`). The ported test suite is the source of truth.
- **Error message text is load-bearing.** The exception-tests suite asserts on exact `what()` strings. If you change a message, the ported tests will fail — update both together.
- **Positions in errors are 0-indexed byte offsets** matching upstream's `count` (which starts at -1 and is incremented by `read_char`; after the first `get_char`, `count` is 0).
- **Skip C++-STL-only constructs.** The upstream `octet_traits` tests (`traits-tests.cpp`) test `std::char_traits<uint8_t>` specialization — not applicable in Rust, so they are not ported.
- **The C++ `shared_ptr<sexp_object_t>` polymorphism** is modeled as a Rust `enum` with `&self` methods. No `Box<dyn ...>` is needed because the only two variants are `List` and `String`.
- When you need to validate behavior against upstream, build the C++ version in `../sexp/build/` and run its `sexp_tests` binary.

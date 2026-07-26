use thiserror::Error;

#[derive(Debug, Error)]
pub enum SexpError {
    #[error("unexpected end of input at position {pos}")]
    UnexpectedEof { pos: usize },

    #[error("illegal character {ch:?} (0x{ch:02x}) at position {pos}")]
    IllegalChar { ch: u8, pos: usize },

    #[error("expected {expected:?} but found {found:?} at position {pos}")]
    CharMismatch {
        expected: char,
        found: char,
        pos: usize,
    },

    #[error("declared length {declared} but actual length {actual} at position {pos}")]
    LengthMismatch {
        declared: u32,
        actual: usize,
        pos: usize,
    },

    #[error("maximum nesting depth {max} exceeded at position {pos}")]
    MaxDepthExceeded { max: usize, pos: usize },

    #[error("unknown escape sequence \\{ch} at position {pos}")]
    UnknownEscape { ch: char, pos: usize },

    #[error("verbatim string too long: {len} bytes")]
    VerbatimTooLong { len: u32 },

    #[error("decimal number too long at position {pos}")]
    DecimalTooLong { pos: usize },

    #[error("octal character too big: {val}")]
    OctalTooBig { val: u32 },
}

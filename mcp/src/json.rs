//! The wire (docs/spec/mcp.md §3): RFC 8259, narrowed by I-JSON (RFC 7493)
//! and by an exact-number rule.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// The largest message read, in bytes (§3.3).
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// The deepest nesting of arrays and objects read (§3.3).
pub const MAX_DEPTH: usize = 64;

/// A JSON value. Objects keep their members in byte order of their names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    /// An integer: no fraction, no exponent, within `i64`.
    Int(i64),
    /// A number with a fraction or an exponent: its exact lexeme.
    Decimal(Decimal),
    Str(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

/// A number with a fraction or an exponent, kept as read (§3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal {
    /// The lexeme, byte for byte.
    pub text: String,
    /// Its first byte's offset in the message.
    pub at: usize,
}

/// Why reading stopped (§3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Size,
    Depth,
    End,
    Unexpected,
    Trailing,
    Control,
    Escape,
    Surrogate,
    Utf8,
    Noncharacter,
    Duplicate,
    LeadingZero,
    NegativeZero,
    IntegerRange,
    NotInteger,
}

impl Kind {
    /// The refusal's word, as `data.refusal` carries it.
    pub fn word(self) -> &'static str {
        ""
    }
}

/// A refusal: its kind, and the byte offset where reading stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub kind: Kind,
    pub at: usize,
}

/// Read one message.
pub fn read(bytes: &[u8]) -> Result<Value, Refusal> {
    let _ = bytes;
    Err(Refusal {
        kind: Kind::End,
        at: 0,
    })
}

/// Print a value on one line, members in byte order of their names.
pub fn print(value: &Value) -> String {
    let _ = value;
    String::new()
}

impl Value {
    /// The value as an integer where one is expected: `None` if it isn't a
    /// number, `not-integer` if it has a fraction or an exponent.
    pub fn integer(&self) -> Option<Result<i64, Refusal>> {
        None
    }
}

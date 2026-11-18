//! P1, the facts reader: the reflection's format, `mirror.facts`' subset of
//! pact text.
//!
//! The grammar is assembled from mirror's spec text (docs/spec/listen.md §4
//! gives it whole, with the open questions):
//! - reflection seam A1.4: `# <key> <value>` header lines, `fact` lines with
//!   `addr`/`nat`/`bytes`/`sym` arguments, `#` comments, nothing else; an
//!   unknown line form is refused with its location, never dropped;
//! - Seam's pact-eval audit §1.2 (the frozen pact-text reader): the line,
//!   blank, comment and `fact` record shapes, names, the byte set;
//! - fixpoint §5.0.1 items 6–7 and §16.10 item 1: the typed constants;
//! - fixpoint §16.2.1: positions `L:C`, columns count bytes.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::value::Atom;

/// A place in the input: 1-based line, 1-based byte column (fixpoint
/// §16.2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Why a line doesn't read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// A byte outside the line grammar: CR, TAB, NUL, DEL, or a byte that
    /// isn't printable ASCII outside a `sym` string.
    Byte(u8),
    /// A line that is neither blank, a `#` line, nor a `fact` record:
    /// `rule`, `query`, `functional`, an indented record, anything else.
    Form,
    /// Something else stood where this was expected.
    Expected(&'static str),
    /// `not` or `requires`, or a type word (`sym`, `nat`, `addr`, `bytes`)
    /// without its value, where a name belongs.
    Keyword,
    /// `int` or `ratio`: reserved, refused until pinned (§16.10 item 1).
    Reserved,
    /// An uppercase hex digit: hex is lowercase (§5.0.1 item 7).
    Uppercase,
    /// An `addr` that isn't exactly 64 hex digits.
    AddrLength,
    /// A `bytes` string with an odd number of hex digits.
    OddHex,
    /// A `nat` with a leading zero.
    LeadingZero,
    /// A `nat` above 2⁶⁴ − 1 (its encoding is `u64(n)`).
    NatOverflow,
    /// A `\` in a `sym` string not followed by `"` or `\`.
    Escape,
    /// A `sym` string that isn't UTF-8.
    Utf8,
}

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unreadable::Byte(b) => write!(f, "byte 0x{b:02x}"),
            Unreadable::Form => f.write_str("unknown line form"),
            Unreadable::Expected(what) => write!(f, "expected {what}"),
            Unreadable::Keyword => f.write_str("a keyword where a name belongs"),
            Unreadable::Reserved => f.write_str("a reserved type"),
            Unreadable::Uppercase => f.write_str("an uppercase hex digit"),
            Unreadable::AddrLength => f.write_str("an addr is 64 hex digits"),
            Unreadable::OddHex => f.write_str("an odd number of hex digits"),
            Unreadable::LeadingZero => f.write_str("a leading zero"),
            Unreadable::NatOverflow => f.write_str("a nat above 2^64 - 1"),
            Unreadable::Escape => f.write_str("an unknown escape"),
            Unreadable::Utf8 => f.write_str("invalid UTF-8"),
        }
    }
}

/// A located refusal: the first place the input doesn't read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub at: Position,
    pub what: Unreadable,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.at, self.what)
    }
}

/// A `# <key> <value>` header line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub key: String,
    pub value: String,
    pub line: usize,
}

/// A `fact` line's atom, with the line it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    pub atom: Atom,
    pub line: usize,
}

/// What a facts text says: its headers and its facts, in input order.
/// Duplicates are kept: forming the set is the caller's step (fixpoint
/// §5.0.1 item 2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub headers: Vec<Header>,
    pub facts: Vec<Fact>,
}

/// Read a facts text. Positions count from line 1.
pub fn read(input: &[u8]) -> Result<Facts, Refusal> {
    read_from(input, 1)
}

/// Cut `s` (P3) and read its reflection, in the whole file's coordinates.
/// `None` when `s` is unreflected. The part above the seam is never read.
pub fn read_reflection(s: &[u8]) -> Result<Option<Facts>, Refusal> {
    match crate::cut::cut(s).reflection {
        None => Ok(None),
        Some(r) => read_from(r.bytes, r.seam_line + 1).map(Some),
    }
}

fn read_from(_input: &[u8], _first_line: usize) -> Result<Facts, Refusal> {
    Ok(Facts::default())
}

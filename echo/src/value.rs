//! Typed constants and atoms: what a `fact` line says, with every argument's
//! type kept.
//!
//! mirror `docs/specs/mirror-fixpoint-parser.md`:
//! - §5.0.1 item 6: the canonical text `sym "<utf-8>"` (only `\"` and `\\`
//!   escape), `nat <decimal>`, `addr <64 lowercase hex>`;
//! - §16.10 item 1: `bytes "<lowercase hex>"`, the `0x03` argument; equality
//!   is by the §5.0 bytes, so a bare name and `sym "<name>"` are one
//!   constant, and an `addr` is never a `sym` with the same text;
//! - §5.0 and §5.0.1 items 10–11: an atom's content is
//!   `enc(p) ‖ u64(k) ‖ arg₀ ‖ …`, each argument `tag ‖ enc(bytes)`.
//!
//! The hash `H` over this content (the address law, prism-020 P2) is not
//! here yet.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write as _};

/// A constant. The variant is the type: two values are equal only when
/// their types and their bytes are.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// `sym "<utf-8>"`, or a bare name. Tag `0x00`.
    Sym(String),
    /// `nat <decimal>`, `0 ..= 2⁶⁴ − 1`. Tag `0x01`, eight bytes big-endian.
    Nat(u64),
    /// `addr <64 lowercase hex>`. Tag `0x02`, the 32 raw bytes.
    Addr([u8; 32]),
    /// `bytes "<lowercase hex>"`. Tag `0x03`.
    Bytes(Vec<u8>),
}

impl Value {
    /// The argument tag (§5.0.1 item 11; §16.10 item 1 for `bytes`).
    pub fn tag(&self) -> u8 {
        match self {
            Value::Sym(_) => 0x00,
            Value::Nat(_) => 0x01,
            Value::Addr(_) => 0x02,
            Value::Bytes(_) => 0x03,
        }
    }

    /// The type's word in the canonical text.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Sym(_) => "sym",
            Value::Nat(_) => "nat",
            Value::Addr(_) => "addr",
            Value::Bytes(_) => "bytes",
        }
    }

    /// The argument's encoding: `tag ‖ enc(bytes)`.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.tag());
        match self {
            Value::Sym(s) => enc(s.as_bytes(), out),
            Value::Nat(n) => enc(&n.to_be_bytes(), out),
            Value::Addr(a) => enc(a, out),
            Value::Bytes(b) => enc(b, out),
        }
    }
}

/// `enc(b) = u64(len(b)) ‖ b` (§5.0).
fn enc(b: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&(b.len() as u64).to_be_bytes());
    out.extend_from_slice(b);
}

fn hex(f: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    bytes.iter().try_for_each(|b| write!(f, "{b:02x}"))
}

/// The canonical text (§5.0.1 item 6, §16.10 item 1). A `sym` always
/// prints typed, so the printed form never loses the type.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Sym(s) => {
                f.write_str("sym \"")?;
                for c in s.chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        c => f.write_char(c)?,
                    }
                }
                f.write_str("\"")
            }
            Value::Nat(n) => write!(f, "nat {n}"),
            Value::Addr(a) => {
                f.write_str("addr ")?;
                hex(f, a)
            }
            Value::Bytes(b) => {
                f.write_str("bytes \"")?;
                hex(f, b)?;
                f.write_str("\"")
            }
        }
    }
}

/// A ground atom `p(a₀, …)`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Atom {
    /// The predicate, a pact name.
    pub pred: String,
    /// The arguments, in order, each with its type.
    pub args: Vec<Value>,
}

impl Atom {
    /// The atom's content: `enc(p) ‖ u64(k) ‖ arg₀ ‖ … ‖ arg_{k−1}`
    /// (§5.0.1 item 10; what `H` hashes under `"splinter\0" ‖ τ`).
    pub fn content(&self) -> Vec<u8> {
        let mut out = Vec::new();
        enc(self.pred.as_bytes(), &mut out);
        out.extend_from_slice(&(self.args.len() as u64).to_be_bytes());
        for a in &self.args {
            a.encode(&mut out);
        }
        out
    }
}

/// `p` with no arguments, else `p(<canonical>, <canonical>, …)`.
impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pred)?;
        if self.args.is_empty() {
            return Ok(());
        }
        f.write_str("(")?;
        for (i, a) in self.args.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{a}")?;
        }
        f.write_str(")")
    }
}

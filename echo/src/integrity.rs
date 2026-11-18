//! V0 integrity (prism-020 P4): a checksum over a reflection, **not a
//! reader**.
//!
//! mirror `docs/math/2026-11-16-mara-the-reflection-seam.md`, amendment 1
//! (`4f6bdc17`): A1.3's table, "each address in `r` recomputes from the
//! content `r` lists, and the header is well-formed", and A1.7 P4, "every
//! address in `r` recomputes from what `r` lists; the header is
//! well-formed; `H(a)` matches".
//!
//! What that means today, with the one reflection format that is pinned
//! (A1.4: `mirror.facts` is the precedent; fixpoint §16.10 item 1):
//! 1. `r` reads under the facts grammar (P1); a refusal is located.
//! 2. **The header is well-formed:** exactly `# format-grammar <40 hex>`,
//!    `# root <64 hex>`, `# grammar <64 hex>`, in that order, lowercase
//!    (§5.0.1 item 7).
//! 3. **The facts are a set, listed as one:** strictly ascending by atom
//!    address at `@parse/item` root (§16.10 item 1, §15.2; §5.0.1 items
//!    1–2), so a repeat or an out-of-order line is refused at its line.
//! 4. **Every address in `r` that `r` lists the content of recomputes:**
//!    `# grammar` = `set_addr` over the facts, ℓ `grammar` (§16.10 item 1).
//!    `# root` is `addr(N₀)`, which only a reading of `a` gives (V2), and
//!    `# format-grammar` is a git blob of the grammar's source; neither is
//!    listed in `r`, so V0 checks their form only.
//! 5. **`H(a)`** is computed, BLAKE3 of the bytes above the seam (seam note
//!    §2.1), and returned. No pinned header carries it yet (§4.1's
//!    `reflect above …` is "a proposal"), so there is nothing in `r` to
//!    match it against: docs/spec/echo.md §9 Q12.

use core::fmt;

use alloc::vec::Vec;

use crate::address::{self, Address, Tau};
use crate::facts::{read_from, Facts, Position, Refusal};

/// The header keys of an F_G file, in order, with their hex lengths
/// (fixpoint §16.10 item 1).
pub const HEADERS: [(&str, usize); 3] = [("format-grammar", 40), ("root", 64), ("grammar", 64)];

/// What V0 established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Integrity {
    /// `addr(F_G)`, recomputed, and equal to `# grammar`.
    pub grammar: Address,
    /// How many facts the set holds.
    pub facts: usize,
    /// `H(a)`, BLAKE3 of the bytes above the seam; `None` for a facts file
    /// read on its own.
    pub above: Option<Address>,
}

/// What is wrong with a header line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderFault {
    /// The header block ended before this key.
    Missing(&'static str),
    /// Another key stands where this one belongs.
    Expected(&'static str),
    /// A header after the three.
    Unexpected,
    /// The value is not lowercase hex of the key's length.
    Hex,
}

/// Why V0 refuses, and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Breach {
    /// The file has no seam: there is no reflection to check.
    Unreflected,
    /// `r` doesn't read (P1).
    Unreadable(Refusal),
    /// A header line is malformed.
    Header { at: Position, fault: HeaderFault },
    /// A fact is not strictly after the one before it.
    Order { at: Position, repeated: bool },
    /// A listed address doesn't recompute from what `r` lists.
    Address {
        at: Position,
        claimed: Address,
        computed: Address,
    },
}

fn hex(f: &mut fmt::Formatter<'_>, a: &Address) -> fmt::Result {
    a.iter().try_for_each(|b| write!(f, "{b:02x}"))
}

impl fmt::Display for Breach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Breach::Unreflected => f.write_str("unreflected: no seam"),
            Breach::Unreadable(r) => write!(f, "unreadable: {r}"),
            Breach::Header { at, fault } => {
                write!(f, "{at} header: ")?;
                match fault {
                    HeaderFault::Missing(k) => write!(f, "`{k}` is missing"),
                    HeaderFault::Expected(k) => write!(f, "`{k}` belongs here"),
                    HeaderFault::Unexpected => f.write_str("a header after `grammar`"),
                    HeaderFault::Hex => f.write_str("not lowercase hex of the key's length"),
                }
            }
            Breach::Order { at, repeated: true } => write!(f, "{at} a fact repeated"),
            Breach::Order { at, .. } => write!(f, "{at} a fact out of ascending address order"),
            Breach::Address {
                at,
                claimed,
                computed,
            } => {
                write!(f, "{at} address: claimed ")?;
                hex(f, claimed)?;
                f.write_str(", recomputed ")?;
                hex(f, computed)
            }
        }
    }
}

/// V0 over a facts file read on its own (lines from 1).
pub fn check_facts(r: &[u8]) -> Result<Integrity, Breach> {
    integrity(r, 1, None)
}

/// V0 over a whole file: cut it (P3), check its reflection in the whole
/// file's coordinates, and give `H(a)` back.
pub fn check_reflection(s: &[u8]) -> Result<Integrity, Breach> {
    let cut = crate::cut::cut(s);
    let r = cut.reflection.ok_or(Breach::Unreflected)?;
    integrity(r.bytes, r.seam_line + 1, Some(address::hash(cut.above)))
}

fn integrity(r: &[u8], first_line: usize, above: Option<Address>) -> Result<Integrity, Breach> {
    let read = read_from(r, first_line).map_err(Breach::Unreadable)?;
    let claimed = header(&read, first_line)?;
    let item = Tau::parse_item();
    let mut members: Vec<Address> = Vec::with_capacity(read.facts.len());
    for fact in &read.facts {
        let a = address::atom(&item, &fact.atom);
        if let Some(&prev) = members.last() {
            if a <= prev {
                return Err(Breach::Order {
                    at: Position {
                        line: fact.line,
                        column: 1,
                    },
                    repeated: a == prev,
                });
            }
        }
        members.push(a);
    }
    // strictly ascending already, so set_addr is the fragment as listed
    let computed = address::fragment(&item, "grammar", &members);
    if computed != claimed.0 {
        return Err(Breach::Address {
            at: claimed.1,
            claimed: claimed.0,
            computed,
        });
    }
    Ok(Integrity {
        grammar: computed,
        facts: members.len(),
        above,
    })
}

/// The three header lines, well-formed; gives `# grammar`'s value and where
/// it stands.
fn header(read: &Facts, first_line: usize) -> Result<(Address, Position), Breach> {
    let h = &read.headers;
    let fault = |line, column, fault| Breach::Header {
        at: Position { line, column },
        fault,
    };
    let mut grammar = ([0; 32], Position { line: 0, column: 0 });
    for (i, (key, len)) in HEADERS.iter().enumerate() {
        let Some(got) = h.get(i) else {
            let line = h.last().map_or(first_line, |x| x.line + 1);
            return Err(fault(line, 1, HeaderFault::Missing(key)));
        };
        if got.key != *key {
            return Err(fault(got.line, 3, HeaderFault::Expected(key)));
        }
        let column = key.len() + 4;
        let v = got.value.as_bytes();
        let lower_hex = |b: &u8| b.is_ascii_digit() || (b'a'..=b'f').contains(b);
        if v.len() != *len || !v.iter().all(lower_hex) {
            return Err(fault(got.line, column, HeaderFault::Hex));
        }
        if *key == "grammar" {
            for (j, out) in grammar.0.iter_mut().enumerate() {
                *out = (nibble(v[2 * j]) << 4) | nibble(v[2 * j + 1]);
            }
            grammar.1 = Position {
                line: got.line,
                column,
            };
        }
    }
    if let Some(extra) = h.get(HEADERS.len()) {
        return Err(fault(extra.line, 3, HeaderFault::Unexpected));
    }
    Ok(grammar)
}

/// A lowercase hex digit's value (only called on one).
fn nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        _ => b - b'a' + 10,
    }
}

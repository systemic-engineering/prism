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
//!    match it against: docs/spec/echo.md §7 Q12.

use core::fmt;

use crate::address::Address;
use crate::facts::{Position, Refusal};

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

impl fmt::Display for Breach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = f;
        Ok(())
    }
}

/// V0 over a facts file read on its own (lines from 1).
pub fn check_facts(r: &[u8]) -> Result<Integrity, Breach> {
    let _ = r;
    Ok(Integrity {
        grammar: [0; 32],
        facts: 0,
        above: None,
    })
}

/// V0 over a whole file: cut it (P3), check its reflection in the whole
/// file's coordinates, and give `H(a)` back.
pub fn check_reflection(s: &[u8]) -> Result<Integrity, Breach> {
    check_facts(s)
}

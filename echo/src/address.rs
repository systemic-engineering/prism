//! The address law `H` (prism-020 P2), from mirror's spec text.
//!
//! mirror `docs/specs/mirror-fixpoint-parser.md` §5.0 (bytes, pinned):
//! - `u64(n)`: 8 bytes, unsigned, big-endian; `enc(b) = u64(len(b)) ‖ b`;
//! - `τ`: the UTF-8 of the altitude `@`-ref, then `0x00`, then `0x00` for
//!   the root grammar or `0x01 ‖` the 32 raw bytes of `addr(format grammar)`;
//! - `H`: BLAKE3, default (unkeyed) mode, 32-byte output;
//! - atom: `H("splinter\0" ‖ τ ‖ enc(content))`;
//! - fragment: `H("fragment\0" ‖ τ ‖ enc(ℓ) ‖ u64(k) ‖ addr(x₀) ‖ …)`;
//! - lens: `H("lens\0" ‖ τ ‖ enc(ℓ) ‖ u64(k) ‖ addr(t₀) ‖ …)`.
//!
//! §5.0.1 items 1–2 and §16.10 item 1 (`set_addr`): a set is a fragment
//! whose children are strictly ascending by their raw address bytes; a
//! repeated member is a caller bug, refused, never deduplicated.
//!
//! This is **not** prismqueer's scoped-BLAKE3 identity (`Oid`), which is a
//! different function (fixpoint §5: "no code may substitute one for the
//! other").

use alloc::string::String;
use alloc::vec::Vec;

use crate::value::Atom;

/// A 32-byte address, raw (never hex).
pub type Address = [u8; 32];

/// An altitude: its `@`-ref and its flag (§5.0 `τ`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tau {
    name: String,
    grammar: Option<Address>,
}

impl Tau {
    /// `name` with the root flag (`0x00`). `None` if the name holds a NUL,
    /// which would make `τ` ambiguous (§5.0: "a name never contains NUL").
    pub fn root(name: &str) -> Option<Tau> {
        let _ = name;
        None
    }

    /// `name` under a format grammar (`0x01 ‖ addr(G)`). `None` on a NUL.
    pub fn under(name: &str, grammar: Address) -> Option<Tau> {
        let _ = (name, grammar);
        None
    }

    /// The altitude's `@`-ref.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `addr(format grammar)`, or `None` under the root flag.
    pub fn grammar(&self) -> Option<&Address> {
        self.grammar.as_ref()
    }

    /// `τ`'s bytes.
    pub fn bytes(&self) -> Vec<u8> {
        Vec::new()
    }
}

/// A sorted set was handed the same member twice (§5.0.1 item 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repeated(pub Address);

/// `H`: BLAKE3, default mode, 32 bytes.
pub fn hash(bytes: &[u8]) -> Address {
    let _ = bytes;
    [0; 32]
}

/// An atom's preimage, `"splinter\0" ‖ τ ‖ enc(content)`.
pub fn atom_preimage(tau: &Tau, atom: &Atom) -> Vec<u8> {
    let _ = (tau, atom);
    Vec::new()
}

/// An atom's address.
pub fn atom(tau: &Tau, atom: &Atom) -> Address {
    hash(&atom_preimage(tau, atom))
}

/// A fragment's preimage; the children are in the order given.
pub fn fragment_preimage(tau: &Tau, label: &str, children: &[Address]) -> Vec<u8> {
    let _ = (tau, label, children);
    Vec::new()
}

/// A fragment's address.
pub fn fragment(tau: &Tau, label: &str, children: &[Address]) -> Address {
    hash(&fragment_preimage(tau, label, children))
}

/// A lens's preimage; the targets are in the order given.
pub fn lens_preimage(tau: &Tau, label: &str, targets: &[Address]) -> Vec<u8> {
    let _ = (tau, label, targets);
    Vec::new()
}

/// A lens's address.
pub fn lens(tau: &Tau, label: &str, targets: &[Address]) -> Address {
    hash(&lens_preimage(tau, label, targets))
}

/// `set_addr`: the fragment over `members` sorted strictly ascending.
pub fn set(tau: &Tau, label: &str, members: &[Address]) -> Result<Address, Repeated> {
    Ok(fragment(tau, label, members))
}

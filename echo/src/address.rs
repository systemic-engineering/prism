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
        Tau::new(name, None)
    }

    /// `name` under a format grammar (`0x01 ‖ addr(G)`). `None` on a NUL.
    pub fn under(name: &str, grammar: Address) -> Option<Tau> {
        Tau::new(name, Some(grammar))
    }

    /// `@parse/item` with the root flag: the altitude of every grammar
    /// fact and of F_G's set address (fixpoint §15.2, §16.10 item 1).
    pub fn parse_item() -> Tau {
        Tau {
            name: "@parse/item".into(),
            grammar: None,
        }
    }

    /// `@pact/fact` with the root flag: the store's altitude, of every
    /// model and fact atom and every context fragment (certificate §3.1.1
    /// A).
    pub fn pact_fact() -> Tau {
        Tau {
            name: "@pact/fact".into(),
            grammar: None,
        }
    }

    /// `@pact/program` with the root flag: the program fragment and its
    /// record atoms (certificate §3.1.1 A).
    pub fn pact_program() -> Tau {
        Tau {
            name: "@pact/program".into(),
            grammar: None,
        }
    }

    /// `@mirror/certificate` with the root flag: every atom and fragment
    /// the certificate itself introduces (certificate §3.1.1 A).
    pub fn mirror_certificate() -> Tau {
        Tau {
            name: "@mirror/certificate".into(),
            grammar: None,
        }
    }

    /// `@song/interval` with the root flag: the interval and `basis` atoms
    /// (Note §6.2 Bytes, §6.4).
    pub fn song_interval() -> Tau {
        Tau {
            name: "@song/interval".into(),
            grammar: None,
        }
    }

    fn new(name: &str, grammar: Option<Address>) -> Option<Tau> {
        (!name.as_bytes().contains(&0)).then(|| Tau {
            name: name.into(),
            grammar,
        })
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
        let mut out = Vec::from(self.name.as_bytes());
        out.push(0x00);
        match &self.grammar {
            None => out.push(0x00),
            Some(g) => {
                out.push(0x01);
                out.extend_from_slice(g);
            }
        }
        out
    }
}

/// A sorted set was handed the same member twice (§5.0.1 item 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repeated(pub Address);

/// `H`: BLAKE3, default mode, 32 bytes.
pub fn hash(bytes: &[u8]) -> Address {
    *blake3::hash(bytes).as_bytes()
}

/// The kind prefixes (§5.0). Never stored; only hashed.
const SPLINTER: &[u8] = b"splinter\0";
const FRAGMENT: &[u8] = b"fragment\0";
const LENS: &[u8] = b"lens\0";

/// `enc(b) = u64(len(b)) ‖ b`.
fn enc(b: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&(b.len() as u64).to_be_bytes());
    out.extend_from_slice(b);
}

/// `kind ‖ τ ‖ enc(ℓ) ‖ u64(k) ‖ addr₀ ‖ …`, the shape fragments and lenses share.
fn tree(kind: &[u8], tau: &Tau, label: &str, children: &[Address]) -> Vec<u8> {
    let mut out = Vec::from(kind);
    out.extend_from_slice(&tau.bytes());
    enc(label.as_bytes(), &mut out);
    out.extend_from_slice(&(children.len() as u64).to_be_bytes());
    children.iter().for_each(|c| out.extend_from_slice(c));
    out
}

/// An atom's preimage, `"splinter\0" ‖ τ ‖ enc(content)`.
pub fn atom_preimage(tau: &Tau, atom: &Atom) -> Vec<u8> {
    let mut out = Vec::from(SPLINTER);
    out.extend_from_slice(&tau.bytes());
    enc(&atom.content(), &mut out);
    out
}

/// An atom's address.
pub fn atom(tau: &Tau, atom: &Atom) -> Address {
    hash(&atom_preimage(tau, atom))
}

/// A fragment's preimage; the children are in the order given.
pub fn fragment_preimage(tau: &Tau, label: &str, children: &[Address]) -> Vec<u8> {
    tree(FRAGMENT, tau, label, children)
}

/// A fragment's address.
pub fn fragment(tau: &Tau, label: &str, children: &[Address]) -> Address {
    hash(&fragment_preimage(tau, label, children))
}

/// A lens's preimage; the targets are in the order given.
pub fn lens_preimage(tau: &Tau, label: &str, targets: &[Address]) -> Vec<u8> {
    tree(LENS, tau, label, targets)
}

/// A lens's address.
pub fn lens(tau: &Tau, label: &str, targets: &[Address]) -> Address {
    hash(&lens_preimage(tau, label, targets))
}

/// `set_addr`: the fragment over `members` sorted strictly ascending.
/// Ascending, unsigned, byte-wise (§5.0.1 item 1), which is `[u8; 32]`'s
/// own order; a repeated member is refused (item 2).
pub fn set(tau: &Tau, label: &str, members: &[Address]) -> Result<Address, Repeated> {
    let mut sorted = Vec::from(members);
    sorted.sort_unstable();
    if let Some(w) = sorted.windows(2).find(|w| w[0] == w[1]) {
        return Err(Repeated(w[0]));
    }
    Ok(fragment(tau, label, &sorted))
}

//! `prismqueer::crossing`: what a value carries to cross a boundary (the
//! Drop, or a gate). Mara's Theorem 1 (mirror `mara/ouroboros` 199aeaa7,
//! §1.2, §6.1, §12 item 8; prism-017 ask 8). The names are provisional; Alex
//! names them.
//!
//! # Two payloads (Theorem 1)
//!
//! Admission has two primitives, as the glass wall's grades do (mirror
//! `docs/math/2026-11-01-mara-the-glass-wall.md` §1, ordered
//! `true < maybe < false`):
//!
//! - a **proof**, an [`Edge`]: an exact witness the receiver checks by
//!   recomputation. Theorem 1's law part: replay instrument `i` on the bytes
//!   at `b` and obtain `v`, the decidable equality `i(b) = v`. The glass
//!   wall's `true`, "an edge to a crystal the receiver can check".
//! - **seals**, [`Signer`]s: someone vouches for something a replay can't
//!   establish (that `i` reads what `v` is used as, a ruler, a closed world).
//!   Theorem 1's judgment part. The glass wall's `maybe`.
//!
//! A [`Claim`] is a value's address with `proof: Option<Edge>` and a set of
//! seals. Three shapes may cross, and the claim says which as data
//! ([`Grade`]): a proof alone is a law, seals alone are a sealed judgment,
//! both are a measurement, `proof × seal`. The fourth shape, neither, is
//! **anonymous** (§6.1): nothing says why the value should be believed. It is
//! refused at construction ([`Refusal::Anonymous`]), so no claim is ever
//! anonymous. An anonymous value is Flux, and stays there (Theorem 3).
//!
//! # What prism checks, and what it never does
//!
//! - **An edge is checked when it is made, and only then exists.**
//!   [`Edge::replay`] runs a [`Law`] on the input crystal's bytes and
//!   compares the result with the output crystal's bytes. They differ:
//!   [`NotAProof`]. prism can check this law part because a replay is a
//!   recomputation over content-addressed bytes. It can't check that the
//!   code it ran is the instrument [`Law::oid`] names: that binding is the
//!   caller's, as a signer's identity is. A proof from an outside checker
//!   (a kernel certificate, a nix narHash) is not an `Edge` here, because
//!   prism can't check it.
//! - **A seal is never verified.** prism has no keys. A [`Signer`] is a
//!   signature *claim* (who, over which address), held and never checked
//!   cryptographically. Verifying a signature is outside prism, as in
//!   [`algedonic`](crate::algedonic).
//! - **Admission is mirror's.** There is no `admit`. The grade is data;
//!   which grades a gate takes and whose seals it accepts (`Sig_R`) are the
//!   receiver's policy (`mirror-hole.md` §11, `Calm { admits, seals }`).
//!
//! # Seals are a set
//!
//! Seals compose by union (glass wall §2a, `maybe ; maybe` is `S₁ ∪ S₂`;
//! Theorem 9, a crystal's required signers are a union). A union is
//! idempotent, so one signer sealing one address twice is **one** seal: it
//! collapses, and is not refused. (An [`IndependenceDraft`] refuses an
//! instrument named twice, because there a repeat makes the judgment claim
//! less than it says. A repeated seal is the same signature again.) A seal is
//! `(by, over)`: one signer over two addresses, say a trigger ruler and the
//! value, is two seals, and both are kept. A seal need not be over the
//! value: a trigger ruler's signer joins every downstream claim as it is
//! (Theorem 9), never merged into another record.
//!
//! [`IndependenceDraft`]: crate::algedonic::IndependenceDraft
//!
//! # No way around the refusal
//!
//! A claim can't be written as a literal:
//!
//! ```compile_fail,E0451
//! use std::collections::BTreeSet;
//! use prismqueer::crossing::Claim;
//! use prismqueer::oid::Oid;
//! let _ = Claim { value: Oid::new("v"), proof: None, seals: BTreeSet::new() };
//! ```
//!
//! nor had by default:
//!
//! ```compile_fail,E0599
//! use prismqueer::crossing::Claim;
//! let _ = Claim::default();
//! ```
//!
//! and an edge can't be written without its replay:
//!
//! ```compile_fail,E0451
//! use prismqueer::crossing::Edge;
//! use prismqueer::oid::Oid;
//! let _ = Edge { from: Oid::new("b"), via: Oid::new("i"), to: Oid::new("v") };
//! ```
//!
//! The control: the refusing constructor compiles, refuses both empty, and
//! admits a seal alone as a sealed judgment.
//!
//! ```
//! use prismqueer::crossing::{attest, Claim, Grade, Refusal};
//! use prismqueer::oid::Oid;
//! let v = Oid::new("v");
//! assert_eq!(Claim::new(v.clone(), None, []), Err(Refusal::Anonymous));
//! let sealed = Claim::new(v.clone(), None, [attest(&v, Oid::new("observer"))]).unwrap();
//! assert_eq!(sealed.grade(), Grade::Sealed);
//! ```

use std::collections::BTreeSet;

use crate::algedonic::Signer;
use crate::brain::Crystal;
use crate::oid::Oid;

/// A deterministic instrument: the same bytes in, the same bytes out
/// (Theorem 1's premise). Its address is the caller's name for it.
pub trait Law {
    /// The instrument's address, as the caller vouches for it.
    fn oid(&self) -> Oid;
    /// Run the instrument on `input`.
    fn apply(&self, input: &[u8]) -> Vec<u8>;
}

/// A proof: the instrument `via`, replayed on the crystal at `from`, gives
/// the crystal at `to`. Minted only by [`Edge::replay`], which checks it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Edge {
    from: Oid,
    via: Oid,
    to: Oid,
}

/// The replay gave other bytes: not a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAProof;

impl Edge {
    /// Replay `law` on `input`'s bytes; an edge to `output` if the result is
    /// `output`'s bytes, refused otherwise. The only way to an edge.
    pub fn replay(law: &impl Law, input: &Crystal, output: &Crystal) -> Result<Self, NotAProof> {
        // 🔴 stub: no replay, so every edge is a proof.
        let _ = law.apply(input.bytes());
        Ok(Edge {
            from: input.address().clone(),
            via: law.oid(),
            to: output.address().clone(),
        })
    }

    /// The input's address.
    pub fn from(&self) -> &Oid {
        &self.from
    }

    /// The instrument's address.
    pub fn via(&self) -> &Oid {
        &self.via
    }

    /// The output's address: the value the edge proves.
    pub fn to(&self) -> &Oid {
        &self.to
    }
}

/// An observer's seal over a value's address: a signature claim prism holds
/// and never verifies.
pub fn attest(value: &Oid, by: Oid) -> Signer {
    Signer::mint(by, value.clone())
}

/// Which admissible shape a claim has (§6.1). Data, never a decision: which
/// shapes cross is the receiver's gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Grade {
    /// A proof alone: a law, the glass wall's `true`.
    Law,
    /// Seals alone: a judgment sealed by an observer, the glass wall's
    /// `maybe`.
    Sealed,
    /// Both: a measurement, `proof × seal` (Theorem 1), the stronger form.
    /// The proof makes the bytes checkable; the seals vouch for what they
    /// are used as.
    Measured,
}

/// Why a claim can't be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No proof and no seal: nothing says why to believe it.
    Anonymous,
    /// The proof ends at another value than the one claimed.
    Elsewhere,
}

/// A value with what it carries to cross: `proof: Option<Edge>` and a set
/// of seals, never both empty. No literal, no default: [`Claim::new`] is the
/// only way, and it refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    value: Oid,
    proof: Option<Edge>,
    seals: BTreeSet<Signer>,
}

impl Claim {
    /// `value` with its proof and seals. Refused when both are empty
    /// ([`Refusal::Anonymous`]) or when the proof proves another value
    /// ([`Refusal::Elsewhere`]). A signer repeated over one address
    /// collapses into one seal.
    pub fn new(
        value: Oid,
        proof: Option<Edge>,
        seals: impl IntoIterator<Item = Signer>,
    ) -> Result<Self, Refusal> {
        // 🔴 stub: refuses nothing and keeps no seal.
        let _ = seals.into_iter().count();
        let seals: BTreeSet<Signer> = BTreeSet::new();
        Ok(Claim {
            value,
            proof,
            seals,
        })
    }

    /// The value's address.
    pub fn value(&self) -> &Oid {
        &self.value
    }

    /// The proof, if any.
    pub fn proof(&self) -> Option<&Edge> {
        self.proof.as_ref()
    }

    /// The seals, ordered by signer then address. Held, never verified.
    pub fn seals(&self) -> &BTreeSet<Signer> {
        &self.seals
    }

    /// The claim's shape.
    pub fn grade(&self) -> Grade {
        // 🔴 stub: every claim reads as a law.
        Grade::Law
    }
}

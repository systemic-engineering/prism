//! Verify-only checks for prism's ledger (prism-019).
//!
//! - [`merkle`]: RFC 9162 §2.1 Merkle tree hashes, inclusion proofs
//!   (§2.1.3.2) and consistency proofs (§2.1.4.2), over SHA-256.
//! - [`note`]: C2SP `signed-note` parsing, key IDs and verifier keys.
//! - [`checkpoint`]: C2SP `tlog-checkpoint` bodies, verified under the
//!   hybrid **AND** rule: Ed25519 *and* ML-DSA-44 must both verify.
//! - [`anchor`]: the SLH-DSA-SHA2-128s anchor over a checkpoint.
//! - [`quorum`]: Lemma B's witness quorum, counting distinct witnesses
//!   whose Ed25519 *and* ML-DSA-44 cosignatures both verify.
//! - [`sshsig`]: OpenSSH SSHSIG with the `ssh-mldsa44-ed25519` composite
//!   key (Alex's seal): ML-DSA-44 *and* Ed25519 must both verify.
//!
//! It never holds a private key: every check takes public keys and
//! signatures. No I/O, no clock, no RNG: `#![no_std]` with `alloc`, so the
//! same crate builds for `wasm32-unknown-unknown`. The spec is
//! `docs/spec/ledger.md`.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod anchor;
pub mod checkpoint;
pub mod commitment;
pub mod merkle;
pub mod note;
pub mod quorum;
pub mod sshsig;

/// A SHA-256 output: a leaf hash, a node hash or a tree root.
pub type Hash = [u8; 32];

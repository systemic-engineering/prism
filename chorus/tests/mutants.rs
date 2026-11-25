//! The broken lean, standing: chorus's AND rule mutated to OR.
//!
//! chorus accepts a checkpoint only if Ed25519 **and** ML-DSA-44 both
//! verify, and counts a witness toward Q only if both its cosignatures
//! verify (bullet-up-008, Alex). The OR mutations (AND → OR on checkpoints
//! and on witnesses) were once run by hand and reverted. Here they stand
//! as test-only rules, so Seam's `B_` row ("the broken lean has its
//! counterexample") is a test, not a memory: checkpoint.rs and quorum.rs
//! assert that each mutant **accepts** the single-signature fixtures the
//! real rule refuses, and that the real rule refuses them.
//!
//! The mutants are folds over what the real rule exposes, with `&&`
//! swapped for `||` at the one place the rule combines its two schemes.
//! Nothing in chorus's public API changes for them:
//! - a checkpoint's per-scheme readings are `CheckpointError::Unsigned`'s
//!   two fields (every other error comes before the rule);
//! - a tally's witnesses with either cosignature valid are `valid` (both)
//!   plus `partial` (exactly one).

use chorus::checkpoint::{verify, CheckpointError, LogKeys};
use chorus::quorum::Tally;

/// The OR rule on a checkpoint: accepted iff Ed25519 **or** ML-DSA-44
/// verifies.
pub fn or_checkpoint(note: &str, keys: &LogKeys) -> bool {
    match verify(note, keys) {
        Ok(_) => true,
        Err(CheckpointError::Unsigned { ed25519, mldsa44 }) => ed25519 || mldsa44,
        Err(_) => false,
    }
}

/// The OR rule on witnesses: a witness counts toward Q if **either** of its
/// cosignatures verifies.
pub fn or_quorum(t: &Tally) -> bool {
    t.valid + t.partial >= t.required
}

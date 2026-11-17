//! C2SP `tlog-checkpoint`, verified under the hybrid AND rule.
//!
//! A checkpoint's note text is three lines: origin, tree size (ASCII
//! decimal), root hash (base64). It carries two log signatures:
//!
//! - **Ed25519** (type 0x01) over the note text (C2SP "Ed25519 Log
//!   Signatures");
//! - **ML-DSA-44** (type 0x06), the log cosigning itself: an 8-byte
//!   timestamp, then FIPS 204 ML-DSA-44 (pure, empty context) over the
//!   `CosignedSubtree` of `tlog-cosignature` with `start = 0`, `end = size`.
//!
//! [`verify`] accepts only if **both** verify. One alone never does: OR
//! would make the post-quantum half decoration (bullet-up-008, Alex).
//!
//! Extension lines are refused: ML-DSA-44 cosignatures don't sign them, so
//! the two signatures would not cover the same statement.

use alloc::string::String;
use alloc::vec::Vec;

use crate::note::{SignatureLine, Vkey};
use crate::Hash;

/// ML-DSA-44 public key length (FIPS 204, Table 2).
pub const MLDSA44_PUBLIC_KEY: usize = 1312;
/// ML-DSA-44 signature length (FIPS 204, Table 2).
pub const MLDSA44_SIGNATURE: usize = 2420;

/// Why a checkpoint did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointError {
    /// The note or a vkey did not parse.
    Note(crate::note::NoteError),
    /// The note text is not origin, size, root hash.
    Body,
    /// The note text has extension lines.
    Extensions,
    /// A vkey of the wrong signature type or key length.
    Key,
    /// Two signature lines with one key name and key ID.
    Duplicate,
    /// The AND rule failed: which of the two signatures verified.
    Unsigned { ed25519: bool, mldsa44: bool },
}

/// A checkpoint: origin, tree size, root hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub origin: String,
    pub size: u64,
    pub root: Hash,
}

impl Checkpoint {
    /// The note text: `origin\nsize\nbase64(root)\n`.
    pub fn body(&self) -> String {
        String::new()
    }

    /// Parse a note text of exactly three lines.
    pub fn parse(text: &str) -> Result<Checkpoint, CheckpointError> {
        let _ = text;
        Err(CheckpointError::Body)
    }

    /// The C2SP `CosignedSubtree` for this checkpoint (`start = 0`,
    /// `end = size`), as a cosigner signs it.
    pub fn cosigned_subtree(&self, cosigner: &str, timestamp: u64) -> Vec<u8> {
        let _ = (cosigner, timestamp);
        Vec::new()
    }
}

/// An Ed25519 log key (type 0x01).
#[derive(Debug, Clone)]
pub struct Ed25519Key {
    pub name: String,
    pub id: u32,
    key: ed25519_dalek::VerifyingKey,
}

impl Ed25519Key {
    pub fn from_vkey(vkey: &Vkey) -> Result<Ed25519Key, CheckpointError> {
        let _ = vkey;
        Err(CheckpointError::Key)
    }

    /// Does this line carry this key's name and ID?
    pub fn names(&self, line: &SignatureLine) -> bool {
        line.name == self.name && line.id == self.id
    }

    /// RFC 8032 Ed25519 (strict) over `message`.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let _ = (message, signature, &self.key);
        false
    }
}

/// An ML-DSA-44 cosigner key (type 0x06): the log's own, or a witness's.
#[derive(Debug, Clone)]
pub struct MlDsaKey {
    pub name: String,
    pub id: u32,
    key: ml_dsa::VerifyingKey<ml_dsa::MlDsa44>,
}

impl MlDsaKey {
    pub fn from_vkey(vkey: &Vkey) -> Result<MlDsaKey, CheckpointError> {
        let _ = vkey;
        Err(CheckpointError::Key)
    }

    /// Does this line carry this key's name and ID?
    pub fn names(&self, line: &SignatureLine) -> bool {
        line.name == self.name && line.id == self.id
    }

    /// The encoded public key, for comparing keys.
    pub fn public_key(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Verify a checkpoint cosignature (`timestamp ‖ ML-DSA-44 signature`);
    /// its timestamp if it verifies.
    pub fn verify(&self, checkpoint: &Checkpoint, cosignature: &[u8]) -> Option<u64> {
        let _ = (checkpoint, cosignature, &self.key);
        None
    }
}

/// The log's two keys: both must sign.
#[derive(Debug, Clone)]
pub struct LogKeys {
    pub ed25519: Ed25519Key,
    pub mldsa44: MlDsaKey,
}

/// A checkpoint that passed the AND rule, with the ML-DSA-44 timestamp the
/// log signed (reported, never compared to a clock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub checkpoint: Checkpoint,
    pub timestamp: u64,
}

/// Verify a signed checkpoint note under the AND rule.
pub fn verify(note: &str, keys: &LogKeys) -> Result<Verified, CheckpointError> {
    let _ = (note, keys);
    Err(CheckpointError::Unsigned {
        ed25519: false,
        mldsa44: false,
    })
}

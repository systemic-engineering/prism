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

use alloc::format;

use crate::note::{
    decode_base64, encode_base64, valid_key_name, SignatureLine, SignedNote, Vkey, ED25519,
    MLDSA44_COSIGNATURE,
};
use crate::Hash;

/// tlog-cosignature's `CosignedSubtree` label.
const SUBTREE_LABEL: &[u8; 12] = b"subtree/v1\n\0";

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
        format!(
            "{}\n{}\n{}\n",
            self.origin,
            self.size,
            encode_base64(&self.root)
        )
    }

    /// Parse a note text of exactly three lines.
    pub fn parse(text: &str) -> Result<Checkpoint, CheckpointError> {
        let lines = text.strip_suffix('\n').ok_or(CheckpointError::Body)?;
        let mut lines = lines.split('\n');
        let (Some(origin), Some(size), Some(root)) = (lines.next(), lines.next(), lines.next())
        else {
            return Err(CheckpointError::Body);
        };
        if lines.next().is_some() {
            return Err(CheckpointError::Extensions);
        }
        // The origin is also a CosignedSubtree field: 1..=255 bytes.
        if !valid_key_name(origin) || origin.len() > 255 {
            return Err(CheckpointError::Body);
        }
        let size = parse_decimal(size).ok_or(CheckpointError::Body)?;
        let root = decode_base64(root)
            .ok()
            .and_then(|r| Hash::try_from(r).ok())
            .ok_or(CheckpointError::Body)?;
        Ok(Checkpoint {
            origin: origin.into(),
            size,
            root,
        })
    }

    /// The C2SP `CosignedSubtree` for this checkpoint (`start = 0`,
    /// `end = size`), as a cosigner signs it.
    pub fn cosigned_subtree(&self, cosigner: &str, timestamp: u64) -> Vec<u8> {
        let mut m = Vec::with_capacity(12 + 1 + cosigner.len() + 8 + 1 + self.origin.len() + 48);
        m.extend_from_slice(SUBTREE_LABEL);
        opaque8(&mut m, cosigner.as_bytes());
        m.extend_from_slice(&timestamp.to_be_bytes());
        opaque8(&mut m, self.origin.as_bytes());
        m.extend_from_slice(&0u64.to_be_bytes());
        m.extend_from_slice(&self.size.to_be_bytes());
        m.extend_from_slice(&self.root);
        m
    }
}

/// `opaque x<1..2^8-1>`: a one-byte length, then the bytes. Callers hold
/// the length to 1..=255 (key names and origins are checked on parse).
fn opaque8(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(bytes.len() as u8);
    out.extend_from_slice(bytes);
}

/// An ASCII decimal with no extra leading zeros that fits a u64.
fn parse_decimal(text: &str) -> Option<u64> {
    let canonical = !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical.then(|| text.parse().ok()).flatten()
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
        let bytes: &[u8; 32] = vkey
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| CheckpointError::Key)?;
        if vkey.sig_type != ED25519 {
            return Err(CheckpointError::Key);
        }
        let key =
            ed25519_dalek::VerifyingKey::from_bytes(bytes).map_err(|_| CheckpointError::Key)?;
        Ok(Ed25519Key {
            name: vkey.name.clone(),
            id: vkey.id,
            key,
        })
    }

    /// Does this line carry this key's name and ID?
    pub fn names(&self, line: &SignatureLine) -> bool {
        line.name == self.name && line.id == self.id
    }

    /// RFC 8032 Ed25519 (strict) over `message`.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let Ok(bytes) = <&[u8; 64]>::try_from(signature) else {
            return false;
        };
        let signature = ed25519_dalek::Signature::from_bytes(bytes);
        self.key.verify_strict(message, &signature).is_ok()
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
        if vkey.sig_type != MLDSA44_COSIGNATURE || vkey.name.len() > 255 {
            return Err(CheckpointError::Key);
        }
        let encoded =
            ml_dsa::EncodedVerifyingKey::<ml_dsa::MlDsa44>::try_from(vkey.public_key.as_slice())
                .map_err(|_| CheckpointError::Key)?;
        Ok(MlDsaKey {
            name: vkey.name.clone(),
            id: vkey.id,
            key: ml_dsa::VerifyingKey::decode(&encoded),
        })
    }

    /// Does this line carry this key's name and ID?
    pub fn names(&self, line: &SignatureLine) -> bool {
        line.name == self.name && line.id == self.id
    }

    /// The encoded public key, for comparing keys.
    pub fn public_key(&self) -> Vec<u8> {
        self.key.encode().to_vec()
    }

    /// Verify a checkpoint cosignature (`timestamp ‖ ML-DSA-44 signature`);
    /// its timestamp if it verifies.
    pub fn verify(&self, checkpoint: &Checkpoint, cosignature: &[u8]) -> Option<u64> {
        if cosignature.len() != 8 + MLDSA44_SIGNATURE {
            return None;
        }
        let (time, signature) = cosignature.split_at(8);
        let timestamp = u64::from_be_bytes(time.try_into().ok()?);
        let signature = ml_dsa::Signature::<ml_dsa::MlDsa44>::try_from(signature).ok()?;
        let message = checkpoint.cosigned_subtree(&self.name, timestamp);
        // FIPS 204 ML-DSA.Verify (Algorithm 3), empty context.
        self.key
            .verify_with_context(&message, &[], &signature)
            .then_some(timestamp)
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
    let note = SignedNote::parse(note).map_err(CheckpointError::Note)?;
    let checkpoint = Checkpoint::parse(&note.text)?;

    // Lines under each key; lines under no key of ours are ignored, as
    // signed-note requires. Two lines under one key are refused.
    let only = |names: &dyn Fn(&SignatureLine) -> bool| {
        let mut lines = note.signatures.iter().filter(|l| names(l));
        match (lines.next(), lines.next()) {
            (_, Some(_)) => Err(CheckpointError::Duplicate),
            (line, None) => Ok(line),
        }
    };
    let ed_line = only(&|l| keys.ed25519.names(l))?;
    let ml_line = only(&|l| keys.mldsa44.names(l))?;

    let ed25519 = ed_line.is_some_and(|l| keys.ed25519.verify(note.text.as_bytes(), &l.signature));
    let timestamp = ml_line.and_then(|l| keys.mldsa44.verify(&checkpoint, &l.signature));

    // The AND rule.
    let both = ed25519 && timestamp.is_some();
    if !both {
        return Err(CheckpointError::Unsigned {
            ed25519,
            mldsa44: timestamp.is_some(),
        });
    }
    Ok(Verified {
        checkpoint,
        timestamp: timestamp.unwrap_or_default(),
    })
}

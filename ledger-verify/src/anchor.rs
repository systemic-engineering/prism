//! The SLH-DSA-SHA2-128s anchor: an offline, hash-only signature over a
//! checkpoint, checked on its own (bullet-up-008, lean 3). If lattices
//! fall, history up to the last anchor still rests on hashes alone.
//!
//! **Provisional format** (C2SP assigns SLH-DSA no signature type): FIPS 205
//! pure SLH-DSA-SHA2-128s over the checkpoint's note text, with the context
//! string [`CONTEXT`].

use crate::checkpoint::Checkpoint;

/// The FIPS 205 context string: domain separation for the anchor key.
pub const CONTEXT: &[u8] = b"prism ledger anchor v1";
/// SLH-DSA-SHA2-128s public key length (FIPS 205, Table 2).
pub const PUBLIC_KEY: usize = 32;
/// SLH-DSA-SHA2-128s signature length (FIPS 205, Table 2).
pub const SIGNATURE: usize = 7856;

/// Why an anchor did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorError {
    /// Not a 32-byte SLH-DSA-SHA2-128s public key.
    Key,
    /// Not a 7856-byte signature.
    Length,
    /// The signature does not verify.
    Signature,
}

/// An SLH-DSA-SHA2-128s anchor key.
#[derive(Debug, Clone)]
pub struct AnchorKey(slh_dsa::VerifyingKey<slh_dsa::Sha2_128s>);

impl AnchorKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<AnchorKey, AnchorError> {
        let _ = bytes;
        Err(AnchorError::Key)
    }

    /// Verify an anchor signature over `checkpoint`.
    pub fn verify(&self, checkpoint: &Checkpoint, signature: &[u8]) -> Result<(), AnchorError> {
        let _ = (checkpoint, signature, &self.0);
        Err(AnchorError::Signature)
    }
}

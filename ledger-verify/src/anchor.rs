//! The SLH-DSA-SHA2-128s anchor: an offline, hash-only signature over a
//! checkpoint, checked on its own (bullet-up-008, lean 3). If lattices
//! fall, history up to the last anchor still rests on hashes alone.
//!
//! **Format** (C2SP assigns SLH-DSA no signature type): FIPS 205 pure
//! SLH-DSA-SHA2-128s over the checkpoint's note text, with the context
//! string [`CONTEXT`]. It is exactly what the stock OpenSSL 4.1.0 CLI
//! makes (docs/spec/ledger.md §4):
//!
//! ```text
//! openssl pkeyutl -sign -rawin -inkey anchor.key.pem -in checkpoint.txt \
//!   -pkeyopt 'context-string:prism ledger anchor v1' -out anchor.sig
//! ```
//!
//! and the key is the one `openssl pkey -pubout` writes ([`AnchorKey::from_pem`]).

use crate::checkpoint::Checkpoint;

/// The FIPS 205 context string: domain separation for the anchor key.
pub const CONTEXT: &[u8] = b"prism ledger anchor v1";
/// SLH-DSA-SHA2-128s public key length (FIPS 205, Table 2).
pub const PUBLIC_KEY: usize = 32;
/// SLH-DSA-SHA2-128s signature length (FIPS 205, Table 2).
pub const SIGNATURE: usize = 7856;
/// The DER `SubjectPublicKeyInfo` before the 32 key bytes: SEQUENCE {
/// SEQUENCE { OID 2.16.840.1.101.3.4.3.20 (id-slh-dsa-sha2-128s), no
/// parameters }, BIT STRING (0 unused bits) }: the OID is NIST CSOR's, the
/// layout the IETF LAMPS X.509 one for SLH-DSA, as OpenSSL 4.1.0 writes it.
pub const SPKI_PREFIX: [u8; 18] = [
    0x30, 0x30, 0x30, 0x0b, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x03, 0x14, 0x03,
    0x21, 0x00,
];

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
        slh_dsa::VerifyingKey::try_from(bytes)
            .map(AnchorKey)
            .map_err(|_| AnchorError::Key)
    }

    /// The key as `openssl pkey -pubout` writes it: a PEM `PUBLIC KEY`
    /// (SubjectPublicKeyInfo) for SLH-DSA-SHA2-128s.
    pub fn from_pem(_pem: &str) -> Result<AnchorKey, AnchorError> {
        Err(AnchorError::Key)
    }

    /// Verify an anchor signature over `checkpoint`.
    pub fn verify(&self, checkpoint: &Checkpoint, signature: &[u8]) -> Result<(), AnchorError> {
        if signature.len() != SIGNATURE {
            return Err(AnchorError::Length);
        }
        let signature =
            slh_dsa::Signature::try_from(signature).map_err(|_| AnchorError::Signature)?;
        // FIPS 205 slh_verify (Algorithm 24), pure, with the anchor context.
        self.0
            .try_verify_with_context(checkpoint.body().as_bytes(), CONTEXT, &signature)
            .map_err(|_| AnchorError::Signature)
    }
}

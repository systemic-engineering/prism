//! SSHSIG (OpenSSH's `PROTOCOL.sshsig`) with the `ssh-mldsa44-ed25519`
//! composite key: what `ssh-keygen -Y sign` makes with OpenSSH 10.6. Alex's
//! seal and the adjudicator's statements are made this way (spectral-003).
//!
//! **The blob** (`PROTOCOL.sshsig` §2), base64 between
//! `-----BEGIN SSH SIGNATURE-----` and `-----END SSH SIGNATURE-----`:
//!
//! ```text
//! byte[6] "SSHSIG"  uint32 1  string publickey  string namespace
//! string reserved   string hash_algorithm       string signature
//! ```
//!
//! **The signed data** (§3; `sshsig.c` rebuilds it with an empty reserved):
//!
//! ```text
//! byte[6] "SSHSIG"  string namespace  string ""  string hash_algorithm
//! string H(message)
//! ```
//!
//! **The composite** (OpenSSH 10.6 `ssh-mldsa-eddsa.c`, after
//! draft-miller-sshm-mldsa44-ed25519-composite-sigs-00 and
//! draft-ietf-lamps-pq-composite-sigs-18):
//!
//! - public key: `string "ssh-mldsa44-ed25519"`, `string (mldsaPK[1312] ||
//!   ed25519PK[32])`;
//! - signature: `string "ssh-mldsa44-ed25519"`, `string (mldsaSig[2420] ||
//!   ed25519Sig[64])`;
//! - both halves sign `M' = "CompositeAlgorithmSignatures2025" ||
//!   "COMPSIG-MLDSA44-Ed25519-SHA512" || 0x00 || SHA-512(data)` (the 0x00
//!   is the empty composite context's length); ML-DSA-44 is FIPS 204 pure
//!   with the context `"COMPSIG-MLDSA44-Ed25519-SHA512"`, Ed25519 is RFC 8032.
//!
//! [`verify`] accepts only if **both** halves verify (the AND rule).

use alloc::vec::Vec;

/// The key and signature algorithm name.
pub const KEY_TYPE: &str = "ssh-mldsa44-ed25519";
/// ML-DSA-44 public key length (FIPS 204, Table 2).
pub const MLDSA44_PUBLIC_KEY: usize = 1312;
/// ML-DSA-44 signature length (FIPS 204, Table 2).
pub const MLDSA44_SIGNATURE: usize = 2420;
/// The composite public key: ML-DSA-44 then Ed25519.
pub const PUBLIC_KEY: usize = MLDSA44_PUBLIC_KEY + 32;
/// The composite signature: ML-DSA-44 then Ed25519.
pub const SIGNATURE: usize = MLDSA44_SIGNATURE + 64;
/// The composite's message prefix.
pub const COMPOSITE_PREFIX: &[u8] = b"CompositeAlgorithmSignatures2025";
/// The composite's label, also the ML-DSA-44 context string.
pub const COMPOSITE_LABEL: &[u8] = b"COMPSIG-MLDSA44-Ed25519-SHA512";

/// Why an SSHSIG did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshsigError {
    /// Not an armored SSH SIGNATURE, or not base64.
    Armor,
    /// The blob or a key does not parse, or has trailing bytes.
    Format,
    /// A version other than 1.
    Version,
    /// A key or signature type other than `ssh-mldsa44-ed25519`.
    KeyType,
    /// Signed by a key other than the allowed one.
    Key,
    /// Another namespace, or an empty one.
    Namespace,
    /// A hash algorithm other than `sha512` or `sha256`.
    HashAlgorithm,
    /// Not both halves verified.
    Signature { ed25519: bool, mldsa44: bool },
}

/// An `ssh-mldsa44-ed25519` public key: the one key a seal may be made by.
#[derive(Debug, Clone)]
pub struct SealKey {
    blob: Vec<u8>,
}

impl SealKey {
    /// A key as `ssh-keygen` writes it in a `.pub` file or an
    /// `allowed_signers` key field: `ssh-mldsa44-ed25519 <base64> [comment]`.
    pub fn from_openssh(_line: &str) -> Result<SealKey, SshsigError> {
        Err(SshsigError::Format)
    }

    /// The SSH wire encoding (RFC 4253 §6.6) of the key.
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }
}

/// Verify an armored SSHSIG over `message` in `namespace`, made by
/// `allowed`: both the ML-DSA-44 and the Ed25519 half must verify.
pub fn verify(
    _armored: &str,
    _namespace: &str,
    _message: &[u8],
    _allowed: &SealKey,
) -> Result<(), SshsigError> {
    Err(SshsigError::Signature {
        ed25519: false,
        mldsa44: false,
    })
}

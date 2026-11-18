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

use alloc::string::String;
use alloc::vec::Vec;

use base64ct::{Base64, Encoding as _};
use sha2::{Digest, Sha256, Sha512};

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
    mldsa44: ml_dsa::VerifyingKey<ml_dsa::MlDsa44>,
    ed25519: ed25519_dalek::VerifyingKey,
}

impl SealKey {
    /// A key as `ssh-keygen` writes it in a `.pub` file or an
    /// `allowed_signers` key field: `ssh-mldsa44-ed25519 <base64> [comment]`.
    pub fn from_openssh(line: &str) -> Result<SealKey, SshsigError> {
        let line = line.strip_suffix('\n').unwrap_or(line);
        let mut fields = line.splitn(3, ' ');
        let (Some(key_type), Some(base64)) = (fields.next(), fields.next()) else {
            return Err(SshsigError::Format);
        };
        let blob = decode_base64(base64).ok_or(SshsigError::Format)?;
        let key = SealKey::from_blob(&blob)?;
        if key_type != KEY_TYPE {
            return Err(SshsigError::KeyType);
        }
        Ok(key)
    }

    /// A key from its SSH wire encoding: `string "ssh-mldsa44-ed25519"`,
    /// `string (mldsaPK || ed25519PK)`, nothing after.
    pub fn from_blob(blob: &[u8]) -> Result<SealKey, SshsigError> {
        let mut r = Reader(blob);
        if r.string()? != KEY_TYPE.as_bytes() {
            return Err(SshsigError::KeyType);
        }
        let pk = r.string()?;
        r.end()?;
        if pk.len() != PUBLIC_KEY {
            return Err(SshsigError::Format);
        }
        let (ml, ed) = pk.split_at(MLDSA44_PUBLIC_KEY);
        let ml = ml_dsa::EncodedVerifyingKey::<ml_dsa::MlDsa44>::try_from(ml)
            .map_err(|_| SshsigError::Format)?;
        let ed: &[u8; 32] = ed.try_into().map_err(|_| SshsigError::Format)?;
        Ok(SealKey {
            blob: blob.to_vec(),
            mldsa44: ml_dsa::VerifyingKey::decode(&ml),
            ed25519: ed25519_dalek::VerifyingKey::from_bytes(ed)
                .map_err(|_| SshsigError::Format)?,
        })
    }

    /// The SSH wire encoding (RFC 4253 §6.6) of the key.
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }
}

/// Verify an armored SSHSIG over `message` in `namespace`, made by
/// `allowed`: both the ML-DSA-44 and the Ed25519 half must verify.
pub fn verify(
    armored: &str,
    namespace: &str,
    message: &[u8],
    allowed: &SealKey,
) -> Result<(), SshsigError> {
    let blob = dearmor(armored)?;
    let mut r = Reader(&blob);
    if r.take(6)? != b"SSHSIG" {
        return Err(SshsigError::Format);
    }
    if r.take(4)? != [0, 0, 0, 1] {
        return Err(SshsigError::Version);
    }
    let publickey = r.string()?;
    let got_namespace = r.string()?;
    let _reserved = r.string()?;
    let hash_algorithm = r.string()?;
    let signature = r.string()?;
    r.end()?;

    // The signer's key must be the allowed one, byte for byte.
    SealKey::from_blob(publickey)?;
    if publickey != allowed.blob.as_slice() {
        return Err(SshsigError::Key);
    }
    if namespace.is_empty() || got_namespace != namespace.as_bytes() {
        return Err(SshsigError::Namespace);
    }
    let digest: Vec<u8> = match hash_algorithm {
        b"sha512" => Sha512::digest(message).to_vec(),
        b"sha256" => Sha256::digest(message).to_vec(),
        _ => return Err(SshsigError::HashAlgorithm),
    };

    let mut s = Reader(signature);
    if s.string()? != KEY_TYPE.as_bytes() {
        return Err(SshsigError::KeyType);
    }
    let composite = s.string()?;
    s.end()?;
    if composite.len() != SIGNATURE {
        return Err(SshsigError::Format);
    }
    let (ml_sig, ed_sig) = composite.split_at(MLDSA44_SIGNATURE);

    // PROTOCOL.sshsig §3, the reserved field empty as sshsig.c signs it.
    let mut data = b"SSHSIG".to_vec();
    put_string(&mut data, namespace.as_bytes());
    put_string(&mut data, b"");
    put_string(&mut data, hash_algorithm);
    put_string(&mut data, &digest);

    // ssh-mldsa-eddsa.c construct_m_prime, with an empty context.
    let mut m = COMPOSITE_PREFIX.to_vec();
    m.extend_from_slice(COMPOSITE_LABEL);
    m.push(0);
    m.extend_from_slice(&Sha512::digest(&data));

    let mldsa44 = ml_dsa::Signature::<ml_dsa::MlDsa44>::try_from(ml_sig).is_ok_and(|sig| {
        allowed
            .mldsa44
            .verify_with_context(&m, COMPOSITE_LABEL, &sig)
    });
    let ed25519 = <&[u8; 64]>::try_from(ed_sig).is_ok_and(|sig| {
        allowed
            .ed25519
            .verify_strict(&m, &ed25519_dalek::Signature::from_bytes(sig))
            .is_ok()
    });

    // The AND rule.
    if ed25519 && mldsa44 {
        Ok(())
    } else {
        Err(SshsigError::Signature { ed25519, mldsa44 })
    }
}

const BEGIN: &str = "-----BEGIN SSH SIGNATURE-----\n";
const END: &str = "\n-----END SSH SIGNATURE-----";

/// PROTOCOL.sshsig §1: the header, base64 lines, the footer (a final
/// newline optional).
fn dearmor(armored: &str) -> Result<Vec<u8>, SshsigError> {
    let armored = armored.strip_suffix('\n').unwrap_or(armored);
    let body = armored
        .strip_prefix(BEGIN)
        .and_then(|rest| rest.strip_suffix(END))
        .ok_or(SshsigError::Armor)?;
    let base64: String = body.split('\n').collect();
    decode_base64(&base64).ok_or(SshsigError::Armor)
}

/// Canonical base64 only: base64ct rejects bad padding and stray bits.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    Base64::decode_vec(text).ok()
}

fn put_string(out: &mut Vec<u8>, bytes: &[u8]) {
    // Every field here is far below 4 GiB: a namespace, a hash name, a
    // digest.
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

/// RFC 4253 §5 reads: `string` is a uint32 length, then the bytes.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], SshsigError> {
        if self.0.len() < n {
            return Err(SshsigError::Format);
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn string(&mut self) -> Result<&'a [u8], SshsigError> {
        let len = self.take(4)?;
        let len = u32::from_be_bytes([len[0], len[1], len[2], len[3]]) as usize;
        self.take(len)
    }
    fn end(&self) -> Result<(), SshsigError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(SshsigError::Format)
        }
    }
}

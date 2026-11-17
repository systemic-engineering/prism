//! C2SP `signed-note`: the text, its signature lines, key IDs and verifier
//! keys (vkeys). Parsing only; the signature checks live with the key types
//! in [`crate::checkpoint`] and [`crate::anchor`].

use alloc::string::String;
use alloc::vec::Vec;

/// Signature type 0x01: Ed25519 over the note text.
pub const ED25519: u8 = 0x01;
/// Signature type 0x06: a timestamped ML-DSA-44 checkpoint cosignature.
pub const MLDSA44_COSIGNATURE: u8 = 0x06;

/// Why a note, a signature line or a vkey did not parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteError {
    /// Not the signed-note shape (text, blank line, signature lines).
    Malformed,
    /// A control character other than newline.
    ControlCharacter,
    /// A key name that is empty or holds a space or `+`.
    KeyName,
    /// Non-canonical or invalid base64.
    Base64,
    /// A signature shorter than its 4-byte key ID.
    ShortSignature,
    /// A vkey whose key ID is not the one its name, type and key give.
    KeyId,
    /// A vkey with no type byte, or a hex key ID that is not 8 lowercase digits.
    Vkey,
}

/// `SHA-256(name ‖ 0x0A ‖ type ‖ public key)[:4]`, big-endian.
pub fn key_id(name: &str, sig_type: u8, public_key: &[u8]) -> u32 {
    let _ = (name, sig_type, public_key);
    0
}

/// A key name: non-empty, no Unicode space, no `+`.
pub fn valid_key_name(name: &str) -> bool {
    let _ = name;
    false
}

/// Canonical standard base64 with padding (RFC 4648 §4 and §3.5).
pub fn decode_base64(text: &str) -> Result<Vec<u8>, NoteError> {
    let _ = text;
    Err(NoteError::Base64)
}

/// Standard base64 with padding.
pub fn encode_base64(bytes: &[u8]) -> String {
    let _ = bytes;
    String::new()
}

/// A verifier key: `name+hex(key ID)+base64(type ‖ public key)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vkey {
    pub name: String,
    pub id: u32,
    pub sig_type: u8,
    pub public_key: Vec<u8>,
}

impl Vkey {
    /// Parse a vkey. The key ID must be the recommended one,
    /// [`key_id`] of its name, type and key.
    pub fn parse(text: &str) -> Result<Vkey, NoteError> {
        let _ = text;
        Err(NoteError::Vkey)
    }
}

/// One signature line: `— name base64(key ID ‖ signature)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureLine {
    pub name: String,
    pub id: u32,
    pub signature: Vec<u8>,
}

/// A signed note: the text (with its final newline) and its signature lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedNote {
    pub text: String,
    pub signatures: Vec<SignatureLine>,
}

impl SignedNote {
    /// Split at the last blank line; parse every line after it as a
    /// signature line.
    pub fn parse(note: &str) -> Result<SignedNote, NoteError> {
        let _ = note;
        Err(NoteError::Malformed)
    }
}

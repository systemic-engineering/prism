//! C2SP `signed-note`: the text, its signature lines, key IDs and verifier
//! keys (vkeys). Parsing only; the signature checks live with the key types
//! in [`crate::checkpoint`] and [`crate::anchor`].

use alloc::string::String;
use alloc::vec::Vec;

use base64ct::{Base64, Encoding};
use sha2::{Digest, Sha256};

/// The em dash (U+2014) and space that open a signature line.
const DASH: &str = "\u{2014} ";

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
    let h = Sha256::new()
        .chain_update(name.as_bytes())
        .chain_update([0x0A, sig_type])
        .chain_update(public_key)
        .finalize();
    u32::from_be_bytes([h[0], h[1], h[2], h[3]])
}

/// A key name: non-empty, no Unicode space, no `+`.
pub fn valid_key_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| c.is_whitespace() || c == '+')
}

/// Canonical standard base64 with padding (RFC 4648 §4 and §3.5).
pub fn decode_base64(text: &str) -> Result<Vec<u8>, NoteError> {
    // base64ct rejects missing padding, stray characters and non-zero
    // trailing bits, so what decodes is canonical. The round trip makes
    // that a checked property here rather than a reading of base64ct.
    let bytes = Base64::decode_vec(text).map_err(|_| NoteError::Base64)?;
    if Base64::encode_string(&bytes) != text {
        return Err(NoteError::Base64);
    }
    Ok(bytes)
}

/// Standard base64 with padding.
pub fn encode_base64(bytes: &[u8]) -> String {
    Base64::encode_string(bytes)
}

/// Signed notes hold no control character other than newline.
fn check_controls(text: &str) -> Result<(), NoteError> {
    if text.chars().any(|c| c < ' ' && c != '\n') {
        return Err(NoteError::ControlCharacter);
    }
    Ok(())
}

/// Eight lowercase hex digits.
fn parse_hex_id(text: &str) -> Option<u32> {
    let lower = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
    if text.len() != 8 || !text.chars().all(lower) {
        return None;
    }
    u32::from_str_radix(text, 16).ok()
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
        let mut parts = text.splitn(3, '+');
        let (Some(name), Some(hex), Some(material)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(NoteError::Vkey);
        };
        if !valid_key_name(name) {
            return Err(NoteError::KeyName);
        }
        let id = parse_hex_id(hex).ok_or(NoteError::Vkey)?;
        let material = decode_base64(material)?;
        let Some((&sig_type, public_key)) = material.split_first() else {
            return Err(NoteError::Vkey);
        };
        if id != key_id(name, sig_type, public_key) {
            return Err(NoteError::KeyId);
        }
        Ok(Vkey {
            name: name.into(),
            id,
            sig_type,
            public_key: public_key.into(),
        })
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
        check_controls(note)?;
        // The text ends in a newline and is followed by a blank line; the
        // last such split separates it from the signatures.
        let at = note.rfind("\n\n").ok_or(NoteError::Malformed)?;
        let (text, signatures) = (&note[..=at], &note[at + 2..]);
        if signatures.is_empty() || !signatures.ends_with('\n') {
            return Err(NoteError::Malformed);
        }
        let signatures = signatures[..signatures.len() - 1]
            .split('\n')
            .map(SignatureLine::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SignedNote {
            text: text.into(),
            signatures,
        })
    }
}

impl SignatureLine {
    /// `— name base64(key ID ‖ signature)`, without its newline.
    fn parse(line: &str) -> Result<SignatureLine, NoteError> {
        let rest = line.strip_prefix(DASH).ok_or(NoteError::Malformed)?;
        let (name, encoded) = rest.split_once(' ').ok_or(NoteError::Malformed)?;
        if !valid_key_name(name) {
            return Err(NoteError::KeyName);
        }
        let bytes = decode_base64(encoded)?;
        if bytes.len() < 4 {
            return Err(NoteError::ShortSignature);
        }
        Ok(SignatureLine {
            name: name.into(),
            id: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            signature: bytes[4..].into(),
        })
    }
}

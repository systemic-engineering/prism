//! Test helpers: decoding, the spec's encodings written out independently
//! of the library, and seeded signers.

use std::sync::OnceLock;

use base64ct::{Base64, Encoding};
use ed25519_dalek::Signer as _;
use ml_dsa::{Keypair as _, MlDsa44};
use sha2::{Digest, Sha256};

pub type Hash = [u8; 32];

pub fn hex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex: {s}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

pub fn hash_hex(s: &str) -> Hash {
    hex(s).try_into().expect("32 bytes")
}

pub fn b64(bytes: &[u8]) -> String {
    Base64::encode_string(bytes)
}

/// A probe's base64 value: 32 bytes, or None (the probe's malformed input).
pub fn b64_hash(s: &str) -> Option<Hash> {
    Base64::decode_vec(s).ok()?.try_into().ok()
}

pub fn sha256(parts: &[&[u8]]) -> Hash {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}

/// signed-note: SHA-256(name || 0x0A || type || key)[:4], big-endian.
pub fn key_id(name: &str, sig_type: u8, key: &[u8]) -> u32 {
    let h = sha256(&[name.as_bytes(), b"\n", &[sig_type], key]);
    u32::from_be_bytes([h[0], h[1], h[2], h[3]])
}

pub fn vkey(name: &str, sig_type: u8, key: &[u8]) -> String {
    let mut material = vec![sig_type];
    material.extend_from_slice(key);
    format!(
        "{name}+{:08x}+{}",
        key_id(name, sig_type, key),
        b64(&material)
    )
}

pub fn line(name: &str, id: u32, signature: &[u8]) -> String {
    let mut bytes = id.to_be_bytes().to_vec();
    bytes.extend_from_slice(signature);
    format!("\u{2014} {name} {}\n", b64(&bytes))
}

pub fn note(text: &str, lines: &[String]) -> String {
    format!("{text}\n{}", lines.concat())
}

/// The checkpoint note text (C2SP tlog-checkpoint).
pub fn body(origin: &str, size: u64, root: &Hash) -> String {
    format!("{origin}\n{size}\n{}\n", b64(root))
}

/// C2SP tlog-cosignature's CosignedSubtree, written from the spec text.
pub fn cosigned_subtree(
    cosigner: &str,
    timestamp: u64,
    origin: &str,
    start: u64,
    end: u64,
    hash: &Hash,
) -> Vec<u8> {
    let mut m = b"subtree/v1\n\0".to_vec();
    assert_eq!(m.len(), 12);
    m.push(cosigner.len() as u8);
    m.extend_from_slice(cosigner.as_bytes());
    m.extend_from_slice(&timestamp.to_be_bytes());
    m.push(origin.len() as u8);
    m.extend_from_slice(origin.as_bytes());
    m.extend_from_slice(&start.to_be_bytes());
    m.extend_from_slice(&end.to_be_bytes());
    m.extend_from_slice(hash);
    m
}

/// C2SP tlog-cosignature's Ed25519 (0x04) signed message, written from the
/// spec text: lines `cosignature/v1`, `time <ASCII decimal>`, origin, size
/// (ASCII decimal), base64(root), each ending in a newline; no extensions.
pub fn cosignature_v1(timestamp: u64, origin: &str, size: u64, root: &Hash) -> String {
    let mut m = String::from("cosignature/v1\n");
    m.push_str("time ");
    m.push_str(&timestamp.to_string());
    m.push('\n');
    m.push_str(origin);
    m.push('\n');
    m.push_str(&size.to_string());
    m.push('\n');
    m.push_str(&b64(root));
    m.push('\n');
    m
}

/// A throwaway Ed25519 key from a fixed seed.
pub struct Ed {
    pub name: String,
    key: ed25519_dalek::SigningKey,
}

impl Ed {
    pub fn new(name: &str, seed: u8) -> Ed {
        Ed {
            name: name.into(),
            key: ed25519_dalek::SigningKey::from_bytes(&[seed; 32]),
        }
    }
    pub fn public(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
    pub fn id(&self) -> u32 {
        key_id(&self.name, 0x01, &self.public())
    }
    pub fn vkey(&self) -> String {
        vkey(&self.name, 0x01, &self.public())
    }
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.key.sign(message).to_bytes().to_vec()
    }
    /// The note signature line over `text`.
    pub fn line(&self, text: &str) -> String {
        line(&self.name, self.id(), &self.sign(text.as_bytes()))
    }
    /// The key ID as a 0x04 cosigner.
    pub fn cosigner_id(&self) -> u32 {
        key_id(&self.name, 0x04, &self.public())
    }
    /// The vkey as a 0x04 cosigner.
    pub fn cosigner_vkey(&self) -> String {
        vkey(&self.name, 0x04, &self.public())
    }
    /// timestamp || Ed25519 over `cosignature_v1`.
    pub fn cosignature(&self, origin: &str, size: u64, root: &Hash, timestamp: u64) -> Vec<u8> {
        let m = cosignature_v1(timestamp, origin, size, root);
        let mut out = timestamp.to_be_bytes().to_vec();
        out.extend_from_slice(&self.sign(m.as_bytes()));
        out
    }
    pub fn cosignature_line(&self, origin: &str, size: u64, root: &Hash, timestamp: u64) -> String {
        line(
            &self.name,
            self.cosigner_id(),
            &self.cosignature(origin, size, root, timestamp),
        )
    }
}

/// A witness: an Ed25519 (0x04) and an ML-DSA-44 (0x06) cosigner, one name.
pub struct Witness {
    pub ed: Ed,
    pub ml: Ml,
}

impl Witness {
    pub fn new(name: &str, ed_seed: u8, ml_seed: u8) -> Witness {
        Witness {
            ed: Ed::new(name, ed_seed),
            ml: Ml::new(name, ml_seed),
        }
    }
}

/// A throwaway ML-DSA-44 key from a fixed seed.
pub struct Ml {
    pub name: String,
    key: ml_dsa::SigningKey<MlDsa44>,
}

impl Ml {
    pub fn new(name: &str, seed: u8) -> Ml {
        Ml {
            name: name.into(),
            key: ml_dsa::SigningKey::<MlDsa44>::from_seed(&[seed; 32].into()),
        }
    }
    pub fn public(&self) -> Vec<u8> {
        self.key.verifying_key().encode().to_vec()
    }
    pub fn id(&self) -> u32 {
        key_id(&self.name, 0x06, &self.public())
    }
    pub fn vkey(&self) -> String {
        vkey(&self.name, 0x06, &self.public())
    }
    /// FIPS 204 ML-DSA-44, deterministic, empty context.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.key.sign(message).encode().to_vec()
    }
    /// timestamp || signature over the checkpoint's CosignedSubtree.
    pub fn cosignature(&self, origin: &str, size: u64, root: &Hash, timestamp: u64) -> Vec<u8> {
        let m = cosigned_subtree(&self.name, timestamp, origin, 0, size, root);
        let mut out = timestamp.to_be_bytes().to_vec();
        out.extend_from_slice(&self.sign(&m));
        out
    }
    pub fn line(&self, origin: &str, size: u64, root: &Hash, timestamp: u64) -> String {
        line(
            &self.name,
            self.id(),
            &self.cosignature(origin, size, root, timestamp),
        )
    }
}

pub const ORIGIN: &str = "example.com/prism-ledger";

/// The log under test: RFC 6962's eight test leaves.
pub fn tree() -> (u64, Hash) {
    (
        8,
        hash_hex("5dc9da79a70659a9ad559cb701ded9a2ab9d823aad2f4960cfe370eff4604328"),
    )
}

/// The log's own two keys.
pub fn log_keys() -> &'static (Ed, Ml) {
    static KEYS: OnceLock<(Ed, Ml)> = OnceLock::new();
    KEYS.get_or_init(|| (Ed::new(ORIGIN, 1), Ml::new(ORIGIN, 2)))
}

/// Seven witnesses: Ed25519 seeds 30..37, ML-DSA-44 seeds 10..17.
pub fn witnesses() -> &'static Vec<Witness> {
    static W: OnceLock<Vec<Witness>> = OnceLock::new();
    W.get_or_init(|| {
        (0..7)
            .map(|i| Witness::new(&format!("witness{i}.example.com"), 30 + i, 10 + i))
            .collect()
    })
}

/// Flip one bit.
pub fn flip(bytes: &[u8], at: usize) -> Vec<u8> {
    let mut b = bytes.to_vec();
    b[at] ^= 0x01;
    b
}

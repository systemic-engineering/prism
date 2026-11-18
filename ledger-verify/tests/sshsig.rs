//! SSHSIG with `ssh-mldsa44-ed25519`, against `ssh-keygen -Y sign` (OpenSSH
//! 10.6p1) with throwaway keys, deleted after. Made with:
//!   ssh-keygen -t mldsa44-ed25519 -f seal -N ''   (and other)
//!   ssh-keygen -t ed25519 -f classic -N ''
//!   ssh-keygen -Y sign -n spectral-seal -f seal seal-msg.txt
//!   ssh-keygen -Y sign -n other-namespace -f seal seal-msg.txt
//!   ssh-keygen -Y sign -n spectral-seal -O hashalg=sha256 -f seal seal-msg.txt
//!   ssh-keygen -Y sign -n spectral-seal -f classic seal-msg.txt
//! Each `.sig` was checked with `ssh-keygen -Y verify` before check-in.

use base64ct::{Base64, Encoding as _};
use ledger_verify::sshsig::{
    verify, SealKey, SshsigError, COMPOSITE_LABEL, COMPOSITE_PREFIX, KEY_TYPE,
    MLDSA44_PUBLIC_KEY, MLDSA44_SIGNATURE, PUBLIC_KEY, SIGNATURE,
};
use sha2::{Digest, Sha512};

use crate::common::flip;

const SEAL_PUB: &str = include_str!("../../docs/spec/seal-vectors/seal.pub");
const OTHER_PUB: &str = include_str!("../../docs/spec/seal-vectors/other.pub");
const CLASSIC_PUB: &str = include_str!("../../docs/spec/seal-vectors/classic.pub");
const MSG: &[u8] = include_bytes!("../../docs/spec/seal-vectors/seal-msg.txt");
const SIG: &str = include_str!("../../docs/spec/seal-vectors/seal-msg.spectral-seal.sig");
const SIG_OTHER_NS: &str =
    include_str!("../../docs/spec/seal-vectors/seal-msg.other-namespace.sig");
const SIG_SHA256: &str = include_str!("../../docs/spec/seal-vectors/seal-msg.sha256.sig");
const SIG_CLASSIC: &str =
    include_str!("../../docs/spec/seal-vectors/seal-msg.classic-ed25519.sig");

const NS: &str = "spectral-seal";

fn seal() -> SealKey {
    SealKey::from_openssh(SEAL_PUB).expect("ssh-keygen's .pub parses")
}

// ── The blob, read and rewritten independently of the library ──

const BEGIN: &str = "-----BEGIN SSH SIGNATURE-----\n";
const END: &str = "-----END SSH SIGNATURE-----\n";

fn dearmor(armored: &str) -> Vec<u8> {
    let body = armored.strip_prefix(BEGIN).unwrap().strip_suffix(END).unwrap();
    Base64::decode_vec(&body.replace('\n', "")).unwrap()
}

/// As ssh-keygen armors: 70 columns.
fn armor(blob: &[u8]) -> String {
    let b64 = Base64::encode_string(blob);
    let mut out = String::from(BEGIN);
    for chunk in b64.as_bytes().chunks(70) {
        out.push_str(std::str::from_utf8(chunk).unwrap());
        out.push('\n');
    }
    out.push_str(END);
    out
}

fn string(bytes: &[u8]) -> Vec<u8> {
    let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(bytes);
    out
}

fn read<'a>(bytes: &mut &'a [u8]) -> &'a [u8] {
    let len = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
    let (field, rest) = bytes[4..].split_at(len);
    *bytes = rest;
    field
}

/// The blob's fields: version, publickey, namespace, reserved, hash, signature.
struct Blob {
    version: u32,
    publickey: Vec<u8>,
    namespace: Vec<u8>,
    reserved: Vec<u8>,
    hash: Vec<u8>,
    signature: Vec<u8>,
}

impl Blob {
    fn parse(blob: &[u8]) -> Blob {
        assert_eq!(&blob[..6], b"SSHSIG");
        let version = u32::from_be_bytes(blob[6..10].try_into().unwrap());
        let mut rest = &blob[10..];
        let b = Blob {
            version,
            publickey: read(&mut rest).to_vec(),
            namespace: read(&mut rest).to_vec(),
            reserved: read(&mut rest).to_vec(),
            hash: read(&mut rest).to_vec(),
            signature: read(&mut rest).to_vec(),
        };
        assert!(rest.is_empty());
        b
    }
    fn encode(&self) -> Vec<u8> {
        let mut out = b"SSHSIG".to_vec();
        out.extend_from_slice(&self.version.to_be_bytes());
        for f in [
            &self.publickey,
            &self.namespace,
            &self.reserved,
            &self.hash,
            &self.signature,
        ] {
            out.extend_from_slice(&string(f));
        }
        out
    }
    fn armored(&self) -> String {
        armor(&self.encode())
    }
}

fn blob() -> Blob {
    Blob::parse(&dearmor(SIG))
}

/// The composite signature bytes, mldsaSig || ed25519Sig.
fn composite(signature_field: &[u8]) -> Vec<u8> {
    let mut rest = signature_field;
    assert_eq!(read(&mut rest), KEY_TYPE.as_bytes());
    let sig = read(&mut rest).to_vec();
    assert!(rest.is_empty());
    sig
}

fn with_composite(sig: &[u8]) -> Vec<u8> {
    let mut out = string(KEY_TYPE.as_bytes());
    out.extend_from_slice(&string(sig));
    out
}

/// PROTOCOL.sshsig §3 and ssh-mldsa-eddsa.c's construct_m_prime, from
/// their text.
fn m_prime(namespace: &str, hash: &str, message: &[u8]) -> Vec<u8> {
    let h: Vec<u8> = match hash {
        "sha512" => Sha512::digest(message).to_vec(),
        "sha256" => sha2::Sha256::digest(message).to_vec(),
        _ => unreachable!(),
    };
    let mut data = b"SSHSIG".to_vec();
    data.extend_from_slice(&string(namespace.as_bytes()));
    data.extend_from_slice(&string(b""));
    data.extend_from_slice(&string(hash.as_bytes()));
    data.extend_from_slice(&string(&h));
    let mut m = b"CompositeAlgorithmSignatures2025".to_vec();
    m.extend_from_slice(b"COMPSIG-MLDSA44-Ed25519-SHA512");
    m.push(0);
    m.extend_from_slice(&Sha512::digest(&data));
    m
}

// ── The format, pinned against the tool's own output ──

#[test]
fn the_composite_encoding_is_what_ssh_keygen_wrote() {
    assert_eq!(COMPOSITE_PREFIX, b"CompositeAlgorithmSignatures2025");
    assert_eq!(COMPOSITE_LABEL, b"COMPSIG-MLDSA44-Ed25519-SHA512");
    assert_eq!((PUBLIC_KEY, SIGNATURE), (1344, 2484));

    let b = blob();
    assert_eq!(b.version, 1);
    assert_eq!(b.namespace, NS.as_bytes());
    assert_eq!(b.reserved, b"");
    assert_eq!(b.hash, b"sha512");
    assert_eq!(b.publickey, seal().blob());
    let mut pk = b.publickey.as_slice();
    assert_eq!(read(&mut pk), KEY_TYPE.as_bytes());
    let pk = read(&mut pk).to_vec();
    assert_eq!(pk.len(), PUBLIC_KEY);
    let sig = composite(&b.signature);
    assert_eq!(sig.len(), SIGNATURE);

    // Each half, verified directly against M' as the source builds it:
    // ML-DSA-44 first with the label as FIPS 204 context, Ed25519 last.
    let m = m_prime(NS, "sha512", MSG);
    let ml = ml_dsa::VerifyingKey::<ml_dsa::MlDsa44>::decode(
        &ml_dsa::EncodedVerifyingKey::<ml_dsa::MlDsa44>::try_from(&pk[..MLDSA44_PUBLIC_KEY])
            .unwrap(),
    );
    let ml_sig =
        ml_dsa::Signature::<ml_dsa::MlDsa44>::try_from(&sig[..MLDSA44_SIGNATURE]).unwrap();
    assert!(ml.verify_with_context(&m, COMPOSITE_LABEL, &ml_sig));
    assert!(!ml.verify_with_context(&m, &[], &ml_sig));
    let ed = ed25519_dalek::VerifyingKey::from_bytes(
        pk[MLDSA44_PUBLIC_KEY..].try_into().unwrap(),
    )
    .unwrap();
    let ed_sig = ed25519_dalek::Signature::from_bytes(
        sig[MLDSA44_SIGNATURE..].try_into().unwrap(),
    );
    assert!(ed.verify_strict(&m, &ed_sig).is_ok());
}

// ── The real vectors ──

#[test]
fn a_seal_made_by_ssh_keygen_verifies() {
    assert_eq!(verify(SIG, NS, MSG, &seal()), Ok(()));
}

#[test]
fn a_seal_with_sha256_verifies() {
    assert_eq!(Blob::parse(&dearmor(SIG_SHA256)).hash, b"sha256");
    assert_eq!(verify(SIG_SHA256, NS, MSG, &seal()), Ok(()));
}

#[test]
fn another_namespace_rejects() {
    assert_eq!(
        verify(SIG, "other-namespace", MSG, &seal()),
        Err(SshsigError::Namespace)
    );
    assert_eq!(
        verify(SIG_OTHER_NS, NS, MSG, &seal()),
        Err(SshsigError::Namespace)
    );
    assert_eq!(verify(SIG_OTHER_NS, "other-namespace", MSG, &seal()), Ok(()));
    assert_eq!(verify(SIG, "", MSG, &seal()), Err(SshsigError::Namespace));
    // The namespace is signed: relabelling the blob does not move it.
    let mut b = Blob::parse(&dearmor(SIG_OTHER_NS));
    b.namespace = NS.as_bytes().to_vec();
    assert_eq!(
        verify(&b.armored(), NS, MSG, &seal()),
        Err(SshsigError::Signature {
            ed25519: false,
            mldsa44: false
        })
    );
}

#[test]
fn another_message_rejects() {
    for message in [&flip(MSG, 0)[..], &MSG[..MSG.len() - 1], b""] {
        assert_eq!(
            verify(SIG, NS, message, &seal()),
            Err(SshsigError::Signature {
                ed25519: false,
                mldsa44: false
            })
        );
    }
}

#[test]
fn another_allowed_key_rejects() {
    let other = SealKey::from_openssh(OTHER_PUB).expect("parses");
    assert_eq!(verify(SIG, NS, MSG, &other), Err(SshsigError::Key));
    // Swapping the blob's key for the allowed one does not verify either.
    let mut b = blob();
    b.publickey = other.blob().to_vec();
    assert_eq!(
        verify(&b.armored(), NS, MSG, &other),
        Err(SshsigError::Signature {
            ed25519: false,
            mldsa44: false
        })
    );
}

// ── The AND rule ──

#[test]
fn corrupting_the_ed25519_half_only_rejects() {
    let sig = composite(&blob().signature);
    for at in [MLDSA44_SIGNATURE, MLDSA44_SIGNATURE + 31, SIGNATURE - 40] {
        let mut b = blob();
        b.signature = with_composite(&flip(&sig, at));
        assert_eq!(
            verify(&b.armored(), NS, MSG, &seal()),
            Err(SshsigError::Signature {
                ed25519: false,
                mldsa44: true
            }),
            "bit flipped at {at}"
        );
    }
}

#[test]
fn corrupting_the_mldsa44_half_only_rejects() {
    let sig = composite(&blob().signature);
    for at in [0, 1000, MLDSA44_SIGNATURE - 100] {
        let mut b = blob();
        b.signature = with_composite(&flip(&sig, at));
        assert_eq!(
            verify(&b.armored(), NS, MSG, &seal()),
            Err(SshsigError::Signature {
                ed25519: true,
                mldsa44: false
            }),
            "bit flipped at {at}"
        );
    }
}

#[test]
fn swapping_in_the_ed25519_half_of_another_signature_rejects() {
    // The sha256 seal's Ed25519 half is valid over another M'.
    let sig = composite(&blob().signature);
    let other = composite(&Blob::parse(&dearmor(SIG_SHA256)).signature);
    let mut mixed = sig[..MLDSA44_SIGNATURE].to_vec();
    mixed.extend_from_slice(&other[MLDSA44_SIGNATURE..]);
    let mut b = blob();
    b.signature = with_composite(&mixed);
    assert_eq!(
        verify(&b.armored(), NS, MSG, &seal()),
        Err(SshsigError::Signature {
            ed25519: false,
            mldsa44: true
        })
    );
}

// ── Key types ──

#[test]
fn a_classic_ed25519_sshsig_is_refused() {
    assert_eq!(
        SealKey::from_openssh(CLASSIC_PUB).err(),
        Some(SshsigError::KeyType)
    );
    assert_eq!(
        verify(SIG_CLASSIC, NS, MSG, &seal()),
        Err(SshsigError::KeyType)
    );
}

#[test]
fn the_seal_key_parses_as_ssh_keygen_writes_it() {
    let key = seal();
    let fields: Vec<&str> = SEAL_PUB.trim_end().splitn(3, ' ').collect();
    assert_eq!(fields[0], KEY_TYPE);
    assert_eq!(key.blob(), Base64::decode_vec(fields[1]).unwrap());
    // Without a comment, and as an allowed_signers key field.
    let bare = format!("{} {}", fields[0], fields[1]);
    assert_eq!(SealKey::from_openssh(&bare).unwrap().blob(), key.blob());
    // The text type and the blob's type must agree.
    let lie = format!("ssh-ed25519 {}", fields[1]);
    assert_eq!(
        SealKey::from_openssh(&lie).err(),
        Some(SshsigError::KeyType)
    );
    // A truncated or extended blob, bad base64, nothing.
    let raw = Base64::decode_vec(fields[1]).unwrap();
    let short = format!("{KEY_TYPE} {}", Base64::encode_string(&raw[..raw.len() - 1]));
    let mut longer = raw.clone();
    longer.push(0);
    let long = format!("{KEY_TYPE} {}", Base64::encode_string(&longer));
    let mut wrong_len = raw.clone();
    wrong_len[4 + KEY_TYPE.len() + 3] -= 1; // the key string claims 1343
    let wrong_len = format!("{KEY_TYPE} {}", Base64::encode_string(&wrong_len[..raw.len() - 1]));
    for bad in [
        short,
        long,
        wrong_len,
        format!("{KEY_TYPE} !{}", &fields[1][1..]),
        KEY_TYPE.to_string(),
        String::new(),
    ] {
        assert_eq!(
            SealKey::from_openssh(&bad).err(),
            Some(SshsigError::Format),
            "{bad:.60}"
        );
    }
}

// ── The blob's other fields ──

#[test]
fn a_malformed_blob_rejects() {
    let check = |b: &Blob, want: SshsigError, what: &str| {
        assert_eq!(verify(&b.armored(), NS, MSG, &seal()), Err(want), "{what}");
    };
    let mut b = blob();
    b.version = 2;
    check(&b, SshsigError::Version, "version 2");
    b.version = 0;
    check(&b, SshsigError::Version, "version 0");

    for hash in [&b"sha384"[..], b"SHA512", b""] {
        let mut b = blob();
        b.hash = hash.to_vec();
        check(&b, SshsigError::HashAlgorithm, "hash");
    }

    let mut b = blob();
    b.signature = {
        let mut s = string(b"ssh-ed25519");
        s.extend_from_slice(&string(&composite(&blob().signature)));
        s
    };
    check(&b, SshsigError::KeyType, "signature type");

    let mut b = blob();
    let sig = composite(&b.signature);
    b.signature = with_composite(&sig[..SIGNATURE - 1]);
    check(&b, SshsigError::Format, "short signature");
    let mut b = blob();
    b.signature.push(0);
    check(&b, SshsigError::Format, "trailing byte in the signature");

    let mut b = blob();
    b.publickey.push(0);
    check(&b, SshsigError::Format, "trailing byte in the key");

    // Trailing bytes after the blob, a bad preamble, a truncated blob.
    let mut bytes = blob().encode();
    bytes.push(0);
    assert_eq!(verify(&armor(&bytes), NS, MSG, &seal()), Err(SshsigError::Format));
    let mut bytes = blob().encode();
    bytes[0] = b's';
    assert_eq!(verify(&armor(&bytes), NS, MSG, &seal()), Err(SshsigError::Format));
    let bytes = blob().encode();
    for cut in [3, 8, 20, bytes.len() - 1] {
        assert_eq!(
            verify(&armor(&bytes[..cut]), NS, MSG, &seal()),
            Err(SshsigError::Format),
            "cut at {cut}"
        );
    }
}

#[test]
fn a_reserved_value_is_ignored_as_openssh_does() {
    // PROTOCOL.sshsig: "Implementations should ignore the reserved field
    // if it is not empty"; sshsig.c signs and verifies with it empty.
    let mut b = blob();
    b.reserved = b"tag".to_vec();
    assert_eq!(verify(&b.armored(), NS, MSG, &seal()), Ok(()));
}

#[test]
fn the_armor_is_required() {
    let body = SIG.strip_prefix(BEGIN).unwrap();
    let no_newline = SIG.strip_suffix('\n').unwrap();
    assert_eq!(verify(no_newline, NS, MSG, &seal()), Ok(()));
    for bad in [
        body.to_string(),
        SIG.replace("END SSH SIGNATURE", "END SSH SIGNATUR"),
        SIG.replace("SSH SIGNATURE", "PGP SIGNATURE"),
        SIG.replacen("U1NI", "U1N!", 1),
        String::new(),
    ] {
        assert_eq!(
            verify(&bad, NS, MSG, &seal()),
            Err(SshsigError::Armor),
            "{bad:.40}"
        );
    }
}

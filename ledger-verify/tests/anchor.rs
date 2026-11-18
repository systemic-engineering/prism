//! The SLH-DSA-SHA2-128s anchor, checked on its own.

use std::sync::OnceLock;

use base64ct::Encoding as _;
use ledger_verify::anchor::{AnchorError, AnchorKey, CONTEXT, PUBLIC_KEY, SIGNATURE, SPKI_PREFIX};
use ledger_verify::checkpoint::Checkpoint;
use slh_dsa::{Sha2_128s, SigningKey};

use crate::common::{flip, tree, ORIGIN};

fn signer() -> &'static SigningKey<Sha2_128s> {
    static KEY: OnceLock<SigningKey<Sha2_128s>> = OnceLock::new();
    KEY.get_or_init(|| SigningKey::slh_keygen_internal(&[3; 16], &[4; 16], &[5; 16]))
}

fn checkpoint() -> Checkpoint {
    let (size, root) = tree();
    Checkpoint {
        origin: ORIGIN.into(),
        size,
        root,
    }
}

/// Signing 128s is slow; once per run.
fn signature() -> &'static Vec<u8> {
    static SIG: OnceLock<Vec<u8>> = OnceLock::new();
    SIG.get_or_init(|| {
        signer()
            .try_sign_with_context(checkpoint().body().as_bytes(), CONTEXT, None)
            .unwrap()
            .to_vec()
    })
}

fn key() -> AnchorKey {
    AnchorKey::from_bytes(&signer().as_ref().to_vec()).unwrap()
}

#[test]
fn the_sizes_are_fips_205_s() {
    assert_eq!(signer().as_ref().to_vec().len(), PUBLIC_KEY);
    assert_eq!(signature().len(), SIGNATURE);
}

#[test]
fn an_anchor_over_the_checkpoint_verifies() {
    assert_eq!(key().verify(&checkpoint(), signature()), Ok(()));
}

#[test]
fn an_anchor_over_another_checkpoint_rejects() {
    let mut other = checkpoint();
    other.size += 1;
    assert_eq!(
        key().verify(&other, signature()),
        Err(AnchorError::Signature)
    );
}

#[test]
fn a_forged_anchor_rejects() {
    for at in [0, 16, 3000, SIGNATURE - 1] {
        assert_eq!(
            key().verify(&checkpoint(), &flip(signature(), at)),
            Err(AnchorError::Signature),
            "bit flipped at {at}"
        );
    }
    assert_eq!(
        key().verify(&checkpoint(), &signature()[..SIGNATURE - 1]),
        Err(AnchorError::Length)
    );
}

#[test]
fn an_anchor_under_another_key_rejects() {
    let other = SigningKey::<Sha2_128s>::slh_keygen_internal(&[6; 16], &[7; 16], &[8; 16]);
    let key = AnchorKey::from_bytes(&other.as_ref().to_vec()).unwrap();
    assert_eq!(
        key.verify(&checkpoint(), signature()),
        Err(AnchorError::Signature)
    );
    assert_eq!(
        AnchorKey::from_bytes(&[0; 31]).err(),
        Some(AnchorError::Key)
    );
}

// The stock tool: OpenSSL 4.1.0's CLI, a throwaway key (deleted after). Made
// with the commands in docs/spec/ledger.md §4:
//   openssl genpkey -algorithm SLH-DSA-SHA2-128s -out anchor.key.pem
//   openssl pkey -in anchor.key.pem -pubout -out anchor.pub.pem
//   openssl pkeyutl -sign -rawin -inkey anchor.key.pem -in anchor-body.txt \
//     -pkeyopt 'context-string:prism ledger anchor v1' -out anchor.sig
//   openssl pkeyutl -sign -rawin -inkey anchor.key.pem -in anchor-body.txt \
//     -out anchor-empty-context.sig

const OPENSSL_PUB: &str = include_str!("../../docs/spec/seal-vectors/anchor.pub.pem");
const OPENSSL_BODY: &str = include_str!("../../docs/spec/seal-vectors/anchor-body.txt");
const OPENSSL_SIG: &[u8] = include_bytes!("../../docs/spec/seal-vectors/anchor.sig");
const OPENSSL_SIG_EMPTY: &[u8] =
    include_bytes!("../../docs/spec/seal-vectors/anchor-empty-context.sig");

fn openssl_key() -> AnchorKey {
    AnchorKey::from_pem(OPENSSL_PUB).expect("openssl's PUBLIC KEY parses")
}

#[test]
fn the_openssl_vector_is_over_the_test_checkpoint() {
    assert_eq!(OPENSSL_BODY, checkpoint().body());
    assert_eq!(Checkpoint::parse(OPENSSL_BODY), Ok(checkpoint()));
    assert_eq!(OPENSSL_SIG.len(), SIGNATURE);
}

#[test]
fn an_anchor_made_by_the_openssl_cli_verifies() {
    assert_eq!(openssl_key().verify(&checkpoint(), OPENSSL_SIG), Ok(()));
}

#[test]
fn an_openssl_anchor_without_the_context_rejects() {
    // `-rawin` without `-pkeyopt context-string:...` signs with the empty
    // context: a valid FIPS 205 signature, but not an anchor.
    assert_eq!(
        openssl_key().verify(&checkpoint(), OPENSSL_SIG_EMPTY),
        Err(AnchorError::Signature)
    );
}

#[test]
fn a_flipped_bit_in_the_openssl_anchor_rejects() {
    for at in [0, 15, 16, 4000, SIGNATURE - 1] {
        assert_eq!(
            openssl_key().verify(&checkpoint(), &flip(OPENSSL_SIG, at)),
            Err(AnchorError::Signature),
            "bit flipped at {at}"
        );
    }
    let mut other = checkpoint();
    other.size += 1;
    assert_eq!(
        openssl_key().verify(&other, OPENSSL_SIG),
        Err(AnchorError::Signature)
    );
}

#[test]
fn an_openssl_anchor_under_another_key_rejects() {
    assert_eq!(
        key().verify(&checkpoint(), OPENSSL_SIG),
        Err(AnchorError::Signature)
    );
}

/// PEM-armor DER as `openssl pkey -pubout` does: 64 columns.
fn pem(der: &[u8]) -> String {
    let b64 = crate::common::b64(der);
    let mut out = String::from("-----BEGIN PUBLIC KEY-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap());
        out.push('\n');
    }
    out.push_str("-----END PUBLIC KEY-----\n");
    out
}

#[test]
fn the_pem_key_is_the_spki_of_the_raw_key() {
    let raw_key = signer().as_ref().to_vec();
    let mut der = SPKI_PREFIX.to_vec();
    der.extend_from_slice(&raw_key);
    let key = AnchorKey::from_pem(&pem(&der)).expect("parses");
    assert_eq!(key.verify(&checkpoint(), signature()), Ok(()));
    // Our encoder agrees with openssl's byte for byte on its key.
    let openssl = openssl_key();
    let lines: String = OPENSSL_PUB
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect();
    let der = base64ct::Base64::decode_vec(&lines).unwrap();
    assert_eq!(der.len(), SPKI_PREFIX.len() + PUBLIC_KEY);
    assert_eq!(&der[..SPKI_PREFIX.len()], &SPKI_PREFIX);
    assert_eq!(pem(&der), OPENSSL_PUB);
    assert_eq!(
        AnchorKey::from_bytes(&der[SPKI_PREFIX.len()..])
            .unwrap()
            .verify(&checkpoint(), OPENSSL_SIG),
        openssl.verify(&checkpoint(), OPENSSL_SIG)
    );
}

#[test]
fn a_malformed_pem_key_rejects() {
    let raw_key = signer().as_ref().to_vec();
    let mut der = SPKI_PREFIX.to_vec();
    der.extend_from_slice(&raw_key);
    // Another parameter set's OID (SLH-DSA-SHA2-128f, ...3.21).
    let mut f = der.clone();
    f[14] = 0x15;
    // A truncated key, a trailing byte, a non-zero unused-bits count.
    let short = der[..der.len() - 1].to_vec();
    let mut long = der.clone();
    long.push(0);
    let mut unused = der.clone();
    unused[17] = 1;
    for (what, bad) in [
        ("128f oid", pem(&f)),
        ("short", pem(&short)),
        ("long", pem(&long)),
        ("unused bits", pem(&unused)),
        ("private", pem(&der).replace("PUBLIC", "PRIVATE")),
        (
            "no footer",
            pem(&der).replace("-----END PUBLIC KEY-----\n", ""),
        ),
        ("not base64", pem(&der).replacen("MDA", "M!A", 1)),
        ("empty", String::new()),
    ] {
        assert_eq!(
            AnchorKey::from_pem(&bad).err(),
            Some(AnchorError::Key),
            "{what}"
        );
    }
}

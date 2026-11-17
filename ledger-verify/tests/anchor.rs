//! The SLH-DSA-SHA2-128s anchor, checked on its own.

use std::sync::OnceLock;

use ledger_verify::anchor::{AnchorError, AnchorKey, CONTEXT, PUBLIC_KEY, SIGNATURE};
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

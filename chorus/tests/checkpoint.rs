//! The hybrid AND rule: a checkpoint verifies only if Ed25519 *and*
//! ML-DSA-44 both verify, under the given keys.

use chorus::checkpoint::{
    verify, Checkpoint, CheckpointError, Ed25519Key, LogKeys, MlDsaKey, Verified, MLDSA44_SIGNATURE,
};
use chorus::note::{NoteError, Vkey};

use crate::common::{body, cosigned_subtree, flip, line, log_keys, note, tree, Ed, Ml, ORIGIN};

const TIME: u64 = 1_763_400_000;

fn keys() -> LogKeys {
    let (ed, ml) = log_keys();
    LogKeys {
        ed25519: Ed25519Key::from_vkey(&Vkey::parse(&ed.vkey()).unwrap()).unwrap(),
        mldsa44: MlDsaKey::from_vkey(&Vkey::parse(&ml.vkey()).unwrap()).unwrap(),
    }
}

fn text() -> String {
    let (size, root) = tree();
    body(ORIGIN, size, &root)
}

fn ed_line() -> String {
    log_keys().0.line(&text())
}

fn ml_line() -> String {
    let (size, root) = tree();
    log_keys().1.line(ORIGIN, size, &root, TIME)
}

fn checkpoint() -> Checkpoint {
    let (size, root) = tree();
    Checkpoint {
        origin: ORIGIN.into(),
        size,
        root,
    }
}

// ── The body ───────────────────────────────────────────────────────────────────────────

#[test]
fn the_body_is_origin_size_root() {
    assert_eq!(
        checkpoint().body(),
        "example.com/prism-ledger\n8\nXcnaeacGWamtVZy3Ad7ZoqudgjqtL0lgz+Nw7/RgQyg=\n"
    );
    assert_eq!(Checkpoint::parse(&checkpoint().body()), Ok(checkpoint()));
}

#[test]
fn bodies_that_are_not_checkpoints_are_refused() {
    let root = "XcnaeacGWamtVZy3Ad7ZoqudgjqtL0lgz+Nw7/RgQyg=";
    for bad in [
        format!("{ORIGIN}\n8\n"),
        format!("{ORIGIN}\n08\n{root}\n"),
        format!("{ORIGIN}\n-8\n{root}\n"),
        format!("{ORIGIN}\n8\n{root}"),
        format!("\n8\n{root}\n"),
        format!("{ORIGIN}\n8\nAAAA\n"),
        format!("{ORIGIN}\n18446744073709551616\n{root}\n"),
    ] {
        assert_eq!(
            Checkpoint::parse(&bad),
            Err(CheckpointError::Body),
            "{bad:?}"
        );
    }
    assert_eq!(
        Checkpoint::parse(&format!("{ORIGIN}\n8\n{root}\nextension\n")),
        Err(CheckpointError::Extensions)
    );
    assert_eq!(
        Checkpoint::parse("0\n0\n47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=\n").map(|c| c.size),
        Ok(0)
    );
}

#[test]
fn the_cosigned_subtree_is_c2sp_s() {
    let (size, root) = tree();
    assert_eq!(
        checkpoint().cosigned_subtree("w.example", TIME),
        cosigned_subtree("w.example", TIME, ORIGIN, 0, size, &root)
    );
}

// ── AND ─────────────────────────────────────────────────────────────────────────────────

#[test]
fn both_valid_accepts() {
    let signed = note(&text(), &[ed_line(), ml_line()]);
    assert_eq!(
        verify(&signed, &keys()),
        Ok(Verified {
            checkpoint: checkpoint(),
            timestamp: TIME
        })
    );
    // Either order.
    let swapped = note(&text(), &[ml_line(), ed_line()]);
    assert!(verify(&swapped, &keys()).is_ok());
}

#[test]
fn ed25519_alone_never_verifies() {
    let signed = note(&text(), &[ed_line()]);
    assert_eq!(
        verify(&signed, &keys()),
        Err(CheckpointError::Unsigned {
            ed25519: true,
            mldsa44: false
        })
    );
}

#[test]
fn mldsa44_alone_never_verifies() {
    let signed = note(&text(), &[ml_line()]);
    assert_eq!(
        verify(&signed, &keys()),
        Err(CheckpointError::Unsigned {
            ed25519: false,
            mldsa44: true
        })
    );
}

#[test]
fn a_forged_mldsa44_beside_a_valid_ed25519_rejects() {
    let (size, root) = tree();
    let (_, ml) = log_keys();
    let good = ml.cosignature(ORIGIN, size, &root, TIME);
    // One bit of the signature, then one bit of the timestamp.
    for forged in [flip(&good, 100), flip(&good, 7)] {
        let signed = note(&text(), &[ed_line(), line(&ml.name, ml.id(), &forged)]);
        assert_eq!(
            verify(&signed, &keys()),
            Err(CheckpointError::Unsigned {
                ed25519: true,
                mldsa44: false
            })
        );
    }
    // The right length, all zeros.
    let zeros = vec![0u8; 8 + MLDSA44_SIGNATURE];
    let signed = note(&text(), &[ed_line(), line(&ml.name, ml.id(), &zeros)]);
    assert!(verify(&signed, &keys()).is_err());
}

#[test]
fn a_forged_ed25519_beside_a_valid_mldsa44_rejects() {
    let (ed, _) = log_keys();
    let forged = flip(&ed.sign(text().as_bytes()), 3);
    let signed = note(&text(), &[line(&ed.name, ed.id(), &forged), ml_line()]);
    assert_eq!(
        verify(&signed, &keys()),
        Err(CheckpointError::Unsigned {
            ed25519: false,
            mldsa44: true
        })
    );
}

#[test]
fn signatures_by_other_keys_count_for_nothing() {
    let (size, root) = tree();
    let other_ed = Ed::new(ORIGIN, 99);
    let other_ml = Ml::new(ORIGIN, 98);
    let signed = note(
        &text(),
        &[
            other_ed.line(&text()),
            other_ml.line(ORIGIN, size, &root, TIME),
        ],
    );
    assert_eq!(
        verify(&signed, &keys()),
        Err(CheckpointError::Unsigned {
            ed25519: false,
            mldsa44: false
        })
    );
    // Unknown lines beside both valid ones are ignored.
    let signed = note(&text(), &[ed_line(), other_ed.line(&text()), ml_line()]);
    assert!(verify(&signed, &keys()).is_ok());
}

#[test]
fn signatures_over_another_checkpoint_reject() {
    let (size, root) = tree();
    let (ed, ml) = log_keys();
    let other = body(ORIGIN, size + 1, &root);
    let signed = note(
        &text(),
        &[ed.line(&other), ml.line(ORIGIN, size + 1, &root, TIME)],
    );
    assert_eq!(
        verify(&signed, &keys()),
        Err(CheckpointError::Unsigned {
            ed25519: false,
            mldsa44: false
        })
    );
}

#[test]
fn two_lines_under_one_key_are_refused() {
    let signed = note(&text(), &[ed_line(), ed_line(), ml_line()]);
    assert_eq!(verify(&signed, &keys()), Err(CheckpointError::Duplicate));
}

#[test]
fn keys_of_the_wrong_type_are_refused() {
    let (ed, ml) = log_keys();
    assert_eq!(
        Ed25519Key::from_vkey(&Vkey::parse(&ml.vkey()).unwrap()).err(),
        Some(CheckpointError::Key)
    );
    assert_eq!(
        MlDsaKey::from_vkey(&Vkey::parse(&ed.vkey()).unwrap()).err(),
        Some(CheckpointError::Key)
    );
}

#[test]
fn a_malformed_note_is_refused() {
    assert_eq!(
        verify(&text(), &keys()),
        Err(CheckpointError::Note(NoteError::Malformed))
    );
}

// ── The MCP crate's vector ─────────────────────────────────────────────────────────

/// docs/spec/mcp-vectors/checkpoint.json is this file's AND checkpoint, as
/// text: the note both log keys sign, the two vkeys, and RFC 9162's
/// inclusion proof for leaf 5 of the eight. The MCP crate verifies it
/// through its own JSON reader and has no signer; this test is where its
/// bytes come from.
#[test]
fn the_mcp_vector_is_the_and_checkpoint() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../docs/spec/mcp-vectors/checkpoint.json")).unwrap();
    let (ed, ml) = log_keys();
    let signed = note(&text(), &[ed_line(), ml_line()]);
    assert_eq!(v["note"], signed.as_str());
    assert_eq!(v["ed25519"], ed.vkey().as_str());
    assert_eq!(v["mldsa44"], ml.vkey().as_str());
    assert!(verify(&signed, &keys()).is_ok());
    let leaf = chorus::note::decode_base64(v["inclusion"]["leaf"].as_str().unwrap()).unwrap();
    let path: Vec<[u8; 32]> = v["inclusion"]["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            chorus::note::decode_base64(p.as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap()
        })
        .collect();
    let (size, root) = tree();
    assert_eq!(v["inclusion"]["index"], 5);
    assert_eq!(
        chorus::merkle::verify_inclusion(5, size, &chorus::merkle::leaf_hash(&leaf), &path, &root),
        Ok(())
    );
}

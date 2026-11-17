//! C2SP signed-note: the spec's own Ed25519 example, key IDs, vkeys, and
//! the parse rules.

use ledger_verify::checkpoint::Ed25519Key;
use ledger_verify::note::{
    decode_base64, encode_base64, key_id, valid_key_name, NoteError, SignedNote, Vkey, ED25519,
    MLDSA44_COSIGNATURE,
};

use crate::common;

/// c2sp.org/signed-note, "Example".
const EXAMPLE_VKEY: &str = "example.com/foo+530d903a+AekyeRrm56hApGFkyQR4ZCbV54Id2LKaANYcrnKv3U2k";
const EXAMPLE_NOTE: &str = "This is an example message.\n\n\u{2014} example.com/foo Uw2QOkn8srV1yJGh2VYRlL1Tnagv1YEq6TfXppzi2ONncAlTgK7Ztg1ERYNZXsYjOBH3mFXmRKuwHjG1Yu72IneyaQM=\n";

/// c2sp.org/tlog-checkpoint, the opening example (no vkey published).
const CHECKPOINT_NOTE: &str = "example.com/behind-the-sofa\n20852163\nCsUYapGGPo4dkMgIAUqom/Xajj7h2fB2MPA3j2jxq2I=\n\n\u{2014} example.com/behind-the-sofa Az3grlgtzPICa5OS8npVmf1Myq/5IZniMp+ZJurmRDeOoRDe4URYN7u5/Zhcyv2q1gGzGku9nTo+zyWE+xeMcTOAYQ8=\n";

#[test]
fn the_c2sp_example_note_verifies() {
    let vkey = Vkey::parse(EXAMPLE_VKEY).unwrap();
    assert_eq!(vkey.name, "example.com/foo");
    assert_eq!(vkey.id, 0x530d903a);
    assert_eq!(vkey.sig_type, ED25519);
    assert_eq!(vkey.public_key.len(), 32);
    assert_eq!(key_id(&vkey.name, ED25519, &vkey.public_key), 0x530d903a);

    let note = SignedNote::parse(EXAMPLE_NOTE).unwrap();
    assert_eq!(note.text, "This is an example message.\n");
    assert_eq!(note.signatures.len(), 1);
    let line = &note.signatures[0];
    assert_eq!(
        (line.name.as_str(), line.id),
        ("example.com/foo", 0x530d903a)
    );

    let key = Ed25519Key::from_vkey(&vkey).unwrap();
    assert!(key.names(line));
    assert!(key.verify(note.text.as_bytes(), &line.signature));
    assert!(!key.verify(b"This is an example message!\n", &line.signature));
}

#[test]
fn the_c2sp_checkpoint_example_parses() {
    let note = SignedNote::parse(CHECKPOINT_NOTE).unwrap();
    assert_eq!(
        note.text,
        "example.com/behind-the-sofa\n20852163\nCsUYapGGPo4dkMgIAUqom/Xajj7h2fB2MPA3j2jxq2I=\n"
    );
    assert_eq!(note.signatures.len(), 1);
    assert_eq!(note.signatures[0].signature.len(), 64);
}

#[test]
fn key_ids_follow_the_recommendation() {
    let key = [7u8; 32];
    assert_eq!(
        key_id("a.example/k", ED25519, &key),
        common::key_id("a.example/k", 0x01, &key)
    );
    assert_ne!(
        key_id("a.example/k", ED25519, &key),
        key_id("a.example/k", MLDSA44_COSIGNATURE, &key)
    );
}

#[test]
fn a_vkey_with_the_wrong_key_id_is_refused() {
    let wrong = EXAMPLE_VKEY.replace("530d903a", "530d903b");
    assert_eq!(Vkey::parse(&wrong), Err(NoteError::KeyId));
    let upper = EXAMPLE_VKEY.replace("530d903a", "530D903A");
    assert_eq!(Vkey::parse(&upper), Err(NoteError::Vkey));
    assert_eq!(
        Vkey::parse("example.com/foo+530d903a"),
        Err(NoteError::Vkey)
    );
    assert_eq!(
        Vkey::parse("example.com/foo+530d903a+"),
        Err(NoteError::Vkey)
    );
}

#[test]
fn base64_is_canonical_only() {
    assert_eq!(decode_base64("AA=="), Ok(vec![0]));
    assert_eq!(decode_base64("AB=="), Err(NoteError::Base64));
    assert_eq!(decode_base64("AA"), Err(NoteError::Base64));
    assert_eq!(decode_base64("A A="), Err(NoteError::Base64));
    assert_eq!(encode_base64(&[0xfb, 0xff]), "+/8=");
    assert_eq!(decode_base64("+/8="), Ok(vec![0xfb, 0xff]));
}

#[test]
fn key_names() {
    assert!(valid_key_name("example.com/foo"));
    assert!(!valid_key_name(""));
    assert!(!valid_key_name("a b"));
    assert!(!valid_key_name("a+b"));
    assert!(!valid_key_name("a\u{00a0}b"));
}

#[test]
fn notes_that_are_not_signed_notes_are_refused() {
    // No blank line.
    assert_eq!(
        SignedNote::parse("text\n\u{2014} k AAAAAA==\n"),
        Err(NoteError::Malformed)
    );
    // No signatures.
    assert_eq!(SignedNote::parse("text\n\n"), Err(NoteError::Malformed));
    // A signature line without the em dash.
    assert_eq!(
        SignedNote::parse("text\n\n- k AAAAAA==\n"),
        Err(NoteError::Malformed)
    );
    // A missing final newline.
    assert_eq!(
        SignedNote::parse("text\n\n\u{2014} k AAAAAA=="),
        Err(NoteError::Malformed)
    );
    // A control character.
    assert_eq!(
        SignedNote::parse("te\txt\n\n\u{2014} k AAAAAA==\n"),
        Err(NoteError::ControlCharacter)
    );
    // A key name with a plus.
    assert_eq!(
        SignedNote::parse("text\n\n\u{2014} k+1 AAAAAA==\n"),
        Err(NoteError::KeyName)
    );
    // Fewer than the 4 key-ID bytes.
    assert_eq!(
        SignedNote::parse("text\n\n\u{2014} k AAA=\n"),
        Err(NoteError::ShortSignature)
    );
    // Non-canonical base64.
    assert_eq!(
        SignedNote::parse("text\n\n\u{2014} k AAAAAB==\n"),
        Err(NoteError::Base64)
    );
}

#[test]
fn the_last_blank_line_splits_text_from_signatures() {
    let note = SignedNote::parse("a\n\nb\n\n\u{2014} k AAAAAAE=\n").unwrap();
    assert_eq!(note.text, "a\n\nb\n");
    assert_eq!(note.signatures[0].id, 0);
    assert_eq!(note.signatures[0].signature, vec![1]);
}

//! Lemma B under the AND rule: accept iff the distinct witnesses whose
//! Ed25519 (0x04) **and** ML-DSA-44 (0x06) cosignatures both verify reach
//! Q = ⌊(N+f)/2⌋ + 1.

use ledger_verify::checkpoint::{Checkpoint, Ed25519CosignerKey, Ed25519Key};
use ledger_verify::note::{SignatureLine, SignedNote, Vkey};
use ledger_verify::quorum::{live, required, QuorumError, Tally, Witness, Witnesses};

use crate::common::{
    cosignature_v1, flip, hash_hex, note, tree, vkey, witnesses, Witness as W, ORIGIN,
};

const TIME: u64 = 1_763_400_000;

fn declare(w: &W) -> Witness {
    let ed = Vkey::parse(&w.ed.cosigner_vkey()).unwrap();
    let ml = Vkey::parse(&w.ml.vkey()).unwrap();
    Witness::from_vkeys(&ed, &ml).unwrap()
}

fn declared() -> Vec<Witness> {
    witnesses().iter().map(declare).collect()
}

fn seven(f: usize) -> Witnesses {
    Witnesses::new(declared(), f).unwrap()
}

fn checkpoint() -> Checkpoint {
    let (size, root) = tree();
    Checkpoint {
        origin: ORIGIN.into(),
        size,
        root,
    }
}

/// The witness's Ed25519 (0x04) cosignature line.
fn ed(w: &W) -> SignatureLine {
    let (size, root) = tree();
    SignatureLine {
        name: w.ed.name.clone(),
        id: w.ed.cosigner_id(),
        signature: w.ed.cosignature(ORIGIN, size, &root, TIME),
    }
}

/// The witness's ML-DSA-44 (0x06) cosignature line.
fn ml(w: &W) -> SignatureLine {
    let (size, root) = tree();
    SignatureLine {
        name: w.ml.name.clone(),
        id: w.ml.id(),
        signature: w.ml.cosignature(ORIGIN, size, &root, TIME),
    }
}

/// Both cosignatures from each witness at `who`.
fn from(who: &[usize]) -> Vec<SignatureLine> {
    who.iter()
        .flat_map(|&i| [ed(&witnesses()[i]), ml(&witnesses()[i])])
        .collect()
}

#[test]
fn q_is_floor_n_plus_f_over_2_plus_1() {
    assert_eq!(required(7, 2), 5);
    assert!(live(7, 2));
    // Mara's small-numbers table (research-ledger-design.md §2.3).
    assert_eq!(
        [
            required(1, 0),
            required(2, 0),
            required(3, 0),
            required(4, 1)
        ],
        [1, 2, 2, 3]
    );
    assert!(live(4, 1) && !live(3, 1) && live(1, 0));
    assert_eq!(seven(2).required(), 5);
    assert_eq!(seven(2).len(), 7);
    assert!(!seven(2).is_empty());
}

#[test]
fn the_ed25519_cosignature_body_is_the_c2sp_text() {
    // C2SP tlog-cosignature, "Ed25519": the spec's own example input.
    let example = Checkpoint {
        origin: "example.com/behind-the-sofa".into(),
        size: 20852163,
        root: hash_hex("0ac5186a91863e8e1d90c808014aa89bf5da8e3ee1d9f07630f0378f68f1ab62"),
    };
    assert_eq!(
        example.cosignature_v1(1679315147),
        "cosignature/v1\n\
         time 1679315147\n\
         example.com/behind-the-sofa\n\
         20852163\n\
         CsUYapGGPo4dkMgIAUqom/Xajj7h2fB2MPA3j2jxq2I=\n"
    );
    // And the tests' own encoder, written from the spec's line list.
    let (size, root) = tree();
    for t in [0, 1, TIME, u64::MAX] {
        assert_eq!(
            checkpoint().cosignature_v1(t),
            cosignature_v1(t, ORIGIN, size, &root)
        );
    }
}

#[test]
fn an_ed25519_cosignature_verifies_with_its_timestamp() {
    let w = &witnesses()[0];
    let key = Ed25519CosignerKey::from_vkey(&Vkey::parse(&w.ed.cosigner_vkey()).unwrap()).unwrap();
    let line = ed(w);
    assert_eq!(key.verify(&checkpoint(), &line.signature), Some(TIME));
    // Another timestamp, a flipped bit, a short value: no.
    let mut retimed = line.signature.clone();
    retimed[7] ^= 1;
    assert_eq!(key.verify(&checkpoint(), &retimed), None);
    assert_eq!(key.verify(&checkpoint(), &flip(&line.signature, 20)), None);
    assert_eq!(key.verify(&checkpoint(), &line.signature[..71]), None);
    // A plain note signature (0x01 style, over the checkpoint text) is not
    // a cosignature.
    let mut plain = TIME.to_be_bytes().to_vec();
    plain.extend_from_slice(&w.ed.sign(checkpoint().body().as_bytes()));
    assert_eq!(key.verify(&checkpoint(), &plain), None);
}

#[test]
fn ed25519_key_types_are_kept_apart() {
    let w = &witnesses()[0];
    let cosigner = Vkey::parse(&w.ed.cosigner_vkey()).unwrap();
    let log = Vkey::parse(&w.ed.vkey()).unwrap();
    assert!(Ed25519Key::from_vkey(&cosigner).is_err());
    assert!(Ed25519CosignerKey::from_vkey(&log).is_err());
    let key = Ed25519CosignerKey::from_vkey(&cosigner).unwrap();
    assert_eq!(key.public_key(), w.ed.public());
}

#[test]
fn a_witness_with_only_its_mldsa44_cosignature_counts_zero() {
    let t = seven(2).tally(&checkpoint(), &[ml(&witnesses()[0])]);
    assert_eq!((t.valid, t.partial, t.invalid), (0, 1, 0));
}

#[test]
fn a_witness_with_only_its_ed25519_cosignature_counts_zero() {
    let t = seven(2).tally(&checkpoint(), &[ed(&witnesses()[0])]);
    assert_eq!((t.valid, t.partial, t.invalid), (0, 1, 0));
}

#[test]
fn four_distinct_valid_of_seven_rejects() {
    let t = seven(2).tally(&checkpoint(), &from(&[0, 1, 2, 3]));
    assert_eq!(
        t,
        Tally {
            valid: 4,
            partial: 0,
            invalid: 0,
            unknown: 0,
            required: 5,
            live: true
        }
    );
    assert!(!t.accepted());
}

#[test]
fn five_with_both_signatures_valid_of_seven_accepts() {
    let t = seven(2).tally(&checkpoint(), &from(&[0, 2, 3, 5, 6]));
    assert_eq!((t.valid, t.partial, t.required), (5, 0, 5));
    assert!(t.accepted());
    assert!(seven(2)
        .tally(&checkpoint(), &from(&[0, 1, 2, 3, 4, 5, 6]))
        .accepted());
}

#[test]
fn five_where_one_has_only_its_mldsa44_rejects() {
    let mut lines = from(&[0, 1, 2, 3]);
    lines.push(ml(&witnesses()[4]));
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.partial), (4, 1));
    assert!(!t.accepted());
}

#[test]
fn five_where_one_has_only_its_ed25519_rejects() {
    let mut lines = from(&[0, 1, 2, 3]);
    lines.push(ed(&witnesses()[4]));
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.partial), (4, 1));
    assert!(!t.accepted());
}

#[test]
fn five_with_one_forged_ed25519_rejects() {
    let mut lines = from(&[0, 1, 2, 3, 4]);
    // lines[8] is witness 4's Ed25519 line; flip a signature bit.
    lines[8].signature = flip(&lines[8].signature, 40);
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.partial, t.invalid), (4, 1, 1));
    assert!(!t.accepted());
}

#[test]
fn five_with_one_forged_mldsa44_rejects() {
    let mut lines = from(&[0, 1, 2, 3, 4]);
    lines[9].signature = flip(&lines[9].signature, 200);
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.partial, t.invalid), (4, 1, 1));
    assert!(!t.accepted());
}

#[test]
fn five_with_one_duplicate_rejects() {
    let t = seven(2).tally(&checkpoint(), &from(&[0, 1, 2, 3, 3]));
    assert_eq!(t.valid, 4);
    assert!(!t.accepted());
}

#[test]
fn five_with_one_unknown_witness_rejects() {
    let stranger = W::new("stranger.example.com", 76, 77);
    let mut lines = from(&[0, 1, 2, 3]);
    lines.extend([ed(&stranger), ml(&stranger)]);
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.unknown), (4, 2));
    assert!(!t.accepted());
}

#[test]
fn a_known_name_under_a_stranger_s_keys_counts_zero() {
    // Same name as witness 4, different keys (so different key IDs).
    let imposter = W::new(&witnesses()[4].ml.name, 79, 78);
    let mut lines = from(&[0, 1, 2, 3]);
    lines.extend([ed(&imposter), ml(&imposter)]);
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.unknown), (4, 2));
    assert!(!t.accepted());
}

#[test]
fn cosignatures_over_another_checkpoint_count_zero() {
    let mut other = checkpoint();
    other.size -= 1;
    let t = seven(2).tally(&other, &from(&[0, 1, 2, 3, 4]));
    assert_eq!((t.valid, t.partial, t.invalid), (0, 0, 10));
}

#[test]
fn cosignatures_read_from_a_note() {
    let (size, root) = tree();
    let lines: Vec<String> = [0, 1, 2, 4, 6]
        .iter()
        .flat_map(|&i| {
            let w = &witnesses()[i];
            [
                w.ed.cosignature_line(ORIGIN, size, &root, TIME),
                w.ml.line(ORIGIN, size, &root, TIME),
            ]
        })
        .collect();
    let signed = note(&checkpoint().body(), &lines);
    let parsed = SignedNote::parse(&signed).unwrap();
    assert!(seven(2).tally(&checkpoint(), &parsed.signatures).accepted());
}

#[test]
fn a_witness_is_declared_with_both_keys_under_one_name() {
    let w = &witnesses()[0];
    let ed = Vkey::parse(&w.ed.cosigner_vkey()).unwrap();
    let ml = Vkey::parse(&w.ml.vkey()).unwrap();
    // Either order.
    assert_eq!(Witness::from_vkeys(&ml, &ed).unwrap().name(), w.ml.name);
    // Only one key, twice: no.
    assert_eq!(
        Witness::from_vkeys(&ml, &ml).err(),
        Some(QuorumError::WitnessKeys)
    );
    assert_eq!(
        Witness::from_vkeys(&ed, &ed).err(),
        Some(QuorumError::WitnessKeys)
    );
    // An Ed25519 log key (0x01) is not a cosigner key.
    let log = Vkey::parse(&w.ed.vkey()).unwrap();
    assert_eq!(
        Witness::from_vkeys(&log, &ml).err(),
        Some(QuorumError::WitnessKeys)
    );
    // A malformed key of the right type.
    let short = Vkey::parse(&vkey(&w.ml.name, 0x06, &[1, 2, 3])).unwrap();
    assert_eq!(
        Witness::from_vkeys(&ed, &short).err(),
        Some(QuorumError::WitnessKeys)
    );
    let short = Vkey::parse(&vkey(&w.ed.name, 0x04, &[1, 2, 3])).unwrap();
    assert_eq!(
        Witness::from_vkeys(&short, &ml).err(),
        Some(QuorumError::WitnessKeys)
    );
    // Two names.
    let other = Vkey::parse(&witnesses()[1].ml.vkey()).unwrap();
    assert_eq!(
        Witness::from_vkeys(&ed, &other).err(),
        Some(QuorumError::WitnessKeys)
    );
}

#[test]
fn witness_sets_that_cannot_work_are_refused() {
    let keys = declared();
    // f = N: Q = N + 1 > N.
    assert_eq!(
        Witnesses::new(keys.clone(), 7).err(),
        Some(QuorumError::Unreachable { n: 7, f: 7, q: 8 })
    );
    assert_eq!(
        Witnesses::new(vec![], 0).err(),
        Some(QuorumError::Unreachable { n: 0, f: 0, q: 1 })
    );
    // f = 6 < N: Q = 7 = N, reachable but not live.
    let w = Witnesses::new(keys.clone(), 6).unwrap();
    assert_eq!(w.required(), 7);
    assert!(!w.tally(&checkpoint(), &[]).live);
    // One witness twice.
    let mut twice = keys.clone();
    twice.push(keys[0].clone());
    assert_eq!(
        Witnesses::new(twice, 2).err(),
        Some(QuorumError::DuplicateWitness)
    );
    // One public key under two names, in either scheme.
    let alias = "alias.example.com";
    let fresh = W::new(alias, 90, 91);
    let alias_ed = Vkey::parse(&vkey(alias, 0x04, &witnesses()[0].ed.public())).unwrap();
    let alias_ml = Vkey::parse(&vkey(alias, 0x06, &witnesses()[0].ml.public())).unwrap();
    let fresh_ed = Vkey::parse(&fresh.ed.cosigner_vkey()).unwrap();
    let fresh_ml = Vkey::parse(&fresh.ml.vkey()).unwrap();
    for (e, m) in [(&alias_ed, &fresh_ml), (&fresh_ed, &alias_ml)] {
        let mut aliased = keys.clone();
        aliased.push(Witness::from_vkeys(e, m).unwrap());
        assert_eq!(
            Witnesses::new(aliased, 2).err(),
            Some(QuorumError::DuplicateWitness)
        );
    }
    // The fresh witness itself is fine.
    let mut eight = keys;
    eight.push(Witness::from_vkeys(&fresh_ed, &fresh_ml).unwrap());
    assert_eq!(Witnesses::new(eight, 2).unwrap().len(), 8);
}

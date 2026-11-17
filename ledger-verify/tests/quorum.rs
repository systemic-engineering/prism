//! Lemma B: accept iff the distinct valid witnesses reach
//! Q = ⌊(N+f)/2⌋ + 1.

use ledger_verify::checkpoint::{Checkpoint, MlDsaKey};
use ledger_verify::note::{SignatureLine, SignedNote, Vkey};
use ledger_verify::quorum::{live, required, QuorumError, Tally, Witnesses};

use crate::common::{flip, note, tree, witnesses, Ml, ORIGIN};

const TIME: u64 = 1_763_400_000;

fn key(w: &Ml) -> MlDsaKey {
    MlDsaKey::from_vkey(&Vkey::parse(&w.vkey()).unwrap()).unwrap()
}

fn seven(f: usize) -> Witnesses {
    Witnesses::new(witnesses().iter().map(key).collect(), f).unwrap()
}

fn checkpoint() -> Checkpoint {
    let (size, root) = tree();
    Checkpoint {
        origin: ORIGIN.into(),
        size,
        root,
    }
}

fn cosign(w: &Ml) -> SignatureLine {
    let (size, root) = tree();
    SignatureLine {
        name: w.name.clone(),
        id: w.id(),
        signature: w.cosignature(ORIGIN, size, &root, TIME),
    }
}

/// Cosignatures from the witnesses at `who`.
fn from(who: &[usize]) -> Vec<SignatureLine> {
    who.iter().map(|&i| cosign(&witnesses()[i])).collect()
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
}

#[test]
fn four_distinct_valid_of_seven_rejects() {
    let t = seven(2).tally(&checkpoint(), &from(&[0, 1, 2, 3]));
    assert_eq!(
        t,
        Tally {
            valid: 4,
            invalid: 0,
            unknown: 0,
            required: 5,
            live: true
        }
    );
    assert!(!t.accepted());
}

#[test]
fn five_distinct_valid_of_seven_accepts() {
    let t = seven(2).tally(&checkpoint(), &from(&[0, 2, 3, 5, 6]));
    assert_eq!((t.valid, t.required), (5, 5));
    assert!(t.accepted());
    assert!(seven(2)
        .tally(&checkpoint(), &from(&[0, 1, 2, 3, 4, 5, 6]))
        .accepted());
}

#[test]
fn five_with_one_duplicate_rejects() {
    let t = seven(2).tally(&checkpoint(), &from(&[0, 1, 2, 3, 3]));
    assert_eq!(t.valid, 4);
    assert!(!t.accepted());
}

#[test]
fn five_with_one_unknown_key_rejects() {
    let stranger = Ml::new("stranger.example.com", 77);
    let mut lines = from(&[0, 1, 2, 3]);
    lines.push(cosign(&stranger));
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.unknown), (4, 1));
    assert!(!t.accepted());
}

#[test]
fn a_known_name_under_a_stranger_s_key_counts_zero() {
    // Same name as witness 4, a different key (so a different key ID).
    let imposter = Ml::new(&witnesses()[4].name, 78);
    let mut lines = from(&[0, 1, 2, 3]);
    lines.push(cosign(&imposter));
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.unknown), (4, 1));
    assert!(!t.accepted());
}

#[test]
fn five_with_one_forged_rejects() {
    let mut lines = from(&[0, 1, 2, 3, 4]);
    lines[4].signature = flip(&lines[4].signature, 200);
    let t = seven(2).tally(&checkpoint(), &lines);
    assert_eq!((t.valid, t.invalid), (4, 1));
    assert!(!t.accepted());
}

#[test]
fn cosignatures_over_another_checkpoint_count_zero() {
    let mut other = checkpoint();
    other.size -= 1;
    let t = seven(2).tally(&other, &from(&[0, 1, 2, 3, 4]));
    assert_eq!((t.valid, t.invalid), (0, 5));
}

#[test]
fn cosignatures_read_from_a_note() {
    let (size, root) = tree();
    let lines: Vec<String> = [0, 1, 2, 4, 6]
        .iter()
        .map(|&i| witnesses()[i].line(ORIGIN, size, &root, TIME))
        .collect();
    let signed = note(&checkpoint().body(), &lines);
    let parsed = SignedNote::parse(&signed).unwrap();
    assert!(seven(2).tally(&checkpoint(), &parsed.signatures).accepted());
}

#[test]
fn witness_sets_that_cannot_work_are_refused() {
    let keys: Vec<MlDsaKey> = witnesses().iter().map(key).collect();
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
    // One public key under two names.
    let alias = Vkey::parse(&crate::common::vkey(
        "alias.example.com",
        0x06,
        &witnesses()[0].public(),
    ))
    .unwrap();
    let mut aliased = keys;
    aliased.push(MlDsaKey::from_vkey(&alias).unwrap());
    assert_eq!(
        Witnesses::new(aliased, 2).err(),
        Some(QuorumError::DuplicateWitness)
    );
}

//! prism-007/008: `prismqueer::music`, against Mara's
//! `2026-10-29-mara-song-of-the-past-void-commas-and-the-beat.md`.
//!
//! A Note is a derivation (head, rule, premises), with no beat in it. Notes
//! compose by union. Beats (Kleene ticks) and chords are computed from the
//! composed piece, never stored (§10: geodesic certificates don't compose).
//! "Does it ring true?" reads a tuning: consonance, or located dissonances
//! with their exact commas (§5). prism is the instrument; picking the tuning
//! is a ruler, and stays with mirror.

#![cfg(feature = "lapack")]

use std::collections::{BTreeMap, BTreeSet};

use prismqueer::field::{Edge, Field, Ratio};
use prismqueer::music::{ring_true, Atom, Note, Piece, Ringing, Rule, Tuning, TuningError};
use prismqueer::spectral::graph_from_edges;

fn atom(s: &str) -> Atom {
    Atom::new(s)
}

fn note(head: &str, rule: &str, premises: &[&str]) -> Note {
    Note::new(
        atom(head),
        Rule::new(rule),
        premises.iter().map(|p| atom(p)),
    )
}

/// Mara §10: P = {fact a; r1: b ← a; r2: c ← b}.
fn p() -> Piece {
    Piece::from([
        note("a", "fact", &[]),
        note("b", "r1", &["a"]),
        note("c", "r2", &["b"]),
    ])
}

#[test]
fn a_piece_is_empty_only_without_notes() {
    let silence = Piece::from([] as [Note; 0]);
    assert!(silence.is_empty());
    assert_eq!(silence.len(), 0);
    assert!(!p().is_empty());
    assert!(!silence.compose(&p()).is_empty());
}

/// Q = {r3: c ← a}.
fn q() -> Piece {
    Piece::from([note("c", "r3", &["a"])])
}

fn beats(pairs: &[(&str, usize)]) -> BTreeMap<Atom, usize> {
    pairs.iter().map(|&(a, t)| (atom(a), t)).collect()
}

#[test]
fn notes_compose_by_union() {
    assert_eq!(p().compose(&q()), q().compose(&p()), "commutative");
    assert_eq!(p().compose(&p()), p(), "idempotent");
    assert_eq!(p().compose(&q()).len(), 4);
}

#[test]
fn beats_are_computed_from_the_composed_piece_never_stored() {
    assert_eq!(p().beats(), beats(&[("a", 0), ("b", 1), ("c", 2)]));
    let pq = p().compose(&q());
    assert_eq!(
        pq.beats(),
        beats(&[("a", 0), ("b", 1), ("c", 1)]),
        "a shorter route from another observer moves c"
    );
    // The geodesic projection changes: P's certificate for c is r2, P ∪ Q's is r3.
    assert_eq!(p().geodesic()[&atom("c")], note("c", "r2", &["b"]));
    assert_eq!(pq.geodesic()[&atom("c")], note("c", "r3", &["a"]));
    assert_eq!(
        pq.chord(1),
        BTreeSet::from([atom("b"), atom("c")]),
        "a chord is all atoms on one beat"
    );
}

#[test]
fn an_atom_nobody_derives_has_no_beat() {
    let piece = Piece::from([note("x", "r", &["missing"])]);
    assert!(
        piece.beats().is_empty(),
        "a premise nobody derives leaves its head dark"
    );
}

#[test]
fn the_cadence_is_the_fixpoint() {
    assert_eq!(p().cadence().beats(), 3, "beats 0, 1, 2, then nothing new");
    assert_eq!(p().compose(&q()).cadence().beats(), 2);
}

#[test]
fn three_major_thirds_ring_false_by_the_lesser_diesis() {
    // Three observers, three circular readings, one loop that belongs to none.
    let triangle = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
    let thirds = Field::transports(
        &triangle,
        [
            ((0, 1), Ratio::of(5, 4)),
            ((1, 2), Ratio::of(5, 4)),
            ((2, 0), Ratio::of(5, 8)),
        ],
    )
    .unwrap();
    match ring_true(&thirds) {
        Ringing::Dissonant(found) => {
            assert_eq!(found.len(), 1);
            assert_eq!(
                found[0].notes(),
                &[0, 1, 2, 0],
                "located: these readings, this loop"
            );
            assert_eq!(
                *found[0].comma(),
                Ratio::of(125, 128),
                "exactly the lesser diesis"
            );
        }
        Ringing::True(_) => panic!("(5/4)³ is not an octave"),
    }
    let just = Field::transports(
        &triangle,
        [
            ((0, 1), Ratio::of(5, 4)),
            ((1, 2), Ratio::of(6, 5)),
            ((2, 0), Ratio::of(2, 3)),
        ],
    )
    .unwrap();
    assert!(
        matches!(ring_true(&just), Ringing::True(_)),
        "a major third and a minor third make the fifth"
    );
}

/// prism-008 Ask 2: one located dissonance on a two-Note loop. Two Notes
/// relate the same two atoms by different exact intervals: two parallel
/// edges, one loop of length 2, its holonomy the comma.
#[test]
fn two_notes_on_one_pair_ring_false_by_their_comma() {
    let pure = note("e", "pure fifth", &["a"]);
    let grave = note("e", "grave fifth", &["a"]);
    let tuning = Tuning::new([
        (pure.clone(), Ratio::of(3, 2)),
        (grave.clone(), Ratio::of(40, 27)),
    ])
    .unwrap();
    assert_eq!(tuning.atoms(), &[atom("a"), atom("e")], "a is 0, e is 1");
    match tuning.ring_true() {
        Ringing::Dissonant(found) => {
            assert_eq!(found.len(), 1, "exactly one located dissonance");
            let d = &found[0];
            println!("{d:#?}");
            assert_eq!(d.notes(), &[0, 1, 0], "on the loop a -> e -> a");
            assert_eq!(
                d.edges(),
                &[Edge::between(0, 1, 0), Edge::between(0, 1, 1)],
                "out along one edge, back along the other"
            );
            assert_eq!(d.by(), &[pure.clone(), grave.clone()], "naming both Notes");
            assert_eq!(*d.comma(), Ratio::of(81, 80), "the syntonic comma, exactly");
        }
        Ringing::True(_) => panic!("a pure and a grave fifth don't agree"),
    }
    // The control: the same two Notes, equal maps.
    let agreed = Tuning::new([(pure, Ratio::of(3, 2)), (grave, Ratio::of(3, 2))]).unwrap();
    assert!(
        matches!(agreed.ring_true(), Ringing::True(_)),
        "two Notes that agree ring true"
    );
}

/// Loki's diaschisma as two Notes: "two tritones" and "an octave" both
/// relate a to c. (45/32)² against 2 leaves 2025/2048 on the 2-loop.
#[test]
fn two_tritones_against_an_octave_leave_the_diaschisma_on_a_two_note_loop() {
    let tritones = note("c", "two tritones", &["a"]);
    let octave = note("c", "octave", &["a"]);
    let tuning = Tuning::new([
        (tritones.clone(), Ratio::of(2025, 1024)),
        (octave.clone(), Ratio::of(2, 1)),
    ])
    .unwrap();
    match tuning.ring_true() {
        Ringing::Dissonant(found) => {
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].by(), &[tritones, octave]);
            assert_eq!(*found[0].comma(), Ratio::of(2025, 2048));
        }
        Ringing::True(_) => panic!("(45/32)² doesn't reach the octave"),
    }
}

#[test]
fn a_note_without_exactly_one_premise_is_no_interval() {
    let fact = note("a", "fact", &[]);
    assert_eq!(
        Tuning::new([(fact.clone(), Ratio::of(3, 2))]),
        Err(TuningError::NotAnInterval(fact))
    );
}

#[test]
fn a_bare_field_names_no_notes() {
    let doubled = Field::multigraph([((0, 1), Ratio::of(3, 2)), ((0, 1), Ratio::of(40, 27))]);
    match ring_true(&doubled) {
        Ringing::Dissonant(found) => {
            assert_eq!(*found[0].comma(), Ratio::of(81, 80));
            assert!(
                found[0].by().is_empty(),
                "a bare field has no Notes to name"
            );
        }
        Ringing::True(_) => panic!("the fifths don't agree"),
    }
}

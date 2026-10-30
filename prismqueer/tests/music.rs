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

use prismqueer::field::{Field, Ratio};
use prismqueer::music::{ring_true, Atom, Note, Piece, Ringing, Rule};
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

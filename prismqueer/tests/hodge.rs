//! prism-005 D: the Hodge split of intervals, the edge-side settle.
//!
//! On a graph (no 2-cells), intervals split into a gradient part (the
//! question of some assertion, im δ) and a harmonic part (the comma, ker δ*).
//! The two are complementary orthogonal projections: what `temper` keeps
//! and what the edge settle keeps (Mara, ruler-and-pact; Lim, Hodge
//! Laplacians on graphs). mirror's FP5, in prism's own suite.

#![cfg(feature = "lapack")]

use prismqueer::spectral::graph_from_edges;
use prismqueer::spectral::instrument::{Cochain, Instrument};
use prismqueer::spectral::settle::Time;

const EPS: f64 = 1e-10;

fn square() -> Instrument {
    Instrument::of(&graph_from_edges(&[(0, 1), (1, 2), (2, 3), (0, 3)])).expect("measured")
}

fn intervals(values: [f64; 4]) -> Cochain {
    let e = [(0, 1), (1, 2), (2, 3), (0, 3)];
    Cochain::new([], e.into_iter().zip(values))
}

/// The circulation 0 -> 1 -> 2 -> 3 -> 0.
const CIRCULATION: [f64; 4] = [1.0, 1.0, 1.0, -1.0];

#[test]
fn gradient_plus_harmonic_is_the_input() {
    let d = square();
    let x = intervals([3.0, -1.0, 2.0, 0.5]);
    let split = d.hodge(&x).expect("intervals only");
    let sum = Cochain::new(
        [],
        [(0, 1), (1, 2), (2, 3), (0, 3)].map(|e| {
            (
                e,
                split.gradient().interval(e).unwrap() + split.harmonic().interval(e).unwrap(),
            )
        }),
    );
    assert!(
        sum.distance(&x) < EPS,
        "gradient + harmonic must reconstruct the input"
    );

    // Closed form: the harmonic part is the projection on the circulation.
    let h: f64 = [3.0, -1.0, 2.0, 0.5]
        .iter()
        .zip(CIRCULATION)
        .map(|(a, b)| a * b)
        .sum::<f64>()
        / 4.0;
    for (e, c) in [(0, 1), (1, 2), (2, 3), (0, 3)]
        .into_iter()
        .zip(CIRCULATION)
    {
        assert!(
            (split.harmonic().interval(e).unwrap() - h * c).abs() < EPS,
            "harmonic on {e:?}"
        );
    }
}

#[test]
fn both_parts_are_idempotent_and_orthogonal() {
    let d = square();
    let split = d.hodge(&intervals([3.0, -1.0, 2.0, 0.5])).unwrap();
    let again_g = d.hodge(split.gradient()).unwrap();
    let again_h = d.hodge(split.harmonic()).unwrap();
    assert!(
        again_g.gradient().distance(split.gradient()) < EPS && again_g.harmonic().norm() < EPS,
        "gradient is fixed"
    );
    assert!(
        again_h.harmonic().distance(split.harmonic()) < EPS && again_h.gradient().norm() < EPS,
        "harmonic is fixed"
    );
    let dot: f64 = [(0, 1), (1, 2), (2, 3), (0, 3)]
        .iter()
        .map(|&e| split.gradient().interval(e).unwrap() * split.harmonic().interval(e).unwrap())
        .sum();
    assert!(dot.abs() < EPS, "the parts are orthogonal");
}

#[test]
fn the_gradient_is_a_question_and_the_harmonic_part_asks_nothing_back() {
    let d = square();
    let split = d.hodge(&intervals([3.0, -1.0, 2.0, 0.5])).unwrap();
    // δ* of the harmonic part vanishes: it is not answered by any assertion.
    let back = d.question(split.harmonic()).unwrap();
    for v in 0..4 {
        assert!(back.pitch(v).unwrap().abs() < EPS, "δ* harmonic at {v}");
    }
}

#[test]
fn the_harmonic_part_is_what_the_edge_settle_keeps() {
    let d = square();
    let x = intervals([3.0, -1.0, 2.0, 0.5]);
    let settled = d.decay(Time(80.0), &x).unwrap();
    let split = d.hodge(&x).unwrap();
    assert!(
        settled.distance(split.harmonic()) < 1e-9,
        "long decay of intervals = their harmonic part"
    );
}

#[test]
fn a_tree_has_no_harmonic_part() {
    let d = Instrument::of(&graph_from_edges(&[(0, 1), (1, 2), (1, 3)])).unwrap();
    let x = Cochain::new([], [((0, 1), 2.0), ((1, 2), -1.0), ((1, 3), 4.0)]);
    let split = d.hodge(&x).unwrap();
    assert!(split.harmonic().norm() < EPS);
    assert!(split.gradient().distance(&x) < EPS);
}

#[test]
fn hodge_takes_intervals_only() {
    let d = square();
    assert!(
        d.hodge(&Cochain::new([(0, 1.0)], [])).is_err(),
        "a pitch is not an interval: refused, not dropped"
    );
}

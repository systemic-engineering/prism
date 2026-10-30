//! prism-004 §4: a spectral window on the Instrument, the frequency
//! restriction mirror reads ("hold the superposition, restrict through
//! frequencies"). A restriction doesn't mutate (the source is borrowed), but
//! it isn't lossless: what falls outside the window is the erasure.

#![cfg(feature = "lapack")]

use prismqueer::spectral::graph_from_edges;
use prismqueer::spectral::instrument::{Angle, Cochain, Instrument, Solo, Window};
use prismqueer::spectral::settle::Time;

const EPS: f64 = 1e-10;

fn square() -> Instrument {
    Instrument::of(&graph_from_edges(&[(0, 1), (1, 2), (2, 3), (0, 3)])).unwrap()
}

fn sample() -> Cochain {
    Cochain::new(
        [(0, 1.0), (1, -2.0), (2, 0.5), (3, 3.0)],
        [((0, 1), 0.7), ((1, 2), -1.1), ((2, 3), 0.4), ((0, 3), 2.0)],
    )
}

/// The square's D has spectrum {0, 0, ±√2, ±√2, ±2}; this window keeps |λ| ≤ 1.5.
fn low() -> Window {
    Window::new(-1.5, 1.5)
}

fn basis(d_edges: &[(u64, u64)], vertices: &[u64]) -> Vec<Cochain> {
    let mut out: Vec<Cochain> = vertices
        .iter()
        .map(|&v| Cochain::new([(v, 1.0)], []))
        .collect();
    out.extend(d_edges.iter().map(|&e| Cochain::new([], [(e, 1.0)])));
    out
}

#[test]
fn split_is_lossless_as_a_pair_and_settle_logs_the_erasure() {
    let d = square();
    let x = sample();
    let r = d.restrict(&low(), &x).unwrap();
    let sum = d.restrict(&Window::new(-10.0, 10.0), r.kept()).unwrap(); // a no-op window
    assert!(sum.erased().norm() < EPS);
    assert!(
        sum.is_lossless(),
        "a window holding every mode erases nothing"
    );
    let rebuilt = add(r.kept(), r.erased());
    assert!(rebuilt.distance(&x) < EPS, "kept + erased = the input");
    assert!(r.erased().norm() > 0.1, "this window does lose something");
    assert!(!r.is_lossless());
}

#[test]
fn the_projector_is_idempotent() {
    let d = square();
    let once = d.restrict(&low(), &sample()).unwrap();
    let twice = d.restrict(&low(), once.kept()).unwrap();
    assert!(
        twice.kept().distance(once.kept()) < EPS && twice.erased().norm() < EPS,
        "P² = P"
    );
}

#[test]
fn it_commutes_with_decay_and_ring() {
    let d = square();
    let x = sample();
    let p_then_decay = d
        .decay(Time(0.4), d.restrict(&low(), &x).unwrap().kept())
        .unwrap();
    let decay_then_p = d
        .restrict(&low(), &d.decay(Time(0.4), &x).unwrap())
        .unwrap();
    assert!(
        p_then_decay.distance(decay_then_p.kept()) < EPS,
        "same D: commutes with decay"
    );
    let rung = d.ring(Angle(0.8), &x).unwrap();
    let p_rung = d
        .ring(Angle(0.8), d.restrict(&low(), &x).unwrap().kept())
        .unwrap();
    assert!(
        p_rung
            .real
            .distance(d.restrict(&low(), &rung.real).unwrap().kept())
            < EPS
    );
    assert!(
        p_rung
            .imag
            .distance(d.restrict(&low(), &rung.imag).unwrap().kept())
            < EPS,
        "commutes with ring"
    );
}

#[test]
fn the_kernel_window_keeps_exactly_the_harmonic_intervals() {
    let d = square();
    let intervals = Cochain::new(
        [],
        [((0, 1), 0.7), ((1, 2), -1.1), ((2, 3), 0.4), ((0, 3), 2.0)],
    );
    let kept = d.restrict(&Window::new(-1e-3, 1e-3), &intervals).unwrap();
    assert!(
        kept.kept()
            .distance(d.hodge(&intervals).unwrap().harmonic())
            < EPS,
        "the kernel window is the edge settle"
    );
}

#[test]
fn the_window_commutes_with_solo_exactly_when_its_band_is_solo_invariant() {
    // Two triangles; solo one of them. The kernel window (H0 per component,
    // H1 per cycle) is invariant under that solo: no bleed.
    let e = [(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)];
    let d = Instrument::of(&graph_from_edges(&e)).unwrap();
    let solo = Solo::new([0, 1, 2]);
    let kernel = Window::new(-1e-3, 1e-3);
    for b in basis(&e, &[0, 1, 2, 3, 4, 5]) {
        let a = d.restrict(&kernel, &d.solo(&solo, &b).unwrap()).unwrap();
        let c = d
            .solo(&solo, d.restrict(&kernel, &b).unwrap().kept())
            .unwrap();
        assert!(
            a.kept().distance(&c) < EPS,
            "commutes on a solo-invariant band"
        );
    }
    // A path, soloing half: the constants aren't solo-invariant, so it bleeds.
    let p = [(0, 1), (1, 2), (2, 3)];
    let d = Instrument::of(&graph_from_edges(&p)).unwrap();
    let solo = Solo::new([0, 1]);
    let bleeds = basis(&p, &[0, 1, 2, 3]).into_iter().any(|b| {
        let a = d.restrict(&kernel, &d.solo(&solo, &b).unwrap()).unwrap();
        let c = d
            .solo(&solo, d.restrict(&kernel, &b).unwrap().kept())
            .unwrap();
        a.kept().distance(&c) > 1e-6
    });
    assert!(
        bleeds,
        "the band isn't solo-invariant here, so window and solo don't commute"
    );
}

#[test]
fn a_window_edge_on_an_eigenvalue_is_refused() {
    // √2 is an eigenvalue of the square's D: membership there is undecidable.
    assert!(square()
        .restrict(&Window::new(-(2f64.sqrt()), 2f64.sqrt()), &sample())
        .is_err());
}

#[test]
fn solo_attenuates_rather_than_restricts() {
    // On an edge crossing the region, solo weighs ½; applied twice, ¼.
    let d = Instrument::of(&graph_from_edges(&[(0, 1)])).unwrap();
    let x = Cochain::new([], [((0, 1), 1.0)]);
    let solo = Solo::new([0]);
    let once = d.solo(&solo, &x).unwrap();
    let twice = d.solo(&solo, &once).unwrap();
    assert!((once.interval((0, 1)).unwrap() - 0.5).abs() < EPS);
    assert!(
        (twice.interval((0, 1)).unwrap() - 0.25).abs() < EPS,
        "not idempotent: an attenuation"
    );
}

fn add(a: &Cochain, b: &Cochain) -> Cochain {
    let pitches = (0..4).map(|v| (v, a.pitch(v).unwrap_or(0.0) + b.pitch(v).unwrap_or(0.0)));
    let e = [(0, 1), (1, 2), (2, 3), (0, 3)];
    let intervals = e.map(|k| {
        (
            k,
            a.interval(k).unwrap_or(0.0) + b.interval(k).unwrap_or(0.0),
        )
    });
    Cochain::new(pitches, intervals)
}

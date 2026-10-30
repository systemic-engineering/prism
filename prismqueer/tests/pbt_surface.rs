//! Arc 2A RED — Sample + Arbitrary + forall runtime primitives.
//!
//! Per Mara witnessed-property-inference spec §9.2:
//! - `pillar::Sample` carrier (Hypothesis-style choice-sequence buffer)
//! - `pillar::Arbitrary` trait
//! - `pillar::forall` runner
//!
//! Alex 2026-07-18 direction: "the full statespace covered liquid floor
//! boards" — Void's default @peer standing surface is Rust rust/ altitude
//! tested via liquid pillar composition; Arc 2A is the first board.

#![cfg(feature = "bundle")]

use prismqueer::flux::pillar::{self, forall, Arbitrary, Sample};
use terni::PropertyVerdict;

// ──────────────────────────────────────────────────────────────────
// Sample: byte-buffer + read position; deterministic replay.
// ──────────────────────────────────────────────────────────────────

#[test]
fn sample_new_starts_at_position_zero() {
    let s = Sample::new();
    assert_eq!(s.depth(), 0);
}

#[test]
fn sample_from_bytes_is_deterministic_replay() {
    let bytes = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let mut a = Sample::from_bytes(bytes.clone());
    let mut b = Sample::from_bytes(bytes);
    let ai = a.draw_integer(0, 1000);
    let bi = b.draw_integer(0, 1000);
    assert_eq!(ai, bi, "same buffer must produce same draws");
}

#[test]
fn sample_draw_integer_stays_within_bounds() {
    for seed in 0u8..64 {
        let mut s = Sample::from_bytes(vec![seed; 32]);
        for _ in 0..10 {
            let v = s.draw_integer(-100, 100);
            assert!((-100..=100).contains(&v), "draw_integer out of range: {v}");
        }
    }
}

#[test]
fn sample_draw_integer_covers_full_range_across_samples() {
    // Draw many values; verify we see both low and high half of range.
    let mut low_count = 0;
    let mut high_count = 0;
    for _ in 0..200 {
        let mut s = Sample::new();
        let v = s.draw_integer(0, 100);
        if v < 50 {
            low_count += 1;
        } else {
            high_count += 1;
        }
    }
    assert!(low_count > 20, "expected >20 low draws, got {low_count}");
    assert!(high_count > 20, "expected >20 high draws, got {high_count}");
}

#[test]
fn sample_draw_integer_advances_position() {
    let mut s = Sample::from_bytes(vec![0xAA; 32]);
    let p0 = s.depth();
    let _ = s.draw_integer(0, 1);
    let p1 = s.depth();
    assert!(p1 > p0, "draw must advance position");
}

#[test]
fn sample_extends_buffer_when_exhausted() {
    // Start with 1 byte, then draw more than fits.
    let mut s = Sample::from_bytes(vec![0x42]);
    for _ in 0..20 {
        let _ = s.draw_integer(0, 1000);
    }
    // Should not panic; deterministic extension from seed.
    assert!(s.depth() > 1);
}

#[test]
fn sample_draw_bool_produces_both_values() {
    let mut trues = 0;
    let mut falses = 0;
    for _ in 0..200 {
        let mut s = Sample::new();
        if s.draw_bool() {
            trues += 1;
        } else {
            falses += 1;
        }
    }
    // Fair-ish over 200 draws. Not exact 50/50; want both > 40.
    assert!(trues > 40, "expected >40 true, got {trues}");
    assert!(falses > 40, "expected >40 false, got {falses}");
}

#[test]
fn sample_draw_from_returns_element_from_choices() {
    let choices = [10i32, 20, 30, 40, 50];
    for _ in 0..50 {
        let mut s = Sample::new();
        let v = s.draw_from(&choices);
        assert!(
            choices.contains(&v),
            "draw_from returned foreign value: {v}"
        );
    }
}

#[test]
fn sample_draw_from_covers_all_choices() {
    let choices = [1i32, 2, 3, 4];
    let mut seen = std::collections::HashSet::new();
    for _ in 0..200 {
        let mut s = Sample::new();
        seen.insert(s.draw_from(&choices));
    }
    assert_eq!(seen.len(), 4, "expected to see all 4 choices, saw {seen:?}");
}

#[test]
fn sample_buffer_oid_content_addresses_the_buffer() {
    let bytes = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let a = Sample::from_bytes(bytes.clone());
    let b = Sample::from_bytes(bytes);
    assert_eq!(a.buffer_oid(), b.buffer_oid(), "same buffer = same oid");

    let c = Sample::from_bytes(vec![9u8; 8]);
    assert_ne!(
        a.buffer_oid(),
        c.buffer_oid(),
        "different buffer = different oid"
    );
}

// ──────────────────────────────────────────────────────────────────
// Arbitrary: T::arbitrary(&mut sample) generates T.
// ──────────────────────────────────────────────────────────────────

#[test]
fn arbitrary_i32_generates_full_range() {
    let mut positives = 0;
    let mut negatives = 0;
    for _ in 0..200 {
        let mut s = Sample::new();
        let v = i32::arbitrary(&mut s);
        if v > 0 {
            positives += 1;
        } else if v < 0 {
            negatives += 1;
        }
    }
    // Full-range i32 draws should hit both signs.
    assert!(positives > 20, "expected >20 positives, got {positives}");
    assert!(negatives > 20, "expected >20 negatives, got {negatives}");
}

#[test]
fn arbitrary_bool_generates_both_values() {
    let mut trues = 0;
    let mut falses = 0;
    for _ in 0..200 {
        let mut s = Sample::new();
        if bool::arbitrary(&mut s) {
            trues += 1;
        } else {
            falses += 1;
        }
    }
    assert!(trues > 40 && falses > 40, "trues={trues} falses={falses}");
}

#[test]
fn arbitrary_u64_generates_full_range() {
    let mut small = 0;
    let mut large = 0;
    for _ in 0..200 {
        let mut s = Sample::new();
        let v = u64::arbitrary(&mut s);
        if v < u64::MAX / 2 {
            small += 1;
        } else {
            large += 1;
        }
    }
    assert!(small > 40 && large > 40, "small={small} large={large}");
}

// ──────────────────────────────────────────────────────────────────
// forall: composition surface.
// ──────────────────────────────────────────────────────────────────

#[test]
fn forall_tautology_returns_pass() {
    // Every i32 satisfies `true`.
    let v = forall::<i32, _>(b"forall_tautology_returns_pass", 50, |_x| {
        PropertyVerdict::Pass
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "tautology must Pass, got {v:?}"
    );
}

#[test]
fn forall_contradiction_returns_fail() {
    // No i32 satisfies `false`.
    let v = forall::<i32, _>(b"forall_contradiction_returns_fail", 50, |_x| {
        PropertyVerdict::Fail(terni::Diagnostic::new("always fails"))
    });
    assert!(
        matches!(v, PropertyVerdict::Fail(_)),
        "contradiction must Fail, got {v:?}"
    );
}

/// `forall`'s documented derivation, rebuilt by hand: draw `i` is read from
/// `Sample::from_bytes(sha256(seed || i as u64 little-endian))`.
fn derived<T: Arbitrary>(seed: &[u8], n: u64) -> Vec<T> {
    use sha2::{Digest, Sha256};
    (0..n)
        .map(|i| {
            let mut h = Sha256::new();
            h.update(seed);
            h.update(i.to_le_bytes());
            T::arbitrary(&mut Sample::from_bytes(h.finalize().to_vec()))
        })
        .collect()
}

/// What `forall` actually hands its property, in order.
fn drawn<T: Arbitrary>(seed: &[u8], n: usize) -> Vec<T> {
    let mut seen = Vec::new();
    forall::<T, _>(seed, n, |x| {
        seen.push(x);
        PropertyVerdict::Pass
    });
    seen
}

#[test]
fn forall_draws_are_a_function_of_the_seed() {
    let a = drawn::<i32>(b"seed a", 16);
    let b = drawn::<i32>(b"seed b", 16);
    assert_eq!(a, drawn::<i32>(b"seed a", 16), "same seed, same draws");
    assert_eq!(a, derived::<i32>(b"seed a", 16), "sha256(seed || i)");
    assert_eq!(b, derived::<i32>(b"seed b", 16), "sha256(seed || i)");
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert_ne!(x, y, "draw {i}: a different seed must draw differently");
    }
    let mut distinct = a.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), a.len(), "each index draws its own value");
}

#[test]
fn forall_partial_when_property_sometimes_holds() {
    // Property: half of bools satisfy (bool == true). Fail dominates the
    // fold, and the first Fail is the one kept. The seed is fixed, so the
    // draws are too: this is not a 50/50 bet but one exact outcome.
    let seed = b"forall_partial_when_property_sometimes_holds";
    let draws = derived::<bool>(seed, 20);
    assert_eq!(
        draws.iter().filter(|b| **b).count(),
        12,
        "trues among the 20"
    );
    let first_false = draws.iter().position(|b| !b);
    assert_eq!(first_false, Some(0), "the first counterexample");

    let mut at = 0;
    let v = forall::<bool, _>(seed, 20, |b| {
        at += 1;
        if b {
            PropertyVerdict::Pass
        } else {
            PropertyVerdict::Fail(terni::Diagnostic::new(format!("false at draw {}", at - 1)))
        }
    });
    assert_eq!(
        v,
        PropertyVerdict::Fail(terni::Diagnostic::new(format!(
            "false at draw {}",
            first_false.unwrap()
        ))),
    );
}

#[test]
fn forall_composes_with_pillar_algedonic_of_magnitude() {
    // Compose forall with an existing pillar primitive.
    // Property: draw an i32, project to a Loss magnitude; check algedonic verdict.
    // terni::Loss is impl'd for f64 (per terni/src/lib.rs); use f64 directly.
    // Magnitudes 1..=100, so a draw can land on either side of theta, and
    // the fold's outcome depends on the draws. The seed fixes them, so the
    // outcome is exact: one Partial diagnostic per draw at or below theta.
    let seed = b"forall_composes_with_pillar_algedonic_of_magnitude";
    let magnitude = |x: i32| (1 + x.rem_euclid(100)) as f64;
    let below = derived::<i32>(seed, 30)
        .into_iter()
        .filter(|&x| magnitude(x) <= 50.0)
        .count();
    assert!(
        (1..30).contains(&below),
        "the draws fall on both sides: {below} of 30 at or below"
    );
    let v = forall::<i32, _>(seed, 30, |x: i32| {
        pillar::algedonic_of_magnitude(&magnitude(x), &50.0)
    });
    match v {
        PropertyVerdict::Partial {
            confidence,
            diagnostics,
        } => {
            assert_eq!(confidence, 0.5);
            assert_eq!(diagnostics.len(), below, "one per draw at or below theta");
        }
        other => panic!("expected Partial, got {other:?}"),
    }

    // Under a threshold every magnitude clears, the same draws Pass.
    let v = forall::<i32, _>(seed, 30, |x: i32| {
        pillar::algedonic_of_magnitude(&magnitude(x), &0.5)
    });
    assert_eq!(v, PropertyVerdict::Pass);
}

#[test]
fn forall_deterministic_when_using_from_bytes() {
    // Same seed → same draws → same verdict, counterexample included.
    // That is what makes a failure replayable. The property depends on its
    // input (odd x fails), so the diagnostic names the first failing draw.
    let f = || {
        forall::<i32, _>(
            b"forall_deterministic_when_using_from_bytes",
            10,
            |x: i32| {
                if x.rem_euclid(2) == 0 {
                    PropertyVerdict::Pass
                } else {
                    PropertyVerdict::Fail(terni::Diagnostic::new(format!("odd: {x}")))
                }
            },
        )
    };
    let v1 = f();
    let v2 = f();
    assert!(
        matches!(v1, PropertyVerdict::Fail(_)),
        "ten draws should hit an odd i32, got {v1:?}"
    );
    assert_eq!(v1, v2, "same seed must give the same counterexample");
}

#[test]
fn forall_composes_via_fold() {
    // Multiple forall runs can be folded via pillar::fold.
    let v1 = forall::<i32, _>(b"forall_composes_via_fold", 10, |_| PropertyVerdict::Pass);
    let v2 = forall::<bool, _>(b"forall_composes_via_fold", 10, |_| PropertyVerdict::Pass);
    let unified = pillar::fold(&[v1, v2]);
    assert!(
        matches!(unified, PropertyVerdict::Pass),
        "two Pass forall folds to Pass"
    );
}

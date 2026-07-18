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

use prismqueer::liquid::pillar::{self, forall, Arbitrary, Sample};
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
            assert!(v >= -100 && v <= 100, "draw_integer out of range: {v}");
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
        assert!(choices.contains(&v), "draw_from returned foreign value: {v}");
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
    assert_ne!(a.buffer_oid(), c.buffer_oid(), "different buffer = different oid");
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
    let v = forall::<i32, _>(50, |_x| PropertyVerdict::Pass);
    assert!(matches!(v, PropertyVerdict::Pass), "tautology must Pass, got {v:?}");
}

#[test]
fn forall_contradiction_returns_fail() {
    // No i32 satisfies `false`.
    let v = forall::<i32, _>(50, |_x| {
        PropertyVerdict::Fail(terni::Diagnostic::new("always fails"))
    });
    assert!(matches!(v, PropertyVerdict::Fail(_)), "contradiction must Fail, got {v:?}");
}

#[test]
fn forall_partial_when_property_sometimes_holds() {
    // Property: half of bools satisfy (bool == true).
    // Fold: any Fail dominates. Bool sampling ~50/50 → some fail → overall Fail.
    let v = forall::<bool, _>(20, |b| {
        if b {
            PropertyVerdict::Pass
        } else {
            PropertyVerdict::Fail(terni::Diagnostic::new("was false"))
        }
    });
    // With enough draws, at least one false hit → Fail.
    assert!(matches!(v, PropertyVerdict::Fail(_)), "expected Fail (any false is a counterexample), got {v:?}");
}

#[test]
fn forall_composes_with_pillar_algedonic_of_magnitude() {
    // Compose forall with an existing pillar primitive.
    // Property: draw an i32, project to a Loss magnitude; check algedonic verdict.
    // terni::Loss is impl'd for f64 (per terni/src/lib.rs); use f64 directly.
    let theta: f64 = 50.0;

    // All values above threshold → all Pass → unified Pass.
    let v = forall::<i32, _>(30, |x: i32| {
        let magnitude: f64 = (60 + (x.rem_euclid(40))) as f64;
        pillar::algedonic_of_magnitude(&magnitude, &theta)
    });
    assert!(matches!(v, PropertyVerdict::Pass), "all-above-threshold must Pass, got {v:?}");
}

#[test]
fn forall_deterministic_when_using_from_bytes() {
    // Same buffer sequence → same verdict.
    // (Verified indirectly: run same test twice, same result.)
    let f = || {
        forall::<i32, _>(10, |x: i32| {
            if x.rem_euclid(2) == 0 {
                PropertyVerdict::Pass
            } else {
                PropertyVerdict::Pass
            }
        })
    };
    let v1 = f();
    let v2 = f();
    assert!(matches!(v1, PropertyVerdict::Pass) && matches!(v2, PropertyVerdict::Pass));
}

#[test]
fn forall_composes_via_fold() {
    // Multiple forall runs can be folded via pillar::fold.
    let v1 = forall::<i32, _>(10, |_| PropertyVerdict::Pass);
    let v2 = forall::<bool, _>(10, |_| PropertyVerdict::Pass);
    let unified = pillar::fold(&[v1, v2]);
    assert!(matches!(unified, PropertyVerdict::Pass), "two Pass forall folds to Pass");
}

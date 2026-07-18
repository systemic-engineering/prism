//! Ouroboros first layer — prismqueer testing prismqueer with its own
//! liquid module.
//!
//! This is the deep substrate work. `prismqueer::liquid` composes over
//! prismqueer's Bundle tower + terni's verdict machinery to produce
//! spectral-commutator property verdicts. Here, we witness that the
//! `LiquidConnection::commutator_magnitude` blanket impl satisfies:
//!
//! 1. Antisymmetry — `‖[A, B]‖ == ‖[B, A]‖` for any pair.
//! 2. Self-annihilation — `‖[A, A]‖ == 0` for any single connection.
//! 3. Non-negativity — magnitudes are always `>= 0`.
//! 4. Triangle inequality — inherited from `Metric`.
//! 5. Abelian vanishing — `Cyclic<N>`-gauged bundles have `‖[A, B]‖ == 0`
//!    for any state, because cyclic groups commute.
//!
//! And that the pillar verdicts (`dispatch_ambiguity`, `algedonic`,
//! `viability`) return the correct `PropertyVerdict` for each case.
//!
//! # Substrate-honesty notice
//!
//! Every claim in this file is empirical. No test uses `.unwrap()` on
//! something the code doesn't actually produce, no assertion is loosened
//! to "just make it green." If a claim doesn't hold, the test fails
//! loudly with the actual observed values in the panic message.
//!
//! Mathematical foundation: `mirror/docs/math/spectral-commutator-four-pillars.md`
//! (Mara `5d3040d`). Operational spec:
//! `mirror/docs/specs/spectral-commutator-as-cybernetic-ground.md` (Mara `3cd9a42`).

#![cfg(feature = "bundle")]

use prismqueer::bundle::examples::{LiquidTestBundle, TestBundle};
use prismqueer::liquid::prelude::*;
use prismqueer::{Loss, Metric, ScalarLoss};

// ──────────────────────────────────────────────────────────────────
// 1. Antisymmetry — |[A, B]| == |[B, A]|.
// ──────────────────────────────────────────────────────────────────

#[test]
/// The commutator is symmetric under argument swap for TestBundle
/// (abelian Cyclic gauge, state-dependent loss). Both sides are zero
/// because Cyclic commutes AND the transport loss depends only on state.
fn commutator_antisymmetric_over_test_bundle_all_strategy_pairs() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        for j in 0..4u8 {
            let a = TestBundle::with_strategy(i);
            let b = TestBundle::with_strategy(j);
            let ab = LiquidConnection::commutator_magnitude(&a, &b, &state);
            let ba = LiquidConnection::commutator_magnitude(&b, &a, &state);
            assert_eq!(
                ab, ba,
                "antisymmetry failed for TestBundle strategies i={i} j={j}: \
                |[A,B]|={ab:?} vs |[B,A]|={ba:?}",
            );
        }
    }
}

#[test]
/// The commutator is symmetric under argument swap for LiquidTestBundle
/// (bundle-dependent loss). For different strategies, magnitudes are
/// non-zero but still equal in the two orderings.
fn commutator_antisymmetric_over_liquid_bundle_all_strategy_pairs() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        for j in 0..4u8 {
            let a = LiquidTestBundle::with_strategy(i);
            let b = LiquidTestBundle::with_strategy(j);
            let ab = LiquidConnection::commutator_magnitude(&a, &b, &state);
            let ba = LiquidConnection::commutator_magnitude(&b, &a, &state);
            assert_eq!(
                ab, ba,
                "antisymmetry failed for LiquidTestBundle strategies i={i} j={j}: \
                |[A,B]|={ab:?} vs |[B,A]|={ba:?}",
            );
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// 2. Self-annihilation — |[A, A]| == 0.
// ──────────────────────────────────────────────────────────────────

#[test]
/// `commutator_magnitude(a, a, state)` returns `Loss::zero()` for any
/// strategy — for both TestBundle and LiquidTestBundle. Because the two
/// holonomies are identical, their Metric distance is zero.
fn commutator_self_annihilates_over_test_bundle() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        let a = TestBundle::with_strategy(i);
        let magnitude = LiquidConnection::commutator_magnitude(&a, &a, &state);
        assert!(
            magnitude.is_zero(),
            "self-annihilation failed for TestBundle strategy {i}: \
            |[A,A]| = {magnitude:?}, expected Loss::zero()",
        );
    }
}

#[test]
fn commutator_self_annihilates_over_liquid_bundle() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        let a = LiquidTestBundle::with_strategy(i);
        let magnitude = LiquidConnection::commutator_magnitude(&a, &a, &state);
        assert!(
            magnitude.is_zero(),
            "self-annihilation failed for LiquidTestBundle strategy {i}: \
            |[A,A]| = {magnitude:?}, expected Loss::zero()",
        );
    }
}

// ──────────────────────────────────────────────────────────────────
// 3. Non-negativity — ‖[A, B]‖ satisfies Metric::is_non_negative.
// ──────────────────────────────────────────────────────────────────

#[test]
/// The `ScalarLoss` returned by `commutator_magnitude` reports itself as
/// non-negative via the `Metric` axiom.
fn commutator_magnitude_is_non_negative() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        for j in 0..4u8 {
            let a = LiquidTestBundle::with_strategy(i);
            let b = LiquidTestBundle::with_strategy(j);
            let m = LiquidConnection::commutator_magnitude(&a, &b, &state);
            assert!(
                m.is_non_negative(),
                "non-negativity failed for strategies i={i} j={j}: {m:?}",
            );
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// 4. Triangle inequality — inherited from Metric.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Direct triangle inequality on the three commutator magnitudes
/// `d(a,b), d(b,c), d(a,c)` for LiquidTestBundle triples.
fn commutator_magnitude_satisfies_triangle_inequality() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        for j in 0..4u8 {
            for k in 0..4u8 {
                let a = LiquidTestBundle::with_strategy(i);
                let b = LiquidTestBundle::with_strategy(j);
                let c = LiquidTestBundle::with_strategy(k);
                let ab = LiquidConnection::commutator_magnitude(&a, &b, &state);
                let bc = LiquidConnection::commutator_magnitude(&b, &c, &state);
                let ac = LiquidConnection::commutator_magnitude(&a, &c, &state);
                assert!(
                    ab.triangle(&bc, &ac),
                    "triangle failed for i={i} j={j} k={k}: \
                    ab={ab:?} bc={bc:?} ac={ac:?}",
                );
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// 5. Abelian vanishing — Cyclic<N> gauged bundles have commutator zero.
// ──────────────────────────────────────────────────────────────────

#[test]
/// The mathematical claim: `Cyclic<N>` is abelian, so `[A, B] = 0` for
/// any pair. For TestBundle (state-dependent loss), this is doubly true
/// because the loss also matches on both sides. Witness it explicitly.
fn commutator_vanishes_for_abelian_gauge_test_bundle() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        for j in 0..4u8 {
            let a = TestBundle::with_strategy(i);
            let b = TestBundle::with_strategy(j);
            let magnitude = LiquidConnection::commutator_magnitude(&a, &b, &state);
            assert!(
                magnitude.is_zero(),
                "abelian-vanishing failed for TestBundle i={i} j={j}: {magnitude:?}",
            );
        }
    }
}

#[test]
/// `commutator_norm` at the Default state is trivially zero for
/// TestBundle because Default `[0.0; 4]` gives transport loss zero on
/// both sides. This is exactly what the mirror-side RED at 028ccc2
/// witnesses.
fn commutator_norm_zero_over_default_state_for_test_bundle() {
    let a = TestBundle::default();
    let b = TestBundle::default();
    let norm = commutator_norm(&a, &b);
    assert!(norm.is_zero(), "commutator_norm over Default state = {norm:?}, expected zero");
}

// ──────────────────────────────────────────────────────────────────
// 6. LiquidTestBundle actually produces non-vanishing commutators.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Sanity: LiquidTestBundle with strategies 2 and 5 produces magnitude
/// exactly `|2 - 5| = 3` (bundle-dependent loss).
///
/// Note `Cyclic<4>::new(5) == Cyclic(5 % 4) == Cyclic(1)`, so
/// `|2 - 1| = 1`. Adjust expectation to match constructor reduction.
fn liquid_bundle_commutator_matches_scalar_loss_computation() {
    let a = LiquidTestBundle::with_strategy(2);
    let b = LiquidTestBundle::with_strategy(5); // reduces to 1
    let state = [1.0, 2.0, 3.0, 4.0];
    let m = LiquidConnection::commutator_magnitude(&a, &b, &state);
    let expected = ScalarLoss::new(2.0).distance_to(&ScalarLoss::new(1.0));
    assert_eq!(m, expected, "got {m:?}, expected {expected:?}");
    assert!(!m.is_zero(), "expected non-vanishing commutator; got {m:?}");
}

// ──────────────────────────────────────────────────────────────────
// 7. Commutator struct deferred-magnitude computation.
// ──────────────────────────────────────────────────────────────────

#[test]
fn commutator_struct_computes_magnitude_lazily() {
    let a = LiquidTestBundle::with_strategy(1);
    let b = LiquidTestBundle::with_strategy(3);
    let state = [1.0, 2.0, 3.0, 4.0];
    let c = commutator(&a, &b, &state);
    let m1 = c.magnitude();
    let m2 = c.magnitude();
    assert_eq!(m1, m2, "repeated .magnitude() must be deterministic");
    let direct = LiquidConnection::commutator_magnitude(&a, &b, &state);
    assert_eq!(m1, direct, "Commutator::magnitude must equal direct call");
}

// ──────────────────────────────────────────────────────────────────
// 8. Pillar I — dispatch_ambiguity byte-visible checks.
// ──────────────────────────────────────────────────────────────────

#[test]
fn pillar_dispatch_ambiguity_pass_when_all_conditions_met() {
    let verdict = pillar::dispatch_ambiguity(3, 3, true, true);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
fn pillar_dispatch_ambiguity_fail_when_fewer_than_two_arms() {
    let verdict = pillar::dispatch_ambiguity(1, 1, true, true);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

#[test]
fn pillar_dispatch_ambiguity_fail_when_witness_count_mismatches() {
    let verdict = pillar::dispatch_ambiguity(3, 2, true, true);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

#[test]
fn pillar_dispatch_ambiguity_fail_when_tie_breaking_not_exhausted() {
    let verdict = pillar::dispatch_ambiguity(3, 3, false, true);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

#[test]
fn pillar_dispatch_ambiguity_fail_when_pivot_song_missing() {
    let verdict = pillar::dispatch_ambiguity(3, 3, true, false);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

// ──────────────────────────────────────────────────────────────────
// 9. Pillar II — algedonic threshold.
// ──────────────────────────────────────────────────────────────────

#[test]
fn pillar_algedonic_pass_above_threshold() {
    let a = LiquidTestBundle::with_strategy(0);
    let b = LiquidTestBundle::with_strategy(3);
    let state = [1.0; 4];
    let c = commutator(&a, &b, &state);
    // magnitude = |0 - 3| = 3.0, theta = 1.0 → Pass
    let theta = ScalarLoss::new(1.0);
    let verdict = pillar::algedonic(&c, &theta);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
fn pillar_algedonic_partial_below_threshold_but_nonzero() {
    let a = LiquidTestBundle::with_strategy(0);
    let b = LiquidTestBundle::with_strategy(1);
    let state = [1.0; 4];
    let c = commutator(&a, &b, &state);
    // magnitude = 1.0, theta = 5.0 → Partial
    let theta = ScalarLoss::new(5.0);
    let verdict = pillar::algedonic(&c, &theta);
    match verdict {
        PropertyVerdict::Partial { confidence, .. } => {
            assert!((0.0..=1.0).contains(&confidence), "confidence out of range: {confidence}");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

#[test]
fn pillar_algedonic_fail_when_commutator_vanishes() {
    // Same strategy → magnitude 0 → Fail
    let a = LiquidTestBundle::with_strategy(2);
    let b = LiquidTestBundle::with_strategy(2);
    let state = [1.0; 4];
    let c = commutator(&a, &b, &state);
    let theta = ScalarLoss::new(0.5);
    let verdict = pillar::algedonic(&c, &theta);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

// ──────────────────────────────────────────────────────────────────
// 10. Pillar III — viability persistence.
// ──────────────────────────────────────────────────────────────────

#[test]
fn pillar_viability_pass_when_accumulated_exceeds_threshold() {
    let a = LiquidTestBundle::with_strategy(0);
    let b = LiquidTestBundle::with_strategy(3);
    let state = [1.0; 4];
    // Each commutator has magnitude 3.0; three of them combine
    // (ScalarLoss.combine = a+b) to 9.0. Threshold 5.0 → Pass.
    let c1 = commutator(&a, &b, &state);
    let c2 = commutator(&a, &b, &state);
    let c3 = commutator(&a, &b, &state);
    let history = vec![c1, c2, c3];
    let theta = ScalarLoss::new(5.0);
    let verdict = pillar::viability(&history, &theta, 3);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
fn pillar_viability_fail_when_accumulated_below_threshold() {
    let a = LiquidTestBundle::with_strategy(0);
    let b = LiquidTestBundle::with_strategy(1);
    let state = [1.0; 4];
    // Each commutator magnitude 1.0; three → 3.0. Threshold 10.0 → Fail.
    let c1 = commutator(&a, &b, &state);
    let c2 = commutator(&a, &b, &state);
    let c3 = commutator(&a, &b, &state);
    let history = vec![c1, c2, c3];
    let theta = ScalarLoss::new(10.0);
    let verdict = pillar::viability(&history, &theta, 3);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

#[test]
fn pillar_viability_partial_when_history_shorter_than_window() {
    let a = LiquidTestBundle::with_strategy(0);
    let b = LiquidTestBundle::with_strategy(3);
    let state = [1.0; 4];
    let c1 = commutator(&a, &b, &state);
    let history = vec![c1];
    let theta = ScalarLoss::new(1.0);
    // history.len() = 1 < omega = 5 → Partial with confidence 1/5.
    let verdict = pillar::viability(&history, &theta, 5);
    match verdict {
        PropertyVerdict::Partial { confidence, .. } => {
            assert!((confidence - 0.2).abs() < 1e-9, "confidence = {confidence}, expected 0.2");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 11. Prelude ergonomics — the delightful use-line compiles.
// ──────────────────────────────────────────────────────────────────

#[test]
/// If this compiles, the prelude re-exports the intended surface:
/// `commutator`, `commutator_norm`, `Commutator`, `LiquidConnection`,
/// `pillar`, `Diagnostic`, `PropertyVerdict`.
fn prelude_reexports_delightful_surface() {
    fn _uses_prelude() {
        let _a = LiquidTestBundle::with_strategy(0);
        let _b = LiquidTestBundle::with_strategy(1);
        let state = [1.0; 4];
        let c = commutator(&_a, &_b, &state);
        let _m = c.magnitude();
        let _v = pillar::dispatch_ambiguity(2, 2, true, true);
        let _d: Diagnostic = Diagnostic::new("probe");
        let _p: PropertyVerdict = PropertyVerdict::Pass;
    }
    _uses_prelude();
}

//! Ouroboros first layer — prismqueer testing prismqueer with its own
//! liquid module.
//!
//! This is the deep substrate work. `prismqueer::liquid` composes over
//! prismqueer's Bundle tower + terni's verdict machinery to produce
//! spectral-commutator property verdicts. Here, we witness that the
//! `FluxThread::commutator_magnitude` blanket impl satisfies:
//!
//! 1. Antisymmetry — `‖[A, B]‖ == ‖[B, A]‖` for any pair.
//! 2. Self-annihilation — `‖[A, A]‖ == 0` for any single connection.
//! 3. Non-negativity — magnitudes are always `>= 0`.
//! 4. Triangle inequality — inherited from `Metric`.
//! 5. Abelian vanishing — `Cyclic<N>`-gauged bundles have `‖[A, B]‖ == 0`
//!    for any state, because cyclic groups commute.
//!
//! And that the pillar verdicts (`dispatch_ambiguity`, `viability`) return
//! the correct `PropertyVerdict` for each case, and that `cross` certifies
//! a commutator's crossing (In | Out | Undecided).
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

use num_rational::BigRational;
use prismqueer::algedonic::{Enclosure, Membership, Region};
use prismqueer::bundle::examples::{FluxConstraint, Perm3, PermBundle, TestBundle};
use prismqueer::bundle::GroupStructure;
use prismqueer::flux::prelude::*;
use prismqueer::{Loss, Metric, ScalarLoss};

/// A float, exactly: the dyadic rational it is.
fn rational(x: f64) -> BigRational {
    BigRational::from_float(x).expect("finite")
}

/// The enclosure of a magnitude these fixtures compute exactly. Their
/// commutators are differences of small integers, so the float is the
/// value, and the point enclosure is certified. A float from a computation
/// that rounds would need its instrument's error bound instead.
fn exactly(m: ScalarLoss) -> Enclosure {
    Enclosure::exact(rational(m.as_f64()))
}

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
            let ab = FluxThread::commutator_magnitude(&a, &b, &state);
            let ba = FluxThread::commutator_magnitude(&b, &a, &state);
            assert_eq!(
                ab, ba,
                "antisymmetry failed for TestBundle strategies i={i} j={j}: \
                |[A,B]|={ab:?} vs |[B,A]|={ba:?}",
            );
        }
    }
}

#[test]
/// The commutator is symmetric under argument swap for FluxConstraint
/// (bundle-dependent loss). For different strategies, magnitudes are
/// non-zero but still equal in the two orderings.
fn commutator_antisymmetric_over_liquid_bundle_all_strategy_pairs() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        for j in 0..4u8 {
            let a = FluxConstraint::with_strategy(i);
            let b = FluxConstraint::with_strategy(j);
            let ab = FluxThread::commutator_magnitude(&a, &b, &state);
            let ba = FluxThread::commutator_magnitude(&b, &a, &state);
            assert_eq!(
                ab, ba,
                "antisymmetry failed for FluxConstraint strategies i={i} j={j}: \
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
/// strategy — for both TestBundle and FluxConstraint. Because the two
/// holonomies are identical, their Metric distance is zero.
fn commutator_self_annihilates_over_test_bundle() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        let a = TestBundle::with_strategy(i);
        let magnitude = FluxThread::commutator_magnitude(&a, &a, &state);
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
        let a = FluxConstraint::with_strategy(i);
        let magnitude = FluxThread::commutator_magnitude(&a, &a, &state);
        assert!(
            magnitude.is_zero(),
            "self-annihilation failed for FluxConstraint strategy {i}: \
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
            let a = FluxConstraint::with_strategy(i);
            let b = FluxConstraint::with_strategy(j);
            let m = FluxThread::commutator_magnitude(&a, &b, &state);
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
/// `d(a,b), d(b,c), d(a,c)` for FluxConstraint triples.
fn commutator_magnitude_satisfies_triangle_inequality() {
    let state = [1.0, 2.0, 3.0, 4.0];
    for i in 0..4u8 {
        for j in 0..4u8 {
            for k in 0..4u8 {
                let a = FluxConstraint::with_strategy(i);
                let b = FluxConstraint::with_strategy(j);
                let c = FluxConstraint::with_strategy(k);
                let ab = FluxThread::commutator_magnitude(&a, &b, &state);
                let bc = FluxThread::commutator_magnitude(&b, &c, &state);
                let ac = FluxThread::commutator_magnitude(&a, &c, &state);
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
            let magnitude = FluxThread::commutator_magnitude(&a, &b, &state);
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
    assert!(
        norm.is_zero(),
        "commutator_norm over Default state = {norm:?}, expected zero"
    );
}

#[test]
/// commutator_norm is the magnitude at the Default state, not zero by
/// construction: two FluxConstraints two strategies apart read 2 there.
fn commutator_norm_is_the_magnitude_at_the_default_state() {
    let a = FluxConstraint::with_strategy(0);
    let b = FluxConstraint::with_strategy(2);
    let norm = commutator_norm(&a, &b);
    assert_eq!(norm, ScalarLoss::new(2.0), "|0 - 2| at [0.0; 4]");
    assert_eq!(norm, FluxThread::commutator_magnitude(&a, &b, &[0.0; 4]));
}

// ──────────────────────────────────────────────────────────────────
// 6. FluxConstraint actually produces non-vanishing commutators.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Sanity: FluxConstraint with strategies 2 and 5 produces magnitude
/// exactly `|2 - 5| = 3` (bundle-dependent loss).
///
/// Note `Cyclic<4>::new(5) == Cyclic(5 % 4) == Cyclic(1)`, so
/// `|2 - 1| = 1`. Adjust expectation to match constructor reduction.
fn liquid_bundle_commutator_matches_scalar_loss_computation() {
    let a = FluxConstraint::with_strategy(2);
    let b = FluxConstraint::with_strategy(5); // reduces to 1
    let state = [1.0, 2.0, 3.0, 4.0];
    let m = FluxThread::commutator_magnitude(&a, &b, &state);
    let expected = ScalarLoss::new(2.0).distance_to(&ScalarLoss::new(1.0));
    assert_eq!(m, expected, "got {m:?}, expected {expected:?}");
    assert!(!m.is_zero(), "expected non-vanishing commutator; got {m:?}");
}

// ──────────────────────────────────────────────────────────────────
// 7. Commutator struct deferred-magnitude computation.
// ──────────────────────────────────────────────────────────────────

#[test]
fn commutator_struct_computes_magnitude_lazily() {
    let a = FluxConstraint::with_strategy(1);
    let b = FluxConstraint::with_strategy(3);
    let state = [1.0, 2.0, 3.0, 4.0];
    let c = commutator(&a, &b, &state);
    let m1 = c.magnitude();
    let m2 = c.magnitude();
    assert_eq!(m1, m2, "repeated .magnitude() must be deterministic");
    let direct = FluxThread::commutator_magnitude(&a, &b, &state);
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
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "got {verdict:?}"
    );
}

#[test]
fn pillar_dispatch_ambiguity_fail_when_witness_count_mismatches() {
    let verdict = pillar::dispatch_ambiguity(3, 2, true, true);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "got {verdict:?}"
    );
}

#[test]
fn pillar_dispatch_ambiguity_fail_when_tie_breaking_not_exhausted() {
    let verdict = pillar::dispatch_ambiguity(3, 3, false, true);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "got {verdict:?}"
    );
}

#[test]
fn pillar_dispatch_ambiguity_fail_when_pivot_song_missing() {
    let verdict = pillar::dispatch_ambiguity(3, 3, true, false);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "got {verdict:?}"
    );
}

// ──────────────────────────────────────────────────────────────────
// 9. Pillar II — the commutator's certified crossing.
// ──────────────────────────────────────────────────────────────────

#[test]
fn pillar_cross_in_above_threshold() {
    let a = FluxConstraint::with_strategy(0);
    let b = FluxConstraint::with_strategy(3);
    let state = [1.0; 4];
    let c = commutator(&a, &b, &state);
    // magnitude = |0 - 3| = 3, region (1, ∞) → In
    let verdict = pillar::cross(&exactly(c.magnitude()), &Region::above(rational(1.0)));
    assert!(matches!(verdict, Membership::In(_)), "got {verdict:?}");
}

#[test]
/// The region is open: an exact magnitude at theta is certified Out, and an
/// exact magnitude one ulp above it is In. Exact, because the value is.
fn pillar_cross_at_the_threshold_is_out() {
    let a = FluxConstraint::with_strategy(0);
    let b = FluxConstraint::with_strategy(3);
    let c = commutator(&a, &b, &[1.0; 4]);
    let m = exactly(c.magnitude());
    let verdict = pillar::cross(&m, &Region::above(rational(3.0)));
    assert!(matches!(verdict, Membership::Out(_)), "got {verdict:?}");
    let verdict = pillar::cross(&m, &Region::above(rational(3.0f64.next_down())));
    assert!(matches!(verdict, Membership::In(_)), "got {verdict:?}");
}

#[test]
/// A signal below the threshold is certified below: Out, a decided claim,
/// never a `Partial` with a confidence.
fn pillar_cross_out_below_threshold_but_nonzero() {
    let a = FluxConstraint::with_strategy(0);
    let b = FluxConstraint::with_strategy(1);
    let state = [1.0; 4];
    let c = commutator(&a, &b, &state);
    // magnitude = 1, region (5, ∞) → Out
    let verdict = pillar::cross(&exactly(c.magnitude()), &Region::above(rational(5.0)));
    assert!(matches!(verdict, Membership::Out(_)), "got {verdict:?}");
}

#[test]
/// A vanished commutator is a reading, zero: certified Out of (½, ∞).
fn pillar_cross_reads_a_vanished_commutator_as_zero() {
    let a = FluxConstraint::with_strategy(2);
    let b = FluxConstraint::with_strategy(2);
    let state = [1.0; 4];
    let c = commutator(&a, &b, &state);
    let m = exactly(c.magnitude());
    assert_eq!(m, Enclosure::exact(rational(0.0)));
    let verdict = pillar::cross(&m, &Region::above(rational(0.5)));
    assert!(matches!(verdict, Membership::Out(_)), "got {verdict:?}");
}

// ──────────────────────────────────────────────────────────────────
// 10. Pillar III — viability persistence.
// ──────────────────────────────────────────────────────────────────

#[test]
fn pillar_viability_pass_when_accumulated_exceeds_threshold() {
    let a = FluxConstraint::with_strategy(0);
    let b = FluxConstraint::with_strategy(3);
    let state = [1.0; 4];
    // Each commutator has magnitude 3.0; three of them combine
    // (ScalarLoss.combine = a+b) to 9.0. Threshold 8.5 → Pass, and only
    // the whole window clears it: any two make 6.0.
    let c1 = commutator(&a, &b, &state);
    let c2 = commutator(&a, &b, &state);
    let c3 = commutator(&a, &b, &state);
    let history = vec![c1, c2, c3];
    let theta = ScalarLoss::new(8.5);
    let verdict = pillar::viability(&history, &theta, 3);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
fn pillar_viability_fail_when_accumulated_below_threshold() {
    let a = FluxConstraint::with_strategy(0);
    let b = FluxConstraint::with_strategy(1);
    let loud = FluxConstraint::with_strategy(3);
    let state = [1.0; 4];
    // Three loud commutators (3.0 each), then two quiet ones (1.0 each).
    // omega = 2 reads only the quiet tail: 2.0, exactly theta, and Pass is
    // strictly above it → Fail. Reading any loud one would Pass.
    let history = vec![
        commutator(&a, &loud, &state),
        commutator(&a, &loud, &state),
        commutator(&a, &loud, &state),
        commutator(&a, &b, &state),
        commutator(&a, &b, &state),
    ];
    let theta = ScalarLoss::new(2.0);
    let verdict = pillar::viability(&history, &theta, 2);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "got {verdict:?}"
    );
}

#[test]
fn pillar_viability_partial_when_history_shorter_than_window() {
    let a = FluxConstraint::with_strategy(0);
    let b = FluxConstraint::with_strategy(3);
    let state = [1.0; 4];
    let c1 = commutator(&a, &b, &state);
    let history = vec![c1];
    let theta = ScalarLoss::new(1.0);
    // history.len() = 1 < omega = 5 → Partial with confidence 1/5.
    let verdict = pillar::viability(&history, &theta, 5);
    match verdict {
        PropertyVerdict::Partial { confidence, .. } => {
            assert!(
                (confidence - 0.2).abs() < 1e-9,
                "confidence = {confidence}, expected 0.2"
            );
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 11. Prelude ergonomics — the delightful use-line compiles.
// ──────────────────────────────────────────────────────────────────

#[test]
/// If this compiles, the prelude re-exports the intended surface:
/// `commutator`, `commutator_norm`, `Commutator`, `FluxThread`,
/// `pillar`, `Diagnostic`, `PropertyVerdict`.
fn prelude_reexports_delightful_surface() {
    fn _uses_prelude() {
        let _a = FluxConstraint::with_strategy(0);
        let _b = FluxConstraint::with_strategy(1);
        let state = [1.0; 4];
        let c = commutator(&_a, &_b, &state);
        let _m = c.magnitude();
        let _v = pillar::dispatch_ambiguity(2, 2, true, true);
        assert_eq!(_v, PropertyVerdict::Pass, "two arms are enough");
        assert!(matches!(
            pillar::dispatch_ambiguity(1, 1, true, true),
            PropertyVerdict::Fail(_)
        ));
        let _d: Diagnostic = Diagnostic::new("probe");
        let _p: PropertyVerdict = PropertyVerdict::Pass;
    }
    _uses_prelude();
}

// ──────────────────────────────────────────────────────────────────
// 12. Perm3 group laws — the substrate-honest S3 witness.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Identity law: `id ∘ x = x = x ∘ id` for every element of S3.
fn perm3_identity_law() {
    let id = Perm3::identity();
    for &x in Perm3::ALL.iter() {
        assert_eq!(id.compose(&x), x, "left identity failed for {x:?}");
        assert_eq!(x.compose(&id), x, "right identity failed for {x:?}");
    }
}

#[test]
/// Inverse law: `x ∘ x⁻¹ = id = x⁻¹ ∘ x` for every element of S3.
fn perm3_inverse_law() {
    let id = Perm3::identity();
    for &x in Perm3::ALL.iter() {
        assert_eq!(
            x.compose(&x.inverse()),
            id,
            "right inverse failed for {x:?}"
        );
        assert_eq!(x.inverse().compose(&x), id, "left inverse failed for {x:?}");
    }
}

#[test]
/// Associativity: `(a ∘ b) ∘ c = a ∘ (b ∘ c)` across all 216 triples.
fn perm3_associativity() {
    for &a in Perm3::ALL.iter() {
        for &b in Perm3::ALL.iter() {
            for &c in Perm3::ALL.iter() {
                let lhs = a.compose(&b).compose(&c);
                let rhs = a.compose(&b.compose(&c));
                assert_eq!(lhs, rhs, "associativity failed: ({a:?} ∘ {b:?}) ∘ {c:?}");
            }
        }
    }
}

#[test]
/// S3 is non-abelian: witnessed at the smallest instance —
/// `(0 1) ∘ (1 2) ≠ (1 2) ∘ (0 1)`.
fn perm3_is_non_abelian_witnessed() {
    let ab = Perm3::SWAP_01.compose(&Perm3::SWAP_12);
    let ba = Perm3::SWAP_12.compose(&Perm3::SWAP_01);
    assert_ne!(ab, ba, "S3 non-abelian witness failed: ab={ab:?} ba={ba:?}");
    // Specifically: swap01 ∘ swap12 should be the 3-cycle CYCLE_012.
    assert_eq!(ab, Perm3::CYCLE_012, "swap01 ∘ swap12 = CYCLE_012");
    assert_eq!(ba, Perm3::CYCLE_021, "swap12 ∘ swap01 = CYCLE_021");
}

// ──────────────────────────────────────────────────────────────────
// 13. PermBundle commutator — non-abelian witness has real work.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Non-commuting permutations MUST produce non-vanishing commutator.
/// This is the substrate-honest deepening: iter 1 witnessed the
/// machinery on `Cyclic<N>` where all commutators trivially vanish;
/// iter 2 witnesses the machinery on `S3` where the commutator has
/// genuine mathematical work to do.
fn perm_bundle_commutator_nonzero_for_non_commuting_permutations() {
    let a = PermBundle::from_perm(Perm3::SWAP_01);
    let b = PermBundle::from_perm(Perm3::SWAP_12);
    let state = [10, 20, 30];
    let m = FluxThread::commutator_magnitude(&a, &b, &state);
    assert!(
        !m.is_zero(),
        "non-commuting perms MUST produce non-vanishing commutator; got {m:?}",
    );
}

#[test]
/// Identity commutes with every element; commutator must vanish.
fn perm_bundle_commutator_vanishes_when_one_gauge_is_identity() {
    let id = PermBundle::from_perm(Perm3::IDENTITY);
    let state = [10, 20, 30];
    for &x in Perm3::ALL.iter() {
        let bx = PermBundle::from_perm(x);
        let m1 = FluxThread::commutator_magnitude(&id, &bx, &state);
        let m2 = FluxThread::commutator_magnitude(&bx, &id, &state);
        assert!(m1.is_zero(), "[id, {x:?}] must vanish; got {m1:?}");
        assert!(m2.is_zero(), "[{x:?}, id] must vanish; got {m2:?}");
    }
}

#[test]
/// Same permutation trivially commutes with itself.
fn perm_bundle_commutator_self_annihilates_across_s3() {
    let state = [10, 20, 30];
    for &x in Perm3::ALL.iter() {
        let a = PermBundle::from_perm(x);
        let b = PermBundle::from_perm(x);
        let m = FluxThread::commutator_magnitude(&a, &b, &state);
        assert!(m.is_zero(), "self-annihilation failed for {x:?}: {m:?}");
    }
}

#[test]
/// Antisymmetry `‖[A, B]‖ == ‖[B, A]‖` holds across all 36 S3×S3 pairs.
fn perm_bundle_commutator_antisymmetric_across_full_s3_grid() {
    let state = [10, 20, 30];
    for &pa in Perm3::ALL.iter() {
        for &pb in Perm3::ALL.iter() {
            let a = PermBundle::from_perm(pa);
            let b = PermBundle::from_perm(pb);
            let ab = FluxThread::commutator_magnitude(&a, &b, &state);
            let ba = FluxThread::commutator_magnitude(&b, &a, &state);
            assert_eq!(
                ab, ba,
                "antisymmetry failed for {pa:?}/{pb:?}: |[A,B]|={ab:?} vs |[B,A]|={ba:?}",
            );
        }
    }
}

#[test]
/// Pillar II crosses In when a non-abelian commutator's magnitude exceeds
/// theta — the machinery detects the signal.
fn perm_bundle_pillar_cross_in_on_non_commuting_pair() {
    let a = PermBundle::from_perm(Perm3::SWAP_01);
    let b = PermBundle::from_perm(Perm3::SWAP_12);
    let state = [10, 20, 30];
    let c = commutator(&a, &b, &state);
    // magnitude is non-zero (verified above); the region (0, ∞) holds any
    // positive signal.
    let verdict = pillar::cross(&exactly(c.magnitude()), &Region::above(rational(0.0)));
    assert!(matches!(verdict, Membership::In(_)), "got {verdict:?}");
}

// ──────────────────────────────────────────────────────────────────
// 14. Pillar III generalized — viability_of_magnitudes over raw Loss.
// ──────────────────────────────────────────────────────────────────

#[test]
/// viability_of_magnitudes Pass when accumulated exceeds theta.
fn viability_of_magnitudes_pass_above_threshold() {
    let history = vec![
        ScalarLoss::new(3.0),
        ScalarLoss::new(3.0),
        ScalarLoss::new(3.0),
    ];
    // accumulated = 9.0 (ScalarLoss::combine is addition), theta = 5.0 → Pass
    let theta = ScalarLoss::new(5.0);
    let verdict = pillar::viability_of_magnitudes(&history, &theta, 3);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
/// viability_of_magnitudes Fail when accumulated below threshold.
fn viability_of_magnitudes_fail_below_threshold() {
    let history = vec![
        ScalarLoss::new(1.0),
        ScalarLoss::new(1.0),
        ScalarLoss::new(1.0),
    ];
    // accumulated = 3.0, theta = 10.0 → Fail
    let theta = ScalarLoss::new(10.0);
    let verdict = pillar::viability_of_magnitudes(&history, &theta, 3);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "got {verdict:?}"
    );
}

#[test]
/// viability_of_magnitudes Partial when history shorter than window.
fn viability_of_magnitudes_partial_when_short() {
    let history = vec![ScalarLoss::new(3.0), ScalarLoss::new(3.0)];
    let theta = ScalarLoss::new(1.0);
    // history.len() = 2 < omega = 5 → Partial with confidence 2/5.
    let verdict = pillar::viability_of_magnitudes(&history, &theta, 5);
    match verdict {
        PropertyVerdict::Partial { confidence, .. } => {
            assert!((confidence - 0.4).abs() < 1e-9, "confidence = {confidence}");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 14b. pillar::fold verdict-fold helper (iter 9).
// ──────────────────────────────────────────────────────────────────

#[test]
/// Empty input to fold → Pass (the neutral element).
fn fold_empty_is_pass() {
    let verdict = pillar::fold(&[]);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
/// All-Pass fold → Pass.
fn fold_all_pass_is_pass() {
    let verdict = pillar::fold(&[
        PropertyVerdict::Pass,
        PropertyVerdict::Pass,
        PropertyVerdict::Pass,
    ]);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
/// Any Fail in the sequence → unified Fail (Fail dominates).
fn fold_with_any_fail_is_fail() {
    let verdict = pillar::fold(&[
        PropertyVerdict::Pass,
        PropertyVerdict::Fail(Diagnostic::new("stalled")),
        PropertyVerdict::Pass,
    ]);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "got {verdict:?}"
    );
}

#[test]
/// All-Partial fold → Partial with min confidence + union
/// diagnostics per merge_with semantics.
fn fold_all_partial_takes_min_confidence() {
    let verdict = pillar::fold(&[
        PropertyVerdict::Partial {
            confidence: 0.9,
            diagnostics: vec![Diagnostic::new("a")],
        },
        PropertyVerdict::Partial {
            confidence: 0.4,
            diagnostics: vec![Diagnostic::new("b")],
        },
        PropertyVerdict::Partial {
            confidence: 0.6,
            diagnostics: vec![Diagnostic::new("c")],
        },
    ]);
    match verdict {
        PropertyVerdict::Partial {
            confidence,
            diagnostics,
        } => {
            assert!(
                (confidence - 0.4).abs() < 1e-9,
                "min confidence 0.4, got {confidence}",
            );
            assert_eq!(diagnostics.len(), 3, "expected 3 diagnostics");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 15. Pillar II on a measured magnitude: cross an enclosure.
// ──────────────────────────────────────────────────────────────────

#[test]
/// An enclosure wholly above theta crosses In.
fn cross_in_above_threshold() {
    let verdict = pillar::cross(
        &Enclosure::of_floats(4.5, 5.5).expect("finite"),
        &Region::above(rational(1.0)),
    );
    assert!(matches!(verdict, Membership::In(_)), "got {verdict:?}");
}

#[test]
/// One ulp above theta is not a pass (Mara 2739e94c §9 A3). A float one ulp
/// above the threshold, enclosed by one ulp either side, straddles it: the
/// crossing can't be certified, so it is Undecided, never Pass.
fn algedonic_one_ulp_above_the_threshold_is_not_a_pass() {
    let theta = Region::above(rational(1.0));
    let v = 1.0f64.next_up();
    let measured = Enclosure::of_floats(v.next_down(), v.next_up()).expect("finite");
    let verdict = pillar::cross(&measured, &theta);
    assert!(
        matches!(verdict, Membership::Undecided(_)),
        "one ulp above theta: {verdict:?}"
    );
    // Only an exact value one ulp above is In, and an exact value at theta is
    // certified Out.
    let exact = pillar::cross(&Enclosure::exact(rational(v)), &theta);
    assert!(matches!(exact, Membership::In(_)), "got {exact:?}");
    let at = pillar::cross(&Enclosure::exact(rational(1.0)), &theta);
    assert!(matches!(at, Membership::Out(_)), "got {at:?}");
}

#[test]
/// Zero is a reading, not the absence of one: certified Out of (1, ∞).
fn cross_reads_zero_as_a_value() {
    let verdict = pillar::cross(&exactly(ScalarLoss::zero()), &Region::above(rational(1.0)));
    assert!(matches!(verdict, Membership::Out(_)), "got {verdict:?}");
}

#[test]
/// An enclosure wholly below theta is certified Out: a decided claim, with
/// no confidence number.
fn cross_out_below_threshold_but_nonzero() {
    let verdict = pillar::cross(
        &Enclosure::of_floats(0.25, 0.375).expect("finite"),
        &Region::above(rational(1.0)),
    );
    assert!(matches!(verdict, Membership::Out(_)), "got {verdict:?}");
}

// ──────────────────────────────────────────────────────────────────
// 16. viability_of_magnitudes windowing (moved from 14 for locality).
// ──────────────────────────────────────────────────────────────────

#[test]
/// viability_of_magnitudes windows properly — only the last omega
/// entries contribute to the accumulated magnitude.
fn viability_of_magnitudes_uses_last_omega_entries() {
    // Early tail is huge, but window is small — recent entries decide.
    // Five entries, omega = 2: the window starts at 5 − 2 = 3, not 5 / 2 = 2.
    let history = vec![
        ScalarLoss::new(100.0), // dropped (outside window)
        ScalarLoss::new(100.0), // dropped
        ScalarLoss::new(100.0), // dropped
        ScalarLoss::new(1.0),   // window[0]
        ScalarLoss::new(1.0),   // window[1]
    ];
    // omega = 2, window = [1.0, 1.0], accumulated = 2.0, theta = 2.0 → Fail:
    // Pass is strictly above theta.
    let theta = ScalarLoss::new(2.0);
    let verdict = pillar::viability_of_magnitudes(&history, &theta, 2);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "windowing failed — got {verdict:?}",
    );
}

#[test]
/// PermBundle commutator inherits `Metric::triangle` across triples.
/// Property inherited from Metric — witnessed for the non-abelian
/// carrier, closing the four core properties (antisymmetric,
/// self-annihilating, non-negative, triangle) for the substrate-
/// honest S3 witness.
fn perm_bundle_commutator_triangle_inequality_holds() {
    let state = [10, 20, 30];
    for &pa in Perm3::ALL.iter() {
        for &pb in Perm3::ALL.iter() {
            for &pc in Perm3::ALL.iter() {
                let a = PermBundle::from_perm(pa);
                let b = PermBundle::from_perm(pb);
                let cc = PermBundle::from_perm(pc);
                let ab = FluxThread::commutator_magnitude(&a, &b, &state);
                let bc = FluxThread::commutator_magnitude(&b, &cc, &state);
                let ac = FluxThread::commutator_magnitude(&a, &cc, &state);
                assert!(
                    ab.triangle(&bc, &ac),
                    "triangle failed for {pa:?}/{pb:?}/{pc:?}: ab={ab:?} bc={bc:?} ac={ac:?}",
                );
            }
        }
    }
}

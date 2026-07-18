//! Fate composition witnesses — `pillar::of_health` (Arc 1 of the
//! witnessed-property-inference arc).
//!
//! Arc 1 per Mara spec `docs/specs/witnessed-property-inference-fate-
//! drives-both.md` §9 and the composition surface §7.2. First
//! empirical composition edge between prismqueer::liquid pillar and
//! prismqueer::fate (source-mirrored from `/Users/alexwolf/dev/
//! projects/fate/` per Alex 2026-07-18 Q7 leave-fate directive).
//!
//! Substrate-honest divergence from spec §7.2: `HolonomyHealth` is a
//! three-variant enum (per `prismqueer::fate::feature`), not a scalar.
//! The threshold discipline (`BERRY_PHASE = 0.847`, ratio bounds 0.1
//! and 10.0) is baked into `holonomy_health(loss)` at fate altitude.
//! `pillar::of_health` matches on the classified enum directly — no
//! pillar-side theta parameter.
//!
//! Gated on `fate` feature; the composition edge only exists when
//! prismqueer::fate compiles.

#![cfg(feature = "fate")]

use prismqueer::fate::feature::{holonomy_health, HolonomyHealth, BERRY_PHASE};
use prismqueer::liquid::prelude::*;

// ──────────────────────────────────────────────────────────────────
// 1. Direct-classification verdict mapping.
// ──────────────────────────────────────────────────────────────────

#[test]
/// `HolonomyHealth::Healthy` → `Pass`. The base case; a training step
/// with loss in `[0.1 × BERRY_PHASE, 10 × BERRY_PHASE]` is a clean
/// signal per `fate::feature::holonomy_health`.
fn of_health_healthy_yields_pass() {
    let verdict = pillar::of_health(&HolonomyHealth::Healthy);
    assert!(
        matches!(verdict, PropertyVerdict::Pass),
        "Healthy → Pass; got {verdict:?}",
    );
}

#[test]
/// `HolonomyHealth::TooShallow` → `Partial { confidence: 0.5, .. }`.
/// Substrate-honest: step barely moved the manifold; signal exists
/// but not decisive. Rice-safe midpoint confidence.
fn of_health_too_shallow_yields_partial_confidence_half() {
    let verdict = pillar::of_health(&HolonomyHealth::TooShallow);
    match verdict {
        PropertyVerdict::Partial { confidence, diagnostics } => {
            assert!(
                (confidence - 0.5).abs() < 1e-9,
                "expected confidence 0.5, got {confidence}",
            );
            assert_eq!(diagnostics.len(), 1);
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

#[test]
/// `HolonomyHealth::OverCutting` → `Fail`. Geometric distortion; the
/// training step blew past the healthy ratio range (loss > 10×
/// `BERRY_PHASE`).
fn of_health_over_cutting_yields_fail() {
    let verdict = pillar::of_health(&HolonomyHealth::OverCutting);
    assert!(
        matches!(verdict, PropertyVerdict::Fail(_)),
        "OverCutting → Fail; got {verdict:?}",
    );
}

// ──────────────────────────────────────────────────────────────────
// 2. End-to-end: fate::holonomy_health → pillar::of_health.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Compose the whole pipeline: raw training loss → `holonomy_health`
/// classifier (fate) → `of_health` verdict marshaler (pillar). Exact
/// BERRY_PHASE loss = Healthy = Pass.
fn end_to_end_berry_phase_loss_yields_pass() {
    let health = holonomy_health(BERRY_PHASE);
    let verdict = pillar::of_health(&health);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
/// Loss = 0 → TooShallow → Partial.
fn end_to_end_zero_loss_yields_partial() {
    let health = holonomy_health(0.0);
    let verdict = pillar::of_health(&health);
    assert!(matches!(verdict, PropertyVerdict::Partial { .. }), "got {verdict:?}");
}

#[test]
/// Loss = 100× BERRY_PHASE → OverCutting → Fail.
fn end_to_end_extreme_loss_yields_fail() {
    let health = holonomy_health(BERRY_PHASE * 100.0);
    let verdict = pillar::of_health(&health);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

// ──────────────────────────────────────────────────────────────────
// 3. Boundary conditions — the ratio thresholds are Rice-safe.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Boundary just below TooShallow (ratio = 0.099) → Partial.
fn boundary_just_below_shallow_threshold_yields_partial() {
    let health = holonomy_health(BERRY_PHASE * 0.099);
    let verdict = pillar::of_health(&health);
    assert!(matches!(verdict, PropertyVerdict::Partial { .. }), "got {verdict:?}");
}

#[test]
/// Boundary just above shallow threshold (ratio = 0.11) → Pass.
fn boundary_just_above_shallow_threshold_yields_pass() {
    let health = holonomy_health(BERRY_PHASE * 0.11);
    let verdict = pillar::of_health(&health);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
/// Boundary just below over-cutting threshold (ratio = 9.9) → Pass.
fn boundary_just_below_over_cutting_yields_pass() {
    let health = holonomy_health(BERRY_PHASE * 9.9);
    let verdict = pillar::of_health(&health);
    assert!(matches!(verdict, PropertyVerdict::Pass), "got {verdict:?}");
}

#[test]
/// Boundary just above over-cutting threshold (ratio = 11.0) → Fail.
fn boundary_just_above_over_cutting_yields_fail() {
    let health = holonomy_health(BERRY_PHASE * 11.0);
    let verdict = pillar::of_health(&health);
    assert!(matches!(verdict, PropertyVerdict::Fail(_)), "got {verdict:?}");
}

// ──────────────────────────────────────────────────────────────────
// 4. Composition with pillar::fold — multi-tick health folded.
// ──────────────────────────────────────────────────────────────────

#[test]
/// K training-step losses → K HolonomyHealth classifications → K
/// verdicts → folded unified verdict. Any OverCutting in the run
/// forces Fail per Beer audit-channel semantics (iter 8 witnesses).
fn folded_training_run_with_over_cutting_yields_fail() {
    let losses = vec![
        BERRY_PHASE,               // Healthy → Pass
        BERRY_PHASE * 2.0,         // Healthy → Pass
        BERRY_PHASE * 15.0,        // OverCutting → Fail
        BERRY_PHASE * 1.5,         // Healthy → Pass
    ];
    let verdicts: Vec<PropertyVerdict> = losses
        .iter()
        .map(|&loss| pillar::of_health(&holonomy_health(loss)))
        .collect();
    let unified = pillar::fold(&verdicts);
    assert!(
        matches!(unified, PropertyVerdict::Fail(_)),
        "Fail must dominate; got {unified:?}",
    );
}

#[test]
/// All-Healthy training run folds to Pass.
fn folded_training_run_all_healthy_yields_pass() {
    let losses = vec![BERRY_PHASE, BERRY_PHASE * 2.0, BERRY_PHASE * 5.0];
    let verdicts: Vec<PropertyVerdict> = losses
        .iter()
        .map(|&loss| pillar::of_health(&holonomy_health(loss)))
        .collect();
    let unified = pillar::fold(&verdicts);
    assert!(matches!(unified, PropertyVerdict::Pass), "got {unified:?}");
}

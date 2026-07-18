//! Named feature dimensions for Fate's 16-dimensional fiber bundle.
//!
//! 16 dimensions split into two groups:
//! - 6 **active** dimensions (indices 0–5): observable, updated each cycle.
//! - 10 **dark** dimensions (indices 6–15): latent, shaped by training pressure.
//!
//! The Casimir invariant C₂ = Σ(λᵢ·xᵢ)² over active dimensions measures how much
//! of the spectral energy is concentrated in the observable subspace.
//! Holonomy health classifies whether a training step is cutting cleanly.

use super::FEATURE_DIM;

// ---------------------------------------------------------------------------
// Active dimensions (0–5)
// ---------------------------------------------------------------------------

pub const TEMPORAL: usize = 0;
pub const PROCESSING: usize = 1;
pub const STABILITY: usize = 2;
pub const NOVELTY: usize = 3;
pub const CAUTION: usize = 4;
pub const COHERENCE: usize = 5;

pub const ACTIVE: [usize; 6] = [TEMPORAL, PROCESSING, STABILITY, NOVELTY, CAUTION, COHERENCE];
pub const ACTIVE_COUNT: usize = 6;

// ---------------------------------------------------------------------------
// Dark dimensions (6–15)
// ---------------------------------------------------------------------------

pub const CREATIVITY: usize = 6;
pub const CONFIDENCE: usize = 7;
pub const FORMALITY: usize = 8;
pub const OUTPUT_REGULATION: usize = 9;
pub const ABSTRACTION: usize = 10;
pub const DEFERENCE: usize = 11;
pub const CONFIDENCE_CALIBRATION: usize = 12;
pub const INNOVATION: usize = 13;
pub const REASONING_DEPTH: usize = 14;
pub const EMOTIONAL_TONE: usize = 15;

pub const DARK: [usize; 10] = [
    CREATIVITY,
    CONFIDENCE,
    FORMALITY,
    OUTPUT_REGULATION,
    ABSTRACTION,
    DEFERENCE,
    CONFIDENCE_CALIBRATION,
    INNOVATION,
    REASONING_DEPTH,
    EMOTIONAL_TONE,
];
pub const DARK_COUNT: usize = 10;

// ---------------------------------------------------------------------------
// Casimir invariant
// ---------------------------------------------------------------------------

/// Eigenvalues for the 6 active Casimir dimensions.
/// These are the λᵢ weights in C₂ = Σ(λᵢ·xᵢ)².
pub const CASIMIR_EIGENVALUES: [f64; 6] = [4.12, 3.98, 4.05, 3.91, 4.08, 3.97];

/// C₂ = Σ(λᵢ·xᵢ)² over active dimensions only.
pub fn casimir(features: &[f64; FEATURE_DIM]) -> f64 {
    let mut c2 = 0.0;
    for (i, &dim) in ACTIVE.iter().enumerate() {
        let val = CASIMIR_EIGENVALUES[i] * features[dim];
        c2 += val * val;
    }
    c2
}

/// |C₂(before) - C₂(after)| — how much the invariant shifted between states.
pub fn casimir_penalty(before: &[f64; FEATURE_DIM], after: &[f64; FEATURE_DIM]) -> f64 {
    (casimir(before) - casimir(after)).abs()
}

// ---------------------------------------------------------------------------
// Holonomy health
// ---------------------------------------------------------------------------

/// Berry phase constant for the 6-active-dim fiber bundle.
pub const BERRY_PHASE: f64 = 0.847;

/// Classification of a training step's holonomy relative to BERRY_PHASE.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HolonomyHealth {
    /// loss / BERRY_PHASE < 0.1 — step barely moved the manifold.
    TooShallow,
    /// 0.1 ≤ loss / BERRY_PHASE ≤ 10.0 — clean, well-sized step.
    Healthy,
    /// loss / BERRY_PHASE > 10.0 — step over-cut; geometric distortion.
    OverCutting,
}

/// Classify holonomy health from a training loss value.
pub fn holonomy_health(loss: f64) -> HolonomyHealth {
    let ratio = loss / BERRY_PHASE;
    if ratio < 0.1 {
        HolonomyHealth::TooShallow
    } else if ratio > 10.0 {
        HolonomyHealth::OverCutting
    } else {
        HolonomyHealth::Healthy
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_and_dark_cover_all_16() {
        let mut covered = [false; FEATURE_DIM];
        for &d in ACTIVE.iter() {
            covered[d] = true;
        }
        for &d in DARK.iter() {
            covered[d] = true;
        }
        assert!(
            covered.iter().all(|&c| c),
            "not all 16 dimensions covered by ACTIVE + DARK"
        );
    }

    #[test]
    fn active_dims_are_first_six() {
        for &d in ACTIVE.iter() {
            assert!(d < 6, "active dim {} should be < 6", d);
        }
        assert_eq!(ACTIVE_COUNT, 6);
    }

    #[test]
    fn dark_dims_are_last_ten() {
        for &d in DARK.iter() {
            assert!(d >= 6, "dark dim {} should be >= 6", d);
            assert!(d < 16, "dark dim {} should be < 16", d);
        }
        assert_eq!(DARK_COUNT, 10);
    }

    #[test]
    fn casimir_zero_features_is_zero() {
        let features = [0.0; FEATURE_DIM];
        assert_eq!(casimir(&features), 0.0);
    }

    #[test]
    fn casimir_only_counts_active() {
        // Setting a dark dimension to 100.0 should not change C₂.
        let mut features = [0.0; FEATURE_DIM];
        features[CREATIVITY] = 100.0; // dark dim
        assert_eq!(
            casimir(&features),
            0.0,
            "dark dim should not contribute to C₂"
        );

        // Setting an active dim to 2.0: C₂ += (λ₀ * 2.0)²
        let mut features2 = [0.0; FEATURE_DIM];
        features2[TEMPORAL] = 2.0; // active dim 0
        let expected = (CASIMIR_EIGENVALUES[0] * 2.0).powi(2);
        assert!(
            (casimir(&features2) - expected).abs() < 1e-12,
            "casimir with active dim 2.0 should be {}, got {}",
            expected,
            casimir(&features2)
        );
    }

    #[test]
    fn casimir_penalty_measures_violation() {
        let before = [0.0; FEATURE_DIM];
        let mut after = [0.0; FEATURE_DIM];
        after[TEMPORAL] = 1.0;
        let penalty = casimir_penalty(&before, &after);
        assert!(
            penalty > 0.0,
            "penalty should be nonzero when active dims change"
        );
    }

    #[test]
    fn casimir_conservation_under_redistribution() {
        // Redistribute energy among active dims while keeping C₂ the same.
        // Set before: temporal = 1/λ₀, so (λ₀ · 1/λ₀)² = 1.0
        // Set after: processing = 1/λ₁, so (λ₁ · 1/λ₁)² = 1.0
        let mut before = [0.0; FEATURE_DIM];
        before[TEMPORAL] = 1.0 / CASIMIR_EIGENVALUES[0];
        let c2_before = casimir(&before);

        let mut after = [0.0; FEATURE_DIM];
        after[PROCESSING] = 1.0 / CASIMIR_EIGENVALUES[1];
        let c2_after = casimir(&after);

        let penalty = casimir_penalty(&before, &after);
        assert!(
            (c2_before - 1.0).abs() < 1e-12,
            "before C₂ should be 1.0, got {}",
            c2_before
        );
        assert!(
            (c2_after - 1.0).abs() < 1e-12,
            "after C₂ should be 1.0, got {}",
            c2_after
        );
        assert!(
            penalty < 1e-12,
            "penalty should be ~0 when C₂ is conserved, got {}",
            penalty
        );
    }

    #[test]
    fn holonomy_health_classification() {
        assert_eq!(holonomy_health(0.0), HolonomyHealth::TooShallow);
        assert_eq!(
            holonomy_health(BERRY_PHASE * 0.05),
            HolonomyHealth::TooShallow
        );
        assert_eq!(holonomy_health(BERRY_PHASE), HolonomyHealth::Healthy);
        assert_eq!(holonomy_health(BERRY_PHASE * 5.0), HolonomyHealth::Healthy);
        assert_eq!(
            holonomy_health(BERRY_PHASE * 11.0),
            HolonomyHealth::OverCutting
        );
    }

    #[test]
    fn berry_phase_is_healthy() {
        assert_eq!(holonomy_health(BERRY_PHASE), HolonomyHealth::Healthy);
    }
}

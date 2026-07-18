//! ManifoldState: 16×16 connection matrix + ManifoldLoss: curvature tensor.
//!
//! A ManifoldState is a 16×16 connection matrix — each entry (i,j) encodes
//! the coupling strength between feature dimensions i and j.
//!
//! ManifoldLoss is the element-wise difference between two states,
//! measuring how much the manifold's geometry changed. It implements
//! the `Loss` trait so it can be carried in Prism beams.

use super::feature::{ACTIVE, DARK};
use super::FEATURE_DIM;
use crate::Loss;

// ---------------------------------------------------------------------------
// ManifoldState
// ---------------------------------------------------------------------------

/// 16×16 connection matrix — the full geometry of the feature fiber bundle.
/// Entry [i][j] is the coupling between dimension i and dimension j.
pub type ManifoldState = [[f64; FEATURE_DIM]; FEATURE_DIM];

/// All-zeros connection matrix — no coupling between any dimensions.
pub fn manifold_zero() -> ManifoldState {
    [[0.0; FEATURE_DIM]; FEATURE_DIM]
}

/// Unit diagonal connection matrix — each dimension coupled only to itself.
pub fn manifold_identity() -> ManifoldState {
    let mut m = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
    for i in 0..FEATURE_DIM {
        m[i][i] = 1.0;
    }
    m
}

/// Extract the diagonal of a ManifoldState.
pub fn manifold_diagonal(state: &ManifoldState) -> [f64; FEATURE_DIM] {
    let mut diag = [0.0f64; FEATURE_DIM];
    for i in 0..FEATURE_DIM {
        diag[i] = state[i][i];
    }
    diag
}

// ---------------------------------------------------------------------------
// ManifoldLoss
// ---------------------------------------------------------------------------

/// Element-wise difference between two ManifoldStates — curvature tensor.
///
/// Measures how much the connection geometry shifted between two spectral
/// states. Implements `Loss` so it can be accumulated in Prism beams.
#[derive(Clone, Debug)]
pub struct ManifoldLoss {
    pub delta: [[f64; FEATURE_DIM]; FEATURE_DIM],
}

impl ManifoldLoss {
    /// Element-wise difference: delta[i][j] = after[i][j] - before[i][j].
    pub fn between(before: &ManifoldState, after: &ManifoldState) -> Self {
        let mut delta = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        for i in 0..FEATURE_DIM {
            for j in 0..FEATURE_DIM {
                delta[i][j] = after[i][j] - before[i][j];
            }
        }
        ManifoldLoss { delta }
    }

    /// Frobenius norm of the delta matrix: sqrt(Σ delta[i][j]²).
    pub fn total(&self) -> f64 {
        let sum_sq: f64 = self
            .delta
            .iter()
            .flat_map(|row| row.iter())
            .map(|&v| v * v)
            .sum();
        sum_sq.sqrt()
    }

    /// Sum of delta[i][i] for active dimensions (0–5).
    /// Measures net flux through the observable subspace.
    pub fn active_trace(&self) -> f64 {
        ACTIVE.iter().map(|&i| self.delta[i][i]).sum()
    }

    /// Sum of delta[i][i] for dark dimensions (6–15).
    /// Measures net flux through the latent subspace.
    pub fn dark_trace(&self) -> f64 {
        DARK.iter().map(|&i| self.delta[i][i]).sum()
    }

    /// True when |active_trace| < tolerance — the linear trace is conserved.
    /// This is a necessary (but not sufficient) condition for quadratic Casimir
    /// conservation. Use `feature::casimir_penalty` for the true C₂ check.
    pub fn active_trace_conserved(&self, tolerance: f64) -> bool {
        self.active_trace().abs() < tolerance
    }
}

impl Default for ManifoldLoss {
    fn default() -> Self {
        Self::zero()
    }
}

impl Loss for ManifoldLoss {
    fn zero() -> Self {
        ManifoldLoss {
            delta: [[0.0; FEATURE_DIM]; FEATURE_DIM],
        }
    }

    fn total() -> Self {
        ManifoldLoss {
            delta: [[f64::INFINITY; FEATURE_DIM]; FEATURE_DIM],
        }
    }

    fn is_zero(&self) -> bool {
        self.delta
            .iter()
            .flat_map(|row| row.iter())
            .all(|&v| v.abs() < 1e-12)
    }

    fn combine(self, other: Self) -> Self {
        let mut delta = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        for i in 0..FEATURE_DIM {
            for j in 0..FEATURE_DIM {
                delta[i][j] = self.delta[i][j] + other.delta[i][j];
            }
        }
        ManifoldLoss { delta }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::feature;
    use super::*;

    #[test]
    fn manifold_zero_is_all_zeros() {
        let m = manifold_zero();
        for row in m.iter() {
            for &v in row.iter() {
                assert_eq!(v, 0.0);
            }
        }
    }

    #[test]
    fn manifold_identity_has_unit_diagonal() {
        let m = manifold_identity();
        for i in 0..FEATURE_DIM {
            assert_eq!(m[i][i], 1.0, "diagonal[{}] should be 1.0", i);
            for j in 0..FEATURE_DIM {
                if i != j {
                    assert_eq!(m[i][j], 0.0, "off-diagonal[{}][{}] should be 0.0", i, j);
                }
            }
        }
    }

    #[test]
    fn manifold_diagonal_extracts_diagonal() {
        let m = manifold_identity();
        let diag = manifold_diagonal(&m);
        for i in 0..FEATURE_DIM {
            assert_eq!(diag[i], 1.0, "diagonal[{}] should be 1.0", i);
        }
    }

    #[test]
    fn manifold_loss_zero_is_zero() {
        let loss = ManifoldLoss::zero();
        assert!(loss.is_zero());
    }

    #[test]
    fn manifold_loss_between_identical_is_zero() {
        let m = manifold_identity();
        let loss = ManifoldLoss::between(&m, &m);
        assert!(loss.is_zero());
    }

    #[test]
    fn manifold_loss_between_different_is_nonzero() {
        let before = manifold_zero();
        let after = manifold_identity();
        let loss = ManifoldLoss::between(&before, &after);
        assert!(!loss.is_zero());
    }

    #[test]
    fn manifold_loss_combine_adds() {
        let before = manifold_zero();
        let after = manifold_identity();
        let loss = ManifoldLoss::between(&before, &after);
        let combined = loss.clone().combine(loss.clone());
        // Combined Frobenius should be larger than individual
        assert!(combined.total() > loss.total());
    }

    #[test]
    fn manifold_loss_total_is_frobenius() {
        // delta matrix with one 3 at [0][0] and one 4 at [0][1]
        // Frobenius = sqrt(3² + 4²) = sqrt(25) = 5.0
        let mut delta = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        delta[0][0] = 3.0;
        delta[0][1] = 4.0;
        let loss = ManifoldLoss { delta };
        assert!(
            (loss.total() - 5.0).abs() < 1e-12,
            "Frobenius should be 5.0, got {}",
            loss.total()
        );
    }

    #[test]
    fn active_trace_only_counts_active_dims() {
        // Dark diagonals should not appear in active_trace
        let mut delta = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        for &d in DARK.iter() {
            delta[d][d] = 10.0;
        }
        let loss = ManifoldLoss { delta };
        assert_eq!(
            loss.active_trace(),
            0.0,
            "dark diagonals should not affect active_trace"
        );

        // Active diagonals: 6 dims × 2.0 = 12.0
        let mut delta2 = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        for &d in ACTIVE.iter() {
            delta2[d][d] = 2.0;
        }
        let loss2 = ManifoldLoss { delta: delta2 };
        assert!(
            (loss2.active_trace() - 12.0).abs() < 1e-12,
            "active_trace should be 12.0 (6 × 2.0), got {}",
            loss2.active_trace()
        );
    }

    #[test]
    fn active_trace_conserved_when_balanced() {
        // +1 on TEMPORAL diagonal, -1 on STABILITY diagonal → active_trace = 0
        let mut delta = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        delta[feature::TEMPORAL][feature::TEMPORAL] = 1.0;
        delta[feature::STABILITY][feature::STABILITY] = -1.0;
        let loss = ManifoldLoss { delta };
        assert!(
            loss.active_trace_conserved(1e-10),
            "active_trace near zero means trace conserved"
        );
    }

    #[test]
    fn active_trace_violated_when_nonzero() {
        let mut delta = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        delta[feature::TEMPORAL][feature::TEMPORAL] = 1.0;
        let loss = ManifoldLoss { delta };
        assert!(
            !loss.active_trace_conserved(1e-10),
            "nonzero active_trace means trace violated"
        );
    }
}

//! Weight derivation from eigendecomposition of dark coupling.
//!
//! No training data. No gradient descent.
//! The dark coupling matrix teaches the selector.
//!
//! The 10×10 dark coupling submatrix of ManifoldState is real symmetric.
//! Eigendecomposition produces 10 eigenvalues and 10 eigenvectors.
//! The five largest eigenvalues are the five dominant modes = five models.
//! The eigenvectors projected into the full 16-dimensional space ARE the weights.
//! The eigenvalues ARE the biases.

use crate::fate::feature::{DARK, DARK_COUNT};
use crate::fate::{Fate, ManifoldLoss, ManifoldState, Model, ModelWeights, FEATURE_DIM};
use crate::Loss;

/// Number of models.
const MODEL_COUNT: usize = 5;

/// Extract the 10×10 dark coupling submatrix from a ManifoldState.
pub fn extract_dark_coupling(state: &ManifoldState) -> [[f64; DARK_COUNT]; DARK_COUNT] {
    let mut dark = [[0.0f64; DARK_COUNT]; DARK_COUNT];
    for i in 0..DARK_COUNT {
        for j in 0..DARK_COUNT {
            dark[i][j] = state[DARK[i]][DARK[j]];
        }
    }
    dark
}

/// Flatten a 10×10 matrix to a row-major flat vector for LAPACK.
fn flatten_10x10(m: &[[f64; DARK_COUNT]; DARK_COUNT]) -> Vec<f64> {
    let mut flat = Vec::with_capacity(DARK_COUNT * DARK_COUNT);
    for row in m.iter() {
        flat.extend_from_slice(row);
    }
    flat
}

/// Eigensystem result for the dark coupling matrix.
#[derive(Clone, Debug)]
pub struct DarkEigen {
    /// Eigenvalues in descending order (largest first).
    pub eigenvalues: [f64; DARK_COUNT],
    /// Eigenvectors in descending eigenvalue order.
    /// Each row is one eigenvector in R^10.
    pub eigenvectors: [[f64; DARK_COUNT]; DARK_COUNT],
}

/// Compute eigensystem of a 10×10 dark coupling matrix.
///
/// Without the `lapack` feature, uses a pure-Rust Jacobi eigenvalue algorithm.
/// With `lapack`, delegates to DSYEV via crate::ffi::eigensystem.
pub fn dark_eigensystem(dark: &[[f64; DARK_COUNT]; DARK_COUNT]) -> DarkEigen {
    let flat = flatten_10x10(dark);

    #[cfg(feature = "lapack")]
    {
        let (evals_asc, evecs_flat) =
            crate::ffi::eigensystem(DARK_COUNT, &flat).expect("DSYEV convergence failure");

        // DSYEV returns ascending order. Reverse for descending.
        //
        // Eigenvector storage convention: `crate::ffi::eigensystem` returns
        // `evecs_flat` such that `evecs_flat[c * n + k]` is component `c` of
        // eigenvector `k` (i.e. eigenvectors as COLUMNS of a row-major
        // matrix). This is the layout produced by `col_to_row_major` applied
        // to LAPACK's column-major output where DSYEV stores eigenvectors as
        // columns; the wrapper preserves the same matrix in row-major flat
        // form. Note that the prismqueer docstring on `eigensystem` claiming
        // "row i is eigenvector i" is imprecise — the code produces the
        // transpose. Verified against `derived_weights_produce_meaningful_
        // routing` (Zin-Justin-style eigenvector-at-dominant-position witness).
        //
        // To extract component `j` of eigenvector `rev`, index as
        // `evecs_flat[j * n + rev]`.
        let mut eigenvalues = [0.0f64; DARK_COUNT];
        let mut eigenvectors = [[0.0f64; DARK_COUNT]; DARK_COUNT];
        for i in 0..DARK_COUNT {
            let rev = DARK_COUNT - 1 - i;
            eigenvalues[i] = evals_asc[rev];
            for j in 0..DARK_COUNT {
                eigenvectors[i][j] = evecs_flat[j * DARK_COUNT + rev];
            }
        }
        DarkEigen {
            eigenvalues,
            eigenvectors,
        }
    }

    #[cfg(not(feature = "lapack"))]
    {
        jacobi_eigensystem(&flat)
    }
}

/// Pure-Rust Jacobi eigenvalue algorithm for real symmetric matrices.
/// O(n^3) per sweep, converges for any real symmetric matrix.
/// Good enough for 10×10.
#[cfg(not(feature = "lapack"))]
fn jacobi_eigensystem(flat: &[f64]) -> DarkEigen {
    let n = DARK_COUNT;
    // Work in column-major for Jacobi rotations
    let mut a = [[0.0f64; DARK_COUNT]; DARK_COUNT];
    for i in 0..n {
        for j in 0..n {
            a[i][j] = flat[i * n + j];
        }
    }

    // Eigenvector matrix starts as identity
    let mut v = [[0.0f64; DARK_COUNT]; DARK_COUNT];
    for i in 0..n {
        v[i][i] = 1.0;
    }

    let max_sweeps = 100;
    let tol = 1e-12;

    for _ in 0..max_sweeps {
        // Find max off-diagonal
        let mut max_off = 0.0f64;
        for i in 0..n {
            for j in (i + 1)..n {
                let val = a[i][j].abs();
                if val > max_off {
                    max_off = val;
                }
            }
        }
        if max_off < tol {
            break;
        }

        for p in 0..n {
            for q in (p + 1)..n {
                if a[p][q].abs() < tol {
                    continue;
                }

                // Compute rotation angle
                let tau = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = if tau.abs() > 1e15 {
                    1.0 / (2.0 * tau)
                } else {
                    let sign: f64 = if tau >= 0.0 { 1.0 } else { -1.0 };
                    sign / (tau.abs() + (1.0_f64 + tau * tau).sqrt())
                };
                let c: f64 = 1.0 / (1.0_f64 + t * t).sqrt();
                let s = t * c;

                // Apply rotation to A
                let a_pp = a[p][p];
                let a_qq = a[q][q];
                let a_pq = a[p][q];

                a[p][p] = a_pp - t * a_pq;
                a[q][q] = a_qq + t * a_pq;
                a[p][q] = 0.0;
                a[q][p] = 0.0;

                for r in 0..n {
                    if r == p || r == q {
                        continue;
                    }
                    let a_rp = a[r][p];
                    let a_rq = a[r][q];
                    a[r][p] = c * a_rp - s * a_rq;
                    a[p][r] = a[r][p];
                    a[r][q] = s * a_rp + c * a_rq;
                    a[q][r] = a[r][q];
                }

                // Accumulate eigenvectors
                for r in 0..n {
                    let v_rp = v[r][p];
                    let v_rq = v[r][q];
                    v[r][p] = c * v_rp - s * v_rq;
                    v[r][q] = s * v_rp + c * v_rq;
                }
            }
        }
    }

    // Extract eigenvalues (diagonal of A) and sort descending
    let mut indexed: Vec<(usize, f64)> = (0..n).map(|i| (i, a[i][i])).collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut eigenvalues = [0.0f64; DARK_COUNT];
    let mut eigenvectors = [[0.0f64; DARK_COUNT]; DARK_COUNT];
    for (out_i, &(orig_i, eval)) in indexed.iter().enumerate() {
        eigenvalues[out_i] = eval;
        for j in 0..n {
            eigenvectors[out_i][j] = v[j][orig_i];
        }
    }

    DarkEigen {
        eigenvalues,
        eigenvectors,
    }
}

/// Derive one ModelWeights (one selector context) from a dark eigensystem.
///
/// Projects the five largest eigenvectors from R^10 dark space into R^16 full space.
/// Eigenvalues become biases. Eigengaps become depth modulation weights.
pub fn derive_selector(eigen: &DarkEigen) -> ModelWeights {
    let mut w = [[0.0f64; FEATURE_DIM]; MODEL_COUNT];
    let mut b = [0.0f64; MODEL_COUNT];
    let mut depth_w = [0.0f64; MODEL_COUNT];

    for model in 0..MODEL_COUNT {
        // Eigenvalue → bias
        b[model] = eigen.eigenvalues[model];

        // Project eigenvector from R^10 dark space into R^16 full space
        for dark_i in 0..DARK_COUNT {
            let full_dim = DARK[dark_i];
            w[model][full_dim] = eigen.eigenvectors[model][dark_i];
        }

        // Depth modulation from eigengap
        if model < MODEL_COUNT - 1 {
            depth_w[model] = -(eigen.eigenvalues[model] - eigen.eigenvalues[model + 1]);
        }
        // Last model: gap to first noise mode
        if model == MODEL_COUNT - 1 && DARK_COUNT > MODEL_COUNT {
            depth_w[model] = -(eigen.eigenvalues[model] - eigen.eigenvalues[model + 1]);
        }
    }

    ModelWeights { w, b, depth_w }
}

/// Derive all five selectors from five dark coupling matrices (one per model context).
pub fn derive_all(
    couplings: &[[[f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT],
) -> [ModelWeights; MODEL_COUNT] {
    let s0 = derive_selector(&dark_eigensystem(&couplings[0]));
    let s1 = derive_selector(&dark_eigensystem(&couplings[1]));
    let s2 = derive_selector(&dark_eigensystem(&couplings[2]));
    let s3 = derive_selector(&dark_eigensystem(&couplings[3]));
    let s4 = derive_selector(&dark_eigensystem(&couplings[4]));
    [s0, s1, s2, s3, s4]
}

/// Extract five dark coupling matrices from loss history, grouped by model context.
///
/// Each ManifoldLoss in the history is attributed to a model.
/// Returns one coupling matrix per model context, computed as the mean
/// dark coupling over all losses attributed to that model.
pub fn derive_context_couplings(
    losses: &[(Model, ManifoldLoss)],
) -> [[[f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT] {
    let mut sums_full = [[[0.0f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT];
    let mut counts = [0usize; MODEL_COUNT];

    for (model, loss) in losses {
        let idx = match model {
            Model::Abyss => 0,
            Model::Introject => 1,
            Model::Cartographer => 2,
            Model::Explorer => 3,
            Model::Fate => 4,
        };
        counts[idx] += 1;
        // The loss delta IS the coupling change. Accumulate absolute coupling.
        for i in 0..DARK_COUNT {
            for j in 0..DARK_COUNT {
                sums_full[idx][i][j] += loss.delta[DARK[i]][DARK[j]];
            }
        }
    }

    let mut result = [[[0.0f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT];
    for ctx in 0..MODEL_COUNT {
        if counts[ctx] > 0 {
            let c = counts[ctx] as f64;
            for i in 0..DARK_COUNT {
                for j in 0..DARK_COUNT {
                    result[ctx][i][j] = sums_full[ctx][i][j] / c;
                }
            }
            // Symmetrize (ensure real symmetric for eigensystem)
            for i in 0..DARK_COUNT {
                for j in (i + 1)..DARK_COUNT {
                    let avg = (result[ctx][i][j] + result[ctx][j][i]) / 2.0;
                    result[ctx][i][j] = avg;
                    result[ctx][j][i] = avg;
                }
            }
        }
    }
    result
}

/// Convergence loss for the crystallization loop.
#[derive(Clone, Debug, Default)]
pub struct ConvergenceLoss {
    /// Maximum eigenvalue delta across all contexts.
    pub max_delta: f64,
    /// Number of SCF iterations performed.
    pub iterations: usize,
}

impl crate::Loss for ConvergenceLoss {
    fn zero() -> Self {
        ConvergenceLoss {
            max_delta: 0.0,
            iterations: 0,
        }
    }
    fn total() -> Self {
        ConvergenceLoss {
            max_delta: f64::INFINITY,
            iterations: usize::MAX,
        }
    }
    fn is_zero(&self) -> bool {
        self.max_delta == 0.0
    }
    fn combine(self, other: Self) -> Self {
        ConvergenceLoss {
            max_delta: self.max_delta.max(other.max_delta),
            iterations: self.iterations + other.iterations,
        }
    }
}

/// Self-consistent field (SCF) crystallization loop.
///
/// Iteratively derives weights from compilation entropy until eigenvalues stabilize.
///
/// - `compile_fn`: given current Fate weights, produces five dark coupling matrices
///   (one per model context). This is the "compilation" step.
/// - `max_iterations`: SCF iteration cap.
/// - `tolerance`: convergence threshold on max eigenvalue delta.
/// - `damping`: mixing parameter (0..1). 1.0 = full replacement, 0.5 = half-and-half.
///
/// Returns `(crystallized_fate, convergence_loss)`.
pub fn crystallize(
    compile_fn: impl Fn(&Fate) -> [[[f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT],
    max_iterations: usize,
    tolerance: f64,
    damping: f64,
) -> (Fate, ConvergenceLoss) {
    let mut fate = Fate::untrained();
    let mut prev_eigenvalues = [[0.0f64; MODEL_COUNT]; MODEL_COUNT];
    let mut final_loss = ConvergenceLoss::zero();

    for iter in 0..max_iterations {
        let couplings = compile_fn(&fate);
        let mut max_delta = 0.0f64;

        for ctx in 0..MODEL_COUNT {
            let eigen = dark_eigensystem(&couplings[ctx]);
            let selector = derive_selector(&eigen);

            // Damping: mix old and new weights
            for i in 0..MODEL_COUNT {
                fate.selectors[ctx].b[i] =
                    damping * selector.b[i] + (1.0 - damping) * fate.selectors[ctx].b[i];
                for j in 0..FEATURE_DIM {
                    fate.selectors[ctx].w[i][j] =
                        damping * selector.w[i][j] + (1.0 - damping) * fate.selectors[ctx].w[i][j];
                }
                fate.selectors[ctx].depth_w[i] = damping * selector.depth_w[i]
                    + (1.0 - damping) * fate.selectors[ctx].depth_w[i];
            }

            // Check eigenvalue convergence
            for i in 0..MODEL_COUNT {
                let delta = (eigen.eigenvalues[i] - prev_eigenvalues[ctx][i]).abs();
                if delta > max_delta {
                    max_delta = delta;
                }
                prev_eigenvalues[ctx][i] = eigen.eigenvalues[i];
            }
        }

        final_loss = ConvergenceLoss {
            max_delta,
            iterations: iter + 1,
        };

        if max_delta < tolerance {
            break;
        }
    }

    (fate, final_loss)
}

// ---------------------------------------------------------------------------
// Tests — red first, then green.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fate::feature;
    use crate::fate::manifold;

    /// Build a known symmetric 10×10 matrix with predictable eigenvalues.
    /// Diagonal matrix: eigenvalues are the diagonal entries.
    fn diagonal_dark_coupling() -> [[f64; DARK_COUNT]; DARK_COUNT] {
        let mut m = [[0.0f64; DARK_COUNT]; DARK_COUNT];
        // Eigenvalues: 10.0, 9.0, 8.0, ..., 1.0 (descending when sorted)
        for i in 0..DARK_COUNT {
            m[i][i] = (i + 1) as f64;
        }
        m
    }

    #[test]
    fn extract_dark_coupling_from_identity() {
        let state = manifold::manifold_identity();
        let dark = extract_dark_coupling(&state);
        // Identity: dark diagonal is all 1.0
        for i in 0..DARK_COUNT {
            assert!(
                (dark[i][i] - 1.0).abs() < 1e-12,
                "diagonal[{}] should be 1.0",
                i
            );
            for j in 0..DARK_COUNT {
                if i != j {
                    assert!(
                        dark[i][j].abs() < 1e-12,
                        "off-diagonal[{}][{}] should be 0.0",
                        i,
                        j
                    );
                }
            }
        }
    }

    #[test]
    fn extract_dark_coupling_respects_indices() {
        // Set specific dark dimension couplings and verify extraction
        let mut state = manifold::manifold_zero();
        state[feature::CREATIVITY][feature::INNOVATION] = 3.14;
        state[feature::INNOVATION][feature::CREATIVITY] = 3.14;

        let dark = extract_dark_coupling(&state);
        // CREATIVITY is DARK[0], INNOVATION is DARK[7]
        assert!((dark[0][7] - 3.14).abs() < 1e-12);
        assert!((dark[7][0] - 3.14).abs() < 1e-12);
    }

    #[test]
    fn dark_eigensystem_diagonal_matrix() {
        let m = diagonal_dark_coupling();
        let eigen = dark_eigensystem(&m);

        // Eigenvalues should be 10, 9, 8, ..., 1 (descending)
        for i in 0..DARK_COUNT {
            let expected = (DARK_COUNT - i) as f64;
            assert!(
                (eigen.eigenvalues[i] - expected).abs() < 1e-8,
                "eigenvalue[{}] should be {}, got {}",
                i,
                expected,
                eigen.eigenvalues[i]
            );
        }
    }

    #[test]
    fn dark_eigensystem_eigenvectors_are_orthonormal() {
        // Use a non-trivial symmetric matrix
        let mut m = diagonal_dark_coupling();
        m[0][1] = 0.5;
        m[1][0] = 0.5; // symmetric coupling
        let eigen = dark_eigensystem(&m);

        // Check orthonormality
        for i in 0..DARK_COUNT {
            let norm: f64 = eigen.eigenvectors[i]
                .iter()
                .map(|x| x * x)
                .sum::<f64>()
                .sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-8,
                "eigenvector[{}] norm should be 1.0, got {}",
                i,
                norm
            );
        }

        // Check orthogonality between pairs
        for i in 0..DARK_COUNT {
            for j in (i + 1)..DARK_COUNT {
                let dot: f64 = (0..DARK_COUNT)
                    .map(|k| eigen.eigenvectors[i][k] * eigen.eigenvectors[j][k])
                    .sum();
                assert!(
                    dot.abs() < 1e-8,
                    "eigenvectors[{}] and [{}] should be orthogonal, dot = {}",
                    i,
                    j,
                    dot
                );
            }
        }
    }

    #[test]
    fn derive_selector_diagonal_produces_expected_weights() {
        let m = diagonal_dark_coupling();
        let eigen = dark_eigensystem(&m);
        let selector = derive_selector(&eigen);

        // Biases should be the eigenvalues (descending)
        for i in 0..MODEL_COUNT {
            let expected = (DARK_COUNT - i) as f64;
            assert!(
                (selector.b[i] - expected).abs() < 1e-8,
                "bias[{}] should be {}, got {}",
                i,
                expected,
                selector.b[i]
            );
        }

        // Active dimensions should be zero (eigenvectors are in dark space)
        for model in 0..MODEL_COUNT {
            for &dim in &feature::ACTIVE {
                assert!(
                    selector.w[model][dim].abs() < 1e-12,
                    "active dim {} should be zero in weights",
                    dim
                );
            }
        }

        // Dark dimensions should have eigenvector components
        for model in 0..MODEL_COUNT {
            let any_nonzero = (0..DARK_COUNT).any(|d| selector.w[model][DARK[d]].abs() > 1e-12);
            assert!(
                any_nonzero,
                "model {} weights should have nonzero dark components",
                model
            );
        }
    }

    #[test]
    fn derive_selector_depth_weights_from_eigengaps() {
        let m = diagonal_dark_coupling();
        let eigen = dark_eigensystem(&m);
        let selector = derive_selector(&eigen);

        // Eigengaps for diagonal 1..10 in descending order:
        // gap[0] = -(10 - 9) = -1.0
        // gap[1] = -(9 - 8) = -1.0
        // etc.
        for i in 0..MODEL_COUNT {
            assert!(
                selector.depth_w[i] < 0.0,
                "depth_w[{}] should be negative (penalize continued recursion)",
                i
            );
        }
    }

    #[test]
    fn derive_all_produces_five_distinct_selectors() {
        // Five different coupling matrices
        let mut couplings = [[[0.0f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT];
        for ctx in 0..MODEL_COUNT {
            for i in 0..DARK_COUNT {
                couplings[ctx][i][i] = (i + 1) as f64 + ctx as f64 * 0.1;
            }
        }
        let selectors = derive_all(&couplings);

        // Each selector should have different biases
        for i in 0..MODEL_COUNT {
            for j in (i + 1)..MODEL_COUNT {
                let differs =
                    (0..MODEL_COUNT).any(|k| (selectors[i].b[k] - selectors[j].b[k]).abs() > 1e-6);
                assert!(
                    differs,
                    "selectors[{}] and [{}] should have different biases",
                    i, j
                );
            }
        }
    }

    #[test]
    fn crystallize_converges_on_static_coupling() {
        // Static coupling: compile_fn always returns the same matrices
        let mut couplings = [[[0.0f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT];
        for ctx in 0..MODEL_COUNT {
            for i in 0..DARK_COUNT {
                couplings[ctx][i][i] = (i + 1) as f64 * (ctx as f64 + 1.0);
            }
        }

        let (fate, loss) = crystallize(
            |_| couplings,
            100,   // max iterations
            1e-10, // tolerance
            1.0,   // full replacement (no damping needed for static)
        );

        // Should converge in exactly 2 iterations (first iteration sets values,
        // second confirms they're stable)
        assert!(
            loss.iterations <= 2,
            "static coupling should converge in 2 iterations, took {}",
            loss.iterations
        );
        assert!(
            loss.max_delta < 1e-10,
            "should converge below tolerance, max_delta = {}",
            loss.max_delta
        );

        // Verify the crystallized weights are non-trivial
        let total_weight: f64 = fate.selectors[0]
            .w
            .iter()
            .flat_map(|row| row.iter())
            .map(|x| x.abs())
            .sum();
        assert!(
            total_weight > 0.0,
            "crystallized weights should be non-trivial"
        );
    }

    #[test]
    fn crystallize_roundtrip_weights_stable() {
        // Test: derive → route → derive again → same weights (crystal)
        let mut couplings = [[[0.0f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT];
        for ctx in 0..MODEL_COUNT {
            for i in 0..DARK_COUNT {
                couplings[ctx][i][i] = (i + 1) as f64 + ctx as f64 * 0.5;
            }
            // Add some off-diagonal coupling
            couplings[ctx][0][1] = 0.3 * (ctx as f64 + 1.0);
            couplings[ctx][1][0] = 0.3 * (ctx as f64 + 1.0);
        }

        // First crystallization
        let (fate1, _) = crystallize(|_| couplings, 100, 1e-12, 1.0);

        // Second crystallization with same input
        let (fate2, _) = crystallize(|_| couplings, 100, 1e-12, 1.0);

        // Weights should be identical (deterministic derivation)
        for ctx in 0..MODEL_COUNT {
            for i in 0..MODEL_COUNT {
                assert!(
                    (fate1.selectors[ctx].b[i] - fate2.selectors[ctx].b[i]).abs() < 1e-10,
                    "bias mismatch at ctx={} model={}",
                    ctx,
                    i
                );
                for j in 0..FEATURE_DIM {
                    assert!(
                        (fate1.selectors[ctx].w[i][j] - fate2.selectors[ctx].w[i][j]).abs() < 1e-10,
                        "weight mismatch at ctx={} model={} dim={}",
                        ctx,
                        i,
                        j
                    );
                }
            }
        }
    }

    #[test]
    fn derived_weights_produce_meaningful_routing() {
        // Create coupling matrices where one dark dimension dominates per context
        let mut couplings = [[[0.0f64; DARK_COUNT]; DARK_COUNT]; MODEL_COUNT];
        for ctx in 0..MODEL_COUNT {
            // Make one dimension dominant
            let dominant = ctx * 2; // 0, 2, 4, 6, 8
            couplings[ctx][dominant][dominant] = 10.0;
            // Others are weak
            for i in 0..DARK_COUNT {
                if i != dominant {
                    couplings[ctx][i][i] = 1.0;
                }
            }
        }

        let (fate, _) = crystallize(|_| couplings, 10, 1e-10, 1.0);

        // For each context, the dominant dark dimension should have the highest
        // weight for the first model (highest eigenvalue)
        for ctx in 0..MODEL_COUNT {
            let dominant_dark_idx = ctx * 2;
            let dominant_full_dim = DARK[dominant_dark_idx];
            let w0 = fate.selectors[ctx].w[0][dominant_full_dim].abs();
            assert!(
                w0 > 0.5,
                "ctx={}: dominant dim {} weight should be large, got {}",
                ctx,
                dominant_full_dim,
                w0
            );
        }
    }

    #[test]
    fn convergence_loss_implements_loss() {
        use crate::Loss;

        let zero = ConvergenceLoss::zero();
        assert!(zero.is_zero());

        let total = ConvergenceLoss::total();
        assert!(total.max_delta.is_infinite());

        let a = ConvergenceLoss {
            max_delta: 1.0,
            iterations: 3,
        };
        let b = ConvergenceLoss {
            max_delta: 2.0,
            iterations: 4,
        };
        let combined = a.combine(b);
        assert_eq!(combined.max_delta, 2.0); // max
        assert_eq!(combined.iterations, 7); // sum
    }

    #[test]
    fn derive_context_couplings_groups_by_model() {
        // Create loss entries attributed to different models
        let mut losses = Vec::new();

        // Abyss loss: strong dark coupling at [0][0]
        let mut delta_a = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        delta_a[feature::CREATIVITY][feature::CREATIVITY] = 5.0;
        losses.push((Model::Abyss, ManifoldLoss { delta: delta_a }));

        // Introject loss: strong dark coupling at [1][1]
        let mut delta_i = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
        delta_i[feature::CONFIDENCE][feature::CONFIDENCE] = 3.0;
        losses.push((Model::Introject, ManifoldLoss { delta: delta_i }));

        let couplings = derive_context_couplings(&losses);

        // Abyss context: CREATIVITY (DARK[0]) diagonal should be 5.0
        assert!(
            (couplings[0][0][0] - 5.0).abs() < 1e-12,
            "Abyss context: CREATIVITY coupling should be 5.0"
        );

        // Introject context: CONFIDENCE (DARK[1]) diagonal should be 3.0
        assert!(
            (couplings[1][1][1] - 3.0).abs() < 1e-12,
            "Introject context: CONFIDENCE coupling should be 3.0"
        );

        // Unused contexts should be zero
        assert!(
            couplings[2][0][0].abs() < 1e-12,
            "Cartographer context should be zero (no losses)"
        );
    }
}

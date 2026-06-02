//! Spectral dimension via the heat kernel on a graph Laplacian.
//!
//! The combinatorial graph Laplacian `L = D − A` (degree diagonal minus
//! adjacency) is real, symmetric, and positive semi-definite. Its eigenvalues
//! `λ_k` (ascending, `λ_0 = 0` for a connected graph) drive a diffusion
//! process whose return probability encodes the *spectral dimension* — the
//! dimension a random walker "feels" at a given diffusion time `σ`.
//!
//! - Heat-kernel trace:        `Z(σ) = Σ_k e^(−λ_k σ)`
//! - Return probability:       `P(σ) = (1/N) Σ_k e^(−λ_k σ)`
//! - Spectral dimension:       `d_s(σ) = −2 · d log P / d log σ`
//!
//! Using the closed form for the log-derivative (no finite differences):
//!
//! ```text
//! d_s(σ) = 2σ · (Σ_k λ_k e^(−λ_k σ)) / (Σ_k e^(−λ_k σ))
//! ```
//!
//! Sanity: a continuum `d`-dimensional manifold has `P ~ σ^(−d/2)`, giving
//! `d_s = d`.
//!
//! ## The λ_0 caveat
//!
//! The zero mode (`λ_0 = 0`) contributes a constant `1` to `Z(σ)`. At LARGE σ
//! every non-zero mode has decayed and that constant dominates, dragging
//! `d_s → 0`. So `d_s` is only meaningful at small-to-intermediate σ (the fine
//! scale), where it plateaus near the true dimension. Read `d_s` in that
//! scaling window, not at `σ → ∞`.
//!
//! This module is compiled only with `--features lapack` because it leans on
//! [`crate::ffi::eigensystem`] (LAPACK `dsyev`).

use crate::ffi;

/// Diffusion time (heat-kernel parameter) `σ`. Larger σ ⇒ coarser scale.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Sigma(f64);

impl Sigma {
    pub fn new(v: f64) -> Self {
        Sigma(v)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

impl From<f64> for Sigma {
    fn from(v: f64) -> Self {
        Sigma(v)
    }
}

impl std::fmt::Display for Sigma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "σ={:.6}", self.0)
    }
}

/// The spectral dimension `d_s(σ)` — the effective dimension a diffusion
/// process feels at scale σ. For a `d`-dimensional manifold this is `d`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SpectralDimension(f64);

impl SpectralDimension {
    pub fn new(v: f64) -> Self {
        SpectralDimension(v)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

impl std::fmt::Display for SpectralDimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "d_s={:.4}", self.0)
    }
}

/// The heat-kernel return probability `P(σ) = (1/N) Σ_k e^(−λ_k σ)`.
///
/// `eigenvalues` are the Laplacian spectrum (any order). `N` is their count.
/// Returns `0.0` for an empty spectrum.
pub fn heat_return_probability(eigenvalues: &[f64], sigma: Sigma) -> f64 {
    let n = eigenvalues.len();
    if n == 0 {
        return 0.0;
    }
    let s = sigma.as_f64();
    let z: f64 = eigenvalues.iter().map(|&lambda| (-lambda * s).exp()).sum();
    z / n as f64
}

/// The spectral dimension `d_s(σ) = 2σ · (Σ λ_k e^(−λ_k σ)) / (Σ e^(−λ_k σ))`.
///
/// `eigenvalues` are the Laplacian spectrum (any order). Returns `0.0` for an
/// empty spectrum.
pub fn spectral_dimension(eigenvalues: &[f64], sigma: Sigma) -> SpectralDimension {
    if eigenvalues.is_empty() {
        return SpectralDimension(0.0);
    }
    let s = sigma.as_f64();
    // Z = Σ e^(−λσ);  W = Σ λ e^(−λσ).  Both normalised by N cancels in the
    // ratio, so we keep the unnormalised sums.
    let mut z = 0.0_f64;
    let mut w = 0.0_f64;
    for &lambda in eigenvalues {
        let weight = (-lambda * s).exp();
        z += weight;
        w += lambda * weight;
    }
    if z == 0.0 {
        return SpectralDimension(0.0);
    }
    SpectralDimension(2.0 * s * w / z)
}

/// Build the combinatorial Laplacian `L = D − A` from an undirected edge list,
/// as a row-major `n×n` matrix suitable for [`ffi::eigensystem`].
///
/// Each edge `(u, v)` adds `1` to `A[u][v]` and `A[v][u]` and increments the
/// degree of both endpoints. Self-loops and multi-edges are taken at face
/// value (degree counts them). Vertices are `0..n`.
pub fn laplacian_from_edges(n: usize, edges: &[(usize, usize)]) -> Vec<f64> {
    let mut l = vec![0.0_f64; n * n];
    for &(u, v) in edges {
        // Off-diagonal: −A. Symmetric for an undirected edge.
        l[u * n + v] -= 1.0;
        l[v * n + u] -= 1.0;
        // Diagonal: +degree.
        l[u * n + u] += 1.0;
        l[v * n + v] += 1.0;
    }
    l
}

/// Eigendecompose a graph's Laplacian (via LAPACK `dsyev`) and return
/// `d_s(σ)` at the given diffusion time.
///
/// Convenience over [`laplacian_from_edges`] + [`ffi::eigensystem`] +
/// [`spectral_dimension`]. Returns `Err(info)` if LAPACK fails to converge.
pub fn graph_spectral_dimension(
    n: usize,
    edges: &[(usize, usize)],
    sigma: Sigma,
) -> Result<SpectralDimension, i32> {
    let lap = laplacian_from_edges(n, edges);
    let (evals, _evecs) = ffi::eigensystem(n, &lap)?;
    Ok(spectral_dimension(&evals, sigma))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 2D periodic grid (torus) on an `l × l` lattice.
    ///
    /// Vertex `(i, j)` ↦ index `i * l + j`. Each vertex connects to its four
    /// neighbours with wrap-around, so every vertex has degree 4 and the graph
    /// is a discrete flat 2-torus — its spectral dimension should plateau at 2.
    fn torus_2d(l: usize) -> (usize, Vec<(usize, usize)>) {
        let n = l * l;
        let mut edges = Vec::new();
        let idx = |i: usize, j: usize| -> usize { (i % l) * l + (j % l) };
        for i in 0..l {
            for j in 0..l {
                let u = idx(i, j);
                // +i and +j neighbours only (each undirected edge added once).
                edges.push((u, idx(i + 1, j)));
                edges.push((u, idx(i, j + 1)));
            }
        }
        (n, edges)
    }

    /// Build a 1D ring (cycle) graph on `n` vertices.
    ///
    /// Vertex `i` connects to `i+1 (mod n)`; every vertex has degree 2. This is
    /// the discrete circle — a flat 1-torus — so its spectral dimension should
    /// plateau at 1.
    fn ring(n: usize) -> (usize, Vec<(usize, usize)>) {
        let edges: Vec<(usize, usize)> = (0..n).map(|i| (i, (i + 1) % n)).collect();
        (n, edges)
    }

    /// Build a 3D periodic lattice (3-torus) on an `l × l × l` grid.
    ///
    /// Vertex `(i,j,k)` ↦ `i·l² + j·l + k`; degree 6 with wrap-around. The
    /// discrete flat 3-torus — spectral dimension should plateau at 3.
    fn torus_3d(l: usize) -> (usize, Vec<(usize, usize)>) {
        let n = l * l * l;
        let mut edges = Vec::new();
        let idx =
            |i: usize, j: usize, k: usize| -> usize { (i % l) * l * l + (j % l) * l + (k % l) };
        for i in 0..l {
            for j in 0..l {
                for k in 0..l {
                    let u = idx(i, j, k);
                    edges.push((u, idx(i + 1, j, k)));
                    edges.push((u, idx(i, j + 1, k)));
                    edges.push((u, idx(i, j, k + 1)));
                }
            }
        }
        (n, edges)
    }

    /// A log-spaced σ grid from `lo` to `hi` with `steps` points.
    fn log_grid(lo: f64, hi: f64, steps: usize) -> Vec<f64> {
        let (a, b) = (lo.ln(), hi.ln());
        (0..steps)
            .map(|i| (a + (b - a) * i as f64 / (steps - 1) as f64).exp())
            .collect()
    }

    // --- unit-level checks on the closed forms (no LAPACK needed) ---

    #[test]
    fn return_probability_at_zero_sigma_is_one() {
        // P(0) = (1/N) Σ e^0 = 1 regardless of spectrum.
        let evals = [0.0, 1.0, 2.5, 4.0];
        let p = heat_return_probability(&evals, Sigma::new(0.0));
        assert!((p - 1.0).abs() < 1e-12, "P(0)={p}");
    }

    #[test]
    fn empty_spectrum_is_zero() {
        assert_eq!(heat_return_probability(&[], Sigma::new(1.0)), 0.0);
        assert_eq!(spectral_dimension(&[], Sigma::new(1.0)).as_f64(), 0.0);
    }

    #[test]
    fn single_nonzero_mode_pure_power_law() {
        // One mode λ: P(σ) = e^(−λσ)/1, d_s = 2σλ. Verifies the closed form
        // against a hand value.
        let evals = [3.0];
        let ds = spectral_dimension(&evals, Sigma::new(2.0)).as_f64();
        assert!((ds - 12.0).abs() < 1e-12, "d_s={ds}, want 2·2·3=12");
    }

    #[test]
    fn laplacian_is_symmetric_and_rows_sum_to_zero() {
        // Path 0-1-2: degrees 1,2,1. Row sums of L must vanish (L·1 = 0).
        let (n, edges) = (3, vec![(0_usize, 1_usize), (1, 2)]);
        let l = laplacian_from_edges(n, &edges);
        for i in 0..n {
            let row_sum: f64 = (0..n).map(|j| l[i * n + j]).sum();
            assert!(row_sum.abs() < 1e-12, "row {i} sums to {row_sum}");
            for j in 0..n {
                assert_eq!(l[i * n + j], l[j * n + i], "asymmetry at ({i},{j})");
            }
        }
        // Diagonal = degree.
        assert_eq!(l[0], 1.0);
        assert_eq!(l[1 * n + 1], 2.0);
        assert_eq!(l[2 * n + 2], 1.0);
    }

    // --- known-answer dimension recovery (LAPACK) ---

    /// d_s should recover 2.0 on a 2D torus, read in the scaling window.
    ///
    /// Window: a log-spaced σ grid where the zero mode hasn't yet swamped the
    /// trace. For an l×l torus the smallest non-zero eigenvalue is
    /// `2(1 − cos(2π/l)) ≈ (2π/l)²`, so the plateau lives around
    /// `σ ∈ [~1, l²/(2π)²]`. We sample the centre of that window.
    #[test]
    fn torus_2d_recovers_dimension_two() {
        let l = 24;
        let (n, edges) = torus_2d(l);
        let lap = laplacian_from_edges(n, &edges);
        let (evals, _) = ffi::eigensystem(n, &lap).expect("dsyev converges");

        // Scaling window: σ in [2, 12]. Below 2 the high-λ modes still bite;
        // above ~ l²/(2π)² ≈ 15 the zero mode begins to dominate.
        let window = [2.0_f64, 4.0, 6.0, 8.0, 10.0, 12.0];
        for &s in &window {
            let ds = spectral_dimension(&evals, Sigma::new(s)).as_f64();
            println!("  [2D l={l}] σ={s:>5.1}  d_s={ds:.4}");
            assert!(
                (ds - 2.0).abs() < 0.2,
                "σ={s}: d_s={ds} not within 0.2 of 2.0 on 2D torus",
            );
        }
    }

    /// d_s should recover 1.0 on a ring (1-torus).
    ///
    /// Ring spectrum: λ_k = 2(1−cos(2πk/n)), k=0..n−1. Smallest non-zero
    /// ≈ (2π/n)². For n=400 the plateau window is wide; we read σ∈[3,60].
    #[test]
    fn ring_recovers_dimension_one() {
        let n = 400;
        let (n, edges) = ring(n);
        let lap = laplacian_from_edges(n, &edges);
        let (evals, _) = ffi::eigensystem(n, &lap).expect("dsyev converges");

        let window = [3.0_f64, 8.0, 15.0, 30.0, 60.0];
        for &s in &window {
            let ds = spectral_dimension(&evals, Sigma::new(s)).as_f64();
            println!("  [ring n={n}] σ={s:>5.1}  d_s={ds:.4}");
            assert!(
                (ds - 1.0).abs() < 0.15,
                "σ={s}: d_s={ds} not within 0.15 of 1.0 on ring",
            );
        }
    }

    /// d_s should recover 3.0 on a 3D torus.
    ///
    /// l=10 ⇒ N=1000 vertices, degree 6. Smallest non-zero λ ≈ (2π/10)² ≈ 0.39,
    /// largest 12. Plateau window σ∈[1.5,5]: small enough that the zero mode is
    /// still negligible, large enough that the lattice-discreteness (max-λ tail)
    /// has averaged out.
    #[test]
    fn torus_3d_recovers_dimension_three() {
        let l = 14;
        let (n, edges) = torus_3d(l);
        let lap = laplacian_from_edges(n, &edges);
        let (evals, _) = ffi::eigensystem(n, &lap).expect("dsyev converges");

        // Window σ∈[3,6]. A discrete d-torus has no perfectly flat d_s plateau:
        // at small σ the lattice band structure overshoots above d, and the
        // d_s(σ) curve descends monotonically through d before the zero mode
        // pulls it toward 0. l=14 widens the scaling window enough that the
        // crossing-through-3 band sits in σ∈[3,6] with d_s within 0.3 of 3.
        let window = [3.0_f64, 3.5, 4.0, 4.5, 5.0, 6.0];
        for &s in &window {
            let ds = spectral_dimension(&evals, Sigma::new(s)).as_f64();
            println!("  [3D l={l}] σ={s:>5.1}  d_s={ds:.4}");
            assert!(
                (ds - 3.0).abs() < 0.3,
                "σ={s}: d_s={ds} not within 0.3 of 3.0 on 3D torus",
            );
        }
    }

    /// Dimensional-flow observation (reported, not hard-asserted beyond a
    /// monotonicity sanity check). On a single 3D torus, sweep a log-spaced σ
    /// grid and print the d_s(σ) curve. Running σ high→low, d_s should rise
    /// from ~0 (large σ, zero mode dominant) up toward 3 (fine scale), then
    /// the lattice cutoff pulls it back down at the very finest σ. This is the
    /// scale-dependence (dimensional flow) the cosmology probe relies on.
    #[test]
    fn dimensional_flow_curve_on_3d_torus() {
        let l = 14;
        let (n, edges) = torus_3d(l);
        let lap = laplacian_from_edges(n, &edges);
        let (evals, _) = ffi::eigensystem(n, &lap).expect("dsyev converges");

        let grid = log_grid(0.05, 500.0, 18);
        println!("\n  d_s(σ) flow — 3D torus, N={n}, degree 6");
        println!("  {:>12}  {:>10}  {:>10}", "σ", "P(σ)", "d_s(σ)");
        let mut peak = 0.0_f64;
        for &s in &grid {
            let p = heat_return_probability(&evals, Sigma::new(s));
            let ds = spectral_dimension(&evals, Sigma::new(s)).as_f64();
            peak = peak.max(ds);
            println!("  {s:>12.4}  {p:>10.6}  {ds:>10.4}");
        }
        // The curve must reach near the embedding dimension somewhere in the
        // sweep, and collapse toward 0 at the coarse end (zero-mode dominance).
        let ds_large = spectral_dimension(&evals, Sigma::new(500.0)).as_f64();
        assert!(peak > 2.5, "flow never approached d≈3 (peak d_s={peak})");
        assert!(
            ds_large < 0.5,
            "d_s did not collapse at large σ (d_s(500)={ds_large})",
        );
    }
}

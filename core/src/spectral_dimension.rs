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

    /// d_s should recover 2.0 on a 2D torus, read in the scaling window.
    ///
    /// Window: a log-spaced σ grid where the zero mode hasn't yet swamped the
    /// trace. For an l×l torus the smallest non-zero eigenvalue is
    /// `2(1 − cos(2π/l)) ≈ (2π/l)²`, so the plateau lives around
    /// `σ ∈ [~1, l²/(2π)²]`. We sample the geometric centre of that window.
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
            assert!(
                (ds - 2.0).abs() < 0.2,
                "σ={s}: d_s={ds} not within 0.2 of 2.0 on 2D torus",
            );
        }
    }
}

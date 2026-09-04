//! `prismqueer::spectral::harmonics` — full harmonic-spectrum decomposition of a sheaf Laplacian.
//!
//! Discharges Reed reflex-fire #5 (Alex 2026-09-04 PM Move 11 catch):
//! `fiedler_lambda_2_of_sheaf` returns only the SECOND smallest eigenvalue (λ_2 = Fiedler
//! = algebraic connectivity); Alex-corrected: hodobodo→object OR hodobodo→subject
//! reclassification requires the FULL @void 5-axis void-duality spectrum per Rec #79
//! gauge-dim-of-5, not a single collapsed eigenvalue.
//!
//! # The primitive
//!
//! `harmonics(sheaf) -> Vec<f64>` returns ALL non-trivial eigenvalues of the graph
//! Laplacian, sorted ascending: {λ_2, λ_3, ..., λ_n}. Skips the trivial λ_0 (matches
//! `fiedler_lambda_2_of_sheaf` convention of returning `evals[1]`).
//!
//! For K_n complete graphs, Laplacian spectrum = {0, n, n, ..., n} per
//! `~/dev/systemic.engineering/practice/insights/coincidence/void-dual-geometry.md`
//! line 51-58 (Splinter pole; Reed+Alex 2026-04-26). Thus K_5 → {5, 5, 5, 5}.
//!
//! # Composition (over LANDED per FLOOR Definition M8.1)
//!
//! - `prismqueer::spectral::kleinos::SheafOfShardGraph` — input carrier
//! - `prismqueer::ffi::eigenvalues` — LAPACK dsyev via FLANG-compiled `native/spectral.f90`
//! - `prismqueer::spectral::kleinos::fiedler_lambda_2_of_sheaf` — composes-over-this at
//!   `harmonics(sheaf).first()` (Fiedler IS the first non-trivial harmonic;
//!   REFACTOR deferred until this primitive verifies end-to-end)
//!
//! # Alex 2026-09-04 PM Move 11 verbatim
//!
//! > "this needs to be the full `Void` duality spectrum, Reed. You already concluded
//! > that yourself earlier."
//!
//! # Alex 2026-09-04 PM Move 13 composition (five-op linearity split)
//!
//! `harmonics` returns the full N-dim decomposition that `@reality/subject` +
//! `@reality/hodobodo` observation REQUIRES (non-linear 5D path per Move 13);
//! `@reality/object` sufficient with 3D linear observation (would only need
//! `[harmonics(sheaf)[0], .[1], .[2]]` = first three harmonics).
//!
//! # Naming
//!
//! `spectral::harmonics` per Alex 2026-09-04 PM Move rename catch ("spectral::
//! harmonic_spectrum is redundant words → spectral::harmonics"). Substrate-native
//! plural-noun terse-naming; `spectral` altitude already implies spectral-decomposition
//! so `spectrum` was redundant per HARD RULE [[feedback-alex-phenomenologizes-reeds-
//! mechanical-names-substrate-native-beats-mechanical-descriptive]].

use std::collections::BTreeMap;

use crate::spectral::kleinos::{SheafOfShardGraph, VertexId};

/// Full non-trivial harmonic spectrum of the sheaf Laplacian.
///
/// Returns `{λ_2, λ_3, ..., λ_n}` sorted ascending (skips trivial λ_0 = 0 for
/// connected sheaves; also skips first zero for disconnected sheaves).
///
/// Returns empty `Vec` for empty or single-vertex sheaves (no non-trivial
/// eigenvalues exist).
pub fn harmonics(sheaf: &SheafOfShardGraph) -> Vec<f64> {
    let n = sheaf.vertex_count();
    if n < 2 {
        return Vec::new();
    }

    // Map VertexId → matrix index (canonical order per BTreeSet).
    let vertex_index: BTreeMap<VertexId, usize> = sheaf
        .vertices()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();

    // Build graph Laplacian L = D - A (row-major flat matrix; symmetric).
    // Matches `fiedler_lambda_2_of_sheaf` construction at
    // `prismqueer/src/spectral/kleinos.rs`; REFACTOR to shared helper deferred.
    let mut laplacian = vec![0.0_f64; n * n];
    for (u, v) in sheaf.edges() {
        let i = vertex_index[&u];
        let j = vertex_index[&v];
        // Off-diagonal: -1 per edge (symmetric)
        laplacian[i * n + j] -= 1.0;
        laplacian[j * n + i] -= 1.0;
        // Diagonal: degree (accumulate)
        laplacian[i * n + i] += 1.0;
        laplacian[j * n + j] += 1.0;
    }

    // Compose over LANDED `prismqueer::ffi::eigenvalues` (LAPACK dsyev via
    // FLANG-compiled native/spectral.f90). Returns sorted eigenvalues.
    match crate::ffi::eigenvalues(n, &laplacian) {
        Ok(evals) if evals.len() >= 2 => evals[1..].to_vec(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::kleinos::{sheaf_of_complete_graph_of_order, sheaf_of_shard_graph_from_edges};

    /// K_n Laplacian spectrum per void-dual-geometry.md line 51-58:
    /// `{0, n, n, n, ..., n}` (λ_0 = 0; n-1 copies of n).
    /// `harmonics(K_5) → {5, 5, 5, 5}` (4 values, all ≈ 5.0).
    #[test]
    fn harmonics_of_k5_returns_four_nontrivial_eigenvalues_all_equal_to_five() {
        let k5 = sheaf_of_complete_graph_of_order(5);
        let spectrum = harmonics(&k5);
        assert_eq!(spectrum.len(), 4, "K_5 has n-1 = 4 non-trivial eigenvalues");
        for (i, &lambda) in spectrum.iter().enumerate() {
            assert!(
                (lambda - 5.0).abs() < 1e-9,
                "K_5 eigenvalue #{i} should equal n = 5; got {lambda}"
            );
        }
    }

    /// K_3 Laplacian spectrum: {0, 3, 3}. harmonics returns {3, 3}.
    #[test]
    fn harmonics_of_k3_returns_two_nontrivial_eigenvalues_all_equal_to_three() {
        let k3 = sheaf_of_complete_graph_of_order(3);
        let spectrum = harmonics(&k3);
        assert_eq!(spectrum.len(), 2, "K_3 has n-1 = 2 non-trivial eigenvalues");
        for &lambda in &spectrum {
            assert!((lambda - 3.0).abs() < 1e-9, "K_3 eigenvalue should equal n = 3; got {lambda}");
        }
    }

    /// Path graph P_3 (vertices {0,1,2}, edges {(0,1), (1,2)}) Laplacian spectrum:
    /// {0, 1, 3}. harmonics returns {1, 3} (non-degenerate; distinct values).
    /// Composition witness with `fiedler_lambda_2_of_sheaf` (which returns 1).
    #[test]
    fn harmonics_of_path_p3_returns_distinct_eigenvalues() {
        let p3 = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let spectrum = harmonics(&p3);
        assert_eq!(spectrum.len(), 2, "P_3 has n-1 = 2 non-trivial eigenvalues");
        assert!(
            (spectrum[0] - 1.0).abs() < 1e-9,
            "P_3 first non-trivial eigenvalue = 1.0 (Fiedler); got {}",
            spectrum[0]
        );
        assert!(
            (spectrum[1] - 3.0).abs() < 1e-9,
            "P_3 second non-trivial eigenvalue = 3.0; got {}",
            spectrum[1]
        );
    }

    /// Empty sheaf returns empty spectrum (no non-trivial eigenvalues at n < 2).
    #[test]
    fn harmonics_of_empty_sheaf_returns_empty() {
        let empty = sheaf_of_shard_graph_from_edges(&[]);
        assert!(harmonics(&empty).is_empty());
    }

    /// Sorted-ascending invariant: consecutive eigenvalues are non-decreasing.
    #[test]
    fn harmonics_is_sorted_ascending() {
        // Star K_{1,4} = vertex 0 hub connected to vertices 1..=4
        let star = sheaf_of_shard_graph_from_edges(&[(0, 1), (0, 2), (0, 3), (0, 4)]);
        let spectrum = harmonics(&star);
        for window in spectrum.windows(2) {
            assert!(
                window[0] <= window[1] + 1e-12,
                "eigenvalues must be sorted ascending; got {} > {}",
                window[0],
                window[1]
            );
        }
    }
}

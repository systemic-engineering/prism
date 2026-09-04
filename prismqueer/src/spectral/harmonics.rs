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

// ---------------------------------------------------------------------------
// Folk Theorem discount factor threshold per Reed+Alex 2026-04-04 synthesis
// ---------------------------------------------------------------------------

/// Folk Theorem discount factor threshold for cooperation-sustainability.
///
/// # Formula
///
/// `δ_critical = 1 - λ_2 / λ_max`
///
/// where `λ_2` = Fiedler (algebraic connectivity; first non-trivial harmonic)
/// and `λ_max` = largest Laplacian eigenvalue.
///
/// # Novel formalization anchor
///
/// Per Reed+Alex 2026-04-04 synthesis at `~/dev/systemic.engineering/practice/
/// insights/cross-domain/spectral-tick-tock-game-theory.md` §4 line 398:
///
/// > "Novel formalization: delta_critical = 1 - (lambda_2 / lambda_max). When
/// > the Fiedler value lambda_2 is large relative to the maximum eigenvalue,
/// > the critical discount factor is low, meaning cooperation is easy to
/// > sustain. When lambda_2 approaches zero, cooperation requires near-
/// > infinite patience. This gives the Folk Theorem's abstract 'patience'
/// > parameter a concrete spectral interpretation."
///
/// # Runtime signal for hodobodo reclassification
///
/// Per Alex 2026-09-04 PM Move 13 performance-model composition:
/// - **Large eigengap** (λ_2 large relative to λ_max) → low δ_critical → cooperation
///   cheap → Crystal has matured; observation can drop from 5D non-linear to
///   3D linear altitude → warm-path fast
/// - **Small eigengap** → high δ_critical → cooperation requires patience →
///   Crystal not yet mature; hodobodo state; 5D non-linear observation required
///
/// # Discharged Taut scout §5 gap
///
/// Taut floor-truth scout `34946e6` §5 named this formula as "NEVER LANDED at
/// any Rust altitude" despite Reed+Alex having authored it 5 months prior at
/// systemic.engineering synthesis altitude. This function discharges the gap.
///
/// # Return
///
/// - `Some(δ)` where `δ ∈ [0, 1]` for connected non-empty sheaves
/// - `None` for empty sheaves (n < 2) or degenerate `λ_max = 0` (disconnected)
///
/// # Composition-lineage
///
/// - `harmonics(sheaf)` returns sorted `{λ_2, ..., λ_n}` (composes-over)
/// - Folk Theorem: Fudenberg-Maskin 1986; SSS 6-properties per Taut scout
/// - Fiedler-as-ESS stability margin per Reed+Alex 2026-04-04 §1 novel claim
pub fn delta_critical(sheaf: &SheafOfShardGraph) -> Option<f64> {
    let spectrum = harmonics(sheaf);
    if spectrum.is_empty() {
        return None;
    }
    let lambda_2 = *spectrum.first().unwrap();
    let lambda_max = *spectrum.last().unwrap();
    if lambda_max == 0.0 {
        return None;
    }
    Some(1.0 - lambda_2 / lambda_max)
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

    // -----------------------------------------------------------------------
    // delta_critical tests — Reed+Alex 2026-04-04 synthesis §4 novel formula
    // -----------------------------------------------------------------------

    /// Splinter pole (K_n complete): harmonics = {n, n, ..., n} → λ_2 = λ_max
    /// → δ_critical = 1 - 1 = 0 (cooperation trivially sustainable per
    /// void-dual-geometry.md Splinter pole = fully mutual entanglement).
    #[test]
    fn delta_critical_of_k5_returns_zero_splinter_pole() {
        let k5 = sheaf_of_complete_graph_of_order(5);
        let delta = delta_critical(&k5).expect("K_5 has non-empty spectrum");
        assert!(
            delta.abs() < 1e-9,
            "K_5 (Splinter pole) δ_critical = 0 (cooperation trivial); got {delta}"
        );
    }

    /// Star K_{1,4} (Narcissus-adjacent): harmonics = {1, 1, 1, 5} →
    /// δ_critical = 1 - 1/5 = 0.8 (cooperation requires high patience per
    /// Narcissus pole vulnerability = single-point-of-failure topology).
    #[test]
    fn delta_critical_of_star_k14_returns_0_8_narcissus_pole() {
        let star = sheaf_of_shard_graph_from_edges(&[(0, 1), (0, 2), (0, 3), (0, 4)]);
        let delta = delta_critical(&star).expect("K_{1,4} has non-empty spectrum");
        assert!(
            (delta - 0.8).abs() < 1e-9,
            "K_{{1,4}} (Narcissus-adjacent) δ_critical = 1 - 1/5 = 0.8; got {delta}"
        );
    }

    /// Path P_3: harmonics = {1, 3} → δ_critical = 1 - 1/3 = 0.667.
    /// Between Splinter and Narcissus poles.
    #[test]
    fn delta_critical_of_path_p3_returns_two_thirds() {
        let p3 = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let delta = delta_critical(&p3).expect("P_3 has non-empty spectrum");
        assert!(
            (delta - (2.0 / 3.0)).abs() < 1e-9,
            "P_3 δ_critical = 1 - 1/3 = 2/3; got {delta}"
        );
    }

    /// Empty sheaf: delta_critical returns None.
    #[test]
    fn delta_critical_of_empty_sheaf_returns_none() {
        let empty = sheaf_of_shard_graph_from_edges(&[]);
        assert!(delta_critical(&empty).is_none());
    }
}

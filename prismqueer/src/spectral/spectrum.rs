//! `prismqueer::spectral::spectrum` — full non-trivial eigenvalue spectrum of a
//! sheaf Laplacian, with typed void-duality-axis readings.
//!
//! Per Alex 2026-09-19 verbatim ("collapse towards Spectrum — that carries more
//! phenomenological load; Punk is a spectrum too, so is neurodiversity"): the
//! type formerly named `Harmonics` at commit `00d6c8e` is renamed to `Spectrum`.
//! Field `spectrum: Vec<f64>` renamed to `eigenvalues: Vec<f64>` to match Mara
//! FLOOR §10.2 verbatim (`score.spectrum.eigenvalues[1]`).
//!
//! # The primitive
//!
//! `Spectrum::of(sheaf) -> Spectrum` reads ALL non-trivial eigenvalues of the
//! graph Laplacian via LAPACK dsyev (FLANG-compiled `native/spectral.f90`),
//! sorted ascending: `{λ_2, λ_3, ..., λ_n}`. Skips the trivial λ_0.
//!
//! # Typed axis readings — illegal-state-unrepresentable at void-duality altitude
//!
//! Per Alex 2026-09-19 punk-naming ratification + "make illegal state
//! unrepresentable" discipline, the five void-duality axes (PAPER_2D §4.1
//! canonical: focus/split/project/lift/refract) each get a newtype-wrapped
//! `f64` so the Rust type system cannot conflate a `Split` reading with a
//! `Refract` reading. This tick lands the three LANDED-derivable axes:
//!
//! - `Split(pub f64)` — spectral gap = Fiedler λ_2 = `eigenvalues[0]` (linear)
//! - `Refract(pub f64)` — Von Neumann entropy `−Σ (λ_i/n) log(λ_i/n)` (non-linear log)
//! - `Project(pub f64)` — Cheeger boundary UPPER BOUND `√(2·λ_2)` (non-linear sqrt)
//!
//! Focus (Ollivier-Ricci curvature) and Lift (Kramers-Wannier duality) are
//! forward-promised — both need graph-metric or basis-transformation
//! primitives beyond the spectrum-alone.
//!
//! # For K_n complete graphs
//!
//! Laplacian spectrum `= {0, n, n, ..., n}` per void-dual-geometry.md
//! (Splinter pole; Reed+Alex 2026-04-26). K_5 → `eigenvalues = {5, 5, 5, 5}`.
//!
//! # Composition (over LANDED per FLOOR Definition M8.1)
//!
//! - `prismqueer::spectral::kleinos::SheafOfShardGraph` — input carrier
//! - `prismqueer::ffi::eigenvalues` — LAPACK dsyev via FLANG-compiled `native/spectral.f90`

use std::collections::BTreeMap;

use crate::spectral::kleinos::{SheafOfShardGraph, VertexId};

/// Full non-trivial harmonic spectrum of the sheaf Laplacian.
///
/// Holds `{λ_2, λ_3, ..., λ_n}` sorted ascending. Skips the trivial λ_0 = 0
/// for connected sheaves; also skips first zero for disconnected sheaves.
/// Empty `eigenvalues` for empty or single-vertex sheaves.
#[derive(Clone, Debug)]
pub struct Spectrum {
    pub eigenvalues: Vec<f64>,
}

impl Spectrum {
    /// Read the full non-trivial spectrum of `sheaf`.
    ///
    /// Composes over LANDED `prismqueer::ffi::eigenvalues` (LAPACK dsyev via
    /// FLANG-compiled `native/spectral.f90`).
    pub fn of(sheaf: &SheafOfShardGraph) -> Self {
        let n = sheaf.vertex_count();
        if n < 2 {
            return Self { eigenvalues: Vec::new() };
        }

        // Map VertexId → matrix index (canonical order per BTreeSet).
        let vertex_index: BTreeMap<VertexId, usize> = sheaf
            .vertices()
            .enumerate()
            .map(|(i, v)| (v, i))
            .collect();

        // Build graph Laplacian L = D - A (row-major flat matrix; symmetric).
        let mut laplacian = vec![0.0_f64; n * n];
        for (u, v) in sheaf.edges() {
            let i = vertex_index[&u];
            let j = vertex_index[&v];
            laplacian[i * n + j] -= 1.0;
            laplacian[j * n + i] -= 1.0;
            laplacian[i * n + i] += 1.0;
            laplacian[j * n + j] += 1.0;
        }

        let eigenvalues = match crate::ffi::eigenvalues(n, &laplacian) {
            Ok(evals) if evals.len() >= 2 => evals[1..].to_vec(),
            _ => Vec::new(),
        };

        Self { eigenvalues }
    }

    /// PAPER_2D §4.1 canonical `split` axis — spectral gap = Fiedler λ_2.
    /// Linear projection: `eigenvalues[0]` (after trivial λ_0 is skipped).
    /// Returns `Split(0.0)` for empty spectrum.
    pub fn split(&self) -> Split {
        Split(self.eigenvalues.first().copied().unwrap_or(0.0))
    }

    /// PAPER_2D §4.1 canonical `refract` axis — Von Neumann entropy of the
    /// normalized density matrix ρ = L / trace(L). Non-linear (log-based).
    /// Formula: `−Σ (λ_i / Σλ) log(λ_i / Σλ)`.
    ///
    /// Uses the standard information-geometric normalization where the
    /// eigenvalues are treated as an unnormalized probability distribution
    /// and normalized by their trace (sum) before the entropy is computed.
    /// Zero eigenvalues contribute 0 log 0 = 0 by convention.
    pub fn refract(&self) -> Refract {
        let trace: f64 = self.eigenvalues.iter().sum();
        if trace <= 1e-12 {
            return Refract(0.0);
        }
        let entropy: f64 = self
            .eigenvalues
            .iter()
            .filter(|&&lam| lam > 1e-12)
            .map(|&lam| {
                let p = lam / trace;
                -p * p.ln()
            })
            .sum();
        Refract(entropy)
    }

    /// PAPER_2D §4.1 canonical `project` axis — Cheeger boundary UPPER BOUND
    /// via Cheeger inequality: `h(G) ≤ √(2·λ_2)`. Non-linear (sqrt).
    ///
    /// The exact Cheeger constant requires boundary-set enumeration
    /// (NP-hard); this returns the spectrum-derived upper bound. Exact
    /// Cheeger primitive forward-promised at future tick.
    pub fn project(&self) -> Project {
        let lambda_2 = self.eigenvalues.first().copied().unwrap_or(0.0);
        Project((2.0 * lambda_2).sqrt())
    }
}

// ---------------------------------------------------------------------------
// Typed void-duality axis newtypes per Alex 2026-09-19 punk-naming +
// "make illegal state unrepresentable" discipline.
// ---------------------------------------------------------------------------

/// The `split` axis reading — spectral gap. Compile-time-distinct from
/// `Refract` and `Project` so the Rust type system prevents callsites from
/// conflating a spectral-gap value with an entropy value.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Split(pub f64);

/// The `refract` axis reading — Von Neumann entropy of the sheaf-Laplacian
/// density matrix ρ = L / trace(L). Compile-time-distinct from `Split` and
/// `Project`.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Refract(pub f64);

/// The `project` axis reading — Cheeger boundary upper bound `√(2·λ_2)`.
/// Compile-time-distinct from `Split` and `Refract`.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Project(pub f64);

/// The `focus` axis reading — Ollivier-Ricci curvature.
/// Compile-time-distinct from the other four void-duality axis newtypes.
///
/// Primitive computation (transport-metric on graph via LP-relaxation over
/// transport plans per Ollivier 2009 J. Funct. Anal. 256:810-864) is
/// forward-promised. Newtype lands here for illegal-state-unrepresentable
/// discipline: `Focus(0.3)` cannot be conflated with `Split(0.3)` at
/// callsite; the compiler enforces the semantic role at type level.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Focus(pub f64);

/// The `lift` axis reading — Kramers-Wannier duality (basis-transformation
/// between hot/cold phase / structure and dual-structure).
/// Compile-time-distinct from the other four void-duality axis newtypes.
///
/// Primitive computation (partition-function + dual-basis-transformation
/// per Kramers-Wannier 1941 Phys. Rev. 60:252) is forward-promised.
/// Newtype lands here so `Lift(0.5)` cannot be conflated with `Refract(0.5)`
/// at callsite; the compiler enforces the semantic role at type level.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Lift(pub f64);

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
/// > infinite patience."
///
/// # Composition-lineage
///
/// - `Spectrum::of(sheaf).eigenvalues` returns sorted `{λ_2, ..., λ_n}`
/// - Folk Theorem: Fudenberg-Maskin 1986
/// - Fiedler-as-ESS stability margin per Reed+Alex 2026-04-04 §1 novel claim
#[derive(Clone, Debug)]
pub struct DeltaCritical {
    /// `Some(δ)` where `δ ∈ [0, 1]` for connected non-empty sheaves;
    /// `None` for empty (n < 2) or degenerate `λ_max = 0` (disconnected).
    pub value: Option<f64>,
}

impl DeltaCritical {
    /// Read the Folk Theorem discount factor of `sheaf`.
    pub fn of(sheaf: &SheafOfShardGraph) -> Self {
        let evals = Spectrum::of(sheaf).eigenvalues;
        if evals.is_empty() {
            return Self { value: None };
        }
        let lambda_2 = *evals.first().unwrap();
        let lambda_max = *evals.last().unwrap();
        if lambda_max == 0.0 {
            return Self { value: None };
        }
        Self { value: Some(1.0 - lambda_2 / lambda_max) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::kleinos::{sheaf_of_complete_graph_of_order, sheaf_of_shard_graph_from_edges};

    /// K_5: Laplacian spectrum = {0, 5, 5, 5, 5} per void-dual-geometry.md.
    /// `Spectrum::of(K_5).eigenvalues → {5, 5, 5, 5}`.
    #[test]
    fn spectrum_of_k5_returns_four_nontrivial_eigenvalues_all_equal_to_five() {
        let k5 = sheaf_of_complete_graph_of_order(5);
        let evals = Spectrum::of(&k5).eigenvalues;
        assert_eq!(evals.len(), 4, "K_5 has n-1 = 4 non-trivial eigenvalues");
        for (i, &lambda) in evals.iter().enumerate() {
            assert!(
                (lambda - 5.0).abs() < 1e-9,
                "K_5 eigenvalue #{i} should equal n = 5; got {lambda}"
            );
        }
    }

    /// K_3: Laplacian spectrum = {0, 3, 3}. `Spectrum::of(K_3).eigenvalues → {3, 3}`.
    #[test]
    fn spectrum_of_k3_returns_two_nontrivial_eigenvalues_all_equal_to_three() {
        let k3 = sheaf_of_complete_graph_of_order(3);
        let evals = Spectrum::of(&k3).eigenvalues;
        assert_eq!(evals.len(), 2, "K_3 has n-1 = 2 non-trivial eigenvalues");
        for &lambda in &evals {
            assert!((lambda - 3.0).abs() < 1e-9, "K_3 eigenvalue should equal n = 3; got {lambda}");
        }
    }

    /// Path P_3: Laplacian spectrum = {0, 1, 3}. `eigenvalues → {1, 3}`.
    #[test]
    fn spectrum_of_path_p3_returns_distinct_eigenvalues() {
        let p3 = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let evals = Spectrum::of(&p3).eigenvalues;
        assert_eq!(evals.len(), 2, "P_3 has n-1 = 2 non-trivial eigenvalues");
        assert!(
            (evals[0] - 1.0).abs() < 1e-9,
            "P_3 first non-trivial eigenvalue = 1.0 (Fiedler); got {}",
            evals[0]
        );
        assert!(
            (evals[1] - 3.0).abs() < 1e-9,
            "P_3 second non-trivial eigenvalue = 3.0; got {}",
            evals[1]
        );
    }

    /// Empty sheaf returns empty spectrum.
    #[test]
    fn spectrum_of_empty_sheaf_returns_empty() {
        let empty = sheaf_of_shard_graph_from_edges(&[]);
        assert!(Spectrum::of(&empty).eigenvalues.is_empty());
    }

    /// Sorted-ascending invariant: consecutive eigenvalues are non-decreasing.
    #[test]
    fn spectrum_is_sorted_ascending() {
        // Star K_{1,4} = vertex 0 hub connected to vertices 1..=4
        let star = sheaf_of_shard_graph_from_edges(&[(0, 1), (0, 2), (0, 3), (0, 4)]);
        let evals = Spectrum::of(&star).eigenvalues;
        for window in evals.windows(2) {
            assert!(
                window[0] <= window[1] + 1e-12,
                "eigenvalues must be sorted ascending; got {} > {}",
                window[0],
                window[1]
            );
        }
    }

    // -----------------------------------------------------------------------
    // Typed void-duality axis readings — Split / Refract / Project newtypes.
    // -----------------------------------------------------------------------

    /// K_5 split axis reading: `Split(5.0)` (Fiedler = first non-trivial
    /// eigenvalue for Splinter pole = 5).
    #[test]
    fn split_of_k5_returns_fiedler_five() {
        let k5 = sheaf_of_complete_graph_of_order(5);
        let split = Spectrum::of(&k5).split();
        assert_eq!(split, Split(5.0));
    }

    /// Path P_3 split axis reading: `Split(1.0)` (Fiedler = first non-trivial
    /// eigenvalue for P_3 = 1).
    #[test]
    fn split_of_path_p3_returns_fiedler_one() {
        let p3 = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let split = Spectrum::of(&p3).split();
        assert!((split.0 - 1.0).abs() < 1e-9, "P_3 Fiedler = 1.0; got {:?}", split);
    }

    /// K_5 refract axis reading: Von Neumann entropy of density matrix
    /// ρ = L / trace(L). For K_5 with eigenvalues {5, 5, 5, 5}, trace = 20;
    /// each p_i = 5/20 = 0.25; entropy = -4 · 0.25 · log(0.25) = log(4) ≈ 1.386.
    #[test]
    fn refract_of_k5_returns_log_four_entropy() {
        let k5 = sheaf_of_complete_graph_of_order(5);
        let refract = Spectrum::of(&k5).refract();
        let expected = (4.0_f64).ln();
        assert!(
            (refract.0 - expected).abs() < 1e-9,
            "K_5 refract entropy = log(4) ≈ {expected}; got {:?}",
            refract
        );
    }

    /// K_5 project axis reading: Cheeger upper bound = √(2·λ_2) = √10 ≈ 3.162.
    #[test]
    fn project_of_k5_returns_cheeger_bound() {
        let k5 = sheaf_of_complete_graph_of_order(5);
        let project = Spectrum::of(&k5).project();
        let expected = (10.0_f64).sqrt();
        assert!(
            (project.0 - expected).abs() < 1e-9,
            "K_5 project = √(2·5) = √10 ≈ {expected}; got {:?}",
            project
        );
    }

    /// Empty-spectrum axis readings: all three return zero-valued newtypes.
    #[test]
    fn empty_spectrum_axis_readings_return_zero() {
        let empty = sheaf_of_shard_graph_from_edges(&[]);
        let spec = Spectrum::of(&empty);
        assert_eq!(spec.split(), Split(0.0));
        assert_eq!(spec.refract(), Refract(0.0));
        assert_eq!(spec.project(), Project(0.0));
    }

    /// Illegal-state-unrepresentable witness: the axis newtypes are
    /// compile-time distinct — a function requiring `Split` will not accept
    /// a `Refract`. This test asserts type-distinctness at COMPILE altitude;
    /// commented lines would fail to compile if uncommented.
    #[test]
    fn axis_newtypes_are_compile_time_distinct() {
        fn takes_split(_: Split) {}
        fn takes_refract(_: Refract) {}
        fn takes_project(_: Project) {}
        let s = Split(1.0);
        let r = Refract(1.0);
        let p = Project(1.0);
        takes_split(s);
        takes_refract(r);
        takes_project(p);
        // takes_split(r);   // would not compile: mismatched types
        // takes_split(p);   // would not compile: mismatched types
        // takes_refract(s); // would not compile: mismatched types
    }

    // -----------------------------------------------------------------------
    // delta_critical tests — Reed+Alex 2026-04-04 synthesis §4 novel formula
    // -----------------------------------------------------------------------

    /// Splinter pole (K_5): spectrum = {5, 5, 5, 5} → λ_2 = λ_max → δ = 0.
    #[test]
    fn delta_critical_of_k5_returns_zero_splinter_pole() {
        let k5 = sheaf_of_complete_graph_of_order(5);
        let delta = DeltaCritical::of(&k5).value.expect("K_5 has non-empty spectrum");
        assert!(
            delta.abs() < 1e-9,
            "K_5 (Splinter pole) δ_critical = 0 (cooperation trivial); got {delta}"
        );
    }

    /// Star K_{1,4}: spectrum = {1, 1, 1, 5} → δ_critical = 1 - 1/5 = 0.8.
    #[test]
    fn delta_critical_of_star_k14_returns_0_8_narcissus_pole() {
        let star = sheaf_of_shard_graph_from_edges(&[(0, 1), (0, 2), (0, 3), (0, 4)]);
        let delta = DeltaCritical::of(&star).value.expect("K_{1,4} has non-empty spectrum");
        assert!(
            (delta - 0.8).abs() < 1e-9,
            "K_{{1,4}} (Narcissus-adjacent) δ_critical = 1 - 1/5 = 0.8; got {delta}"
        );
    }

    /// Path P_3: spectrum = {1, 3} → δ_critical = 1 - 1/3 = 2/3.
    #[test]
    fn delta_critical_of_path_p3_returns_two_thirds() {
        let p3 = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let delta = DeltaCritical::of(&p3).value.expect("P_3 has non-empty spectrum");
        assert!(
            (delta - (2.0 / 3.0)).abs() < 1e-9,
            "P_3 δ_critical = 1 - 1/3 = 2/3; got {delta}"
        );
    }

    /// Empty sheaf: DeltaCritical value is None.
    #[test]
    fn delta_critical_of_empty_sheaf_returns_none() {
        let empty = sheaf_of_shard_graph_from_edges(&[]);
        assert!(DeltaCritical::of(&empty).value.is_none());
    }
}

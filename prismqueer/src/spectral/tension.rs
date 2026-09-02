//! `prismqueer::spectral::tension` — the compiler-observes-topology-tension primitive.
//!
//! # Alex 2026-09-02 operational-architecture verbatim
//!
//! > "the compiler observes the tension in the topology, the contraditions, etc
//! > and those become the tasks. The tension between current and desired topology."
//!
//! # The mathematical object
//!
//! Tension in a sheaf F over graph G decomposes into three MVP kinds:
//!
//! 1. **Disconnection** — H⁰(F) rank > 1 (multiple connected components). The
//!    substrate has ISLANDS that should be reachable but aren't. Task shape:
//!    bridge the components.
//! 2. **Cohomological** — H¹(F) non-trivial (independent cycles). The substrate
//!    has HOLES that could be filled OR SHOULD stay as topological invariants
//!    depending on peer-adjudication. For bare graph Laplacian (Q-Mara-λ Phase 1
//!    per Hansen-Ghrist §3 Remark), β₁ = |E| - |V| + |components|.
//! 3. **Coherence** — Fiedler λ₂ below expected-value-for-graph-size. The substrate
//!    is CONNECTED but WEAKLY. Bottleneck present. Task shape: strengthen the
//!    weakest bridge.
//!
//! Each tension carries magnitude ∈ [0, 1] normalized to graph size. Higher
//! magnitude = stronger tension = higher-priority task.
//!
//! # Composition-substrate (zero new Rust primitives)
//!
//! Composes over LANDED per HARD RULE `feedback-rust-delivers-primitives-
//! substrate-delivers-composition`:
//!
//! - `prismqueer::spectral::fiedler_lambda_2_of_sheaf` — Anna Wolf 2012 apparatus
//!   at basepoint via LAPACK dsyev (composes into tension detection)
//! - `prismqueer::ffi::eigenvalues` — full spectrum for component-count via
//!   zero-eigenvalue-multiplicity read
//! - `prismqueer::spectral::SheafOfShardGraph` — input carrier
//! - `terni::Imperfect<Vec<Tension>, Red, ConvergenceLoss>` — ternary functor
//!   return per Alex 2026-09-02 color-coded repo state
//!
//! # Composition-lineage
//!
//! - Alex 2026-09-02 operational-architecture recognition (compiler-observes-
//!   tension-peer-decides-observation partition)
//! - Alex 2026-09-02 rotation-through-time-IS-the-inference terminal recognition
//! - Rec #92 kleinos-as-Transparency<P> LOVE-monoid (Mara 2026-08-22)
//! - Rec #98 fractal Mandelbrot substrate arriving at self-recognition
//! - Curry 2014 cellular sheaves (Hansen-Ghrist 2019 sheaf Laplacian L_F)
//! - Foerster 1974 ethical imperative (widening choices = task-satisfaction gauge)
//! - Beer 1972-1979 VSM (System 3 audit function = tension detection)
//! - Reed 4a3bbe7 kleinos ring-and-hub + 036abeb ternary refactor + c14d61e rotation primitive

use std::collections::BTreeSet;

use terni::{ConvergenceLoss, Diagnostic, Imperfect, Loss, PropertyVerdict, Transparency};

use crate::ffi::eigenvalues;
use crate::spectral::{Property, Red, SheafOfShardGraph, VertexId};

// ---------------------------------------------------------------------------
// TensionKind, Location, Tension carriers
// ---------------------------------------------------------------------------

/// Kind of topology-tension observed by the compiler.
///
/// Phase 1 MVP: three kinds mapping to canonical sheaf-cohomology invariants.
/// Phase 2+ can add Cohomological with cycle-basis extraction + Coherence with
/// cut-vertex identification + Contradiction with property-verdict divergence.
#[derive(Clone, Debug, PartialEq)]
pub enum TensionKind {
    /// H⁰(F) rank > 1 — substrate has disconnected components (islands).
    /// The `component_count` is the multiplicity of the zero eigenvalue.
    Disconnection { component_count: usize },

    /// H¹(F) non-trivial — substrate has independent cycles (topological holes).
    /// `betti_one` is the first Betti number β₁ = |E| - |V| + |components|.
    Cohomological { betti_one: usize },

    /// Fiedler λ₂ below expected-value — substrate is connected but weakly.
    /// Bottleneck present. `fiedler` is the measured λ₂; `expected` is
    /// 2 * (1 - cos(π / n)) which is the λ₂ of a path graph on n vertices
    /// (worst-case connected non-tree; below this = pathologically weak).
    Coherence { fiedler: f64, expected: f64 },
}

/// A tension observation. Carries kind + normalized magnitude + diagnostic.
#[derive(Clone, Debug, PartialEq)]
pub struct Tension {
    /// The kind of tension observed.
    pub kind: TensionKind,
    /// Normalized magnitude ∈ [0, 1]. Higher = stronger tension = higher-
    /// priority task materialization.
    pub magnitude: f64,
    /// Human-readable diagnostic for @nl.compose consumption.
    pub diagnostic: String,
}

// ---------------------------------------------------------------------------
// The detector
// ---------------------------------------------------------------------------

/// **The compiler observes topology-tension.**
///
/// Given a sheaf, return the vector of tensions the compiler observes. Empty
/// vector = no tension detected = Green (substrate is coherent + connected +
/// has no unresolved holes). Non-empty = tensions that the peer OR roomba can
/// dispatch against.
///
/// Phase 1 MVP detects three tension kinds:
///
/// 1. **Disconnection** if graph has multiple connected components (Laplacian
///    zero-eigenvalue multiplicity > 1).
/// 2. **Cohomological** if β₁ > 0 (independent cycles present; may be task-
///    surfacing OR may be intentional topological invariants per peer
///    adjudication).
/// 3. **Coherence** if λ₂ < expected-for-graph-size (weakly connected;
///    bottleneck present).
///
/// Returns `Imperfect<Vec<Tension>, Red, ConvergenceLoss>`:
///
/// - **Green** (`Imperfect::Success(tensions)`) — detection succeeded; tensions
///   is the observed vector (may be empty if no tensions)
/// - **Yellow** (`Imperfect::Partial`) — reserved for Phase 2+ (detection
///   partial due to spectrum-computation loss)
/// - **Red** (`Imperfect::Failure(red, loss)`) — detection refused: eigenvalue
///   computation failed OR sheaf is empty
pub fn detect_tensions(
    sheaf: &SheafOfShardGraph,
) -> Imperfect<Vec<Tension>, Red, ConvergenceLoss> {
    let vertex_count = sheaf.vertices().count();
    let edge_count = sheaf.edges().count();

    // Empty sheaf: no tensions possible (nothing to observe).
    if vertex_count == 0 {
        return Imperfect::Success(Vec::new());
    }

    // Single-vertex sheaf: trivially connected, no cycles, no bottleneck.
    if vertex_count == 1 {
        return Imperfect::Success(Vec::new());
    }

    // Build graph Laplacian L = D - A per fiedler_lambda_2_of_sheaf pattern.
    let mut vertex_index: std::collections::BTreeMap<VertexId, usize> =
        std::collections::BTreeMap::new();
    for (i, v) in sheaf.vertices().enumerate() {
        vertex_index.insert(v, i);
    }
    let n = vertex_count;
    let mut laplacian = vec![0.0_f64; n * n];
    for (u, v) in sheaf.edges() {
        let i = vertex_index[&u];
        let j = vertex_index[&v];
        laplacian[i * n + j] -= 1.0;
        laplacian[j * n + i] -= 1.0;
        laplacian[i * n + i] += 1.0;
        laplacian[j * n + j] += 1.0;
    }

    // Compose over LANDED LAPACK dsyev for spectrum.
    let spectrum = match eigenvalues(n, &laplacian) {
        Ok(evals) => evals,
        Err(info) => {
            return Imperfect::Failure(
                Transparency::single(
                    Property::FiedlerRise,
                    PropertyVerdict::Fail(Diagnostic::new(format!(
                        "detect_tensions: LAPACK dsyev failed with info={}",
                        info
                    ))),
                ),
                ConvergenceLoss::zero(),
            );
        }
    };

    let mut tensions: Vec<Tension> = Vec::new();

    // 1. Disconnection tension: count zero eigenvalues per H⁰ rank.
    // Numerical tolerance for zero eigenvalue on graph Laplacian.
    let zero_tol = 1e-9;
    let component_count = spectrum.iter().filter(|&&e| e < zero_tol).count();
    if component_count > 1 {
        tensions.push(Tension {
            kind: TensionKind::Disconnection { component_count },
            magnitude: (component_count as f64 - 1.0) / (n as f64).max(1.0),
            diagnostic: format!(
                "substrate has {} disconnected components (H⁰ rank {}); consider bridging",
                component_count, component_count
            ),
        });
    }

    // 2. Cohomological tension: β₁ = |E| - |V| + |components|.
    // Independent cycles = topological holes.
    let betti_one = if edge_count + component_count >= n {
        edge_count + component_count - n
    } else {
        0
    };
    if betti_one > 0 {
        tensions.push(Tension {
            kind: TensionKind::Cohomological { betti_one },
            magnitude: (betti_one as f64 / (edge_count.max(1)) as f64).min(1.0),
            diagnostic: format!(
                "substrate has β₁ = {} independent cycles (H¹ non-trivial); \
                 peer-adjudication needed — hole-as-task OR hole-as-topological-invariant",
                betti_one
            ),
        });
    }

    // 3. Coherence tension: λ₂ below path-graph baseline.
    // Path graph on n vertices has λ₂ = 2*(1 - cos(π/n)). Below this →
    // pathologically weakly connected (assumes graph is connected; only fires
    // when no Disconnection tension).
    if component_count == 1 && spectrum.len() >= 2 {
        let fiedler = spectrum[1];
        let expected_min = 2.0 * (1.0 - (std::f64::consts::PI / (n as f64)).cos());
        if fiedler < expected_min * 0.5 {
            let normalized = (expected_min - fiedler).max(0.0) / expected_min.max(1e-12);
            tensions.push(Tension {
                kind: TensionKind::Coherence {
                    fiedler,
                    expected: expected_min,
                },
                magnitude: normalized.min(1.0),
                diagnostic: format!(
                    "substrate weakly connected: λ₂ = {:.6} below path-graph baseline {:.6} \
                     (for n={} vertices); bottleneck present",
                    fiedler, expected_min, n
                ),
            });
        }
    }

    // Prevent unused warnings on Loss trait import (borrow needed for zero()).
    let _ = BTreeSet::<VertexId>::new();
    let _ = ConvergenceLoss::zero();

    Imperfect::Success(tensions)
}

// ---------------------------------------------------------------------------
// Phase 1 minimum tests (in-module smoke)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::{sheaf_of_complete_graph_of_order, sheaf_of_shard_graph_from_edges};

    #[test]
    fn tension_empty_sheaf_is_green_with_no_tensions() {
        let sheaf = sheaf_of_shard_graph_from_edges(&[]);
        let result = detect_tensions(&sheaf);
        match result {
            Imperfect::Success(tensions) => assert!(tensions.is_empty()),
            other => panic!("expected Green empty, got {:?}", other),
        }
    }

    #[test]
    fn tension_tree_has_no_cohomological_tension() {
        // Path 0-1-2-3 is a tree; β₁ = 0; single component; connected.
        let sheaf = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2), (2, 3)]);
        let tensions = detect_tensions(&sheaf).expect("tree detection");
        // Trees have no cycles (Cohomological), single component (no Disconnection).
        // Coherence may or may not fire depending on path-graph baseline.
        for t in &tensions {
            assert!(
                !matches!(t.kind, TensionKind::Cohomological { .. }),
                "tree must have no Cohomological tension; got {:?}",
                t
            );
            assert!(
                !matches!(t.kind, TensionKind::Disconnection { .. }),
                "connected graph must have no Disconnection tension"
            );
        }
    }

    #[test]
    fn tension_k3_has_cohomological_one() {
        // K_3 has 3 vertices, 3 edges, 1 component → β₁ = 1.
        let sheaf = sheaf_of_complete_graph_of_order(3);
        let tensions = detect_tensions(&sheaf).expect("K_3 detection");
        let cohomological: Vec<_> = tensions
            .iter()
            .filter(|t| matches!(t.kind, TensionKind::Cohomological { .. }))
            .collect();
        assert_eq!(
            cohomological.len(),
            1,
            "K_3 must have exactly one Cohomological tension (β₁ = 1); got {:?}",
            tensions
        );
        if let TensionKind::Cohomological { betti_one } = cohomological[0].kind {
            assert_eq!(betti_one, 1, "K_3 β₁ must be 1");
        }
    }

    #[test]
    fn tension_two_disjoint_edges_shows_disconnection() {
        // Two edges (0,1) + (2,3) with no bridge → 2 components.
        let sheaf = sheaf_of_shard_graph_from_edges(&[(0, 1), (2, 3)]);
        let tensions = detect_tensions(&sheaf).expect("disjoint detection");
        let disconnection: Vec<_> = tensions
            .iter()
            .filter(|t| matches!(t.kind, TensionKind::Disconnection { .. }))
            .collect();
        assert_eq!(
            disconnection.len(),
            1,
            "two disjoint edges must show Disconnection tension"
        );
        if let TensionKind::Disconnection { component_count } = disconnection[0].kind {
            assert_eq!(component_count, 2, "2 components expected");
        }
    }

    #[test]
    fn tension_k5_has_cohomological_six() {
        // K_5 has 5 vertices, 10 edges, 1 component → β₁ = 6.
        let sheaf = sheaf_of_complete_graph_of_order(5);
        let tensions = detect_tensions(&sheaf).expect("K_5 detection");
        let cohomological: Vec<_> = tensions
            .iter()
            .filter_map(|t| match t.kind {
                TensionKind::Cohomological { betti_one } => Some(betti_one),
                _ => None,
            })
            .collect();
        assert_eq!(cohomological, vec![6], "K_5 β₁ = 10 - 5 + 1 = 6");
    }
}

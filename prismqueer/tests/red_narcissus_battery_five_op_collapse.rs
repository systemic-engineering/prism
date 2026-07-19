//! RED — Narcissus Detection Battery × Void 5-op basis collapse verification.
//!
//! Per Alex 2026-07-19 direct-transcript:
//!
//! > "The 8 tests condense to the 5 dimensions of the void, 3 of the 8
//! > are mathematically non-orthogonal and collapse into one of the 5
//! > dimensions."
//!
//! ## Composition anchors
//!
//! - Project Singularity (2026-04-26 Reed + Alex) — the 8-test Battery:
//!   `~/.reed/tasks/pending/singularity.md` §Narcissus Detection Battery.
//! - Singularity is Self-Knowledge (2026-04-19 Reed + Alex) — the
//!   geometric framing K_{1,n-1} vs K_n as poles of quantum-information
//!   manifold: `practice/insights/ai/singularity-as-self-knowledge.md`.
//! - Recognition #79 (`docs/math/the-tower/recognition-79-gauge-is-
//!   void-duality-basis.md`) — Void's 5-op basis PROMOTED via `974a3f6`
//!   this arc: focus / project / split / shift / settle.
//! - Mara §§1-5 of `docs/math/the-tower/recognition-the-frame-is-a-
//!   narcissistic-eigenbehavior.md` (`5ddb076`, this session) —
//!   star-graph Laplacian spectrum {0, 1, ..., 1, n}; Fiedler cut;
//!   recognition-bomb mass placement at spectral goldilocks zone.
//! - Reed 8-property RED file `red_trust_chain_liquid_void.rs`
//!   (`560ea67`) — sibling RED file at prismqueer test altitude.
//!
//! ## The claim
//!
//! **Void's 5-op basis (Recognition #79) manifests in the Narcissus
//! Detection Battery.** The 8 tests factor onto 5 axes as follows:
//!
//! | # | Test                           | Void axis | Test count on axis |
//! |---|--------------------------------|-----------|--------------------|
//! | 1 | Betweenness centralization     | focus     | 3 (collapse)       |
//! | 2 | Degree Gini                    | focus     | 3 (collapse)       |
//! | 7 | Single-node fragility          | focus     | 3 (collapse)       |
//! | 3 | Spectral ratio λ_max/λ_2       | project   | 2 (distinct moments)|
//! | 4 | Von Neumann entropy            | project   | 2 (distinct moments)|
//! | 6 | Peripheral conductance         | split     | 1                  |
//! | 8 | Permeability index             | shift     | 1                  |
//! | 5 | Clustering coefficient         | settle    | 1                  |
//!
//! **The 3-way collapse is at focus**: tests {1, 2, 7} all measure
//! "how concentrated is graph mass at the center" through different
//! lenses (paths / edges / removal-impact). Same shadow, three lights.
//!
//! Under Ricci flow parameterization K_{1,n-1} → K_n adding uniform
//! peripheral edges, tests {1, 2, 7} share a common decay rate. They
//! project onto the same eigenvector of the correlation matrix.
//!
//! ## Empirical verification target
//!
//! rank(test-matrix under Ricci flow parameterization) = 5, with the
//! null space spanned by two linear dependencies among {1, 2, 7}
//! (dimension-2 null space; 3 tests − 1 axis = 2 dependencies).
//!
//! ## Ouroboros closure (what this makes deployable)
//!
//! Once verified: **every mirror invocation implicitly runs the
//! Narcissus Detection Battery** through its Void-basis measurements.
//! The compiler doesn't need a separate "narcissus detection" pass;
//! it detects narcissus-topology by measuring itself against its own
//! Void basis. Any structure the compiler examines gets automatically
//! scored on the 5 axes. Three-or-more-axis-exceeded threshold
//! (Project Singularity §Battery) becomes intrinsic.
//!
//! The compiler gains: detect paradigm-scale narcissism it encounters,
//! then AUTHOR bombs (@cyberpunk/bugz) at the Fiedler-eigenvector cut
//! to force Ricci flow K_{1,n-1} → K_n phase transition.
//!
//! Counter-singularity architecture deployable at machine speed.
//!
//! ## RED discipline
//!
//! All properties currently return `PropertyVerdict::Fail` via `defer()`
//! naming the authorship boundary whose landing unlocks GREEN. Outer
//! test `assert!(matches!(v, Pass))` fails — that's RED. The empirical
//! discharge lands when:
//!   (a) real graph-Laplacian primitives compose at prismqueer::graph
//!       (currently forward-promised);
//!   (b) Ricci-flow parameterization surface lands (Mara territory);
//!   (c) SVD/eigendecomposition of test-matrix computable via
//!       prismqueer::ffi::eigenvalues (BLOCKED per matrix.rs §GREEN blocker).

#![cfg(feature = "bundle")]

use prismqueer::liquid::pillar::{forall, Arbitrary, Sample};
use terni::{Diagnostic, PropertyVerdict};

// =====================================================================
// Forward-promised carriers (test-altitude stubs).
//
// Real types under `prismqueer::graph::{ConnectedGraph, Laplacian,
// RicciFlow, NarcissusBattery, VoidBasisProjection}` will land when
// Mara @graph family-root + Ricci-flow parameterization compose with
// @io/matrix numerical primitives (post prismqueer lapack build fix).
// =====================================================================

#[derive(Clone, Debug, PartialEq)]
struct ConnectedGraph {
    /// Number of vertices n. For test purposes we parameterize the graph
    /// as one point on the Ricci flow K_{1,n-1} → K_n.
    n: usize,
    /// Ricci flow parameter τ ∈ [0, 1]. τ=0 corresponds to K_{1,n-1};
    /// τ=1 to K_n. Intermediate values interpolate via uniform
    /// peripheral-edge addition per Project Singularity §Intervention.
    tau: f64,
}

#[derive(Clone, Debug, PartialEq)]
enum VoidBasisAxis {
    Focus,   // attention-concentration; where mass points
    Project, // spectral-decomposition; eigenvalue arrangement
    Split,   // partition/connectivity; cut detectability
    Shift,   // perturbation-response; sensitivity to change
    Settle,  // steady-state; equilibrium distribution
}

#[derive(Clone, Debug, PartialEq)]
enum BatteryTest {
    BetweennessCentralization, // #1 → focus
    DegreeGini,                // #2 → focus
    SpectralRatio,             // #3 → project
    VonNeumannEntropy,         // #4 → project
    ClusteringCoefficient,     // #5 → settle
    PeripheralConductance,     // #6 → split
    SingleNodeFragility,       // #7 → focus
    PermeabilityIndex,         // #8 → shift
}

// =====================================================================
// Arbitrary implementations — sample graphs across the Ricci flow.
// =====================================================================

impl Arbitrary for ConnectedGraph {
    fn arbitrary(sample: &mut Sample) -> Self {
        let n = sample.draw_integer(4, 32) as usize;
        let tau_int = sample.draw_integer(0, 1000);
        let tau = (tau_int as f64) / 1000.0;
        ConnectedGraph { n, tau }
    }
}

impl Arbitrary for BatteryTest {
    fn arbitrary(sample: &mut Sample) -> Self {
        match sample.draw_integer(0, 7) {
            0 => BatteryTest::BetweennessCentralization,
            1 => BatteryTest::DegreeGini,
            2 => BatteryTest::SpectralRatio,
            3 => BatteryTest::VonNeumannEntropy,
            4 => BatteryTest::ClusteringCoefficient,
            5 => BatteryTest::PeripheralConductance,
            6 => BatteryTest::SingleNodeFragility,
            _ => BatteryTest::PermeabilityIndex,
        }
    }
}

// =====================================================================
// defer() — substrate-honest RED verdict naming the authorship
// boundary whose landing unlocks GREEN.
// =====================================================================

fn defer(property_name: &str, authorship_boundary: &str) -> PropertyVerdict {
    PropertyVerdict::Fail(Diagnostic::new(format!(
        "RED @narcissus-battery × void-basis-collapse witness: `{}` first-\
         witness discharge lands when {} composes at runtime. Per Alex \
         2026-07-19 \"the 8 tests condense to the 5 dimensions of the void, \
         3 of the 8 are mathematically non-orthogonal and collapse into one \
         of the 5 dimensions.\" Verification target: rank(test-matrix under \
         Ricci flow parameterization) = 5 with 2-dim null space among tests \
         {{Betweenness, Gini, Fragility}} all measuring focus axis.",
        property_name, authorship_boundary
    )))
}

// =====================================================================
// RED property witnesses (9 total, including the load-bearing rank-5
// verification and the 3-way focus-collapse witness).
// =====================================================================

// ---------------------------------------------------------------------
// Property 1: rank_of_battery_matrix_is_five
// LOAD-BEARING. The empirical verification of Alex's claim.
// ---------------------------------------------------------------------
#[test]
fn red_pillar_rank_of_battery_matrix_is_five() {
    // Under Ricci flow parameterization K_{1,n-1} → K_n, the 8×N matrix
    // M[i, k] = test_i(G_k) where G_k is the k-th graph on the flow has
    // rank exactly 5. SVD verifies: σ_6, σ_7, σ_8 ≈ 0 relative to σ_1..σ_5.
    // Two-dimensional null space spans the 3-way focus-axis collapse.
    let v = forall::<ConnectedGraph, _>(30, |_g| {
        defer(
            "rank_of_battery_matrix_is_five",
            "prismqueer::graph Laplacian primitives + Ricci-flow \
             parameterization surface + SVD via matrix.rs LAPACK dsyev \
             (BLOCKED per matrix.rs §GREEN blocker at prismqueer::ffi::\
             eigenvalues wrapper build)",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (1/9): {v:?}");
}

// ---------------------------------------------------------------------
// Property 2: three_way_collapse_of_focus_axis
// The specific 3-way linear dependency Alex named.
// ---------------------------------------------------------------------
#[test]
fn red_pillar_three_way_collapse_of_focus_axis() {
    // Tests {Betweenness centralization, Degree Gini, Single-node
    // fragility} are mutually linearly dependent under Ricci flow.
    // Verification: correlation matrix restricted to these three has
    // rank 1 within numerical tolerance. All three project onto the
    // same eigenvector of the full battery correlation matrix.
    //
    // Same shadow, three lights: paths (betweenness), edges (Gini),
    // removal-impact (fragility) — all measure center-concentration.
    let v = forall::<ConnectedGraph, _>(30, |_g| {
        defer(
            "three_way_collapse_of_focus_axis",
            "Ricci-flow correlation computation + rank-1 witness for the \
             {Betweenness, Gini, Fragility} sub-battery",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (2/9): {v:?}");
}

// ---------------------------------------------------------------------
// Property 3: orthogonality_of_five_op_basis_projections
// ---------------------------------------------------------------------
#[test]
fn red_pillar_orthogonality_of_five_op_basis_projections() {
    // The 5 Void-basis axes (focus / project / split / shift / settle)
    // are mutually orthogonal when computed on graph-Laplacian carriers.
    // Verification: dot product of any two distinct axis-projections
    // over the Ricci-flow-parameterized test family is zero within
    // numerical tolerance.
    let v = forall::<ConnectedGraph, _>(30, |_g| {
        defer(
            "orthogonality_of_five_op_basis_projections",
            "Void 5-op basis operators computed on graph-Laplacian \
             carriers via Recognition #79 axis-map + inner-product verification",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (3/9): {v:?}");
}

// ---------------------------------------------------------------------
// Property 4a-e: each Void axis has the correct tests projecting onto it
// ---------------------------------------------------------------------
#[test]
fn red_pillar_focus_axis_carries_tests_1_2_7() {
    // focus ← {Betweenness centralization (#1), Degree Gini (#2),
    // Single-node fragility (#7)}. Three tests. All measure "how
    // concentrated is graph mass at center."
    let v = forall::<BatteryTest, _>(30, |_test| {
        defer(
            "focus_axis_carries_tests_1_2_7",
            "axis-projection map computable via @cyberpunk/bugz shard-\
             decl (`shards/epistemologic/cybernetic/bugz.mirror` \
             carriers: `star_graph_fiedler_target`, `narcissus_eigenmode`)",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (4a/9): {v:?}");
}

#[test]
fn red_pillar_project_axis_carries_tests_3_4() {
    // project ← {Spectral ratio λ_max/λ_2 (#3), Von Neumann entropy (#4)}.
    // Two tests on distinct moments of the spectral distribution.
    // NOT collinear — max/min ratio vs entropy measure different moments.
    let v = forall::<BatteryTest, _>(30, |_test| {
        defer(
            "project_axis_carries_tests_3_4",
            "spectral-moment computation via matrix.rs eigenvalue \
             decomposition (BLOCKED per M0.5 blocker)",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (4b/9): {v:?}");
}

#[test]
fn red_pillar_split_axis_carries_test_6() {
    // split ← {Peripheral conductance (#6)}. Measures whether the graph
    // can be partitioned along non-hub cuts.
    let v = forall::<BatteryTest, _>(30, |_test| {
        defer(
            "split_axis_carries_test_6",
            "peripheral-conductance computation via graph-cut analysis",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (4c/9): {v:?}");
}

#[test]
fn red_pillar_shift_axis_carries_test_8() {
    // shift ← {Permeability index (#8)}. Measures openness to external
    // perturbation.
    let v = forall::<BatteryTest, _>(30, |_test| {
        defer(
            "shift_axis_carries_test_8",
            "permeability-index computation via boundary-flow analysis",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (4d/9): {v:?}");
}

#[test]
fn red_pillar_settle_axis_carries_test_5() {
    // settle ← {Clustering coefficient (#5)}. Measures local steady-state
    // triangle density.
    let v = forall::<BatteryTest, _>(30, |_test| {
        defer(
            "settle_axis_carries_test_5",
            "clustering-coefficient computation via triangle enumeration",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (4e/9): {v:?}");
}

// ---------------------------------------------------------------------
// Property 5: narcissus_detection_battery_is_void_basis_measurement
// The ouroboros closure. What the recognition CLAIMS at math altitude.
// ---------------------------------------------------------------------
#[test]
fn red_pillar_narcissus_detection_battery_is_void_basis_measurement() {
    // The load-bearing Recognition-witness property. If verified:
    // every mirror invocation implicitly runs the Narcissus Detection
    // Battery through its Void-basis measurements. No separate
    // narcissus-detection pass needed. Detection intrinsic.
    //
    // Chains with #R-mirror-is-the-counter-singularity: mirror can
    // now DETECT paradigm-scale narcissism it encounters and AUTHOR
    // recognition bombs to force phase transition, all in one substrate.
    let v = forall::<ConnectedGraph, _>(30, |_g| {
        defer(
            "narcissus_detection_battery_is_void_basis_measurement",
            "complete rank-5 factorization + axis-assignment verification \
             + ouroboros-closure claim first-witness (this property IS the \
             recognition; landing it empirically closes the second-witness \
             gate for #R-mirror-is-the-counter-singularity)",
        )
    });
    assert!(matches!(v, PropertyVerdict::Pass), "RED (5/9): {v:?}");
}

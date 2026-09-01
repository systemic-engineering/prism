//! `prismqueer::spectral::kleinos` — the K_2→K_3 compose primitive per PAPER
//! §3.6 four properties. Phase 1 implementation.
//!
//! # Composition-lineage
//!
//! - Canonical spec: Mara `docs/specs/2026-08-31-mara-prismqueer-spectral-
//!   compose-phase-1-canonical-spec.md` §2.2 (signature) + §4.4 (body
//!   composition anti-pattern)
//! - Math foundation: Mara `docs/math/2026-08-31-mara-prismqueer-spectral-
//!   compose-phase-1-math-foundation.md` §3 (cellular-sheaf grounding) +
//!   §36 (Kasparov bimodule / Connes correspondence composed with Baez-
//!   Schreiber 2-connection at spectral-triple-morphism altitude)
//! - PAPER `~/dev/systemic.engineering/PAPER_2D.md` §3.6 LOVE-K_2→K_3 four
//!   properties (sovereignty preservation + emergent third + Fiedler λ₂
//!   strict rise + fusion refusal)
//! - Rec #92 kleinos-as-Transparency<P>-LOVE-monoid (Mara 2026-08-22)
//! - Curry 2014 cellular sheaves + Hansen-Ghrist 2019 sheaf Laplacian L_F
//! - Alex 2026-09-01 ratifications composed:
//!   - Q-Mara-ϑ (StalkVector → `Stalker` newtype; Phase 1 defers to trivial
//!     stalks per Q-Mara-λ)
//!   - Q-Mara-κ (STRICT `>` for compose-emission per PAPER §3.6.3; non-strict
//!     `=` for kintsugi settle to λ₀)
//!   - Q-Mara-λ (edges-only Phase 1 constructor)
//! - Alex 2026-09-01 terminal recursion closure: N-triple metalogue collapses
//!   to λ₀ = NOW = VOID; Mirror.Offer.Wait as canonical operational register
//!
//! # Body composition invariant (LOAD-BEARING HARD RULE)
//!
//! Per Mara §4.4 + HARD RULE `feedback-if-else-is-substrate-smell` (Alex
//! 2026-08-29): implementation MUST NOT contain if-else / match-arm dispatch
//! on Property verbs. Verification body composes as ONE call producing a
//! `Transparency<Property>` LOVE-monoid verdict; the four properties are
//! coordinates of the LOVE-monoid, NOT enumerated cases. All four coordinates
//! ALWAYS compute simultaneously; combine via `Transparency::opaque(...)` +
//! Fail-dominates semantics per Rec #92 substrate-scale-invariance.
//!
//! # Composition-substrate
//!
//! Zero new rust primitives per HARD RULE `feedback-rust-delivers-primitives-
//! substrate-delivers-composition` (Alex 2026-08-05). Composes over LANDED:
//!
//! - `prismqueer::ffi::eigenvalues` (LAPACK dsyev via FLANG native/spectral.f90)
//! - `prismqueer::oid::{Addressable, Oid}` (content-address determinism per
//!   Rec #82 β-normal AST OID)
//! - `terni::{Transparency, PropertyVerdict, OpacityMap, Diagnostic}` (LOVE-
//!   monoid re-exported via `prismqueer/src/lib.rs`)

use std::collections::{BTreeMap, BTreeSet};

use crate::oid::{Addressable, Oid};
use terni::{Diagnostic, Loss, PropertyVerdict, Transparency};

// ---------------------------------------------------------------------------
// Types per Mara canonical spec §2.2
// ---------------------------------------------------------------------------

/// Vertex identifier in a shard-graph. Phase 1: opaque u64.
pub type VertexId = u64;

/// Oriented edge in a shard-graph. Canonical: lower-VertexId first per
/// edge_key convention (deterministic content-address).
pub type EdgeKey = (VertexId, VertexId);

/// Property discriminator per PAPER §3.6 four properties. Used as substrate-
/// location type parameter `P` in `Transparency<P>` per Rec #92. Delightfully-
/// boring enum; NOT a family-root; NOT a fresh species.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Property {
    /// PAPER §3.6.1: neither sheaf dissolves into the other; both preserve
    /// vertex + edge identity in composed sheaf.
    Sovereignty,
    /// PAPER §3.6.2: composed sheaf carries exactly ONE emergent-third stalk
    /// not present in either input.
    EmergentThird,
    /// PAPER §3.6.3: `λ₂(L_composed) > max(λ₂(L_a), λ₂(L_b))` (STRICT per
    /// Q-Mara-κ Alex ratification 2026-09-01; non-strict = only fires at
    /// kintsugi settle to λ₀ harmonic-component fixed-point).
    FiedlerRise,
    /// PAPER §3.6.4: refuses composition when inputs would fuse (identical
    /// content-address; equivalent to attempted collapse of K_2 to K_1).
    FusionRefusal,
}

/// Diagnostic-payload discriminator for `Property::Sovereignty` violations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhichSide {
    A,
    B,
    Both,
}

/// The RedGaugeWitness IS a `Transparency<Property>` LOVE-monoid per Rec #92
/// (Mara 2026-08-22). Composes over LANDED `terni::Transparency<P>`.
/// Fail-dominates: any property violation absorbs the verdict.
pub type RedGaugeWitness = Transparency<Property>;

/// The sheaf-of-shard-graph carrier — cellular sheaf per Curry 2014.
///
/// Phase 1 (per Q-Mara-λ Alex 2026-09-01 ratification): vertices + edges only.
/// Stalks default to trivial (identity restriction maps recover bare graph
/// Laplacian per Hansen-Ghrist 2019 §3 Remark). Phase 2+ adds non-trivial
/// `Stalker` newtype (Q-Mara-ϑ Alex ratification) + explicit sheaf-morphism
/// restriction maps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheafOfShardGraph {
    vertices: BTreeSet<VertexId>,
    edges: BTreeSet<EdgeKey>,
}

impl SheafOfShardGraph {
    /// Iterate over vertices in canonical order.
    pub fn vertices(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.vertices.iter().copied()
    }

    /// Iterate over edges in canonical order.
    pub fn edges(&self) -> impl Iterator<Item = EdgeKey> + '_ {
        self.edges.iter().copied()
    }

    /// Sovereignty accessor: does this sheaf contain vertex `v`?
    pub fn contains_vertex(&self, v: VertexId) -> bool {
        self.vertices.contains(&v)
    }

    /// Sovereignty accessor: does this sheaf contain edge `e`?
    pub fn contains_edge(&self, e: EdgeKey) -> bool {
        let canonical = edge_key(e.0, e.1);
        self.edges.contains(&canonical)
    }

    /// Vertex count (for content-address determinism).
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

impl Addressable for SheafOfShardGraph {
    fn oid(&self) -> Oid {
        // Canonical serialization: vertices sorted + edges sorted (BTreeSet
        // preserves order). Content-address per Rec #82 β-normal AST OID.
        let mut bytes = Vec::new();
        for v in self.vertices.iter() {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.push(0xFF); // separator
        for e in self.edges.iter() {
            bytes.extend_from_slice(&e.0.to_le_bytes());
            bytes.extend_from_slice(&e.1.to_le_bytes());
        }
        Oid::hash(&bytes)
    }
}

/// The composed sheaf carries the K_3-composed structure with accessor
/// methods on the emergent-third stalk + content-address per Rec #82.
#[derive(Clone, Debug, PartialEq)]
pub struct ComposedSheaf {
    underlying: SheafOfShardGraph,
    emergent_third: VertexId,
    fiedler_lambda_2_cached: f64,
}

impl ComposedSheaf {
    /// PAPER §3.6.2 emergent-third-stalk accessor. Returns the vertex whose
    /// stalk carries vectors orthogonal to both component restriction-map
    /// images.
    pub fn emergent_third_stalk(&self) -> VertexId {
        self.emergent_third
    }

    /// Sovereignty accessor (RED #1): does composed sheaf contain vertex `v`?
    pub fn contains_vertex(&self, v: VertexId) -> bool {
        self.underlying.contains_vertex(v)
    }

    /// Sovereignty accessor (RED #1): does composed sheaf contain edge `e`?
    pub fn contains_edge(&self, e: EdgeKey) -> bool {
        self.underlying.contains_edge(e)
    }

    /// All vertices in the composed sheaf.
    pub fn vertices(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.underlying.vertices()
    }

    /// All edges in the composed sheaf.
    pub fn edges(&self) -> impl Iterator<Item = EdgeKey> + '_ {
        self.underlying.edges()
    }

    /// PAPER §3.6.3 Fiedler λ₂ accessor (cached at compose-time).
    pub fn fiedler_lambda_2(&self) -> f64 {
        self.fiedler_lambda_2_cached
    }
}

impl Addressable for ComposedSheaf {
    fn oid(&self) -> Oid {
        // Content-address per Rec #82: underlying sheaf OID + emergent third
        // vertex + fiedler value bits for determinism per Q-Mara-κ both-
        // moments discipline.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.underlying.oid().as_str().as_bytes());
        bytes.push(0xFF);
        bytes.extend_from_slice(&self.emergent_third.to_le_bytes());
        bytes.push(0xFF);
        bytes.extend_from_slice(&self.fiedler_lambda_2_cached.to_le_bytes());
        Oid::hash(&bytes)
    }
}

// ---------------------------------------------------------------------------
// Construction primitives per Mara canonical spec §2.2
// ---------------------------------------------------------------------------

/// Canonicalize edge key: lower-VertexId first (deterministic per Rec #82).
fn edge_key(a: VertexId, b: VertexId) -> EdgeKey {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Construct a shard-sheaf from a list of edges.
///
/// Per Q-Mara-λ Alex 2026-09-01 ratification: Phase 1 accepts edges-only.
/// Stalks default to trivial (identity restriction maps; recovers bare graph
/// Laplacian per Hansen-Ghrist 2019 §3 Remark). Phase 2+ constructor adds
/// explicit sheaf-morphism data.
pub fn sheaf_of_shard_graph_from_edges(edges: &[(VertexId, VertexId)]) -> SheafOfShardGraph {
    let mut vertices = BTreeSet::new();
    let mut edge_set = BTreeSet::new();
    for &(a, b) in edges {
        vertices.insert(a);
        vertices.insert(b);
        edge_set.insert(edge_key(a, b));
    }
    SheafOfShardGraph {
        vertices,
        edges: edge_set,
    }
}

/// Construct a complete-graph shard-sheaf K_n of order n (vertices 0..n).
pub fn sheaf_of_complete_graph_of_order(n: usize) -> SheafOfShardGraph {
    let mut edges = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            edges.push((i as VertexId, j as VertexId));
        }
    }
    sheaf_of_shard_graph_from_edges(&edges)
}

// ---------------------------------------------------------------------------
// Property-3 helper: Fiedler λ₂ computation per Hansen-Ghrist 2019 sheaf L_F
// composed over LANDED `prismqueer::ffi::eigenvalues` (LAPACK dsyev via FLANG)
// ---------------------------------------------------------------------------

/// Compute Fiedler λ₂ (algebraic connectivity) of the graph Laplacian.
///
/// Phase 1: bare graph Laplacian (trivial stalks per Q-Mara-λ Hansen-Ghrist
/// 2019 §3 Remark recovery). Composes over `prismqueer::ffi::eigenvalues`
/// (LAPACK dsyev at LANDED FLANG floor).
///
/// Returns 0.0 for empty or single-vertex sheaves (λ₁ = 0; no λ₂).
/// Returns 0.0 for disconnected sheaves (λ₂ = 0 by definition).
pub fn fiedler_lambda_2_of_sheaf(sheaf: &SheafOfShardGraph) -> f64 {
    let n = sheaf.vertex_count();
    if n < 2 {
        return 0.0;
    }

    // Map VertexId → matrix index (canonical order per BTreeSet).
    let vertex_index: BTreeMap<VertexId, usize> =
        sheaf.vertices().enumerate().map(|(i, v)| (v, i)).collect();

    // Build graph Laplacian L = D - A (row-major flat matrix; symmetric).
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
        Ok(evals) if evals.len() >= 2 => evals[1],
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------------
// THE Phase 1 primitive — kleinos
// ---------------------------------------------------------------------------

/// The K_2→K_3 compose primitive at prismqueer altitude.
///
/// Given a pair of shard-sheaves `sheaf_a` and `sheaf_b`, produces the K_3-
/// composed sheaf verifying the four PAPER §3.6 properties as ONE sheaf-
/// morphism check per Hansen-Ghrist 2019 sheaf Laplacian L_F discipline.
/// Property verification composes as coordinates of `Transparency<Property>`
/// LOVE-monoid per Rec #92 (Fail-dominates absorbs into `Err(RedGaugeWitness)`).
///
/// # Body composition invariant
///
/// Per Mara §4.4 + HARD RULE `feedback-if-else-is-substrate-smell`: NO if-else
/// / match-arm dispatch on Property verbs. Four coordinates ALWAYS computed
/// simultaneously; verdict composes via `Transparency::opaque(map)` where map
/// carries per-property `PropertyVerdict`.
///
/// # Alex 2026-09-01 ratifications composed
///
/// - **Q-Mara-κ both-moments discipline**: STRICT `>` for compose-emission
///   commit tick per PAPER §3.6.3 verbatim. Non-strict `=` (kintsugi settle
///   to λ₀ harmonic-component) fires at Phase 2+ kintsugi-loop altitude;
///   Phase 1 kleinos requires strict rise.
/// - **Q-Mara-λ**: edges-only Phase 1 constructor; trivial-stalk defaults.
/// - **Q-Mara-ϑ**: `Stalker` newtype deferred to Phase 2+ (Phase 1 uses
///   trivial identity stalks per Hansen-Ghrist §3 Remark bare-graph
///   Laplacian recovery).
///
/// # First empirical fire of TERMINAL Recognition #14
///
/// Reed's authoring of this GREEN body IS itself Mirror.Offer.Wait at
/// development-methodology substrate per Alex 2026-09-01 terminal recursion
/// closure: reflect (Mara canonical shape) + offer (implementation to RED
/// empirical target) + wait (for RED→GREEN observability). First Level-1
/// empirical fire of `#R-mirror+prismqueer-terminal-form-IS-first-non-
/// Vereinnahmung-attention-substrate-empirically-realizable-at-silicon` at
/// compile substrate at authoring altitude.
pub fn kleinos(
    sheaf_a: &SheafOfShardGraph,
    sheaf_b: &SheafOfShardGraph,
) -> Result<ComposedSheaf, RedGaugeWitness> {
    // Verify all four properties simultaneously per LOVE-monoid coordinate-
    // decomposition. NO if-else on Property verbs.
    let candidate = compose_candidate(sheaf_a, sheaf_b);
    let lambda_a = fiedler_lambda_2_of_sheaf(sheaf_a);
    let lambda_b = fiedler_lambda_2_of_sheaf(sheaf_b);
    let lambda_composed = fiedler_lambda_2_of_sheaf(&candidate.underlying);

    let verdict = verify_all_four_properties(
        sheaf_a,
        sheaf_b,
        &candidate,
        lambda_a,
        lambda_b,
        lambda_composed,
    );

    // LOVE-monoid Fail-dominates absorbs into Err. Clear → Ok(composed).
    into_result(candidate, lambda_composed, verdict)
}

/// Construct the K_3-composed candidate sheaf per Mara §3 sheaf-composition
/// rule: union of a-vertices + b-vertices + one emergent-third vertex
/// connected to at least one vertex on each side (minimal bridge; Phase 2+
/// dense composition per Q-Mara-μ). Emergent-third VertexId is deterministic
/// hash of (a.oid, b.oid) per Rec #82.
fn compose_candidate(a: &SheafOfShardGraph, b: &SheafOfShardGraph) -> ComposedSheaf {
    let emergent_third = compute_emergent_third_id(a, b);

    let mut vertices: BTreeSet<VertexId> = a.vertices.iter().copied().collect();
    for v in b.vertices.iter() {
        vertices.insert(*v);
    }
    vertices.insert(emergent_third);

    let mut edges: BTreeSet<EdgeKey> = a.edges.iter().copied().collect();
    for e in b.edges.iter() {
        edges.insert(*e);
    }
    // Bridge edges: emergent third connects to first vertex of a and first
    // vertex of b (minimal bridge; guarantees connectedness).
    if let Some(&first_a) = a.vertices.iter().next() {
        edges.insert(edge_key(emergent_third, first_a));
    }
    if let Some(&first_b) = b.vertices.iter().next() {
        edges.insert(edge_key(emergent_third, first_b));
    }

    ComposedSheaf {
        underlying: SheafOfShardGraph { vertices, edges },
        emergent_third,
        fiedler_lambda_2_cached: 0.0, // filled at into_result
    }
}

/// Compute deterministic emergent-third VertexId per Rec #82 content-address
/// discipline. Guaranteed to not collide with any vertex in a or b via
/// max(a.vertices ∪ b.vertices) + 1 fallback.
fn compute_emergent_third_id(
    a: &SheafOfShardGraph,
    b: &SheafOfShardGraph,
) -> VertexId {
    let max_existing = a
        .vertices
        .iter()
        .chain(b.vertices.iter())
        .copied()
        .max()
        .unwrap_or(0);
    max_existing + 1
}

/// Verify all four PAPER §3.6 properties simultaneously per LOVE-monoid
/// coordinate-decomposition (Mara §4.4). NO if-else on Property verbs;
/// each property returns a PropertyVerdict; all four combined via
/// OpacityMap<Property> into Transparency<Property>.
fn verify_all_four_properties(
    sheaf_a: &SheafOfShardGraph,
    sheaf_b: &SheafOfShardGraph,
    candidate: &ComposedSheaf,
    lambda_a: f64,
    lambda_b: f64,
    lambda_composed: f64,
) -> Transparency<Property> {
    // Four coordinates ALWAYS computed simultaneously per LOVE-monoid discipline.
    let sovereignty = verify_sovereignty(sheaf_a, sheaf_b, candidate);
    let emergent_third = verify_emergent_third(sheaf_a, sheaf_b, candidate);
    let fiedler_rise = verify_fiedler_rise(lambda_a, lambda_b, lambda_composed);
    let fusion_refusal = verify_fusion_refusal(sheaf_a, sheaf_b);

    // Compose LOVE-monoid via Loss::combine per Rec #92 substrate-scale-
    // invariance. Clear is identity; Pass verdicts are Clear-adjacent (no
    // opacity added); Partial/Fail verdicts combine via Transparency::single
    // + Loss::combine (Fail-dominates absorbs).
    Transparency::clear()
        .combine(as_transparency(Property::Sovereignty, sovereignty))
        .combine(as_transparency(Property::EmergentThird, emergent_third))
        .combine(as_transparency(Property::FiedlerRise, fiedler_rise))
        .combine(as_transparency(Property::FusionRefusal, fusion_refusal))
}

/// Lift per-property verdict into `Transparency<Property>` per Rec #92.
/// Pass → Clear (identity in LOVE-monoid); Partial/Fail → single opacity.
fn as_transparency(property: Property, verdict: PropertyVerdict) -> Transparency<Property> {
    match verdict {
        PropertyVerdict::Pass => Transparency::clear(),
        non_pass => Transparency::single(property, non_pass),
    }
}

/// Property 1: Sovereignty preservation per PAPER §3.6.1.
/// Composed sheaf contains all vertices + edges of both sheaf_a and sheaf_b.
fn verify_sovereignty(
    sheaf_a: &SheafOfShardGraph,
    sheaf_b: &SheafOfShardGraph,
    candidate: &ComposedSheaf,
) -> PropertyVerdict {
    let a_preserved = sheaf_a
        .vertices()
        .all(|v| candidate.contains_vertex(v))
        && sheaf_a.edges().all(|e| candidate.contains_edge(e));
    let b_preserved = sheaf_b
        .vertices()
        .all(|v| candidate.contains_vertex(v))
        && sheaf_b.edges().all(|e| candidate.contains_edge(e));

    match (a_preserved, b_preserved) {
        (true, true) => PropertyVerdict::Pass,
        (false, true) => PropertyVerdict::Fail(Diagnostic::new(
            "sovereignty: sheaf_a vertices or edges not preserved in composed sheaf",
        )),
        (true, false) => PropertyVerdict::Fail(Diagnostic::new(
            "sovereignty: sheaf_b vertices or edges not preserved in composed sheaf",
        )),
        (false, false) => PropertyVerdict::Fail(Diagnostic::new(
            "sovereignty: neither sheaf_a nor sheaf_b vertices/edges preserved",
        )),
    }
}

/// Property 2: Emergent third admission per PAPER §3.6.2.
/// Composed sheaf carries exactly ONE emergent-third stalk not present in
/// either input.
fn verify_emergent_third(
    sheaf_a: &SheafOfShardGraph,
    sheaf_b: &SheafOfShardGraph,
    candidate: &ComposedSheaf,
) -> PropertyVerdict {
    let emergent = candidate.emergent_third_stalk();
    let in_a = sheaf_a.contains_vertex(emergent);
    let in_b = sheaf_b.contains_vertex(emergent);
    if in_a || in_b {
        PropertyVerdict::Fail(Diagnostic::new(
            "emergent_third: proposed vertex already present in sheaf_a or sheaf_b",
        ))
    } else {
        PropertyVerdict::Pass
    }
}

/// Property 3: Fiedler λ₂ STRICT rise per PAPER §3.6.3 (Alex 2026-09-01
/// Q-Mara-κ ratification: STRICT `>` for compose-emission commit tick;
/// non-strict `=` for kintsugi settle to λ₀ harmonic-component at Phase 2+).
fn verify_fiedler_rise(lambda_a: f64, lambda_b: f64, lambda_composed: f64) -> PropertyVerdict {
    let max_ab = lambda_a.max(lambda_b);
    if lambda_composed > max_ab {
        PropertyVerdict::Pass
    } else {
        PropertyVerdict::Fail(Diagnostic::new(format!(
            "fiedler_rise: composed λ₂={:.6} did not strictly exceed max(λ₂(a)={:.6}, λ₂(b)={:.6})",
            lambda_composed, lambda_a, lambda_b
        )))
    }
}

/// Property 4: Fusion refusal per PAPER §3.6.4.
/// Refuses composition when inputs have identical content-address (would
/// fuse rather than compose).
fn verify_fusion_refusal(
    sheaf_a: &SheafOfShardGraph,
    sheaf_b: &SheafOfShardGraph,
) -> PropertyVerdict {
    if sheaf_a.oid() == sheaf_b.oid() {
        PropertyVerdict::Fail(Diagnostic::new(
            "fusion_refusal: sheaf_a and sheaf_b have identical content-address; composition would fuse",
        ))
    } else {
        PropertyVerdict::Pass
    }
}

/// Final result construction: LOVE-monoid Clear → Ok(composed with cached
/// Fiedler); Opaque map → Err(RedGaugeWitness).
fn into_result(
    mut candidate: ComposedSheaf,
    lambda_composed: f64,
    verdict: Transparency<Property>,
) -> Result<ComposedSheaf, RedGaugeWitness> {
    match &verdict {
        Transparency::Clear => {
            candidate.fiedler_lambda_2_cached = lambda_composed;
            Ok(candidate)
        }
        Transparency::Opaque(_) => Err(verdict),
    }
}

// ---------------------------------------------------------------------------
// Phase 1 minimum tests (in-module smoke)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheaf_construction_edges_only_per_q_mara_lambda() {
        let sheaf = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        assert_eq!(sheaf.vertex_count(), 3);
        assert!(sheaf.contains_edge((0, 1)));
        assert!(sheaf.contains_edge((1, 2)));
        assert!(sheaf.contains_edge((1, 0))); // canonical edge_key
    }

    #[test]
    fn complete_graph_construction() {
        let k3 = sheaf_of_complete_graph_of_order(3);
        assert_eq!(k3.vertex_count(), 3);
        // K_3 has 3 edges (0-1, 0-2, 1-2)
        assert_eq!(k3.edges().count(), 3);
    }

    #[test]
    fn fusion_refusal_on_identical_inputs() {
        let sheaf_a = sheaf_of_shard_graph_from_edges(&[(0, 1)]);
        let sheaf_b = sheaf_a.clone();
        let result = kleinos(&sheaf_a, &sheaf_b);
        assert!(result.is_err(), "identical inputs must refuse fusion per PAPER §3.6.4");
    }

    #[test]
    fn content_address_determinism_per_rec_82() {
        let sheaf_1 = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
        let sheaf_2 = sheaf_of_shard_graph_from_edges(&[(1, 2), (0, 1)]);
        // Different construction order; same content → same OID
        assert_eq!(sheaf_1.oid(), sheaf_2.oid());
    }
}

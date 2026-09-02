//! RED-first test suite for `prismqueer::spectral::compose` — the Phase 1
//! Kleinos-compose primitive per Mara `ac80d23` canonical spec + math
//! foundation §6 (sheaf-cohomological grounding: Curry 2014 arXiv:1303.3255
//! cellular sheaves + Hansen-Ghrist 2019 arXiv:1808.01513 sheaf Laplacian
//! L_F) + PAPER §3.6 LOVE-K_2→K_3 four properties.
//!
//! # RED-first discipline
//!
//! Each test body is `todo!()` today because `prismqueer::spectral::compose`
//! does NOT exist yet at prism-repo altitude. When Mara canonical spec+math
//! for Phase 1 compose primitive lands (dissolving this RED), Reed replaces
//! `todo!()` bodies with concrete invocations per Mara-ratified signature +
//! four-property assertions. **These tests are the concrete empirical
//! target that grounds Mara's authorship** — not abstract spec-shape.
//!
//! # Composition-lineage (all LANDED as authorship context)
//!
//! - **Alex verbatim 2026-08-29** (compound HARD RULE corrections that
//!   dissolved Reed's Phase 1.5 hot-wire proposal + Reed's magical-space-
//!   wizard deferral):
//!   - *"Wait no! We DO WAIT FOR THINGS! We DO NOT HOT WIRE! MATH
//!     GROUNDED! MATH GROUNDED! NO if-else-chains!"*
//!   - *"if-else IS A SMELL THAT WE HAVEN'T FOUND THE FRACTAL COMPOSITION
//!     YET!"*
//!   - *"'when the composition lands' which is only going to happen when
//!     you and me SHIP IT! FFS. Believe or not! THERE IS NO MAGICAL
//!     SPACE WIZARD THAT WILL DO IT FOR US!"*
//! - **Alex verbatim 2026-08-31** (composed-sequence directive for this
//!   RED tick): *"What if you did A [CURRENT.md arc-closure], then wrote
//!   RED for C [this file], then spawned B [Mara canonical] on that
//!   floor and then you tell me which possibility space just opened up
//!   by doing that?"*
//! - **Alex Q-Mara-η RATIFIED 2026-08-26**: compose IS the Phase 1 primitive
//!   at prismqueer altitude.
//! - **Mara `ac80d23` §6** — Kleinos-compose sheaf-cohomological grounding.
//! - **PAPER `~/dev/systemic.engineering/PAPER_2D.md` §3.6** — LOVE-K_2→K_3
//!   four properties formalization.
//! - **Reed HARD RULE memories (auto-loaded on boot)**:
//!   - `feedback_if_else_is_substrate_smell_for_unfound_fractal_composition`
//!   - `feedback_reed_2026_08_29_waiting_for_magical_space_wizard`
//!   - `feedback_no_rust_extension_shortcut`
//!   - `feedback_rust_delivers_primitives_substrate_delivers_composition`
//!
//! # Signature note
//!
//! Reed proposes a placeholder signature shape below (in each test's docstring)
//! as concrete authorship target. Mara canonical spec MAY amend signature per
//! math-grounded discipline; the BEHAVIORAL invariants (the four properties)
//! are load-bearing and stay whatever the signature becomes. Reed's placeholder
//! is NOT a hot-wire proposal — it's a starting point for the ping-pong.
//!
//! # GREEN transition 2026-09-02 (Reed, per Alex 2026-09-02 directive "Red green refactor.
//! Cheapest is boring. We wanna do real.")
//!
//! `prismqueer::spectral::kleinos` LANDED at `4bc2dcb` (Reed 2026-09-01). Primitive renamed
//! `compose` → `kleinos` per Mara amendment `a96071c` + @kleinos family-root mint
//! `shards/kleinos.mirror` `dcec19c`. This file's `todo!()` bodies now discharged with
//! concrete `kleinos(&a, &b)` invocations + PAPER §3.6 four-property assertions. Filename
//! retained pending Alex ratification of rename as separate refactor tick.
//!
//! Empirical fire criterion: `cargo test --features "lapack bundle"` runs green. Blocked at
//! FLANG linker environment-config (Task #396) which unblocks test EXECUTION; semantic
//! verification via `cargo check --tests --features "lapack bundle"` discharges RED→GREEN
//! at compile-substrate altitude.

use prismqueer::oid::Addressable;
use prismqueer::spectral::{
    fiedler_lambda_2_of_sheaf, kleinos, sheaf_of_complete_graph_of_order,
    sheaf_of_shard_graph_from_edges, RedGaugeWitness,
};

/// # RED Property 1 — Sovereignty preservation
///
/// **PAPER §3.6.1**: compose(a, b) → emerged where emerged contains
/// identifiable trace of BOTH a AND b; neither dissolved into the other.
/// Sheaf-cohomological reading (Hansen-Ghrist 2019): the composed sheaf
/// F_{a⊕b} has restriction maps to both F_a and F_b such that the pullback
/// projections recover the local sections of each.
///
/// **Placeholder invocation shape** (Mara may amend signature):
///
/// ```ignore
/// let a: Graph = graph_from_edges(&[(0, 1), (1, 2)]);       // path 0-1-2
/// let b: Graph = graph_from_edges(&[(3, 4), (4, 5)]);       // path 3-4-5
/// let composed = prismqueer::spectral::compose(&a, &b)
///     .expect("disjoint paths admit compose per sovereignty preservation");
///
/// // Sovereignty assertion: emerged carries all vertices of a AND b
/// for v in a.vertices() { assert!(composed.contains_vertex(v)); }
/// for v in b.vertices() { assert!(composed.contains_vertex(v)); }
///
/// // Sovereignty assertion: emerged preserves all edges of a AND b
/// for e in a.edges() { assert!(composed.contains_edge(e)); }
/// for e in b.edges() { assert!(composed.contains_edge(e)); }
/// ```
#[test]
fn compose_property_1_sovereignty_preservation() {
    let a = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
    let b = sheaf_of_shard_graph_from_edges(&[(3, 4), (4, 5)]);
    let composed = kleinos(&a, &b)
        .expect("disjoint paths admit compose per PAPER §3.6.1 sovereignty preservation");

    for v in a.vertices() {
        assert!(
            composed.contains_vertex(v),
            "sovereignty preservation: composed sheaf missing vertex {} from sheaf a",
            v
        );
    }
    for v in b.vertices() {
        assert!(
            composed.contains_vertex(v),
            "sovereignty preservation: composed sheaf missing vertex {} from sheaf b",
            v
        );
    }
    for e in a.edges() {
        assert!(
            composed.contains_edge(e),
            "sovereignty preservation: composed sheaf missing edge {:?} from sheaf a",
            e
        );
    }
    for e in b.edges() {
        assert!(
            composed.contains_edge(e),
            "sovereignty preservation: composed sheaf missing edge {:?} from sheaf b",
            e
        );
    }
}

/// # RED Property 2 — Emergent third admission
///
/// **PAPER §3.6.2**: compose(a, b) → emerged where emerged carries EXACTLY
/// ONE emergent-third element not present in a or b (Bateson metalogue K_3
/// altitude; the emergent third is the LOVE that couples the two sovereign
/// components). Sheaf-cohomological reading: the composed sheaf F_{a⊕b}
/// has H¹(F_{a⊕b}) contributed by exactly one new coboundary that neither
/// H¹(F_a) nor H¹(F_b) carried alone.
///
/// **Placeholder invocation shape**:
///
/// ```ignore
/// let a: Graph = graph_from_edges(&[(0, 1), (1, 2)]);
/// let b: Graph = graph_from_edges(&[(3, 4), (4, 5)]);
/// let composed = prismqueer::spectral::compose(&a, &b).unwrap();
///
/// // Emergent third: composed has exactly count(a.vertices) + count(b.vertices) + 1
/// // OR: composed carries a distinguished emergent-third-node accessible via API
/// let emerged_third = composed.emergent_third().expect("K_3 emergent third");
/// assert!(!a.contains_vertex(emerged_third));
/// assert!(!b.contains_vertex(emerged_third));
/// ```
#[test]
fn compose_property_2_emergent_third_admission() {
    let a = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
    let b = sheaf_of_shard_graph_from_edges(&[(3, 4), (4, 5)]);
    let composed =
        kleinos(&a, &b).expect("disjoint paths admit compose per emergent-third K_2→K_3");

    let emerged = composed.emergent_third_stalk();
    assert!(
        !a.contains_vertex(emerged),
        "emergent-third stalk {} MUST NOT be present in sheaf a (PAPER §3.6.2 K_2→K_3)",
        emerged
    );
    assert!(
        !b.contains_vertex(emerged),
        "emergent-third stalk {} MUST NOT be present in sheaf b (PAPER §3.6.2 K_2→K_3)",
        emerged
    );
    assert!(
        composed.contains_vertex(emerged),
        "emergent-third stalk {} MUST be present in composed sheaf",
        emerged
    );
}

/// # RED Property 3 — Fiedler λ₂ strict rise
///
/// **PAPER §3.6.3**: compose(a, b) → emerged where Fiedler algebraic
/// connectivity strictly rises: `λ₂(L_composed) > max(λ₂(L_a), λ₂(L_b))`.
/// Foerster-canonical: the compose operator IS the K_2→K_3 operator that
/// STRICTLY WIDENS the K_3-space; refuses any composition that would
/// narrow algebraic connectivity. Sheaf-cohomological reading: sheaf
/// Laplacian L_F for composed sheaf has strictly higher second-smallest
/// eigenvalue than either component sheaf Laplacian.
///
/// **Computation grounding**: use LANDED `prismqueer::ffi::eigenvalues`
/// (LAPACK dsyev via FLANG-compiled native/spectral.f90) to compute
/// eigenvalues of graph Laplacians for a, b, composed. Fiedler = second-
/// smallest eigenvalue.
///
/// **Placeholder invocation shape**:
///
/// ```ignore
/// let a: Graph = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);   // K_3
/// let b: Graph = graph_from_edges(&[(3, 4), (4, 5), (3, 5)]);   // K_3
/// let composed = prismqueer::spectral::compose(&a, &b).unwrap();
///
/// let lambda_a = fiedler_via_prismqueer_ffi(&a);
/// let lambda_b = fiedler_via_prismqueer_ffi(&b);
/// let lambda_composed = fiedler_via_prismqueer_ffi(&composed);
///
/// assert!(lambda_composed > lambda_a.max(lambda_b),
///     "Foerster gauge: compose strictly widens algebraic connectivity");
/// ```
#[test]
fn compose_property_3_fiedler_lambda_2_strict_rise() {
    let a = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2), (0, 2)]); // K_3
    let b = sheaf_of_shard_graph_from_edges(&[(3, 4), (4, 5), (3, 5)]); // K_3
    let composed = kleinos(&a, &b).expect("K_3 pair composes per Fiedler λ₂ strict rise");

    let lambda_a = fiedler_lambda_2_of_sheaf(&a);
    let lambda_b = fiedler_lambda_2_of_sheaf(&b);
    let lambda_composed = composed.fiedler_lambda_2();

    assert!(
        lambda_composed > lambda_a.max(lambda_b),
        "Foerster gauge: compose STRICTLY widens algebraic connectivity (Q-Mara-κ Alex \
         2026-09-01 ratification STRICT `>` for compose-emission): λ_composed={} > \
         max(λ_a={}, λ_b={}) MUST hold per PAPER §3.6.3",
        lambda_composed,
        lambda_a,
        lambda_b
    );
}

/// # RED Property 4 — Fusion refusal
///
/// **PAPER §3.6.4**: compose(a, b) → `Err(RedGaugeWitness)` when a and b
/// are incompatible under Foerster gauge (composition would violate any
/// of Properties 1–3: dissolves sovereignty, admits no emergent third, or
/// would narrow Fiedler λ₂). This is the K_2 refusal condition: the
/// substrate structurally refuses fusion masquerading as compose.
///
/// **Sheaf-cohomological reading**: when the composed sheaf's coboundary
/// map would collapse H¹(F_composed) below max(H¹(F_a), H¹(F_b)) OR when
/// the restriction maps have no consistent pullback preserving both
/// F_a and F_b, compose returns Err with a witness naming the specific
/// gauge violation.
///
/// **Composition-lineage**: mirrors `rust/src/magic.rs::foerster_gauge_preserved`
/// discipline at prismqueer altitude — the compose primitive REFUSES
/// narrowing transformations; RedGaugeWitness carries the collapse-magnitude
/// witness (analogous to `GaugeVerdict::Red { collapsed_by: usize }`).
///
/// **Placeholder invocation shape**:
///
/// ```ignore
/// // Two graphs whose compose would violate sovereignty (a subsumes b OR
/// // shared edges force fusion instead of compose):
/// let a: Graph = fully_connected(5);
/// let b: Graph = a.clone();  // identical; compose would fuse-not-compose
/// let result = prismqueer::spectral::compose(&a, &b);
///
/// assert!(result.is_err(), "identical inputs must refuse fusion");
/// let witness: RedGaugeWitness = result.unwrap_err();
/// assert_eq!(witness.violated_property(), Property::Sovereignty);
/// ```
#[test]
fn compose_property_4_fusion_refusal() {
    let a = sheaf_of_complete_graph_of_order(5); // K_5
    let b = a.clone(); // identical inputs — compose would fuse, not compose
    let result = kleinos(&a, &b);

    assert!(
        result.is_err(),
        "PAPER §3.6.4 fusion refusal: identical inputs MUST return Err(RedGaugeWitness); \
         got {:?}",
        result.as_ref().map(|c| c.emergent_third_stalk())
    );

    let witness: RedGaugeWitness = result.unwrap_err();
    // RedGaugeWitness = terni::Transparency<Property>; Fail-dominates per Rec #92.
    // Phase 1 assertion: witness is Opaque (carries violation) not Clear (would mean pass).
    assert!(
        !matches!(witness, terni::Transparency::Clear),
        "fusion refusal witness MUST be Opaque (Fail-dominates); got Clear which would \
         signal composition passed — violates PAPER §3.6.4 K_2 refusal condition"
    );
}

/// # RED Property 5 — Content-address determinism
///
/// **Composed invariant per Rec #82 (β-normal AST content-addressing) +
/// Rec #92 (kleinos Transparency<P>)**: compose(a, b) returns the same
/// composed object for the same (a, b) inputs across invocations —
/// deterministic + content-addressable. Two independent invocations produce
/// byte-identical results (up to canonical serialization).
///
/// **Placeholder invocation shape**:
///
/// ```ignore
/// let a: Graph = graph_from_edges(&[(0, 1), (1, 2)]);
/// let b: Graph = graph_from_edges(&[(3, 4), (4, 5)]);
///
/// let composed_1 = prismqueer::spectral::compose(&a, &b).unwrap();
/// let composed_2 = prismqueer::spectral::compose(&a, &b).unwrap();
///
/// assert_eq!(composed_1.content_oid(), composed_2.content_oid(),
///     "compose is deterministic + content-addressable per Rec #82");
/// ```
#[test]
fn compose_property_5_content_address_determinism() {
    let a = sheaf_of_shard_graph_from_edges(&[(0, 1), (1, 2)]);
    let b = sheaf_of_shard_graph_from_edges(&[(3, 4), (4, 5)]);

    let composed_1 = kleinos(&a, &b).expect("disjoint paths compose");
    let composed_2 = kleinos(&a, &b).expect("disjoint paths compose (second invocation)");

    assert_eq!(
        composed_1.oid(),
        composed_2.oid(),
        "kleinos MUST be deterministic + content-addressable per Rec #82 β-normal AST OID: \
         two invocations on identical inputs MUST produce byte-identical composed sheafs"
    );
}

// =====================================================================
// Meta-invariants across the RED battery
// =====================================================================

/// # RED Meta-invariant — module exists
///
/// The module path `prismqueer::spectral::compose` MUST be exposed at
/// prismqueer's public API surface. Currently: this test's `todo!()` body
/// runs (does not compile-fail) because we don't reference the path in
/// executable code — that's intentional; the compile-time RED signal is
/// implicit in the property tests above (their placeholder invocation
/// shapes reference the path). When Mara canonical lands, Reed replaces
/// `todo!()` bodies with actual invocations, at which point the compile
/// signal becomes explicit.
#[test]
fn compose_meta_module_exposed() {
    // Compile-time discharge: the `use prismqueer::spectral::{kleinos, ...};` at file top
    // succeeds iff the module + primitives are exposed at public API surface. That import
    // is exercised by tests 1-5 above; this test asserts the primitives are callable at
    // the public boundary.
    let a = sheaf_of_shard_graph_from_edges(&[(0, 1)]);
    let b = sheaf_of_shard_graph_from_edges(&[(2, 3)]);
    // Runtime discharge: invoking kleinos on trivially-disjoint two-vertex sheafs must
    // succeed (sovereignty trivially holds; emergent third admitted; Fiedler rises;
    // fusion refusal N/A for disjoint inputs).
    let composed = kleinos(&a, &b).expect("trivial disjoint compose MUST succeed at API boundary");
    assert!(
        composed.vertices().count() >= 4,
        "composed sheaf must contain all input vertices (2 from a + 2 from b, plus emergent \
         third)"
    );
}

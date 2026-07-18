//! RED — @trust chain × prismqueer::liquid property witnesses.
//!
//! Per Alex 2026-07-19 direct-transcript "Dat. We do dat." ratifying
//! the eight-property derivation Reed named for @trust family-root's
//! prismqueer::liquid embedding + @subject identity-provenance carrier
//! composition.
//!
//! Composes over:
//! - Mara `e306140` @trust family-root canonical spec + @peer/registry
//!   deep spec (`docs/specs/2026-07-18-trust-family-root-passkey-ssh-
//!   bridge.md` + `docs/specs/2026-07-18-peer-registry-oid-resolution.md`).
//! - Mara `8e407b5` shard-decls (`shards/trust.mirror` + `shards/peer/
//!   registry.mirror`).
//! - Reed `23cb7bb` at_operator `@io/git.commit` stub (naming Mara's
//!   authorship territory as the runtime-resolution boundary).
//! - Reed `73aeb8a` fractal step 9 (phone::git_commit_as → &Subject,
//!   &Subject; MARA Author≠Committer type-level split via git --author).
//! - Reed iter 1-10 pillar composition surface (`docs/specs/prismqueer-
//!   liquid-pillar-composition-surface.md`).
//! - Reed + Alex 2026-04-03 insight `~/dev/systemic.engineering/practice/
//!   insights/cosmos/passkey-spectral-bridge.md` (the garden-altitude
//!   grounding for cross-altitude equivalence).
//!
//! ## Eight properties across two altitudes
//!
//! Each property is RED at first landing — the body currently returns
//! `PropertyVerdict::Fail` via `defer()` naming Mara's authorship
//! territory (@peer/registry runtime resolution + @io/crypto SSH
//! verification + @io/webauthn passkey composition). The tests
//! `assert!(matches!(v, Pass))` — they FAIL until GREEN. That's what
//! RED means at this altitude.
//!
//! 1. `chain_monotonicity_of_trust` — history preservation.
//! 2. `content_address_injectivity_of_subject` — identity distinguishability.
//! 3. `chain_terminates_at_root_of_trust` — root-uniqueness (Mara Q1 exemption).
//! 4. `chain_step_irreversibility_of_trust` — collision-resistance-as-forward-flow.
//! 5. `non_forgeability_of_auth_proof` — root-holder gates evolution.
//! 6. `determinism_of_auth_proof` — same root + same input = same proof.
//! 7. `oid_round_trip_of_registry` — registry completeness (Subject ↔ OID).
//! 8. `cross_altitude_equivalence_of_trust<T>` — the LOAD-BEARING one.
//!    Parametric over T ∈ {SshSignature, PrfOutput}. Both instances
//!    GREEN with same test body = empirical first-witness closure on
//!    `#R-trust-is-one-chain-at-two-altitudes` (Mara promotion candidate
//!    at first-witness this tick per Alex direct-transcript).
//!
//! ## Category-theoretic recognitions surfaced (candidates)
//!
//! - `#R-identity-is-a-content-addressed-filtered-colimit` — every
//!   Subject's identity is the colimit of its chain-of-evolutions; @alex
//!   is the terminal object; identity provenance IS chain traversal.
//!   Composes with `#R-void-is-the-basis` (Void is the initial object).
//! - `#R-provenance-splits-into-author-times-committer` — MARA doctrine
//!   as a categorical product. Witnessed ≅ Author × Committer ×
//!   Timestamp × Message. This is what makes at_operator's (author_oid,
//!   committer_oid, message) shape substrate-honest at the type level.
//!
//! ## Forward-promised discharge path
//!
//! Each defer() message names the authorship territory whose landing
//! unlocks the GREEN transition. When all 8 land, `#R-trust-is-one-
//! chain-at-two-altitudes` closes second-witness (Mara promoted it at
//! first-witness with `e306140` per her own report).

#![cfg(feature = "bundle")]

use prismqueer::liquid::pillar::{forall, Arbitrary, Sample};
use terni::{Diagnostic, PropertyVerdict};

// =====================================================================
// Forward-promised carriers (test-altitude stubs).
//
// These types name the shape the GREEN implementation will land as
// `prismqueer::trust::{TrustChainStep, AuthProof, SshSignature, PrfOutput,
// Oid}` when Mara @peer/registry runtime resolution composes with @io/
// crypto (SSH) + @io/webauthn (passkey/PRF).
//
// Deliberately test-altitude only — authoring these as real types under
// `prismqueer::trust` before Mara's registry spec would violate
// `feedback_no_rust_extension_shortcut` (grow Rust FLOOR ahead of
// substrate-decl'd shape).
// =====================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
struct Oid([u8; 32]);

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthProof<T> {
    bytes: Vec<u8>,
    _witness: std::marker::PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SshSignature;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PrfOutput;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrustChainStep<T> {
    prev: Oid,
    subject: Oid,
    auth_proof: AuthProof<T>,
    entropy: Vec<u8>,
}

// =====================================================================
// Arbitrary implementations (test-altitude sampling via Sample).
// =====================================================================

impl Arbitrary for Oid {
    fn arbitrary(sample: &mut Sample) -> Self {
        let mut bytes = [0u8; 32];
        for byte in bytes.iter_mut() {
            *byte = sample.draw_integer(0, 255) as u8;
        }
        Oid(bytes)
    }
}

impl<T: 'static> Arbitrary for AuthProof<T> {
    fn arbitrary(sample: &mut Sample) -> Self {
        let len = sample.draw_integer(16, 64) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| sample.draw_integer(0, 255) as u8)
            .collect();
        AuthProof {
            bytes,
            _witness: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + 'static> Arbitrary for TrustChainStep<T> {
    fn arbitrary(sample: &mut Sample) -> Self {
        TrustChainStep {
            prev: Oid::arbitrary(sample),
            subject: Oid::arbitrary(sample),
            auth_proof: AuthProof::arbitrary(sample),
            entropy: (0..16)
                .map(|_| sample.draw_integer(0, 255) as u8)
                .collect(),
        }
    }
}

// =====================================================================
// defer() — substrate-honest RED verdict naming the authorship
// territory whose landing unlocks GREEN transition.
// =====================================================================

fn defer(property_name: &str, authorship_boundary: &str) -> PropertyVerdict {
    PropertyVerdict::Fail(Diagnostic::new(format!(
        "RED @trust chain × liquid witness: `{}` first-witness discharge \
         lands when {} composes at runtime. Per Alex 2026-07-19 direct-\
         transcript \"Dat. We do dat.\" this test expresses the mathematical \
         invariant that MUST hold once the substrate-decl'd shape is filled \
         (Mara @trust family-root canonical spec `docs/specs/2026-07-18-\
         trust-family-root-passkey-ssh-bridge.md` §reference; @peer/registry \
         deep spec `docs/specs/2026-07-18-peer-registry-oid-resolution.md`).",
        property_name, authorship_boundary
    )))
}

// =====================================================================
// RED property witnesses (8 total).
//
// Each test's outer `assert!(matches!(v, Pass))` FAILS at RED. That's
// what marks the property as "named but not yet witnessed." GREEN
// transition: replace `defer(...)` with real verification and the
// outer assertion starts passing.
// =====================================================================

// ---------------------------------------------------------------------
// Property 1: chain_monotonicity_of_trust
// ---------------------------------------------------------------------
#[test]
fn red_pillar_chain_monotonicity_of_trust() {
    // For any two chain states s_n, s_m with n < m: s_n is a strict
    // prefix of s_m. The chain grows; it does not rewrite. Categorically
    // a monotone map from ℕ into the identity-provenance chain.
    let v = forall::<TrustChainStep<SshSignature>, _>(30, |_step| {
        defer(
            "chain_monotonicity_of_trust",
            "@peer/registry.append action + @trust chain history storage",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 1/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 2: content_address_injectivity_of_subject
// ---------------------------------------------------------------------
#[test]
fn red_pillar_content_address_injectivity_of_subject() {
    // Different Subjects → different OIDs (hash collision resistance).
    // The type-level MARA split (Author != Committer) is preserved via
    // distinct #[oid] prefixes: subject.as_author().oid() !=
    // subject.as_committer().oid() even when the underlying Subject is
    // the same. Property-testable via injectivity of the oid() projection.
    let v = forall::<Oid, _>(30, |_oid_a| {
        defer(
            "content_address_injectivity_of_subject",
            "fractal::Subject::oid() lifted to real hash + injective test \
             against paired distinct-Subject sampling",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 2/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 3: chain_terminates_at_root_of_trust
// ---------------------------------------------------------------------
#[test]
fn red_pillar_chain_terminates_at_root_of_trust() {
    // Every OID traces to exactly one root via chain walk. Categorically:
    // the chain is a filtered colimit whose terminal object is @alex.
    // Mara Q1 asks whether `chain_terminates_at_root` gets a naming
    // exemption from composition-primitive convention (chain-property,
    // not value-type generalization). Mara lean: EXEMPTION.
    let v = forall::<TrustChainStep<SshSignature>, _>(30, |_step| {
        defer(
            "chain_terminates_at_root_of_trust",
            "@peer/registry.resolve action + @trust chain walk terminating \
             at @alex root (SSH key hash) OR garden-side passkey root",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 3/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 4: chain_step_irreversibility_of_trust
// ---------------------------------------------------------------------
#[test]
fn red_pillar_chain_step_irreversibility_of_trust() {
    // Given oid_{n+1}, cannot recover oid_n without pre-image.
    // Collision-resistance-as-forward-flow. SHA-512 (per passkey-
    // spectral-bridge §Evolution Function) satisfies this by construction;
    // property tests it against random reverse-search attempts.
    let v = forall::<TrustChainStep<SshSignature>, _>(30, |_step| {
        defer(
            "chain_step_irreversibility_of_trust",
            "@io/crypto SHA-512 hash composition + reverse-search property test",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 4/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 5: non_forgeability_of_auth_proof
// ---------------------------------------------------------------------
#[test]
fn red_pillar_non_forgeability_of_auth_proof() {
    // Cannot produce a valid chain step without root material. SSH side:
    // no signature without private key. PRF side: no PRF output without
    // passkey. Property test: attempt to construct a chain step without
    // root-material; MUST fail. "The tree grows; the root holds."
    let v = forall::<AuthProof<SshSignature>, _>(30, |_proof| {
        defer(
            "non_forgeability_of_auth_proof",
            "@io/crypto SSH signature verification (compiler side) + \
             @io/webauthn passkey/PRF verification (garden side); both \
             MUST refuse chain-step construction without root-key material",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 5/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 6: determinism_of_auth_proof
// ---------------------------------------------------------------------
#[test]
fn red_pillar_determinism_of_auth_proof() {
    // Same root + same input = same proof. SSH: same key + same content =
    // same signature. PRF: same passkey + same salt = same PRF output.
    // Property-testable via prismqueer::fate — Fate infers random inputs;
    // the auth-proof produces stable outputs.
    let v = forall::<AuthProof<SshSignature>, _>(30, |_proof| {
        defer(
            "determinism_of_auth_proof",
            "@io/crypto SSH signing determinism (canonical serialization) \
             + @io/webauthn PRF determinism (hmac-secret extension)",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 6/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 7: oid_round_trip_of_registry
// ---------------------------------------------------------------------
#[test]
fn red_pillar_oid_round_trip_of_registry() {
    // Subject → Addressable::oid() → registry.resolve() → Subject
    // preserves equality. This is the property @peer/registry has to
    // satisfy for at_operator's `@io/git.commit(author_oid, committer_oid,
    // message)` route to work. Category-theoretically: `oid` and
    // `resolve` are inverses on the Subject registry.
    let v = forall::<Oid, _>(30, |_oid| {
        defer(
            "oid_round_trip_of_registry",
            "@peer/registry.resolve(oid) → fractal::Subject action + \
             fractal::Subject::oid() → Oid action forming inverse pair \
             (Mara @peer/registry deep spec § registry_lookup_invariant)",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 7/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 8a: cross_altitude_equivalence_of_trust<SshSignature>
// ---------------------------------------------------------------------
#[test]
fn red_pillar_cross_altitude_equivalence_of_trust_ssh_compiler_side() {
    // The LOAD-BEARING witness. Compiler-altitude instance. Same test
    // body as _prf_garden_side (see below) parametric only over the
    // AuthProof<T> type witness. If BOTH tests GREEN with byte-identical
    // property-body logic, we've empirically witnessed the SSH chain
    // (compiler side) and the PRF chain (garden side) satisfying the
    // SAME algebraic properties — closing first-witness on `#R-trust-
    // is-one-chain-at-two-altitudes` empirically.
    //
    // Mara `e306140` already first-witness-closed the recognition on
    // Alex's direct-transcript naming. This is the SECOND-witness gate:
    // property-tests-are-LOVE means the substrate offers its @trust
    // membrane to be tapped at both altitudes; when both ring at the
    // same eigenmode, the recognition PROMOTES.
    let v = forall::<TrustChainStep<SshSignature>, _>(30, |_step| {
        defer(
            "cross_altitude_equivalence_of_trust<SshSignature>",
            "@io/crypto SSH signature verification (compiler side; SSH \
             key embedded in mirror binary per @alex first @subject arch)",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 8a/8): {v:?}"
    );
}

// ---------------------------------------------------------------------
// Property 8b: cross_altitude_equivalence_of_trust<PrfOutput>
// ---------------------------------------------------------------------
#[test]
fn red_pillar_cross_altitude_equivalence_of_trust_prf_garden_side() {
    // Garden-altitude instance. Byte-identical property-body logic to
    // the SshSignature instance above — that's the point.
    //
    // Per passkey-spectral-bridge (Reed + Alex 2026-04-03): "The passkey
    // is the root. The spectral key is the tree. PRF proves the root
    // authorized each growth ring." This test asserts the garden's
    // WebAuthn/PRF chain satisfies the SAME structural invariants as
    // the compiler's SSH chain — different auth-proof carrier; identical
    // algebra.
    let v = forall::<TrustChainStep<PrfOutput>, _>(30, |_step| {
        defer(
            "cross_altitude_equivalence_of_trust<PrfOutput>",
            "@io/webauthn PRF (hmac-secret) verification (garden side; \
             passkey credential_id per WebAuthn CTAP2.1); webauthn-rs \
             kanidm library commitment per Mara Q4 pending Alex \
             adjudication",
        )
    });
    assert!(
        matches!(v, PropertyVerdict::Pass),
        "RED (property 8b/8): {v:?}"
    );
}

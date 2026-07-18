//! Prism law witnesses — the OPTIC BASE the Bundle tower stands on.
//!
//! Iter 1-2 tested Bundle/Connection/Gauge/Transport at the spectral
//! commutator altitude (`liquid_ouroboros.rs`). Iter 3-4 tested
//! `mirror/rust/src/collapse.rs` byte discipline and Pillar III
//! composition. Iter 5 closes the ouroboros at the LAST unexplored
//! layer: the Prism trait itself.
//!
//! `IdentityPrism<S: Clone>` is the **monoid identity** of the Prism
//! composition monoid — per `prismqueer/src/lib.rs` header:
//!
//! > A Prism is a **monoid** lifted into that semifunctor: prisms
//! > compose associatively (`focus | project | settle` chains), and
//! > an identity prism exists (pass-through on all three stages).
//!
//! These tests witness the identity law empirically across multiple
//! state types + witness that `apply(prism, beam)` is equivalent to
//! the manual `focus ∘ project ∘ settle` pipeline (the DSL contract
//! per the header).
//!
//! Composed over `terni::PropertyVerdict` for uniform verdict
//! marshaling — same machinery `prismqueer::liquid::pillar` returns
//! for commutator + magnitude verdicts. The property ouroboros is
//! now complete: Prism → Bundle → Liquid → collapse → back through
//! Pillar III.

#![cfg(feature = "bundle")]

use prismqueer::bundle::IdentityPrism;
use prismqueer::{apply, apply_h, Beam, Focus, Optic, Prism, Project, Settle};
use terni::{Diagnostic, PropertyVerdict};

// ──────────────────────────────────────────────────────────────────
// 1. IdentityPrism value roundtrip — the monoid identity law.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Applying `IdentityPrism<u32>` to a seed beam preserves the value.
fn identity_prism_preserves_u32_value() {
    let prism: IdentityPrism<u32> = IdentityPrism::new();
    let beam: Optic<(), u32> = Optic::ok((), 42);
    let refracted = apply(&prism, beam);
    assert!(refracted.is_ok());
    assert_eq!(refracted.value(), Some(&42));
}

#[test]
/// Applying `IdentityPrism<String>` preserves the string across the
/// three-stage pipeline. Different type; same law.
fn identity_prism_preserves_string_value() {
    let prism: IdentityPrism<String> = IdentityPrism::new();
    let beam: Optic<(), String> = Optic::ok((), "substrate-honest".to_string());
    let refracted = apply(&prism, beam);
    assert!(refracted.is_ok());
    assert_eq!(refracted.value(), Some(&"substrate-honest".to_string()));
}

#[test]
/// Applying `IdentityPrism<Vec<i32>>` preserves the vector.
/// Compound type; same law.
fn identity_prism_preserves_vec_value() {
    let prism: IdentityPrism<Vec<i32>> = IdentityPrism::new();
    let seed: Vec<i32> = vec![1, 1, 2, 3, 5, 8, 13];
    let beam: Optic<(), Vec<i32>> = Optic::ok((), seed.clone());
    let refracted = apply(&prism, beam);
    assert!(refracted.is_ok());
    assert_eq!(refracted.value(), Some(&seed));
}

#[test]
/// Applying `IdentityPrism<[f64; 4]>` preserves a Bundle-style 4-vector
/// state. This is the SAME shape used by `TestBundle` in the ouroboros
/// tests — the identity acts as expected on that fiber's state space.
fn identity_prism_preserves_bundle_fiber_state() {
    let prism: IdentityPrism<[f64; 4]> = IdentityPrism::new();
    let seed: [f64; 4] = [1.0, 2.0, 3.0, 4.0];
    let beam: Optic<(), [f64; 4]> = Optic::ok((), seed);
    let refracted = apply(&prism, beam);
    assert!(refracted.is_ok());
    assert_eq!(refracted.value(), Some(&seed));
}

// ──────────────────────────────────────────────────────────────────
// 2. apply() equals the manual focus ∘ project ∘ settle pipeline.
// ──────────────────────────────────────────────────────────────────

#[test]
/// The convenience `apply(prism, beam)` function is equivalent to the
/// manual DSL pipeline `beam.apply(Focus(prism)).apply(Project(prism))
/// .apply(Settle(prism))`. Contract per prismqueer/src/lib.rs docblock.
fn apply_equals_manual_focus_project_settle_pipeline() {
    let prism: IdentityPrism<u32> = IdentityPrism::new();

    let via_apply = apply(&prism, Optic::<(), u32>::ok((), 7));

    let via_manual = Optic::<(), u32>::ok((), 7)
        .apply(Focus(&prism))
        .apply(Project(&prism))
        .apply(Settle(&prism));

    // Both refracted beams have the same value.
    assert_eq!(via_apply.value(), via_manual.value());
    assert_eq!(via_apply.value(), Some(&7));
}

// ──────────────────────────────────────────────────────────────────
// 3. apply_h returns Success — heterogeneous action pattern.
// ──────────────────────────────────────────────────────────────────

#[test]
/// `apply_h(prism, state)` on an IdentityPrism returns
/// `Imperfect::Success(state)` — no loss, no failure. This is the
/// operator-action pattern used throughout the mirror bootstrap
/// (spectral-triple operator on Hilbert space).
fn apply_h_over_identity_prism_returns_success_with_input_state() {
    let prism: IdentityPrism<u32> = IdentityPrism::new();
    let out = apply_h::<_, u32, u32, u32, std::convert::Infallible, prismqueer::ScalarLoss>(
        &prism, 100,
    );
    match out {
        terni::Imperfect::Success(v) => assert_eq!(v, 100),
        other => panic!("expected Success(100), got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 4. Identity Prism composed with itself — monoid identity.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Applying `IdentityPrism` twice preserves the value. This is the
/// monoid identity law: `id ∘ id = id`. The refracted beam after two
/// applications matches the seed value.
fn identity_prism_composed_with_itself_still_preserves_value() {
    let prism: IdentityPrism<u32> = IdentityPrism::new();

    // First application.
    let first = apply(&prism, Optic::<(), u32>::ok((), 99));
    let first_value = *first.value().expect("first refracted beam must have value");

    // Feed the value forward into a second identity application.
    let second = apply(&prism, Optic::<(), u32>::ok((), first_value));
    assert_eq!(second.value(), Some(&99));
}

// ──────────────────────────────────────────────────────────────────
// 5. IdentityPrism composed with itself — zero loss over pipeline.
// ──────────────────────────────────────────────────────────────────

#[test]
/// IdentityPrism produces a Success beam — no loss accumulated. The
/// refracted beam's `is_ok()` returns true; is_err() returns false.
fn identity_prism_produces_success_beam_with_no_loss() {
    let prism: IdentityPrism<[f64; 4]> = IdentityPrism::new();
    let seed = [10.0, 20.0, 30.0, 40.0];
    let beam: Optic<(), [f64; 4]> = Optic::ok((), seed);
    let refracted = apply(&prism, beam);
    assert!(refracted.is_ok(), "IdentityPrism must produce Success beam");
    assert!(!refracted.is_err(), "IdentityPrism must not produce Failure");
    // Value present + equals seed.
    assert_eq!(refracted.value(), Some(&seed));
}

// ──────────────────────────────────────────────────────────────────
// 6. Verdict composition — report Prism laws through PropertyVerdict.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Report the IdentityPrism law as a `terni::PropertyVerdict`.
/// Pass when the roundtrip value matches; Fail(Diagnostic) otherwise.
/// Same PropertyVerdict machinery `prismqueer::liquid::pillar` uses —
/// unified across the property-testing ouroboros.
fn prism_law_composes_to_property_verdict() {
    let prism: IdentityPrism<u32> = IdentityPrism::new();
    let seed = 314;
    let refracted = apply(&prism, Optic::<(), u32>::ok((), seed));
    let verdict = match refracted.value() {
        Some(v) if *v == seed => PropertyVerdict::Pass,
        Some(v) => PropertyVerdict::Fail(Diagnostic::new(
            format!("IdentityPrism roundtrip failed: expected {seed}, got {v}"),
        )),
        None => PropertyVerdict::Fail(Diagnostic::new(
            "IdentityPrism produced dark beam — no value",
        )),
    };
    assert!(
        matches!(verdict, PropertyVerdict::Pass),
        "expected Pass, got {verdict:?}",
    );
}

//! PropertyVerdict composition witnesses — `merge_with` semantics.
//!
//! Iter 4 + iter 6 landed `viability_of_magnitudes` +
//! `algedonic_of_magnitude` so any `Loss + PartialOrd` can compose
//! into either pillar. Iter 8 witnesses the LAYER ABOVE: how do
//! multiple `PropertyVerdict`s combine into a single unified verdict?
//!
//! Per `terni/src/transparency.rs` (`PropertyVerdict::merge_with`):
//!
//! - `Fail` dominates — a `Fail` on either side absorbs the other.
//! - `Pass` is the **neutral element** (Pass ∪ X == X == X ∪ Pass).
//! - `Partial + Partial` combines diagnostics and takes the
//!   **minimum confidence** — confidence only goes down through
//!   accumulation (Beer's audit-channel semantics; a chain of
//!   partial witnesses is only as strong as its weakest link).
//!
//! These tests witness the semantics empirically — the composition
//! layer of the property-testing ouroboros closes.
//!
//! Iter 8 companion: mirror rust/src/collapse.rs prop_tests
//! composes multi-tick algedonic verdicts using this same
//! `merge_with` semantics.

use terni::{Diagnostic, PropertyVerdict};

// ──────────────────────────────────────────────────────────────────
// 1. Identity law — Pass ∪ Pass = Pass.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Two Passes compose to Pass. The base of the merge semilattice.
fn merge_pass_with_pass_stays_pass() {
    let mut a = PropertyVerdict::Pass;
    let b = PropertyVerdict::Pass;
    a.merge_with(&b);
    assert!(matches!(a, PropertyVerdict::Pass), "got {a:?}");
}

// ──────────────────────────────────────────────────────────────────
// 2. Pass is the neutral element.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Pass ∪ Partial = Partial (Pass is neutral; the Partial survives).
fn merge_pass_with_partial_yields_partial() {
    let mut a = PropertyVerdict::Pass;
    let b = PropertyVerdict::Partial {
        confidence: 0.7,
        diagnostics: vec![Diagnostic::new("probe")],
    };
    a.merge_with(&b);
    match a {
        PropertyVerdict::Partial { confidence, .. } => {
            assert!((confidence - 0.7).abs() < 1e-9, "confidence = {confidence}");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

#[test]
/// Partial ∪ Pass = Partial (Pass on the right is also neutral —
/// the Partial persists).
fn merge_partial_with_pass_stays_partial() {
    let mut a = PropertyVerdict::Partial {
        confidence: 0.7,
        diagnostics: vec![Diagnostic::new("probe")],
    };
    let b = PropertyVerdict::Pass;
    a.merge_with(&b);
    match a {
        PropertyVerdict::Partial { confidence, .. } => {
            assert!((confidence - 0.7).abs() < 1e-9, "confidence = {confidence}");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 3. Fail dominates — the absorbing element.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Pass ∪ Fail = Fail (Fail dominates on the right).
fn merge_pass_with_fail_yields_fail() {
    let mut a = PropertyVerdict::Pass;
    let b = PropertyVerdict::Fail(Diagnostic::new("boom"));
    a.merge_with(&b);
    assert!(matches!(a, PropertyVerdict::Fail(_)), "got {a:?}");
}

#[test]
/// Fail ∪ Pass = Fail (Fail dominates on the left; the incoming
/// Pass has no effect).
fn merge_fail_with_pass_stays_fail() {
    let mut a = PropertyVerdict::Fail(Diagnostic::new("boom"));
    let b = PropertyVerdict::Pass;
    a.merge_with(&b);
    assert!(matches!(a, PropertyVerdict::Fail(_)), "got {a:?}");
}

#[test]
/// Partial ∪ Fail = Fail (Fail dominates over Partial too).
fn merge_partial_with_fail_yields_fail() {
    let mut a = PropertyVerdict::Partial {
        confidence: 0.9,
        diagnostics: vec![Diagnostic::new("probe")],
    };
    let b = PropertyVerdict::Fail(Diagnostic::new("boom"));
    a.merge_with(&b);
    assert!(matches!(a, PropertyVerdict::Fail(_)), "got {a:?}");
}

#[test]
/// Fail ∪ Fail = Fail (idempotent under Fail; the left Fail
/// stays, dominates the incoming one).
fn merge_fail_with_fail_stays_fail() {
    let mut a = PropertyVerdict::Fail(Diagnostic::new("first"));
    let b = PropertyVerdict::Fail(Diagnostic::new("second"));
    a.merge_with(&b);
    // Fail dominates from the left — the incoming Fail's diagnostic
    // does not overwrite (per merge_with semantics: "Fail dominates;
    // no change" when self is already Fail).
    match a {
        PropertyVerdict::Fail(d) => {
            assert_eq!(d.as_str(), "first", "left Fail's diagnostic must persist");
        }
        other => panic!("expected Fail, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 4. Partial ∪ Partial — min confidence, union diagnostics.
// ──────────────────────────────────────────────────────────────────

#[test]
/// Two Partials merge: confidence = min(c1, c2); diagnostics
/// concatenated (left-then-right). Beer audit-channel: a chain of
/// partial witnesses is only as strong as its weakest link.
fn merge_partial_partial_takes_min_confidence_and_unions_diagnostics() {
    let mut a = PropertyVerdict::Partial {
        confidence: 0.9,
        diagnostics: vec![Diagnostic::new("left-probe")],
    };
    let b = PropertyVerdict::Partial {
        confidence: 0.3,
        diagnostics: vec![Diagnostic::new("right-probe")],
    };
    a.merge_with(&b);
    match a {
        PropertyVerdict::Partial { confidence, diagnostics } => {
            assert!(
                (confidence - 0.3).abs() < 1e-9,
                "expected min confidence 0.3, got {confidence}",
            );
            assert_eq!(diagnostics.len(), 2, "expected 2 diagnostics, got {}", diagnostics.len());
            assert_eq!(diagnostics[0].as_str(), "left-probe");
            assert_eq!(diagnostics[1].as_str(), "right-probe");
        }
        other => panic!("expected Partial, got {other:?}"),
    }
}

#[test]
/// merge_with is commutative on the underlying semilattice for
/// {Pass, Fail} but NOT symmetric on Fail diagnostics: left Fail
/// wins. Witness the asymmetric-Fail-diagnostic behavior
/// explicitly so the semantics are not accidentally changed.
fn merge_fail_left_wins_diagnostic_asymmetric() {
    let mut a = PropertyVerdict::Fail(Diagnostic::new("alpha"));
    let b = PropertyVerdict::Fail(Diagnostic::new("beta"));
    a.merge_with(&b);
    match &a {
        PropertyVerdict::Fail(d) => assert_eq!(d.as_str(), "alpha"),
        other => panic!("expected Fail, got {other:?}"),
    }

    // Reversed: now beta wins because it's on the left.
    let mut c = PropertyVerdict::Fail(Diagnostic::new("beta"));
    let d = PropertyVerdict::Fail(Diagnostic::new("alpha"));
    c.merge_with(&d);
    match &c {
        PropertyVerdict::Fail(diag) => assert_eq!(diag.as_str(), "beta"),
        other => panic!("expected Fail, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────────────
// 5. Folded composition — the multi-tick unified verdict.
// ──────────────────────────────────────────────────────────────────

#[test]
/// A fold of all-Pass verdicts stays Pass. This is the
/// substrate-honest witness for the unified verdict when every
/// tick succeeds — the compilation loop's algedonic health is
/// green.
fn folded_all_pass_verdicts_stay_pass() {
    let verdicts = vec![
        PropertyVerdict::Pass,
        PropertyVerdict::Pass,
        PropertyVerdict::Pass,
    ];
    let mut unified = PropertyVerdict::Pass;
    for v in &verdicts {
        unified.merge_with(v);
    }
    assert!(matches!(unified, PropertyVerdict::Pass), "got {unified:?}");
}

#[test]
/// A fold of Pass verdicts + one Fail = Fail. Substrate-honest
/// witness: if any tick fails, the unified verdict is Fail.
fn folded_pass_verdicts_with_one_fail_yields_fail() {
    let verdicts = vec![
        PropertyVerdict::Pass,
        PropertyVerdict::Fail(Diagnostic::new("stalled tick")),
        PropertyVerdict::Pass,
    ];
    let mut unified = PropertyVerdict::Pass;
    for v in &verdicts {
        unified.merge_with(v);
    }
    assert!(matches!(unified, PropertyVerdict::Fail(_)), "got {unified:?}");
}

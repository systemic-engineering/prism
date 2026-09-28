//! T0 (Mara nonlinear-inference.md): audit every public `Loss` against the
//! laws its trait promises.
//!
//! `terni::Loss` docstring: "Loss forms a monoid: `zero()` is the identity
//! element, `combine` is associative, and `total()` is the absorbing element
//! (annihilator)." So, for all a, b, c:
//!   identity:   zero·a = a = a·zero
//!   associative: (a·b)·c = a·(b·c)
//!   absorbing:  total·a = total = a·total

#![cfg(feature = "lapack")]

use std::collections::{BTreeSet, HashSet};
use std::fmt::Debug;

use prismqueer::spectral::settle::{Mode, Motion};
use prismqueer::ScalarLoss;
use terni::{
    ApertureLoss, ConvergenceLoss, Diagnostic, Loss, PropertyVerdict, RoutingLoss, Transparency,
};

fn audit<L: Loss + PartialEq + Debug>(name: &str, xs: &[L]) -> Vec<String> {
    let (z, t) = (L::zero(), L::total());
    let mut v = Vec::new();
    for a in xs {
        if z.clone().combine(a.clone()) != *a {
            v.push(format!("{name}: left identity fails on {a:?}"));
        }
        if a.clone().combine(z.clone()) != *a {
            v.push(format!("{name}: right identity fails on {a:?}"));
        }
        if t.clone().combine(a.clone()) != t {
            v.push(format!("{name}: total does not absorb from the left on {a:?}"));
        }
        if a.clone().combine(t.clone()) != t {
            v.push(format!("{name}: total does not absorb from the right on {a:?}"));
        }
        for b in xs {
            for c in xs {
                let l = a.clone().combine(b.clone()).combine(c.clone());
                let r = a.clone().combine(b.clone().combine(c.clone()));
                if l != r {
                    v.push(format!("{name}: not associative on ({a:?}, {b:?}, {c:?})"));
                }
            }
        }
    }
    v
}

fn all_violations() -> Vec<String> {
    let fail = |m: &str| PropertyVerdict::Fail(Diagnostic::new(m));
    let mode = |e: f64| Mode { eigenvalue: e, was: 1.0, now: 0.5 };
    let mut v = Vec::new();
    v.extend(audit("Vec<u8>", &[vec![], vec![1u8], vec![2, 3]]));
    v.extend(audit(
        "HashSet<u8>",
        &[HashSet::new(), HashSet::from([1u8]), HashSet::from([1u8, 2])],
    ));
    v.extend(audit(
        "BTreeSet<u8>",
        &[BTreeSet::new(), BTreeSet::from([1u8]), BTreeSet::from([1u8, 2])],
    ));
    v.extend(audit("String", &[String::new(), "a".into(), "b".into()]));
    v.extend(audit("usize", &[0usize, 1, usize::MAX - 1, usize::MAX]));
    v.extend(audit("u64", &[0u64, 1, u64::MAX - 1, u64::MAX]));
    v.extend(audit("f64", &[0.0f64, 0.1, 0.2, 0.3, 1e16]));
    v.extend(audit("(usize, String)", &[(0usize, String::new()), (1, "a".into())]));
    v.extend(audit(
        "ConvergenceLoss",
        &[ConvergenceLoss::new(0), ConvergenceLoss::new(3), ConvergenceLoss::new(7)],
    ));
    v.extend(audit(
        "ApertureLoss",
        &[
            ApertureLoss::new(vec![], 4),
            ApertureLoss::new(vec![1], 4),
            ApertureLoss::new(vec![1, 2], 4),
        ],
    ));
    v.extend(audit(
        "RoutingLoss",
        &[RoutingLoss::new(0.0, 1.0), RoutingLoss::new(0.5, 0.4), RoutingLoss::new(1.2, 0.1)],
    ));
    v.extend(audit(
        "Transparency<u8>",
        &[
            Transparency::Clear,
            Transparency::single(1u8, fail("one")),
            Transparency::single(2u8, fail("two")),
        ],
    ));
    v.extend(audit(
        "ScalarLoss",
        &[ScalarLoss::new(0.0), ScalarLoss::new(0.1), ScalarLoss::new(0.2), ScalarLoss::new(0.3)],
    ));
    v.extend(audit(
        "Motion",
        &[
            Motion::zero(),
            Motion { moving: vec![mode(1.0)], ..Motion::zero() },
            Motion { faded: vec![mode(3.0)], ..Motion::zero() },
        ],
    ));
    v
}

/// `"Impl: law"`, with the witnessing sample stripped.
fn kinds(violations: &[String]) -> BTreeSet<String> {
    violations
        .iter()
        .map(|v| v.split(" on ").next().unwrap_or(v).to_string())
        .collect()
}

/// Tripwire. The laws hold everywhere except these measured violations,
/// each an open decision for Alex (not a bug to paper over):
///
/// - Collections (`Vec`, `String`, `HashSet`, `BTreeSet`, and tuples that
///   contain them): `total()` equals `zero()` (empty), so it cannot absorb.
///   Structural, not a slip: a free monoid has no absorbing element. Options:
///   adjoin an absorbing element (as `Motion::opaque` and `Transparency`'s
///   catastrophic sentinel do), or weaken the trait's third law.
/// - `ApertureLoss`: `total()` does not absorb.
/// - `f64`, `ScalarLoss`: not associative at floating-point round-off
///   (Mara Q-Mara-NT7).
///
/// This test fails if any violation disappears or a new one appears.
#[test]
fn loss_laws_hold_except_known_violations() {
    let v = all_violations();
    for line in &v {
        eprintln!("{line}");
    }
    let absorb = |name: &str| {
        [
            format!("{name}: total does not absorb from the left"),
            format!("{name}: total does not absorb from the right"),
        ]
    };
    let mut expected: BTreeSet<String> = BTreeSet::new();
    for name in ["Vec<u8>", "HashSet<u8>", "BTreeSet<u8>", "String", "(usize, String)", "ApertureLoss"] {
        expected.extend(absorb(name));
    }
    expected.insert("f64: not associative".into());
    expected.insert("ScalarLoss: not associative".into());
    assert_eq!(kinds(&v), expected);
}

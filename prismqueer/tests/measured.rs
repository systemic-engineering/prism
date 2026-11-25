//! `Measured<Metrix>` (prism-023 §1; `docs/spec/measured.md`): a precision
//! matrix estimated from integer counts, exactly over ℚ, under a declared
//! support. Full support is S⁻¹; chordal support is the clique/separator
//! formula (Lauritzen 1996 §5.3, Dempster 1972); non-chordal support is
//! refused, never iterated. A `Measured` is not a `Metrix`: it becomes one
//! only through `MetrixDraft` (certificate) and a seal (signature).
//!
//! Every number below is worked by hand in exact rationals, in the doc
//! comment of the test that checks it.

#![cfg(all(feature = "exact", feature = "bundle"))]

use std::cell::Cell;
use std::path::Path;

use num_bigint::BigInt;
use prismqueer::algedonic::{Clock, Draft, Question, Region, Ruler, Wald};
use prismqueer::flux::pillar::{forall, Arbitrary, Sample};
use prismqueer::hypothesis::Frame;
use prismqueer::measured::{
    covariance, decompose, measure, measure_under, Estimator, Measured, Method, Refused, Shape,
    Support,
};
use prismqueer::metrix::{MetrixDraft, MetrixError};
use prismqueer::oid::Oid;
use prismqueer::room::{Coefficient, Entry, Q, Z};
use terni::{Diagnostic, PropertyVerdict};

// --- Fixtures ----------------------------------------------------------------

fn q(n: i64, d: i64) -> Q {
    Q::new(n, d).expect("non-zero denominator")
}

fn rows(m: &[&[(i64, i64)]]) -> Vec<Vec<Q>> {
    m.iter()
        .map(|r| r.iter().map(|&(n, d)| q(n, d)).collect())
        .collect()
}

fn counts(m: &[&[i64]]) -> Vec<Vec<Z>> {
    m.iter()
        .map(|r| r.iter().map(|&n| Z::from(n)).collect())
        .collect()
}

fn frame(dim: usize) -> Frame {
    Frame::new(Oid::new("streams"), dim)
}

/// A window ruler: `(lo, ∞)`, sealed by `by`.
fn window_by(lo: i64, by: &str) -> Ruler {
    Draft::new(
        Region::above(num_rational::BigRational::from_integer(BigInt::from(lo))),
        Question::Max,
        Wald::new(
            num_rational::BigRational::new(1.into(), 20.into()),
            num_rational::BigRational::new(1.into(), 20.into()),
        )
        .expect("valid rates"),
        Clock::agreed(10, 1, 100),
    )
    .seal(Oid::new(by))
}

fn window() -> Ruler {
    window_by(0, "window-keeper")
}

fn full(d: usize) -> Method {
    Method {
        estimator: Estimator::Mle,
        support: Support::full(d),
    }
}

fn with(estimator: Estimator, edges: &[(usize, usize)]) -> Method {
    Method {
        estimator,
        support: Support::new(edges.iter().copied()),
    }
}

fn measured(f: &Frame, x: &[&[i64]], method: Method) -> Measured {
    measure(&window(), f, &counts(x), method).expect("measurable")
}

/// The two-stream sample: (1,2), (2,2), (3,5), (4,7).
const TWO: [&[i64]; 4] = [&[1, 2], &[2, 2], &[3, 5], &[4, 7]];

/// The three-stream sample: (0,0,0), (1,1,0), (2,1,2), (1,2,2).
const THREE: [&[i64]; 4] = [&[0, 0, 0], &[1, 1, 0], &[2, 1, 2], &[1, 2, 2]];

/// The path A–B–C: no A–C edge.
const PATH: [(usize, usize); 2] = [(0, 1), (1, 2)];

/// Gauss–Jordan over ℚ, exact: the test's own inverse, independent of the
/// module's.
fn inverse(m: &[Vec<Q>]) -> Option<Vec<Vec<Q>>> {
    let n = m.len();
    let mut a: Vec<Vec<Q>> = m
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut row = r.clone();
            row.extend((0..n).map(|j| Q::from(i64::from(i == j))));
            row
        })
        .collect();
    for c in 0..n {
        let p = (c..n).find(|&r| a[r][c] != Q::from(0))?;
        a.swap(c, p);
        let inv = a[c][c].recip()?;
        a[c] = a[c].iter().map(|x| x.mul(&inv)).collect();
        for r in 0..n {
            if r != c && a[r][c] != Q::from(0) {
                let f = a[r][c].clone();
                let pivot = a[c].clone();
                a[r] = a[r]
                    .iter()
                    .zip(&pivot)
                    .map(|(x, y)| x.add(&f.mul(y).neg()))
                    .collect();
            }
        }
    }
    Some(a.into_iter().map(|r| r[n..].to_vec()).collect())
}

// --- Two streams, full support -------------------------------------------------

/// The 4 samples (1,2), (2,2), (3,5), (4,7).
///
/// - Means: x̄_A = 10/4 = 5/2, x̄_B = 16/4 = 4.
/// - Deviations: A = (−3/2, −1/2, 1/2, 3/2), B = (−2, −2, 1, 3).
/// - Sums: Σa² = 9/4 + 1/4 + 1/4 + 9/4 = 5; Σb² = 4 + 4 + 1 + 9 = 18;
///   Σab = 3 + 1 + 1/2 + 9/2 = 9.
/// - **MLE** (÷4): S = [[5/4, 9/4], [9/4, 9/2]], det S = 45/8 − 81/16 = 9/16,
///   K = S⁻¹ = (16/9)·[[9/2, −9/4], [−9/4, 5/4]] = [[8, −4], [−4, 20/9]].
///   LDLᵀ of K: d₁ = 8, l₂₁ = −4/8 = −1/2, d₂ = 20/9 − (1/4)·8 = 2/9;
///   d₁d₂ = 16/9 = det K = 1/det S.
#[test]
fn two_streams_full_support_mle_is_the_exact_inverse() {
    let f = frame(2);
    let s = covariance(&f, &counts(&TWO), Estimator::Mle).expect("a covariance");
    assert_eq!(s, rows(&[&[(5, 4), (9, 4)], &[(9, 4), (9, 2)]]));
    let m = measured(&f, &TWO, full(2));
    assert_eq!(
        m.estimate(),
        rows(&[&[(8, 1), (-4, 1)], &[(-4, 1), (20, 9)]])
    );
    assert_eq!(m.samples(), 4);
    let draft = m.draft().expect("positive definite");
    assert_eq!(draft.certificate().d(), [q(8, 1), q(2, 9)]);
    assert_eq!(draft.certificate().l()[1][0], q(-1, 2));
}

/// The same sample, **Unbiased** (÷3): S = [[5/3, 3], [3, 6]], det S =
/// 10 − 9 = 1, K = [[6, −3], [−3, 5/3]] = (3/4)·K_MLE.
/// LDLᵀ: d₁ = 6, l₂₁ = −1/2, d₂ = 5/3 − (1/4)·6 = 1/6; d₁d₂ = 1.
#[test]
fn two_streams_full_support_unbiased_divides_by_n_minus_one() {
    let f = frame(2);
    let s = covariance(&f, &counts(&TWO), Estimator::Unbiased).expect("a covariance");
    assert_eq!(s, rows(&[&[(5, 3), (3, 1)], &[(3, 1), (6, 1)]]));
    let method = Method {
        estimator: Estimator::Unbiased,
        support: Support::full(2),
    };
    let m = measured(&f, &TWO, method);
    assert_eq!(
        m.estimate(),
        rows(&[&[(6, 1), (-3, 1)], &[(-3, 1), (5, 3)]])
    );
    let draft = m.draft().expect("positive definite");
    assert_eq!(draft.certificate().d(), [q(6, 1), q(1, 6)]);
}

/// The one declared edge of metrix v0 is the full support on two streams.
#[test]
fn one_edge_on_two_streams_is_the_full_support() {
    let f = frame(2);
    let one = measured(&f, &TWO, with(Estimator::Mle, &[(1, 0)]));
    let all = measured(&f, &TWO, full(2));
    assert_eq!(one.estimate(), all.estimate());
}

// --- Three streams on a path -----------------------------------------------------

/// The 4 samples (0,0,0), (1,1,0), (2,1,2), (1,2,2) on A, B, C.
///
/// - Means: all 1. Deviations A = (−1, 0, 1, 0), B = (−1, 0, 0, 1),
///   C = (−1, −1, 1, 1).
/// - Sums: AA 2, BB 2, CC 4, AB 1, AC 2, BC 2; ÷4:
///   S = [[1/2, 1/4, 1/2], [1/4, 1/2, 1/2], [1/2, 1/2, 1]].
/// - Cliques {A,B}, {B,C}; separator {B}.
///   - S_AB = [[1/2, 1/4], [1/4, 1/2]], det 3/16,
///     S_AB⁻¹ = (16/3)·[[1/2, −1/4], [−1/4, 1/2]] = [[8/3, −4/3], [−4/3, 8/3]].
///   - S_BC = [[1/2, 1/2], [1/2, 1]], det 1/4,
///     S_BC⁻¹ = 4·[[1, −1/2], [−1/2, 1/2]] = [[4, −2], [−2, 2]].
///   - S_B⁻¹ = 2.
/// - K̂ = [S_AB⁻¹]⁰ + [S_BC⁻¹]⁰ − [S_B⁻¹]⁰:
///   K̂_AA = 8/3, K̂_AB = −4/3, K̂_BB = 8/3 + 4 − 2 = 14/3, K̂_BC = −2,
///   K̂_CC = 2, **K̂_AC = 0**.
/// - Full support instead: S⁻¹ = [[4, 0, −2], [0, 4, −2], [−2, −2, 3]]
///   (check: row A of S⁻¹·S = (4·1/2 − 2·1/2, 4·1/4 − 2·1/2, 4·1/2 − 2·1)
///   = (1, 0, 0)), so K̂ ≠ S⁻¹ and S⁻¹_AC = −2.
#[test]
fn the_path_has_an_exact_zero_where_no_edge_is_declared() {
    let f = frame(3);
    let s = covariance(&f, &counts(&THREE), Estimator::Mle).expect("a covariance");
    assert_eq!(
        s,
        rows(&[
            &[(1, 2), (1, 4), (1, 2)],
            &[(1, 4), (1, 2), (1, 2)],
            &[(1, 2), (1, 2), (1, 1)]
        ])
    );
    let path = measured(&f, &THREE, with(Estimator::Mle, &PATH));
    let k_hat = rows(&[
        &[(8, 3), (-4, 3), (0, 1)],
        &[(-4, 3), (14, 3), (-2, 1)],
        &[(0, 1), (-2, 1), (2, 1)],
    ]);
    assert_eq!(path.estimate(), k_hat);
    assert_eq!(path.estimate()[0][2], Q::from(0));
    assert_eq!(path.estimate()[2][0], Q::from(0));

    let whole = measured(&f, &THREE, full(3));
    assert_eq!(
        whole.estimate(),
        rows(&[
            &[(4, 1), (0, 1), (-2, 1)],
            &[(0, 1), (4, 1), (-2, 1)],
            &[(-2, 1), (-2, 1), (3, 1)]
        ])
    );
    assert_ne!(path.estimate(), whole.estimate());
}

/// The fit, not a mask: K̂⁻¹ agrees with S on the support and the diagonal
/// (Dempster's condition), and its A–C entry is the implied marginal
/// covariance S_AB·S_BC/S_BB = (1/4)(1/2)/(1/2) = 1/4, not S_AC = 1/2.
/// Its draft certifies: LDLᵀ of K̂ is d = (8/3, 14/3 − (1/2)²·8/3 = 4, 2 − (1/2)²·4 = 1)
/// with l_BA = −1/2, l_CB = −2/4 = −1/2.
#[test]
fn the_path_fit_reproduces_the_covariance_on_its_support() {
    let f = frame(3);
    let path = measured(&f, &THREE, with(Estimator::Mle, &PATH));
    let sigma = inverse(path.estimate()).expect("invertible");
    assert_eq!(
        sigma,
        rows(&[
            &[(1, 2), (1, 4), (1, 4)],
            &[(1, 4), (1, 2), (1, 2)],
            &[(1, 4), (1, 2), (1, 1)]
        ])
    );
    let draft = path.draft().expect("positive definite");
    assert_eq!(draft.certificate().d(), [q(8, 3), q(4, 1), q(1, 1)]);
}

/// Fewer samples suffice for a sparser support: 3 samples on 3 streams are
/// too few for the full support (n ≤ d) but enough for the path (n > 2, the
/// largest clique).
#[test]
fn a_chordal_support_needs_more_samples_than_its_largest_clique_only() {
    let f = frame(3);
    let x = counts(&THREE[..3]);
    assert_eq!(
        measure(&window(), &f, &x, full(3)),
        Err(Refused::TooFewSamples {
            samples: 3,
            needed: 4
        })
    );
    let path = measure(&window(), &f, &x, with(Estimator::Mle, &PATH)).expect("measurable");
    assert_eq!(path.estimate()[0][2], Q::from(0));
    assert!(path.draft().is_ok());
}

// --- The chordality check ------------------------------------------------------

#[test]
fn maximum_cardinality_search_names_the_cliques_and_separators() {
    let path = decompose(3, &Support::new(PATH)).expect("chordal");
    assert_eq!(path.order(), [0, 1, 2]);
    assert_eq!(path.cliques(), [vec![0, 1], vec![1, 2]]);
    assert_eq!(path.separators(), [vec![1]]);

    // The four-cycle with the chord A–C: two triangles on the chord.
    let chorded =
        decompose(4, &Support::new([(0, 1), (1, 2), (2, 3), (3, 0), (0, 2)])).expect("chordal");
    assert_eq!(chorded.order(), [0, 1, 2, 3]);
    assert_eq!(chorded.cliques(), [vec![0, 1, 2], vec![0, 2, 3]]);
    assert_eq!(chorded.separators(), [vec![0, 2]]);

    // Two components: an empty separator, contributing nothing.
    let apart = decompose(3, &Support::new([(0, 1)])).expect("chordal");
    assert_eq!(apart.cliques(), [vec![0, 1], vec![2]]);
    assert_eq!(apart.separators(), [Vec::<usize>::new()]);

    // Full support: one clique.
    let whole = decompose(3, &Support::full(3)).expect("chordal");
    assert_eq!(whole.cliques(), [vec![0, 1, 2]]);
    assert!(whole.separators().is_empty());

    // A star, its centre visited late: cliques in discovery order.
    let star = decompose(4, &Support::new([(0, 3), (1, 3), (2, 3)])).expect("chordal");
    assert_eq!(star.order(), [0, 3, 1, 2]);
    assert_eq!(star.cliques(), [vec![0, 3], vec![1, 3], vec![2, 3]]);
    assert_eq!(star.separators(), [vec![3], vec![3]]);
}

/// A star's separator {centre} counts twice: K̂_cc = Σ over three cliques
/// − 2·S_cc⁻¹. Samples on (A, B, C, centre): (0,0,0,0), (1,0,0,1),
/// (0,1,0,1), (0,0,1,1), (1,1,1,1). Checked by Dempster's condition: K̂⁻¹
/// equals S on every edge and the diagonal, and K̂ is zero off them.
#[test]
fn a_separator_counts_with_its_multiplicity() {
    let f = frame(4);
    let x: [&[i64]; 5] = [
        &[0, 0, 0, 0],
        &[1, 0, 0, 1],
        &[0, 1, 0, 1],
        &[0, 0, 1, 1],
        &[1, 1, 1, 1],
    ];
    let star = [(0, 3), (1, 3), (2, 3)];
    let m = measured(&f, &x, with(Estimator::Mle, &star));
    let s = covariance(&f, &counts(&x), Estimator::Mle).expect("a covariance");
    let sigma = inverse(m.estimate()).expect("invertible");
    for i in 0..4 {
        for j in 0..4 {
            let declared = i == j || star.contains(&(i.min(j), i.max(j)));
            if declared {
                assert_eq!(sigma[i][j], s[i][j], "Σ̂[{i}][{j}]");
            } else {
                assert_eq!(m.estimate()[i][j], Q::from(0), "K̂[{i}][{j}]");
            }
        }
    }
}

/// The four-cycle A–B–C–D–A has no chord: no closed form, so no number.
#[test]
fn the_four_cycle_is_unresolved_never_a_number() {
    let f = frame(4);
    let cycle = Support::new([(0, 1), (1, 2), (2, 3), (3, 0)]);
    assert_eq!(
        decompose(4, &cycle).map(|_| ()),
        Err(Refused::Unresolved(cycle.clone()))
    );
    let x = counts(&[
        &[0, 0, 0, 0],
        &[1, 0, 0, 1],
        &[0, 1, 0, 2],
        &[0, 0, 1, 1],
        &[1, 1, 1, 0],
        &[2, 0, 1, 3],
    ]);
    let method = Method {
        estimator: Estimator::Mle,
        support: cycle.clone(),
    };
    assert_eq!(
        measure(&window(), &f, &x, method.clone()),
        Err(Refused::Unresolved(cycle.clone()))
    );
    // Refused before the sample is counted: too few samples is still Unresolved.
    assert_eq!(
        measure(&window(), &f, &x[..2], method),
        Err(Refused::Unresolved(cycle))
    );
}

// --- Refusals ------------------------------------------------------------------

#[test]
fn n_at_most_d_is_too_few_samples() {
    let f = frame(2);
    assert_eq!(
        measure(&window(), &f, &counts(&TWO[..2]), full(2)),
        Err(Refused::TooFewSamples {
            samples: 2,
            needed: 3
        })
    );
    assert_eq!(
        measure(&window(), &f, &[], full(2)),
        Err(Refused::TooFewSamples {
            samples: 0,
            needed: 3
        })
    );
    assert_eq!(
        covariance(&f, &[], Estimator::Mle),
        Err(Refused::TooFewSamples {
            samples: 0,
            needed: 1
        })
    );
    assert_eq!(
        covariance(&f, &counts(&TWO[..1]), Estimator::Unbiased),
        Err(Refused::TooFewSamples {
            samples: 1,
            needed: 2
        })
    );
}

/// B = 2A: (1,2), (2,4), (3,6). MLE S = [[2/3, 4/3], [4/3, 8/3]], det 0.
/// `measure` refuses it as `NotInvertible`; handed to `MetrixDraft`
/// directly, LDLᵀ gives d₁ = 2/3, l₂₁ = 2, d₂ = 8/3 − 4·(2/3) = 0: refused
/// at pivot 2, never clipped.
#[test]
fn collinear_streams_are_not_invertible_and_never_clipped() {
    let f = frame(2);
    let x = counts(&[&[1, 2], &[2, 4], &[3, 6]]);
    assert_eq!(
        measure(&window(), &f, &x, full(2)),
        Err(Refused::NotInvertible { clique: vec![0, 1] })
    );
    let s = covariance(&f, &x, Estimator::Mle).expect("a covariance");
    assert_eq!(s, rows(&[&[(2, 3), (4, 3)], &[(4, 3), (8, 3)]]));
    assert_eq!(
        MetrixDraft::new(&f, s),
        Err(MetrixError::NotDefinite {
            pivot: 2,
            value: Q::from(0)
        })
    );
}

/// On the path, a singular clique {B, C} is named (C = B), while {A, B}
/// alone would be fine.
#[test]
fn the_singular_clique_is_named() {
    let f = frame(3);
    let x = counts(&[&[0, 0, 0], &[1, 1, 1], &[2, 0, 0], &[0, 2, 2]]);
    assert_eq!(
        measure(&window(), &f, &x, with(Estimator::Mle, &PATH)),
        Err(Refused::NotInvertible { clique: vec![1, 2] })
    );
}

#[test]
fn a_wrong_shape_is_refused() {
    let f = frame(2);
    assert_eq!(
        measure(&window(), &f, &counts(&[&[1, 2], &[3], &[4, 5]]), full(2)),
        Err(Refused::WrongShape(Shape::Sample {
            index: 1,
            len: 1,
            streams: 2
        }))
    );
    assert_eq!(
        covariance(&f, &counts(&[&[1, 2, 3]]), Estimator::Mle),
        Err(Refused::WrongShape(Shape::Sample {
            index: 0,
            len: 3,
            streams: 2
        }))
    );
    for (from, to) in [(0, 2), (1, 1)] {
        assert_eq!(
            measure(
                &window(),
                &f,
                &counts(&TWO),
                with(Estimator::Mle, &[(from, to)])
            ),
            Err(Refused::WrongShape(Shape::Edge { from, to }))
        );
    }
}

// --- The address -----------------------------------------------------------------

#[test]
fn the_address_is_a_function_of_window_frame_method_and_samples() {
    let f = frame(2);
    let base = measured(&f, &TWO, full(2));
    assert_eq!(base.oid(), measured(&f, &TWO, full(2)).oid());
    assert_eq!(base.window(), &window().oid());

    let other_window =
        measure(&window_by(1, "window-keeper"), &f, &counts(&TWO), full(2)).expect("measurable");
    let unbiased = measured(
        &f,
        &TWO,
        Method {
            estimator: Estimator::Unbiased,
            support: Support::full(2),
        },
    );
    let other_sample = measured(&f, &[&[1, 2], &[2, 2], &[3, 5], &[4, 8]], full(2));
    let other_frame = measure(
        &window(),
        &Frame::new(Oid::new("other streams"), 2),
        &counts(&TWO),
        full(2),
    )
    .expect("measurable");
    let mut seen = vec![base.oid()];
    for m in [&other_window, &unbiased, &other_sample, &other_frame] {
        assert!(!seen.contains(&m.oid()), "a fresh address for {m:?}");
        seen.push(m.oid());
    }
    assert_ne!(base.samples_oid(), other_sample.samples_oid());
    assert_eq!(base.samples_oid(), unbiased.samples_oid());

    // Another support on three streams is another address.
    let f3 = frame(3);
    assert_ne!(
        measured(&f3, &THREE, with(Estimator::Mle, &PATH)).oid(),
        measured(&f3, &THREE, full(3)).oid()
    );
    // One support written two ways is one address.
    assert_eq!(
        measured(&f3, &THREE, with(Estimator::Mle, &[(2, 1), (0, 1), (1, 0)])).oid(),
        measured(&f3, &THREE, with(Estimator::Mle, &PATH)).oid()
    );
}

/// The window's signer sits beside the window's address: one window judgment
/// sealed by two keepers measures to one address, and no signer's name is in
/// the preimage.
#[test]
fn the_address_contains_no_signer() {
    let f = frame(2);
    let by_alice = measure(&window_by(0, "alice"), &f, &counts(&TWO), full(2)).expect("measurable");
    let by_bob = measure(&window_by(0, "bob"), &f, &counts(&TWO), full(2)).expect("measurable");
    assert_eq!(by_alice.oid(), by_bob.oid());
    let pre = by_alice.preimage();
    assert!(pre.starts_with(b"measured\0"));
    for name in [&b"alice"[..], b"bob"] {
        assert!(!pre.windows(name.len()).any(|w| w == name));
    }
}

// --- To a Metrix -------------------------------------------------------------------

/// The typed road: sealed by the window ruler's signer, refused for another
/// window. The open road: any signer may seal the draft, and the `Metrix`
/// names who did. Both have the one metrix address.
#[test]
fn the_window_ruler_seals_and_any_other_seal_is_named() {
    let f = frame(2);
    let w = window();
    let m = measured(&f, &TWO, full(2));

    assert_eq!(
        m.clone().seal(&window_by(7, "window-keeper")),
        Err(Refused::OtherWindow {
            expected: w.oid(),
            found: window_by(7, "window-keeper").oid()
        })
    );

    let by_window = m.clone().seal(&w).expect("the window's own ruler");
    assert_eq!(by_window.signer().by(), &Oid::new("window-keeper"));
    assert_eq!(by_window.rows(), m.estimate());
    assert_eq!(by_window.frame(), f.oid());

    let by_other = m
        .draft()
        .expect("positive definite")
        .seal(Oid::new("someone else"));
    assert_eq!(by_other.signer().by(), &Oid::new("someone else"));
    assert_eq!(by_other.oid(), by_window.oid());
    assert_ne!(by_other.oid(), m.oid());
}

#[test]
fn a_draft_refusal_surfaces_as_a_refusal() {
    assert_eq!(
        Refused::from(MetrixError::NotSquare),
        Refused::Draft(MetrixError::NotSquare)
    );
}

// --- Dempster's condition, over random supports --------------------------------

/// A random support on 2–5 streams and a random integer sample of d + 1 to
/// d + 4 counts in 0..=4.
#[derive(Debug, Clone)]
struct Case {
    d: usize,
    edges: Vec<(usize, usize)>,
    x: Vec<Vec<i64>>,
}

impl Arbitrary for Case {
    fn arbitrary(sample: &mut Sample) -> Self {
        let mut r = u64::arbitrary(sample);
        let mut next = move || {
            r = r.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = r;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let d = 2 + (next() % 4) as usize;
        let edges = (0..d)
            .flat_map(|i| (i + 1..d).map(move |j| (i, j)))
            .filter(|_| next() % 2 == 0)
            .collect::<Vec<_>>();
        let n = d + 1 + (next() % 4) as usize;
        let x = (0..n)
            .map(|_| (0..d).map(|_| (next() % 5) as i64).collect())
            .collect();
        Case { d, edges, x }
    }
}

/// An independent oracle: a graph on ≤ 5 vertices is chordal iff no vertex
/// subset of size ≥ 4 induces a cycle (connected, every degree 2).
fn has_chordless_cycle(d: usize, edges: &[(usize, usize)]) -> bool {
    let adj = |a: usize, b: usize| edges.contains(&(a.min(b), a.max(b)));
    (0u32..1 << d).any(|mask| {
        let vs: Vec<usize> = (0..d).filter(|v| mask & (1 << v) != 0).collect();
        if vs.len() < 4
            || !vs
                .iter()
                .all(|&v| vs.iter().filter(|&&u| adj(u, v)).count() == 2)
        {
            return false;
        }
        let mut seen = vec![vs[0]];
        let mut at = 0;
        while at < seen.len() {
            let v = seen[at];
            for &u in &vs {
                if adj(u, v) && !seen.contains(&u) {
                    seen.push(u);
                }
            }
            at += 1;
        }
        seen.len() == vs.len()
    })
}

fn check(holds: bool, what: &str) -> PropertyVerdict {
    if holds {
        PropertyVerdict::Pass
    } else {
        PropertyVerdict::Fail(Diagnostic::new(what.to_owned()))
    }
}

/// For every chordal support, K̂ is zero off the support, K̂⁻¹ equals S on
/// the support and the diagonal (Dempster 1972: the covariance-selection
/// MLE), and its draft certifies. Every non-chordal support is Unresolved.
#[test]
fn the_fit_meets_dempsters_condition_on_every_chordal_support() {
    let (fits, unresolved) = (Cell::new(0), Cell::new(0));
    let verdict = forall::<Case, _>(b"measured dempster", 256, |c| {
        let f = frame(c.d);
        let rows: Vec<&[i64]> = c.x.iter().map(Vec::as_slice).collect();
        let x = counts(&rows);
        let method = with(Estimator::Mle, &c.edges);
        let support = method.support.clone();
        match measure(&window(), &f, &x, method) {
            Err(Refused::Unresolved(s)) => {
                unresolved.set(unresolved.get() + 1);
                check(
                    s == support && has_chordless_cycle(c.d, &c.edges),
                    "Unresolved, but chordal",
                )
            }
            Err(Refused::NotInvertible { .. }) => check(
                !has_chordless_cycle(c.d, &c.edges),
                "NotInvertible before Unresolved",
            ),
            Err(other) => check(false, &format!("unexpected refusal {other:?}")),
            Ok(m) => {
                fits.set(fits.get() + 1);
                let s = covariance(&f, &x, Estimator::Mle).expect("a covariance");
                let k = m.estimate();
                let Some(sigma) = inverse(k) else {
                    return check(false, "K̂ singular");
                };
                let agrees = (0..c.d).all(|i| {
                    (0..c.d).all(|j| {
                        let declared = i == j || c.edges.contains(&(i.min(j), i.max(j)));
                        if declared {
                            sigma[i][j] == s[i][j]
                        } else {
                            k[i][j] == Q::from(0)
                        }
                    })
                });
                check(
                    agrees && !has_chordless_cycle(c.d, &c.edges) && m.draft().is_ok(),
                    "Dempster's condition fails, or a non-chordal fit, or no certificate",
                )
            }
        }
    });
    assert!(matches!(verdict, PropertyVerdict::Pass), "{verdict:?}");
    assert!(fits.get() >= 64, "only {} fits", fits.get());
    assert!(
        unresolved.get() >= 8,
        "only {} unresolved",
        unresolved.get()
    );
}

// --- No float on this path --------------------------------------------------------

/// The module's code (comments aside) names no float type and no LAPACK:
/// the pattern of `no_clock.rs`, on one file.
#[test]
fn the_measured_module_has_no_float() {
    // The module lives in prism's floor, prism-laws (docs/spec/laws.md).
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../laws/src/measured.rs");
    let text = std::fs::read_to_string(&file).expect("laws/src/measured.rs is readable");
    let forbidden = ["f64", "f32", "lapack", "ndarray", "blas", "band::"];
    let hits: Vec<String> = text
        .lines()
        .enumerate()
        .filter_map(|(n, line)| {
            let code = line.split("//").next().unwrap_or("").to_lowercase();
            forbidden
                .iter()
                .any(|w| code.contains(w))
                .then(|| format!("laws/src/measured.rs:{}: {}", n + 1, line.trim()))
        })
        .collect();
    assert!(text.lines().count() > 50, "the scan found the module");
    assert!(
        hits.is_empty(),
        "a float on the exact path:\n{}",
        hits.join("\n")
    );
}

/// `measure_under` is `measure` with the window named by its address: the
/// same `Measured`, and the seal still takes the ruler.
#[test]
fn measuring_under_an_address_is_measuring_under_its_ruler() {
    let f = frame(2);
    let by_ruler = measured(&f, &TWO, full(2));
    let by_address = measure_under(window().oid(), &f, &counts(&TWO), full(2)).expect("measurable");
    assert_eq!(by_address, by_ruler);
    let other = window_by(1, "window-keeper");
    assert!(matches!(
        by_address.seal(&other),
        Err(Refused::OtherWindow { .. })
    ));
}

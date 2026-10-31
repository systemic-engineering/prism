//! Factoring over ℚ, exactly, and mirror's algebraic tag (`0x06`): the
//! minimal polynomial of an eigenvalue and the root's index among its real
//! roots.
//!
//! Laws:
//! 1. A certified factorization's product is the input's primitive part,
//!    and each factor is irreducible over ℚ. The product is checked exactly
//!    here; irreducibility is checked on every small case by exhaustion: no
//!    rational root, and (for monic quartics) no monic quadratic factor.
//! 2. Recombination is what makes it right: x⁴ + 1 factors mod every prime,
//!    and is irreducible over ℚ.
//! 3. The lift reaches past the Mignotte bound: factors with coefficients
//!    far larger than the prime come back exactly.
//! 4. Out of budget, the answer is uncertified, never wrong: the factors it
//!    names are irreducible, and with the unresolved rest their product is
//!    still the input.
//! 5. A measured eigenvalue reads as `Algebraic { minimal, index }`: its
//!    irreducible factor, and its place among that factor's real roots.
//!
//! Every expected value is worked by hand. Polynomials are written leading
//! coefficient first, the way they are on paper.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use prismqueer::poly::{Algebraic, Budget, Factored, Identified, NotSquareFree, QPoly, ZPoly};

/// A ℤ[x] polynomial, leading coefficient first.
fn z(coefficients: &[i64]) -> ZPoly {
    ZPoly::from_descending(coefficients.iter().map(|&c| BigInt::from(c)))
}

fn product(factors: &[ZPoly]) -> ZPoly {
    factors.iter().fold(z(&[1]), |acc, f| &acc * f)
}

/// Plenty: no case here needs more than a few dozen subset trials.
const PLENTY: Budget = Budget(10_000);

/// The certified irreducible factors, or a panic naming what came back.
fn factors(p: &ZPoly) -> Vec<ZPoly> {
    match p.factor(PLENTY).expect("square-free") {
        Factored::Certified(f) => f.factors().to_vec(),
        other => panic!("not certified within {PLENTY:?}: {other:?}"),
    }
}

/// The positive divisors of a nonzero integer, by trial (test sizes only).
fn divisors(n: &BigInt) -> Vec<BigInt> {
    let n = n.abs();
    let mut d = Vec::new();
    let mut k = BigInt::from(1);
    while k <= n {
        if (&n % &k).is_zero() {
            d.push(k.clone());
        }
        k += 1;
    }
    d
}

/// Exhaustive evidence that `g` has no factor of degree ≤ deg/2, for the
/// small cases here: no rational root ±a/b with a | g(0), b | lc(g); and, at
/// degree ≥ 4 and monic, no monic quadratic x² + bx + c with c | g(0) and
/// |b| ≤ 2‖g‖₂ (Mignotte: a factor's coefficient of x is at most
/// C(2, 1)·‖g‖₂). Degree ≤ 5 is all this covers, and all it is asked to.
fn no_small_factor(g: &ZPoly) {
    let n = g.degree().expect("nonzero");
    assert!(n <= 5, "the exhaustive check covers degree ≤ 5");
    if n <= 1 {
        return;
    }
    let c0 = g.coefficients()[0].clone();
    assert!(!c0.is_zero(), "{g:?} has the root 0");
    let lc = g.leading().expect("nonzero").clone();
    for a in divisors(&c0) {
        for b in divisors(&lc) {
            for sign in [1, -1] {
                let r = BigRational::new(&a * sign, b.clone());
                assert!(!g.eval(&r).is_zero(), "{g:?} has the root {r}");
            }
        }
    }
    if n >= 4 {
        assert_eq!(&lc, &BigInt::from(1), "the quadratic check is for monic g");
        let norm2: BigInt = g.coefficients().iter().map(|c| c * c).sum();
        let bound: BigInt = (norm2.sqrt() + 1) * 2;
        let gq = g.to_rational();
        for c in divisors(&c0) {
            for c in [c.clone(), -c] {
                let mut b = -bound.clone();
                while b <= bound {
                    let d = QPoly::from_descending(
                        [BigInt::from(1), b.clone(), c.clone()]
                            .into_iter()
                            .map(BigRational::from_integer),
                    );
                    let (_, r) = gq.div_rem(&d).expect("d ≠ 0");
                    assert!(!r.is_zero(), "{g:?} has the factor x² + {b}x + {c}");
                    b += 1;
                }
            }
        }
    }
}

/// The factorization law: the product is the input, every factor is
/// irreducible (by exhaustion, small cases), and the factors are the
/// expected ones, in the canonical order (degree, then coefficients).
fn assert_factors(p: &ZPoly, expected: &[ZPoly]) {
    let got = factors(p);
    assert_eq!(product(&got), p.primitive(), "product");
    for g in &got {
        no_small_factor(g);
    }
    assert_eq!(got, expected);
}

// --- Law 1: the factorization and its certificate ---------------------------

#[test]
fn x4_minus_1_is_two_lines_and_a_circle() {
    // (x − 1)(x + 1)(x² + 1) = (x² − 1)(x² + 1) = x⁴ − 1.
    assert_factors(
        &z(&[1, 0, 0, 0, -1]),
        &[z(&[1, -1]), z(&[1, 1]), z(&[1, 0, 1])],
    );
}

#[test]
fn two_irreducible_quadratics_and_root_two_at_index_one() {
    // (x² − 2)(x² − 3) = x⁴ − 5x² + 6.
    let p = z(&[1, 0, -5, 0, 6]);
    assert_factors(&p, &[z(&[1, 0, -3]), z(&[1, 0, -2])]);
    let f = match p.factor(PLENTY).expect("square-free") {
        Factored::Certified(f) => f,
        other => panic!("{other:?}"),
    };
    // √2 ≈ 1.41421356: the larger of x² − 2's roots ±√2, so index 1.
    assert_eq!(
        f.identify(std::f64::consts::SQRT_2, 1e-9),
        Identified::Algebraic(Algebraic::new(z(&[1, 0, -2]), 1))
    );
    // −√3 is the smaller of x² − 3's: index 0.
    assert_eq!(
        f.identify(-(3f64.sqrt()), 1e-9),
        Identified::Algebraic(Algebraic::new(z(&[1, 0, -3]), 0))
    );
}

#[test]
fn the_cyclotomic_phi_12_is_irreducible() {
    // Φ₁₂ = x⁴ − x² + 1: the primitive 12th roots of unity, none real.
    assert_factors(&z(&[1, 0, -1, 0, 1]), &[z(&[1, 0, -1, 0, 1])]);
}

#[test]
fn x12_minus_1_is_the_product_of_its_cyclotomics() {
    // x¹² − 1 = Φ₁Φ₂Φ₃Φ₄Φ₆Φ₁₂: x − 1, x + 1, x² + x + 1, x² + 1,
    // x² − x + 1, x⁴ − x² + 1. Degrees 1 + 1 + 2 + 2 + 2 + 4 = 12.
    let mut p = vec![0; 13];
    p[0] = 1;
    p[12] = -1;
    assert_factors(
        &z(&p),
        &[
            z(&[1, -1]),
            z(&[1, 1]),
            z(&[1, -1, 1]),
            z(&[1, 0, 1]),
            z(&[1, 1, 1]),
            z(&[1, 0, -1, 0, 1]),
        ],
    );
}

#[test]
fn a_leading_coefficient_is_carried_through_the_lift() {
    // (3x + 1)(2x² − 3) = 6x³ + 2x² − 9x − 3: lc 6, two non-monic factors.
    assert_factors(&z(&[6, 2, -9, -3]), &[z(&[3, 1]), z(&[2, 0, -3])]);
}

#[test]
fn content_and_sign_are_units_over_q() {
    // −2x² + 4 = −2(x² − 2): the primitive part x² − 2 is factored.
    assert_factors(&z(&[-2, 0, 4]), &[z(&[1, 0, -2])]);
    // x² − x = x(x − 1): the root 0 is a factor like any other.
    assert_factors(&z(&[1, -1, 0]), &[z(&[1, 0]), z(&[1, -1])]);
    // Linear and constant inputs.
    assert_factors(&z(&[5, 3]), &[z(&[5, 3])]);
    assert_factors(&z(&[7]), &[]);
}

#[test]
fn a_repeated_root_or_zero_is_refused() {
    assert_eq!(z(&[1, -2, 1]).factor(PLENTY), Err(NotSquareFree));
    assert_eq!(z(&[]).factor(PLENTY), Err(NotSquareFree));
}

// --- Law 2: recombination ------------------------------------------------------

#[test]
fn x4_plus_1_is_irreducible_though_it_factors_mod_every_prime() {
    // x⁴ + 1 = Φ₈. Mod p it splits into two quadratics or four lines,
    // always: only recombination finds that no subset lifts to a factor.
    assert_factors(&z(&[1, 0, 0, 0, 1]), &[z(&[1, 0, 0, 0, 1])]);
}

#[test]
fn swinnerton_dyer_s2_is_irreducible() {
    // S₂ = x⁴ − 10x² + 1, the minimal polynomial of √2 + √3: like x⁴ + 1,
    // it splits mod every prime.
    assert_factors(&z(&[1, 0, -10, 0, 1]), &[z(&[1, 0, -10, 0, 1])]);
}

// --- Law 3: the lift reaches past the bound -----------------------------------

#[test]
fn large_coefficients_come_back_exactly() {
    // (x² − 1000x + 7)(x² + 3x − 200): discriminants 999972 and 809, neither
    // a square, so both irreducible. Coefficients far past any small prime.
    let a = z(&[1, -1000, 7]);
    let b = z(&[1, 3, -200]);
    let p = &a * &b;
    // x⁴ + (3 − 1000)x³ + (−200 − 3000 + 7)x² + (200000 + 21)x − 1400.
    assert_eq!(p, z(&[1, -997, -3193, 200021, -1400]));
    assert_factors(&p, &[a, b]);
}

// --- Law 4: the budget refuses, never lies ------------------------------------

#[test]
fn out_of_budget_is_uncertified_never_wrong() {
    // (x² − 2)(x² − 3)(x² − 5): three quadratics, which split mod p into up
    // to six pieces. Every budget either certifies the right answer or names
    // only true irreducible factors and an unresolved rest.
    let truth = [z(&[1, 0, -5]), z(&[1, 0, -3]), z(&[1, 0, -2])];
    let p = product(&truth);
    let mut refused = 0;
    for b in 0..64 {
        match p.factor(Budget(b)).expect("square-free") {
            Factored::Certified(f) => assert_eq!(f.factors(), &truth[..]),
            Factored::Uncertified {
                irreducible,
                unresolved,
            } => {
                refused += 1;
                for g in &irreducible {
                    assert!(truth.contains(g), "{g:?} is not a true factor");
                }
                assert!(unresolved.degree() >= Some(1));
                assert_eq!(&product(&irreducible) * &unresolved, p);
            }
        }
    }
    assert!(refused > 0, "budget 0 must refuse something");
    // Budget 0 on x⁴ + 1: no subset may be tried, so nothing is certified.
    assert_eq!(
        z(&[1, 0, 0, 0, 1]).factor(Budget(0)),
        Ok(Factored::Uncertified {
            irreducible: vec![],
            unresolved: z(&[1, 0, 0, 0, 1]),
        })
    );
}

#[test]
fn swinnerton_dyer_s3_needs_a_real_budget() {
    // S₃ = x⁸ − 40x⁶ + 352x⁴ − 960x² + 576, the minimal polynomial of
    // √2 + √3 + √5: degree 8, and mod every prime its factors have degree
    // ≤ 2, so at least four of them. One trial can't certify it; enough can.
    let s3 = z(&[1, 0, -40, 0, 352, 0, -960, 0, 576]);
    assert!(matches!(
        s3.factor(Budget(1)),
        Ok(Factored::Uncertified { .. })
    ));
    assert_eq!(factors(&s3), vec![s3.clone()]);
}

// --- Law 5: the algebraic tag ---------------------------------------------------

#[test]
fn an_exact_rational_root_is_its_own_linear_tag() {
    // x(x − 2): 2 is the root of x − 2, index 0.
    let f = match z(&[1, -2, 0]).factor(PLENTY).expect("square-free") {
        Factored::Certified(f) => f,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        f.identify(2.0000000000000004, 1e-9),
        Identified::Algebraic(Algebraic::new(z(&[1, -2]), 0))
    );
    assert_eq!(
        f.identify(-3.0e-16, 1e-9),
        Identified::Algebraic(Algebraic::new(z(&[1, 0]), 0))
    );
    assert_eq!(f.identify(1.0, 0.1), Identified::Absent);
    assert_eq!(f.identify(f64::NAN, 0.1), Identified::Unmeasured);
    // A window over both roots refuses, naming both.
    assert_eq!(
        f.identify(1.0, 1.5),
        Identified::Ambiguous(vec![
            Algebraic::new(z(&[1, 0]), 0),
            Algebraic::new(z(&[1, -2]), 0),
        ])
    );
}

#[test]
fn an_algebraic_tag_has_an_isolating_interval() {
    let tag = Algebraic::new(z(&[1, 0, -2]), 1);
    let iv = tag.isolating();
    // The interval holds √2 and only it: one root of x² − 2, positive.
    let roots = z(&[1, 0, -2]).real_roots();
    assert_eq!(roots.count_in(iv.lo(), iv.hi()), 1);
    assert!(iv.lo() >= &BigRational::from_integer(BigInt::from(0)));
    assert_eq!(tag.minimal(), &z(&[1, 0, -2]));
    assert_eq!(tag.index(), 1);
}

// --- Against room: P₄ and C₅, measured ------------------------------------------

#[cfg(feature = "lapack")]
mod against_room {
    use super::*;
    use prismqueer::room::laplacian;
    use prismqueer::spectral::graph_from_edges;

    /// The tag of every measured eigenvalue of a graph's L₀.
    fn tags(edges: &[(u64, u64)]) -> (ZPoly, Vec<Identified>) {
        let g = graph_from_edges(edges);
        let l0 = laplacian(g.multigraph());
        let minimal = l0.minimal_polynomial();
        // Symmetric, so diagonalisable: the minimal polynomial is the last
        // invariant factor and the square-free charpoly both.
        let last = l0
            .invariant_factors()
            .nontrivial()
            .last()
            .expect("n ≥ 1")
            .primitive();
        assert_eq!(minimal, last);
        assert_eq!(minimal, l0.charpoly().poly().square_free());
        let f = match minimal.factor(PLENTY).expect("square-free") {
            Factored::Certified(f) => f,
            other => panic!("{other:?}"),
        };
        assert_eq!(product(f.factors()), minimal);
        let modes = l0.measure().decompose().expect("measured");
        let tags = modes
            .spectrum()
            .iter()
            .map(|&x| f.identify(x, 1e-9))
            .collect();
        (minimal, tags)
    }

    fn tag(minimal: &[i64], index: usize) -> Identified {
        Identified::Algebraic(Algebraic::new(z(minimal), index))
    }

    #[test]
    fn the_path_p4_reads_as_x_x_minus_2_and_two_minus_root_two() {
        // P₄'s L₀ = [[1,−1,0,0],[−1,2,−1,0],[0,−1,2,−1],[0,0,−1,1]] has
        // eigenvalues 2 − 2cos(kπ/4), k = 0..3: 0, 2 − √2, 2, 2 + √2.
        // charpoly x(x − 2)(x² − 4x + 2): (x − 2)(x² − 4x + 2) =
        // x³ − 6x² + 10x − 4, so x⁴ − 6x³ + 10x² − 4x. Check: trace 6 =
        // 0 + (2 − √2) + 2 + (2 + √2); e₂ = 2(2 − √2) + (2 − √2)(2 + √2) +
        // 2(2 + √2) = 4 − 2√2 + 2 + 4 + 2√2 = 10; det 0. All distinct, so
        // the minimal polynomial is the charpoly. x² − 4x + 2 has roots
        // 2 ± √2 (disc 16 − 8 = 8, not a square: irreducible), 2 − √2 ≈
        // 0.586 at index 0, 2 + √2 ≈ 3.414 at index 1.
        let (minimal, tags) = tags(&[(0, 1), (1, 2), (2, 3)]);
        assert_eq!(minimal, z(&[1, -6, 10, -4, 0]));
        assert_eq!(
            tags,
            vec![
                tag(&[1, 0], 0),
                tag(&[1, -4, 2], 0),
                tag(&[1, -2], 0),
                tag(&[1, -4, 2], 1),
            ]
        );
    }

    #[test]
    fn the_cycle_c5_reads_in_the_golden_ratio() {
        // C₅'s L₀ has eigenvalues 2 − 2cos(2πk/5), k = 0..4: 0 once, and
        // (5 − √5)/2 ≈ 1.382 and (5 + √5)/2 ≈ 3.618 twice each (2cos(2π/5)
        // = (√5 − 1)/2 = φ − 1, 2cos(4π/5) = −φ). Their polynomial is
        // x² − 5x + 5 (sum 5, product (25 − 5)/4 = 5; disc 5, irreducible).
        // charpoly x(x² − 5x + 5)², minimal x(x² − 5x + 5) = x³ − 5x² + 5x.
        let (minimal, tags) = tags(&[(0, 1), (1, 2), (2, 3), (3, 4), (0, 4)]);
        assert_eq!(minimal, z(&[1, -5, 5, 0]));
        assert_eq!(
            tags,
            vec![
                tag(&[1, 0], 0),
                tag(&[1, -5, 5], 0),
                tag(&[1, -5, 5], 0),
                tag(&[1, -5, 5], 1),
                tag(&[1, -5, 5], 1),
            ]
        );
    }
}

//! `prismqueer::poly` — exact polynomials over ℤ and ℚ, and the real roots
//! of an integer polynomial, isolated exactly.
//!
//! Laws:
//! 1. ℤ[x] and ℚ[x] arithmetic agrees with hand expansion; division with
//!    remainder over ℚ[x] and pseudo-division over ℤ[x] satisfy their
//!    identities exactly.
//! 2. gcd over ℤ[x] is primitive with a positive leading coefficient, and
//!    the square-free part p / gcd(p, p′) keeps each root once.
//! 3. Real-root isolation gives disjoint rational intervals, one root each,
//!    in increasing order: the root index.
//! 4. A measured eigenvalue is matched to its root only when one root is in
//!    its window; otherwise it refuses, naming the candidates.
//!
//! Every expected value is worked by hand. Polynomials are written leading
//! coefficient first, the way they are on paper.

use num_bigint::BigInt;
use num_rational::BigRational;
use prismqueer::poly::{Located, QPoly, ZPoly};

/// A ℤ[x] polynomial, leading coefficient first.
fn z(coefficients: &[i64]) -> ZPoly {
    ZPoly::from_descending(coefficients.iter().map(|&c| BigInt::from(c)))
}

fn q(num: i64, den: i64) -> BigRational {
    BigRational::new(BigInt::from(num), BigInt::from(den))
}

/// A ℚ[x] polynomial, leading coefficient first, from (num, den) pairs.
fn qp(coefficients: &[(i64, i64)]) -> QPoly {
    QPoly::from_descending(coefficients.iter().map(|&(n, d)| q(n, d)))
}

// --- Law 1: arithmetic --------------------------------------------------------

#[test]
fn integer_polynomials_normalise_and_report_their_shape() {
    let p = z(&[0, 0, 3, -1]); // leading zeros dropped: 3x − 1
    assert_eq!(p, z(&[3, -1]));
    assert_eq!(p.degree(), Some(1));
    assert_eq!(p.leading(), Some(&BigInt::from(3)));
    assert_eq!(p.coefficients(), &[BigInt::from(-1), BigInt::from(3)]);
    let zero = z(&[0, 0]);
    assert!(zero.is_zero());
    assert_eq!(zero.degree(), None);
    assert_eq!(zero.leading(), None);
    assert_eq!(
        ZPoly::from_ascending([BigInt::from(-1), BigInt::from(3), BigInt::from(0)]),
        p
    );
}

#[test]
fn integer_arithmetic_matches_hand_expansion() {
    let a = z(&[1, -1]); // x − 1
    let b = z(&[1, 2]); // x + 2
    assert_eq!(&a + &b, z(&[2, 1]));
    assert_eq!(&a - &b, z(&[-3]));
    assert_eq!(&a - &a, z(&[]));
    assert_eq!(&a * &b, z(&[1, 1, -2]));
    assert_eq!(&a * &z(&[]), z(&[]));
    assert_eq!(-&a, z(&[-1, 1]));
    // (x − 1)²(x + 2) = x³ − 3x + 2.
    assert_eq!(&(&a * &a) * &b, z(&[1, 0, -3, 2]));
}

#[test]
fn derivative_content_and_primitive_part() {
    let p = z(&[1, 0, -3, 2]);
    assert_eq!(p.derivative(), z(&[3, 0, -3]));
    assert_eq!(z(&[7]).derivative(), z(&[]));
    let p = z(&[-6, 4, 10]); // −2·(3x² − 2x − 5)
    assert_eq!(p.content(), BigInt::from(2));
    assert_eq!(
        p.primitive(),
        z(&[3, -2, -5]),
        "positive leading coefficient"
    );
    assert_eq!(z(&[]).content(), BigInt::from(0));
    assert_eq!(z(&[]).primitive(), z(&[]));
}

#[test]
fn evaluation_at_an_exact_rational() {
    let p = z(&[1, 0, -3, 2]); // (x − 1)²(x + 2)
    assert_eq!(p.eval(&q(1, 1)), q(0, 1));
    assert_eq!(p.eval(&q(-2, 1)), q(0, 1));
    // (1/2)³ − 3/2 + 2 = 1/8 + 1/2 = 5/8.
    assert_eq!(p.eval(&q(1, 2)), q(5, 8));
    assert_eq!(z(&[]).eval(&q(3, 7)), q(0, 1));
}

#[test]
fn pseudo_division_over_the_integers() {
    // a = 3x³ + x + 1, b = 2x + 1: lc(b)^(3 − 1 + 1) · a = q·b + r.
    let a = z(&[3, 0, 1, 1]);
    let b = z(&[2, 1]);
    let (quotient, remainder) = a.pseudo_div(&b).expect("b ≠ 0");
    let scale = z(&[8]);
    assert_eq!(&scale * &a, &(&quotient * &b) + &remainder);
    assert!(remainder.degree().unwrap_or(0) < 1);
    // 8·(3(−1/2)³ − 1/2 + 1) = 8·(1/8) = 1.
    assert_eq!(remainder, z(&[1]));
    assert_eq!(a.pseudo_div(&z(&[])), None, "no division by zero");
    // A divisor of higher degree: quotient 0, remainder a.
    let (quotient, remainder) = b.pseudo_div(&a).expect("a ≠ 0");
    assert_eq!((quotient, remainder), (z(&[]), b));
}

#[test]
fn rational_polynomials_divide_with_remainder() {
    let a = qp(&[(1, 1), (0, 1), (0, 1), (-1, 1)]); // x³ − 1
    let b = qp(&[(2, 1), (-2, 1)]); // 2x − 2
    let (quotient, remainder) = a.div_rem(&b).expect("b ≠ 0");
    // x³ − 1 = (2x − 2)·(x² + x + 1)/2.
    assert_eq!(quotient, qp(&[(1, 2), (1, 2), (1, 2)]));
    assert!(remainder.is_zero());
    let c = qp(&[(1, 1), (0, 1), (1, 1)]); // x² + 1
    let (quotient, remainder) = c.div_rem(&b).expect("b ≠ 0");
    assert_eq!(&(&quotient * &b) + &remainder, c);
    assert_eq!(remainder, qp(&[(2, 1)]), "c(1) = 2");
    assert_eq!(a.div_rem(&QPoly::from_descending([])), None);
}

#[test]
fn rational_polynomials_have_arithmetic_and_a_monic_gcd() {
    let a = qp(&[(1, 2), (1, 3)]); // x/2 + 1/3
    let b = qp(&[(1, 1), (-1, 1)]); // x − 1
    assert_eq!(&a + &b, qp(&[(3, 2), (-2, 3)]));
    assert_eq!(&a - &a, QPoly::from_descending([]));
    assert_eq!(-&b, qp(&[(-1, 1), (1, 1)]));
    assert_eq!(a.monic(), qp(&[(1, 1), (2, 3)]));
    assert_eq!(a.derivative(), qp(&[(1, 2)]));
    assert_eq!(a.eval(&q(2, 1)), q(4, 3));
    assert_eq!(a.leading(), Some(&q(1, 2)));
    assert_eq!(a.degree(), Some(1));
    assert_eq!(a.coefficients(), &[q(1, 3), q(1, 2)]);
    // gcd((x − 1)(x + 2)/4, 3(x − 1)²) = x − 1, monic.
    let p = qp(&[(1, 4), (1, 4), (-1, 2)]);
    let r = qp(&[(3, 1), (-6, 1), (3, 1)]);
    assert_eq!(p.gcd(&r), b);
    assert_eq!(p.gcd(&QPoly::from_descending([])), p.monic());
    // The primitive part over ℤ: clear denominators, divide by content.
    assert_eq!(a.primitive(), z(&[3, 2]));
    assert_eq!(z(&[2, -4]).to_rational(), qp(&[(2, 1), (-4, 1)]));
}

// --- Law 2: gcd and square-free part -----------------------------------------

#[test]
fn gcd_of_integer_polynomials() {
    // (x − 1)²(x + 2) and (x − 1)(x − 3): gcd x − 1.
    let a = z(&[1, 0, -3, 2]);
    let b = z(&[1, -4, 3]);
    assert_eq!(a.gcd(&b), z(&[1, -1]));
    // Primitive, positive leading, whatever the inputs' contents and signs.
    assert_eq!(z(&[-6, 6]).gcd(&z(&[4, -8, 4])), z(&[1, -1]));
    assert_eq!(a.gcd(&z(&[])), a);
    assert_eq!(z(&[]).gcd(&z(&[])), z(&[]));
    // Coprime: the gcd is 1.
    assert_eq!(z(&[1, 0, 1]).gcd(&z(&[1, -1])), z(&[1]));
}

#[test]
fn the_square_free_part_keeps_each_root_once() {
    // (x − 1)²(x + 2) → (x − 1)(x + 2) = x² + x − 2.
    assert_eq!(z(&[1, 0, -3, 2]).square_free(), z(&[1, 1, -2]));
}

#[test]
fn a_repeated_irreducible_quadratic_is_kept_once() {
    // (x² + 1)²(x − 2) = x⁵ − 2x⁴ + 2x³ − 4x² + x − 2 → (x² + 1)(x − 2).
    let p = z(&[1, -2, 2, -4, 1, -2]);
    assert_eq!(p.square_free(), z(&[1, -2, 1, -2]));
    // With a content and a negative sign it is still primitive and positive.
    assert_eq!((&z(&[-3]) * &p).square_free(), z(&[1, -2, 1, -2]));
}

#[test]
fn the_triangles_charpoly_is_square_free_as_x_squared_minus_3x() {
    // L₀ of the triangle: x³ − 6x² + 9x = x(x − 3)².
    assert_eq!(z(&[1, -6, 9, 0]).square_free(), z(&[1, -3, 0]));
}

// --- Law 3: real-root isolation ----------------------------------------------

#[test]
fn the_triangles_roots_isolate_to_0_and_3() {
    let roots = z(&[1, -6, 9, 0]).real_roots();
    assert_eq!(roots.polynomial(), &z(&[1, -3, 0]));
    assert_eq!(roots.len(), 2);
    let intervals = roots.intervals();
    // Disjoint, increasing, one root each: 0 in the first, 3 in the second.
    assert!(intervals[0].contains(&q(0, 1)));
    assert!(intervals[1].contains(&q(3, 1)));
    assert!(intervals[0].hi() <= intervals[1].lo());
    // Exactly: the Cauchy bound 1 + 3 = 4, bisected once at 0.
    let ends: Vec<_> = intervals.iter().map(|i| (i.lo(), i.hi())).collect();
    assert_eq!(
        ends,
        vec![(&q(-4, 1), &q(0, 1)), (&q(0, 1), &q(4, 1))],
        "(−4, 0] and (0, 4]"
    );
    assert_eq!(
        roots.index_of(&q(3, 1)),
        Some(1),
        "3 is the root of index 1"
    );
    assert_eq!(roots.index_of(&q(0, 1)), Some(0));
    assert_eq!(roots.index_of(&q(1, 1)), None, "1 isn't a root");
}

#[test]
fn isolation_separates_close_and_irrational_roots() {
    // (x² − 2)(x − 1)(x − 3/2)·2 = (x² − 2)(2x² − 5x + 3). In increasing
    // order: −√2, 1, √2 ≈ 1.414, 3/2; √2 and 3/2 are 0.09 apart.
    let p = &z(&[1, 0, -2]) * &z(&[2, -5, 3]);
    let roots = p.real_roots();
    assert_eq!(roots.len(), 4);
    let iv = roots.intervals();
    for w in iv.windows(2) {
        assert!(w[0].hi() <= w[1].lo(), "disjoint and increasing");
    }
    assert!(iv[0].lo() < &q(-1414, 1000) && iv[0].hi() > &q(-1415, 1000));
    assert!(iv[1].contains(&q(1, 1)));
    // √2 lies in the third; 3/2 in the fourth.
    assert!(iv[2].lo() < &q(1415, 1000) && iv[2].hi() >= &q(1414, 1000));
    assert!(iv[3].contains(&q(3, 2)));
    assert_eq!(roots.index_of(&q(3, 2)), Some(3));
    // And every interval holds exactly one root.
    for i in iv {
        assert_eq!(roots.count_in(i.lo(), i.hi()), 1);
    }
}

#[test]
fn polynomials_without_real_roots_have_none() {
    assert_eq!(z(&[1, 0, 1]).real_roots().len(), 0);
    assert_eq!(z(&[5]).real_roots().len(), 0);
    assert!(z(&[1, 0, 1]).real_roots().is_empty());
}

// --- Law 4: matching a measurement, or refusing -------------------------------

#[test]
fn a_measured_eigenvalue_is_matched_to_its_root() {
    let roots = z(&[1, -6, 9, 0]).real_roots();
    // Measured floats near 0 and 3, as dsyev returns them.
    assert_eq!(roots.locate(-2.0e-16, 1e-9), Located::Root(0));
    assert_eq!(roots.locate(2.9999999999999996, 1e-9), Located::Root(1));
    // A window that holds a root on its edge still holds it.
    assert_eq!(roots.locate(2.5, 0.5), Located::Root(1));
}

#[test]
fn an_ambiguous_match_refuses() {
    // The window [−0.5, 3.5] is wider than the separation 3: both roots are
    // in it, and the match refuses, naming both rather than guessing.
    let roots = z(&[1, -6, 9, 0]).real_roots();
    assert_eq!(roots.locate(1.5, 2.0), Located::Ambiguous(0..2));
    // Close roots under a coarse reading: 1 and 3/2 within 0.3 of 1.25.
    let close = (&z(&[1, 0, -2]) * &z(&[2, -5, 3])).real_roots();
    assert_eq!(close.locate(1.25, 0.3), Located::Ambiguous(1..4));
}

#[test]
fn a_measurement_near_no_root_is_absent_and_a_non_finite_one_unmeasured() {
    let roots = z(&[1, -6, 9, 0]).real_roots();
    assert_eq!(roots.locate(1.5, 0.1), Located::Absent);
    assert_eq!(roots.locate(f64::NAN, 0.1), Located::Unmeasured);
    assert_eq!(roots.locate(1.0, f64::INFINITY), Located::Unmeasured);
    assert_eq!(roots.locate(1.0, -1.0), Located::Unmeasured);
}

// --- Against room: the triangle's measured modes -------------------------------

#[cfg(feature = "lapack")]
mod against_room {
    use super::*;
    use prismqueer::room::{coboundary, laplacian};
    use prismqueer::spectral::graph_from_edges;

    #[test]
    fn the_triangles_modes_certify_against_their_exact_roots() {
        let g = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
        let l0 = laplacian(g.multigraph());
        let p = l0.charpoly().poly();
        assert_eq!(p, z(&[1, -6, 9, 0]));
        let roots = p.real_roots();
        let modes = l0.measure().decompose().expect("measured");
        let located: Vec<Located> = modes
            .spectrum()
            .iter()
            .map(|&x| roots.locate(x, 1e-9))
            .collect();
        // 0 once, 3 twice: the index is the exact root's, not the mode's.
        assert_eq!(
            located,
            vec![Located::Root(0), Located::Root(1), Located::Root(1)]
        );
        // δ's adjoint composes too: the charpoly through Map agrees.
        let delta = coboundary(g.multigraph());
        let l = delta.then(&delta.adjoint()).expect("C⁰ → C¹ → C⁰");
        assert_eq!(l.symmetric().expect("L₀").charpoly().poly(), p);
    }
}

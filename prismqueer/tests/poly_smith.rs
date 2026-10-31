//! The Smith form of xI − A over ℚ[x]: the invariant factors of an integer
//! matrix, its complete similarity invariant, and its rational canonical
//! form.
//!
//! Laws:
//! 1. The invariant factors are monic, each dividing the next, and their
//!    product is the characteristic polynomial.
//! 2. They tell apart what the charpoly can't: the shear [[1, 1], [0, 1]]
//!    and I₂ share (x − 1)², but not (1, (x − 1)²) and (x − 1, x − 1).
//! 3. They are a similarity invariant: P·A·P⁻¹ for a unimodular P, and a
//!    permutation's relabelling, keep them.
//! 4. The rational canonical form is built from them, and has them.
//!
//! Expected factors are worked by hand from the Jordan structure.

use num_bigint::BigInt;
use num_rational::BigRational;
use prismqueer::poly::{Invariants, NotSquare, QPoly};

/// An integer matrix from rows.
fn m(rows: &[&[i64]]) -> Vec<Vec<BigInt>> {
    rows.iter()
        .map(|r| r.iter().map(|&x| BigInt::from(x)).collect())
        .collect()
}

/// A ℚ[x] polynomial with integer coefficients, leading first.
fn p(coefficients: &[i64]) -> QPoly {
    QPoly::from_descending(
        coefficients
            .iter()
            .map(|&c| BigRational::from_integer(BigInt::from(c))),
    )
}

fn mul(a: &[Vec<BigInt>], b: &[Vec<BigInt>]) -> Vec<Vec<BigInt>> {
    (0..a.len())
        .map(|i| {
            (0..b[0].len())
                .map(|j| (0..b.len()).map(|k| &a[i][k] * &b[k][j]).sum())
                .collect()
        })
        .collect()
}

/// The invariant factors of `a`, checked monic and each dividing the next.
fn factors(a: &[Vec<BigInt>]) -> Vec<QPoly> {
    let inv = Invariants::of(a).expect("square");
    let f = inv.factors().to_vec();
    assert_eq!(f.len(), a.len(), "one factor per row");
    for d in &f {
        assert_eq!(d.leading(), Some(&BigRational::from_integer(1.into())));
    }
    for w in f.windows(2) {
        let (_, r) = w[1].div_rem(&w[0]).expect("nonzero");
        assert!(r.is_zero(), "{:?} divides {:?}", w[0], w[1]);
    }
    f
}

/// The charpoly det(xI − A), by cofactor expansion over ℚ[x]: independent
/// of the Smith form.
fn charpoly(a: &[Vec<BigInt>]) -> QPoly {
    let n = a.len();
    let rows: Vec<Vec<QPoly>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let x = if i == j { p(&[1, 0]) } else { p(&[0]) };
                    &x - &QPoly::constant(BigRational::from_integer(a[i][j].clone()))
                })
                .collect()
        })
        .collect();
    poly_det(&rows)
}

/// A determinant over ℚ[x], by cofactor expansion along the first row.
fn poly_det(rows: &[Vec<QPoly>]) -> QPoly {
    let n = rows.len();
    if n == 0 {
        return p(&[1]);
    }
    let mut det = p(&[0]);
    for j in 0..n {
        let minor: Vec<Vec<QPoly>> = rows[1..]
            .iter()
            .map(|r| (0..n).filter(|&k| k != j).map(|k| r[k].clone()).collect())
            .collect();
        let term = &rows[0][j] * &poly_det(&minor);
        det = if j % 2 == 0 {
            &det + &term
        } else {
            &det - &term
        };
    }
    det
}

fn shear() -> Vec<Vec<BigInt>> {
    m(&[&[1, 1], &[0, 1]])
}

// --- Law 2: the shear --------------------------------------------------------

#[test]
fn the_shear_and_the_identity_share_a_charpoly_but_not_a_smith_form() {
    let identity = m(&[&[1, 0], &[0, 1]]);
    assert_eq!(charpoly(&shear()), charpoly(&identity), "(x − 1)² both");
    assert_eq!(factors(&shear()), vec![p(&[1]), p(&[1, -2, 1])]);
    assert_eq!(factors(&identity), vec![p(&[1, -1]), p(&[1, -1])]);
}

#[test]
fn the_shear_beside_a_second_eigenvalue_still_differs_from_the_identity() {
    // shear ⊕ [2] and I₂ ⊕ [2]: both (x − 1)²(x − 2). Here the pivots aren't
    // all units, so the factors only divide each other after the
    // divisibility step.
    let a = m(&[&[1, 1, 0], &[0, 1, 0], &[0, 0, 2]]);
    let b = m(&[&[1, 0, 0], &[0, 1, 0], &[0, 0, 2]]);
    assert_eq!(charpoly(&a), charpoly(&b));
    // (x − 1)²(x − 2) = x³ − 4x² + 5x − 2; (x − 1)(x − 2) = x² − 3x + 2.
    assert_eq!(factors(&a), vec![p(&[1]), p(&[1]), p(&[1, -4, 5, -2])]);
    assert_eq!(factors(&b), vec![p(&[1]), p(&[1, -1]), p(&[1, -3, 2])]);
}

#[test]
fn distinct_eigenvalues_give_one_nontrivial_factor() {
    // diag(1, 2, 3): cyclic, so (1, 1, (x − 1)(x − 2)(x − 3)).
    let a = m(&[&[1, 0, 0], &[0, 2, 0], &[0, 0, 3]]);
    assert_eq!(factors(&a), vec![p(&[1]), p(&[1]), p(&[1, -6, 11, -6])]);
    let inv = Invariants::of(&a).expect("square");
    assert_eq!(inv.nontrivial(), &[p(&[1, -6, 11, -6])]);
}

// --- Law 3: similarity invariance ---------------------------------------------

#[test]
fn a_permutation_similar_pair_has_one_smith_form() {
    // Swap coordinates 0 and 2: (PAPᵀ)ᵢⱼ = A_σ(i)σ(j).
    let a = m(&[&[1, 1, 0], &[0, 1, 0], &[0, 0, 2]]);
    let relabelled = m(&[&[2, 0, 0], &[0, 1, 0], &[0, 1, 1]]);
    assert_eq!(factors(&a), factors(&relabelled));
}

#[test]
fn conjugating_by_a_unimodular_matrix_keeps_the_invariant_factors() {
    let pm = m(&[&[1, 2, 0], &[0, 1, 3], &[0, 0, 1]]);
    let pinv = m(&[&[1, -2, 6], &[0, 1, -3], &[0, 0, 1]]);
    assert_eq!(mul(&pm, &pinv), m(&[&[1, 0, 0], &[0, 1, 0], &[0, 0, 1]]));
    let cases = [
        m(&[&[1, 1, 0], &[0, 1, 0], &[0, 0, 2]]),
        m(&[&[1, 0, 0], &[0, 1, 0], &[0, 0, 2]]),
        m(&[&[2, -1, 0], &[3, 1, 4], &[0, 5, -2]]),
        m(&[&[3, 0, 0], &[0, 3, 0], &[0, 0, 3]]),
    ];
    for a in &cases {
        let conjugate = mul(&mul(&pm, a), &pinv);
        assert_ne!(&conjugate, a, "a genuine change of basis");
        assert_eq!(factors(&conjugate), factors(a), "{a:?}");
    }
    // 3I is scalar: (x − 3, x − 3, x − 3).
    assert_eq!(factors(&cases[3]), vec![p(&[1, -3]); 3]);
}

// --- Law 1: the product is the charpoly ----------------------------------------

#[test]
fn the_product_of_the_invariant_factors_is_the_charpoly() {
    let cases = [
        shear(),
        m(&[&[2, -1, 0], &[3, 1, 4], &[0, 5, -2]]),
        m(&[&[0, 1, 0, 0], &[0, 0, 1, 0], &[0, 0, 0, 1], &[-1, 0, 0, 0]]),
        m(&[&[5]]),
    ];
    for a in &cases {
        let inv = Invariants::of(a).expect("square");
        assert_eq!(inv.product(), charpoly(a), "{a:?}");
    }
}

#[test]
fn the_empty_matrix_has_no_factors_and_a_ragged_one_isnt_square() {
    let inv = Invariants::of(&[]).expect("0 × 0 is square");
    assert!(inv.factors().is_empty());
    assert_eq!(inv.product(), p(&[1]));
    assert!(inv.rational_canonical().is_empty());
    assert_eq!(Invariants::of(&m(&[&[1, 2], &[3]])), Err(NotSquare));
    assert_eq!(Invariants::of(&m(&[&[1, 2]])), Err(NotSquare));
}

// --- Law 4: the rational canonical form ---------------------------------------

#[test]
fn the_rational_canonical_form_is_the_companions_of_the_factors() {
    let q = |x: i64| BigRational::from_integer(BigInt::from(x));
    // (x − 1)² = x² − 2x + 1: companion [[0, −1], [1, 2]].
    let shear_form = Invariants::of(&shear())
        .expect("square")
        .rational_canonical();
    assert_eq!(shear_form, vec![vec![q(0), q(-1)], vec![q(1), q(2)]]);
    let identity = m(&[&[1, 0], &[0, 1]]);
    let identity_form = Invariants::of(&identity)
        .expect("square")
        .rational_canonical();
    assert_eq!(identity_form, vec![vec![q(1), q(0)], vec![q(0), q(1)]]);
    // And the form has the same invariant factors as the matrix it came from.
    for a in [
        shear(),
        m(&[&[1, 1, 0], &[0, 1, 0], &[0, 0, 2]]),
        m(&[&[2, -1, 0], &[3, 1, 4], &[0, 5, -2]]),
    ] {
        let inv = Invariants::of(&a).expect("square");
        let again = Invariants::of_rational(&inv.rational_canonical()).expect("square");
        assert_eq!(again, inv);
    }
}

#[test]
fn rational_entries_are_accepted() {
    // A = [[1/2, 1], [0, 1/2]]: a shear at 1/2, so (1, (x − 1/2)²).
    let h = BigRational::new(1.into(), 2.into());
    let one = BigRational::from_integer(1.into());
    let zero = BigRational::from_integer(0.into());
    let a = vec![vec![h.clone(), one.clone()], vec![zero, h]];
    let inv = Invariants::of_rational(&a).expect("square");
    let quarter = BigRational::new(1.into(), 4.into());
    assert_eq!(
        inv.factors(),
        &[
            p(&[1]),
            QPoly::from_descending([one.clone(), -one, quarter])
        ]
    );
}

// --- Against room: integer maps and their exact charpoly ----------------------

#[cfg(feature = "lapack")]
mod against_room {
    use super::*;
    use prismqueer::room::{dirac, laplacian};
    use prismqueer::spectral::graph_from_edges;

    #[test]
    fn the_product_agrees_with_rooms_charpoly() {
        let triangle = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
        let k4 = graph_from_edges(&[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
        for g in [&triangle, &k4] {
            let l0 = laplacian(g.multigraph());
            let inv = l0.invariant_factors();
            assert_eq!(inv.product(), l0.charpoly().poly().to_rational());
            let d = dirac(g.multigraph());
            let inv = d.invariant_factors();
            let p = d.symmetric().expect("D = D*").charpoly().poly();
            assert_eq!(inv.product(), p.to_rational());
        }
        // L₀ of the triangle is diagonalisable with eigenvalues 0, 3, 3: its
        // minimal polynomial is x(x − 3), so (1, x − 3, x² − 3x).
        let inv = laplacian(triangle.multigraph()).invariant_factors();
        assert_eq!(inv.factors(), &[p(&[1]), p(&[1, -3]), p(&[1, -3, 0])]);
    }
}

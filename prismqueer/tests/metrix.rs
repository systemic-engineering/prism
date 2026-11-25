//! `Metrix` and `Form` (prism-021 D5; Mara 54c88f72 §8–§9, probe c012e29f
//! P11).
//!
//! A `Metrix` is a frame's ruler for length: symmetric positive definite
//! over exact ℚ, certified by an exact LDLᵀ (every pivot of D > 0), and
//! sealed by a signer beside its address. No default, the Euclidean
//! identity included. A `Form` is a symmetric matrix of any signature,
//! whose inertia (n₊, n₋, n₀) is computed by Sylvester's law through an
//! LDLᵀ with symmetric pivoting, never by leading minors.
//!
//! Isometry defects Tᵀ·G_B·T − G_A are `Form`s; relative spectra of two
//! `Metrix` on one frame are exact algebraic multisets.

#![cfg(all(feature = "exact", feature = "bundle"))]

use std::collections::BTreeSet;
use std::marker::PhantomData;

use prismqueer::flux::pillar::{forall, Arbitrary, Sample};
use prismqueer::gl::Gl;
use prismqueer::hypothesis::Frame;
use prismqueer::metrix::{
    isometry_defect, relative_spectrum, Form, Inertia, Metrix, MetrixDraft, MetrixError,
};
use prismqueer::oid::Oid;
use prismqueer::poly::{Algebraic, Budget, ZPoly};
use prismqueer::room::{Q, Z};
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

fn ints(m: &[&[i64]]) -> Vec<Vec<Q>> {
    m.iter()
        .map(|r| r.iter().map(|&n| Q::from(n)).collect())
        .collect()
}

fn frame(name: &str, dim: usize) -> Frame {
    Frame::new(Oid::new(name), dim)
}

fn signer() -> Oid {
    Oid::new("engineer")
}

fn metrix(f: &Frame, m: &[&[i64]]) -> Metrix {
    MetrixDraft::new(f, ints(m))
        .expect("positive definite")
        .seal(signer())
}

fn form(m: &[&[i64]]) -> Form {
    Form::new(ints(m)).expect("symmetric")
}

fn inertia(positive: usize, negative: usize, zero: usize) -> Inertia {
    Inertia {
        positive,
        negative,
        zero,
    }
}

fn budget() -> Budget {
    Budget(1 << 12)
}

/// The algebraic number with this minimal polynomial (leading first) and
/// index among its real roots.
fn alg(descending: &[i64], index: usize) -> Algebraic {
    Algebraic::new(
        ZPoly::from_descending(descending.iter().map(|&c| c.into())),
        index,
    )
}

// --- Certification and refusal -----------------------------------------------

/// [[2,1],[1,2]] certifies: L = [[1,0],[1/2,1]], D = (2, 3/2), and the
/// certificate re-multiplies to G exactly.
#[test]
fn a_definite_matrix_certifies_with_its_ldl() {
    let f = frame("a", 2);
    let draft = MetrixDraft::new(&f, ints(&[&[2, 1], &[1, 2]])).expect("PD");
    let c = draft.certificate();
    assert_eq!(c.d(), &[q(2, 1), q(3, 2)]);
    assert_eq!(c.l(), &rows(&[&[(1, 1), (0, 1)], &[(1, 2), (1, 1)]])[..]);
    assert!(c.verifies(&ints(&[&[2, 1], &[1, 2]])));
    assert!(!c.verifies(&ints(&[&[2, 1], &[1, 3]])));
    let g = draft.seal(signer());
    assert_eq!(g.rows(), &ints(&[&[2, 1], &[1, 2]])[..]);
    assert_eq!(g.frame(), f.oid());
    assert_eq!(g.dim(), 2);
    assert!(g.certificate().verifies(g.rows()));
}

/// [[1,2],[2,1]]: d₁ = 1, d₂ = 1 − 4 = −3. Refused at pivot 2, never
/// clipped into the cone.
#[test]
fn an_indefinite_matrix_is_refused_at_its_pivot() {
    let f = frame("a", 2);
    assert_eq!(
        MetrixDraft::new(&f, ints(&[&[1, 2], &[2, 1]])).err(),
        Some(MetrixError::NotDefinite {
            pivot: 2,
            value: q(-3, 1)
        })
    );
    assert_eq!(
        MetrixDraft::new(&f, ints(&[&[-1, 0], &[0, 1]])).err(),
        Some(MetrixError::NotDefinite {
            pivot: 1,
            value: q(-1, 1)
        })
    );
}

/// A zero pivot is semidefinite, not definite: [[1,1],[1,1]] has d₂ = 0,
/// and [[0,0],[0,−1]] (Mara P11a, passes leading minors ≥ 0) has d₁ = 0.
#[test]
fn a_zero_pivot_is_not_definite() {
    let f = frame("a", 2);
    assert_eq!(
        MetrixDraft::new(&f, ints(&[&[1, 1], &[1, 1]])).err(),
        Some(MetrixError::NotDefinite {
            pivot: 2,
            value: q(0, 1)
        })
    );
    assert_eq!(
        MetrixDraft::new(&f, ints(&[&[0, 0], &[0, -1]])).err(),
        Some(MetrixError::NotDefinite {
            pivot: 1,
            value: q(0, 1)
        })
    );
    let g3 = frame("b", 3);
    assert_eq!(
        MetrixDraft::new(&g3, ints(&[&[1, 0, 0], &[0, 1, 0], &[0, 0, 0]])).err(),
        Some(MetrixError::NotDefinite {
            pivot: 3,
            value: q(0, 1)
        })
    );
}

#[test]
fn a_non_symmetric_matrix_is_refused() {
    let f = frame("a", 2);
    assert_eq!(
        MetrixDraft::new(&f, ints(&[&[2, 1], &[0, 2]])).err(),
        Some(MetrixError::NotSymmetric { row: 1, col: 0 })
    );
    assert_eq!(
        Form::new(ints(&[&[0, 1], &[-1, 0]])).err(),
        Some(MetrixError::NotSymmetric { row: 1, col: 0 })
    );
}

#[test]
fn a_matrix_of_the_wrong_size_is_refused() {
    let f = frame("a", 2);
    assert_eq!(
        MetrixDraft::new(&f, ints(&[&[1]])).err(),
        Some(MetrixError::WrongSize {
            expected: 2,
            found: 1
        })
    );
    assert_eq!(
        MetrixDraft::new(&f, ints(&[&[1, 0], &[0]])).err(),
        Some(MetrixError::NotSquare)
    );
    assert_eq!(
        Form::new(ints(&[&[1, 0]])).err(),
        Some(MetrixError::NotSquare)
    );
}

/// Over ℤ the draft reads the integers in ℚ: a lattice's Gram matrix is a
/// `Metrix` like any other.
#[test]
fn integer_entries_certify_over_q() {
    let f = frame("a", 2);
    let z = vec![
        vec![
            Z::from(num_bigint::BigInt::from(2)),
            Z::from(num_bigint::BigInt::from(1)),
        ],
        vec![
            Z::from(num_bigint::BigInt::from(1)),
            Z::from(num_bigint::BigInt::from(2)),
        ],
    ];
    let a = MetrixDraft::new(&f, z).expect("PD").seal(signer());
    let b = metrix(&f, &[&[2, 1], &[1, 2]]);
    assert_eq!(a.oid(), b.oid());
}

/// The Euclidean identity is a judgment too: it exists only as a declared
/// draft, sealed by someone.
#[test]
fn the_identity_is_accepted_only_through_a_signed_draft() {
    let f = frame("a", 3);
    let declared = MetrixDraft::identity(&f);
    assert_eq!(declared.certificate().d(), &[q(1, 1), q(1, 1), q(1, 1)]);
    let i = declared.seal(Oid::new("euclid"));
    assert_eq!(i.signer().by(), &Oid::new("euclid"));
    assert_eq!(i.signer().over(), &i.oid());
    assert_eq!(
        i.oid(),
        metrix(&f, &[&[1, 0, 0], &[0, 1, 0], &[0, 0, 1]]).oid()
    );
}

/// The signer sits beside the address: one judgment, two signers, one
/// address. The address names the frame (its basis): one matrix on two
/// frames is two addresses.
#[test]
fn the_address_names_the_frame_and_not_the_signer() {
    let (f, g) = (frame("a", 2), frame("b", 2));
    let m: &[&[i64]] = &[&[2, 1], &[1, 2]];
    let one = MetrixDraft::new(&f, ints(m)).expect("PD");
    let by_a = one.clone().seal(Oid::new("alice"));
    let by_b = one.seal(Oid::new("bob"));
    assert_eq!(by_a.oid(), by_b.oid());
    assert_ne!(by_a.signer(), by_b.signer());
    assert_ne!(by_a.oid(), metrix(&g, m).oid());
    assert_ne!(by_a.oid(), metrix(&f, &[&[3, 1], &[1, 2]]).oid());
}

/// Autoref probe: does `T` implement `Default`? Resolved at compile time on
/// a concrete type, read at run time.
struct Probe<T>(PhantomData<T>);
trait HasDefault {
    fn has_default(&self) -> bool {
        true
    }
}
impl<T: Default> HasDefault for Probe<T> {}
trait LacksDefault {
    fn has_default(&self) -> bool {
        false
    }
}
impl<T> LacksDefault for &Probe<T> {}

#[test]
fn there_is_no_default_metrix() {
    assert!(!(&Probe::<Metrix>(PhantomData)).has_default());
    assert!(!(&Probe::<MetrixDraft>(PhantomData)).has_default());
    assert!(
        Probe::<Vec<Q>>(PhantomData).has_default(),
        "the probe works"
    );
}

// --- Inertia of a Form -------------------------------------------------------

#[test]
fn inertia_of_two_by_two_forms() {
    assert_eq!(form(&[&[1, 0], &[0, -1]]).inertia(), inertia(1, 1, 0));
    // Leading minor 0: needs a symmetric (2×2) pivot.
    assert_eq!(form(&[&[0, 1], &[1, 0]]).inertia(), inertia(1, 1, 0));
    assert_eq!(form(&[&[1, 1], &[1, 1]]).inertia(), inertia(1, 0, 1));
    assert_eq!(form(&[&[0, 0], &[0, -1]]).inertia(), inertia(0, 1, 1));
    assert_eq!(form(&[&[0, 0], &[0, 0]]).inertia(), inertia(0, 0, 2));
    assert_eq!(form(&[&[2, 1], &[1, 2]]).inertia(), inertia(2, 0, 0));
}

#[test]
fn inertia_of_diagonal_forms_counts_signs() {
    assert_eq!(
        form(&[&[3, 0, 0, 0], &[0, -2, 0, 0], &[0, 0, 0, 0], &[0, 0, 0, 5]]).inertia(),
        inertia(2, 1, 1)
    );
    assert_eq!(
        Form::new(Vec::<Vec<Q>>::new()).expect("empty").inertia(),
        inertia(0, 0, 0)
    );
}

#[test]
fn inertia_of_three_by_three_forms() {
    // Zero diagonal throughout: pivots are all 2×2 or the remaining zero.
    assert_eq!(
        form(&[&[0, 1, 1], &[1, 0, 1], &[1, 1, 0]]).inertia(),
        inertia(1, 2, 0)
    );
    // Leading 2×2 minor 0 but not the whole: [[0,1,0],[1,0,0],[0,0,−3]].
    assert_eq!(
        form(&[&[0, 1, 0], &[1, 0, 0], &[0, 0, -3]]).inertia(),
        inertia(1, 2, 0)
    );
    // Rank 1: v·vᵀ with v = (1, 2, 3).
    assert_eq!(
        form(&[&[1, 2, 3], &[2, 4, 6], &[3, 6, 9]]).inertia(),
        inertia(1, 0, 2)
    );
    // The Krein grading diag(I₂, −I₁) (Mara §8.2, `Map::grading`).
    assert_eq!(
        form(&[&[1, 0, 0], &[0, 1, 0], &[0, 0, -1]]).inertia(),
        inertia(2, 1, 0)
    );
    // Zero first pivot, nonzero later diagonal: [[0,1,0],[1,2,1],[0,1,0]].
    assert_eq!(
        form(&[&[0, 1, 0], &[1, 2, 1], &[0, 1, 0]]).inertia(),
        inertia(1, 1, 1)
    );
}

#[test]
fn a_metrix_is_a_positive_definite_form() {
    let f = frame("a", 2);
    let g = metrix(&f, &[&[2, 1], &[1, 2]]);
    let as_form = Form::of(&g);
    assert_eq!(as_form.inertia(), inertia(2, 0, 0));
    assert_eq!(as_form.rows(), g.rows());
}

#[test]
fn a_difference_of_metrics_is_a_form_on_one_frame() {
    let f = frame("a", 2);
    let w1 = metrix(&f, &[&[2, 0], &[0, 1]]);
    let w2 = metrix(&f, &[&[1, 0], &[0, 3]]);
    let d = Form::difference(&w2, &w1).expect("one frame");
    assert_eq!(d.rows(), &ints(&[&[-1, 0], &[0, 2]])[..]);
    assert_eq!(d.inertia(), inertia(1, 1, 0));
    let other = metrix(&frame("b", 2), &[&[1, 0], &[0, 1]]);
    assert!(matches!(
        Form::difference(&w2, &other),
        Err(MetrixError::OtherFrame { .. })
    ));
}

// --- Sylvester's law under congruence -----------------------------------------

/// A symmetric n×n form with small entries (often singular, often with a
/// zero diagonal) and an invertible P over ℚ, both drawn.
#[derive(Clone, Debug)]
struct Congruence {
    f: Vec<Vec<Q>>,
    p: Gl<Q>,
    n: usize,
}

struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A small integer in −k..=k, zero one time in three.
    fn small(&mut self, k: i64) -> i64 {
        if self.next().is_multiple_of(3) {
            0
        } else {
            (self.next() % (2 * k as u64 + 1)) as i64 - k
        }
    }
}

impl Arbitrary for Congruence {
    fn arbitrary(sample: &mut Sample) -> Self {
        let mut mix = Mix(u64::arbitrary(sample));
        let n = 1 + (mix.next() % 4) as usize;
        // Drawn row by row; the upper triangle is read on both sides.
        let drawn: Vec<Vec<i64>> = (0..n)
            .map(|_| (0..n).map(|_| mix.small(3)).collect())
            .collect();
        let f = (0..n)
            .map(|i| (0..n).map(|j| Q::from(drawn[i.min(j)][i.max(j)])).collect())
            .collect();
        let p = loop {
            let rows = (0..n)
                .map(|_| {
                    (0..n)
                        .map(|_| q(mix.small(4), 1 + (mix.next() % 3) as i64))
                        .collect()
                })
                .collect();
            if let Ok(p) = Gl::new(rows) {
                break p;
            }
        };
        Congruence { f, p, n }
    }
}

fn check(holds: bool, what: &str) -> PropertyVerdict {
    if holds {
        PropertyVerdict::Pass
    } else {
        PropertyVerdict::Fail(Diagnostic::new(what.to_owned()))
    }
}

fn passes(v: PropertyVerdict) {
    assert!(matches!(v, PropertyVerdict::Pass), "{v:?}");
}

/// Sylvester's law of inertia: inertia(PᵀFP) = inertia(F) for invertible P.
#[test]
fn sylvester_inertia_is_invariant_under_congruence() {
    passes(forall::<Congruence, _>(b"metrix sylvester", 128, |c| {
        let f = Form::new(c.f.clone()).expect("symmetric");
        let Ok(moved) = f.congruent(&c.p) else {
            return check(false, "congruence refused");
        };
        let i = f.inertia();
        check(
            moved.inertia() == i && i.positive + i.negative + i.zero == c.n,
            "inertia moved under congruence",
        )
    }));
}

// --- Isometry defect ---------------------------------------------------------

/// The 3-4-5 rotation with G_A = G_B = I: RᵀIR − I = 0.
#[test]
fn a_rotation_is_an_isometry_of_the_identity() {
    let (a, b) = (frame("a", 2), frame("b", 2));
    let (ia, ib) = (
        MetrixDraft::identity(&a).seal(signer()),
        MetrixDraft::identity(&b).seal(signer()),
    );
    let r = Gl::new(rows(&[&[(3, 5), (-4, 5)], &[(4, 5), (3, 5)]])).expect("rotation");
    let d = isometry_defect(&r, &ia, &ib).expect("sizes agree");
    assert!(d.is_zero());
    assert_eq!(d.inertia(), inertia(0, 0, 2));
}

/// The shear S = [[1,1],[0,1]] with I: SᵀS − I = [[0,1],[1,1]], det −1,
/// inertia (1, 1, 0). Not an isometry, though its trace is the identity's.
#[test]
fn a_shear_has_an_indefinite_defect() {
    let (a, b) = (frame("a", 2), frame("b", 2));
    let (ia, ib) = (
        MetrixDraft::identity(&a).seal(signer()),
        MetrixDraft::identity(&b).seal(signer()),
    );
    let s = Gl::new(ints(&[&[1, 1], &[0, 1]])).expect("shear");
    let d = isometry_defect(&s, &ia, &ib).expect("sizes agree");
    assert!(!d.is_zero());
    assert_eq!(d.rows(), &ints(&[&[0, 1], &[1, 1]])[..]);
    assert_eq!(d.inertia(), inertia(1, 1, 0));
}

/// Orientation (Mara §8.1): T reads A into B, so the defect is
/// Tᵀ·G_B·T − G_A. With T = diag(2, 1), G_A = diag(4, 1), G_B = I it is
/// zero. Swapping the two rulers is a different question, with a
/// different answer.
#[test]
fn the_defect_pulls_g_b_back_along_t() {
    let (a, b) = (frame("a", 2), frame("b", 2));
    let ga = metrix(&a, &[&[4, 0], &[0, 1]]);
    let gb = MetrixDraft::identity(&b).seal(signer());
    let t = Gl::new(ints(&[&[2, 0], &[0, 1]])).expect("T");
    assert!(isometry_defect(&t, &ga, &gb).expect("sizes").is_zero());
    // Swapped: Tᵀ·diag(4, 1)·T − I = diag(16 − 1, 1 − 1) ≠ 0.
    let back = isometry_defect(&t, &gb, &ga).expect("sizes");
    assert_eq!(back.rows(), &ints(&[&[15, 0], &[0, 0]])[..]);
    assert_eq!(back.inertia(), inertia(1, 0, 1));
}

/// Mara P11j: an indefinite defect with eigenvalues 3 and −3/4.
/// T = [[2,0],[0,1/2]], G_A = G_B = I: TᵀT − I = diag(3, −3/4).
#[test]
fn mara_p11j_defect_is_indefinite() {
    let (a, b) = (frame("a", 2), frame("b", 2));
    let (ia, ib) = (
        MetrixDraft::identity(&a).seal(signer()),
        MetrixDraft::identity(&b).seal(signer()),
    );
    let t = Gl::new(rows(&[&[(2, 1), (0, 1)], &[(0, 1), (1, 2)]])).expect("T");
    let d = isometry_defect(&t, &ia, &ib).expect("sizes");
    assert_eq!(
        d.rows(),
        &rows(&[&[(3, 1), (0, 1)], &[(0, 1), (-3, 4)]])[..]
    );
    assert_eq!(d.inertia(), inertia(1, 1, 0));
}

#[test]
fn a_defect_across_sizes_is_refused() {
    let (a, b) = (frame("a", 2), frame("b", 3));
    let (ia, ib) = (
        MetrixDraft::identity(&a).seal(signer()),
        MetrixDraft::identity(&b).seal(signer()),
    );
    let s = Gl::new(ints(&[&[1, 1], &[0, 1]])).expect("shear");
    assert_eq!(
        isometry_defect(&s, &ia, &ib).err(),
        Some(MetrixError::WrongSize {
            expected: 2,
            found: 3
        })
    );
    let big = Gl::new(ints(&[&[1, 0, 1], &[0, 1, 0], &[0, 0, 1]])).expect("3×3");
    assert_eq!(
        isometry_defect(&big, &ia, &ia).err(),
        Some(MetrixError::WrongSize {
            expected: 2,
            found: 3
        })
    );
}

// --- Relative spectrum -------------------------------------------------------

fn multiset(s: &[(Algebraic, usize)]) -> BTreeSet<(String, usize, usize)> {
    s.iter()
        .map(|(a, m)| (format!("{:?}", a.minimal()), a.index(), *m))
        .collect()
}

/// G₁ = I, G₂ = diag(2, 3): the relative eigenvalues are {2, 3}.
#[test]
fn the_relative_spectrum_of_a_diagonal_pair() {
    let f = frame("a", 2);
    let i = MetrixDraft::identity(&f).seal(signer());
    let g = metrix(&f, &[&[2, 0], &[0, 3]]);
    let s = relative_spectrum(&i, &g, budget()).expect("one frame");
    assert!(s.is_certified());
    assert_eq!(s.charpoly().coefficients(), &[q(1, 1), q(-5, 1), q(6, 1)]);
    assert_eq!(
        multiset(s.eigenvalues()),
        multiset(&[(alg(&[1, -2], 0), 1), (alg(&[1, -3], 0), 1)])
    );
}

/// G₁ = diag(1, 2), G₂ = [[2,1],[1,1]]: G₁⁻¹G₂ = [[2,1],[1/2,1/2]],
/// charpoly x² − 5/2·x + 1/2, i.e. 2x² − 5x + 1 (disc 17): irreducible,
/// two real roots (5 ± √17)/4.
#[test]
fn the_relative_spectrum_of_a_non_diagonal_pair_is_an_irreducible_quadratic() {
    let f = frame("a", 2);
    let g1 = metrix(&f, &[&[1, 0], &[0, 2]]);
    let g2 = metrix(&f, &[&[2, 1], &[1, 1]]);
    let s = relative_spectrum(&g1, &g2, budget()).expect("one frame");
    assert!(s.is_certified());
    assert_eq!(s.charpoly().coefficients(), &[q(1, 1), q(-5, 2), q(1, 2)]);
    assert_eq!(
        multiset(s.eigenvalues()),
        multiset(&[(alg(&[2, -5, 1], 0), 1), (alg(&[2, -5, 1], 1), 1)])
    );
}

/// No budget: diag(1, 2, 3) splits into three lines mod every good prime,
/// so naming any of them takes a recombination trial. With none, nothing is
/// named and the whole square-free charpoly is unresolved; never a guess.
#[test]
fn without_a_budget_the_relative_spectrum_is_unresolved() {
    let f = frame("a", 3);
    let i = MetrixDraft::identity(&f).seal(signer());
    let g = metrix(&f, &[&[1, 0, 0], &[0, 2, 0], &[0, 0, 3]]);
    let s = relative_spectrum(&i, &g, Budget(0)).expect("one frame");
    assert!(!s.is_certified());
    assert!(s.eigenvalues().is_empty());
    // (x − 1)(x − 2)(x − 3).
    assert_eq!(
        s.unresolved(),
        Some(&ZPoly::from_descending([1, -6, 11, -6].map(Into::into)))
    );
    // With a budget, certified, and nothing left unresolved.
    let s = relative_spectrum(&i, &g, budget()).expect("one frame");
    assert_eq!(s.unresolved(), None);
    assert_eq!(s.eigenvalues().len(), 3);
}

/// A form has its size; a P larger than it is refused, not truncated.
#[test]
fn a_congruence_larger_than_the_form_is_refused() {
    let f = form(&[&[2, 1], &[1, 2]]);
    assert_eq!(f.dim(), 2);
    // diag(1, 1, 2): its third axis is past the form's two.
    let p = Gl::new(ints(&[&[1, 0, 0], &[0, 1, 0], &[0, 0, 2]])).expect("invertible");
    assert_eq!(p.size(), 3);
    assert_eq!(
        f.congruent(&p).err(),
        Some(MetrixError::WrongSize {
            expected: 2,
            found: 3
        })
    );
    // A smaller P acts as the identity past its size.
    let two = Gl::new(ints(&[&[2]])).expect("invertible");
    let moved = f.congruent(&two).expect("within the form");
    assert_eq!(moved.dim(), 2);
    assert_eq!(moved.rows(), &ints(&[&[8, 2], &[2, 2]])[..]);
}

/// Seam's ×10 axis (Mara P11d): {1, 1, 100}, with multiplicity.
#[test]
fn a_repeated_relative_eigenvalue_keeps_its_multiplicity() {
    let f = frame("a", 3);
    let i = MetrixDraft::identity(&f).seal(signer());
    let g = metrix(&f, &[&[1, 0, 0], &[0, 1, 0], &[0, 0, 100]]);
    let s = relative_spectrum(&i, &g, budget()).expect("one frame");
    assert_eq!(
        multiset(s.eigenvalues()),
        multiset(&[(alg(&[1, -1], 0), 2), (alg(&[1, -100], 0), 1)])
    );
}

/// Basis-invariant (Mara P11e): a simultaneous congruence P leaves the
/// relative spectrum where it was; per-axis units are such a P.
#[test]
fn the_relative_spectrum_is_invariant_under_simultaneous_congruence() {
    let (f, g) = (frame("a", 2), frame("b", 2));
    let g1 = metrix(&f, &[&[1, 0], &[0, 2]]);
    let g2 = metrix(&f, &[&[2, 1], &[1, 1]]);
    let p = Gl::new(ints(&[&[1, 2], &[0, 3]])).expect("P");
    let moved = |m: &Metrix| {
        let r = Form::of(m).congruent(&p).expect("sizes").rows().to_vec();
        MetrixDraft::new(&g, r).expect("PD stays PD").seal(signer())
    };
    let before = relative_spectrum(&g1, &g2, budget()).expect("one frame");
    let after = relative_spectrum(&moved(&g1), &moved(&g2), budget()).expect("one frame");
    assert_eq!(before.charpoly(), after.charpoly());
    assert_eq!(
        multiset(before.eigenvalues()),
        multiset(after.eigenvalues())
    );
}

/// Two rulers on two frames have no relative spectrum: the eigenvalues of
/// G₁⁻¹G₂ mean something only in one basis.
#[test]
fn a_relative_spectrum_across_frames_is_refused() {
    let i = MetrixDraft::identity(&frame("a", 2)).seal(signer());
    let j = MetrixDraft::identity(&frame("b", 2)).seal(signer());
    assert!(matches!(
        relative_spectrum(&i, &j, budget()),
        Err(MetrixError::OtherFrame { .. })
    ));
}

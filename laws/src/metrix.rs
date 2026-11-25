//! `prism_laws::metrix` — a frame's ruler for length, and the forms that
//! measure how rulers disagree (prism-021 D5; Mara, mirror 54c88f72 §8–§9,
//! probe c012e29f P11). The names are provisional.
//!
//! # `Metrix`: positive definite, certified, signed, no default
//!
//! A [`Metrix`] is the Gram matrix G of an inner product ⟨u, v⟩ = uᵀGv on a
//! frame's stalk, in that frame's basis: a symmetric **positive definite**
//! n×n matrix over exact ℚ. Angles and Cauchy–Schwarz exist only then.
//!
//! - **Certified.** [`MetrixDraft::new`] runs an exact LDLᵀ over ℚ, with no
//!   pivoting: G = L·D·Lᵀ, L unit lower triangular, D diagonal. G is
//!   positive definite iff every pivot of D is > 0. The first pivot that
//!   isn't is the refusal ([`MetrixError::NotDefinite`], its index and
//!   value). The [`Certificate`] (L, D) is kept, and
//!   [`Certificate::verifies`] re-multiplies it exactly.
//! - **Never clipped.** An indefinite or semidefinite input is refused, never
//!   "fixed" into the cone: clipping eigenvalues is averaging toward the
//!   cone (Mara §8.2), a choice someone would have to sign.
//! - **Signed.** Only [`MetrixDraft::seal`] turns a draft into a `Metrix`,
//!   minting its [`Signer`]. The signer sits **beside** the address, never
//!   in it (`docs/spec/ruler.md`): one judgment signed by two people has one
//!   address and two signers.
//! - **In a named basis.** The address is
//!   `"metrix\0" ‖ u64le(len) ‖ frame oid ‖ u64le(n) ‖ rational(gᵢⱼ)…`
//!   (row by row, rationals as `ruler.md` encodes them) in the public scope.
//!   The frame is in the preimage (Mara §8.1): two congruent metrics in two
//!   bases are two addresses, not an accidental collision.
//! - **No default.** The Euclidean identity is a judgment like any other
//!   ("declare the metric", *Whose Metric?* §5): [`MetrixDraft::identity`]
//!   declares it, and someone seals it.
//!
//! ```compile_fail,E0599
//! use prism_laws::metrix::Metrix;
//! let _ = Metrix::default();
//! ```
//!
//! The control: a declared, sealed identity is a `Metrix`.
//!
//! ```
//! use prism_laws::hypothesis::Frame;
//! use prism_laws::metrix::MetrixDraft;
//! use prism_laws::oid::Oid;
//! let euclid = MetrixDraft::identity(&Frame::new(Oid::new("a"), 2)).seal(Oid::new("euclid"));
//! assert_eq!(euclid.signer().over(), &euclid.oid());
//! ```
//!
//! # Not over ℤ/n
//!
//! "Positive" needs an ordered ring, and ℤ/n has no order (Mara §8.1,
//! BROKEN). A draft's entries are [`Ordered`]: ℚ, or ℤ read in ℚ (a
//! lattice's Gram matrix). `ZMod` is not admitted:
//!
//! ```compile_fail,E0277
//! use prism_laws::hypothesis::Frame;
//! use prism_laws::metrix::MetrixDraft;
//! use prism_laws::oid::Oid;
//! use prism_laws::room::{Modulus, ZMod};
//! let five = Modulus::new(5).unwrap();
//! let _ = MetrixDraft::new(&Frame::new(Oid::new("a"), 1), vec![vec![ZMod::new(1, &five)]]);
//! ```
//!
//! The control: the same over ℤ certifies.
//!
//! ```
//! use num_bigint::BigInt;
//! use prism_laws::hypothesis::Frame;
//! use prism_laws::metrix::MetrixDraft;
//! use prism_laws::oid::Oid;
//! use prism_laws::room::Z;
//! let draft = MetrixDraft::new(&Frame::new(Oid::new("a"), 1), vec![vec![Z::from(BigInt::from(1))]]);
//! assert!(draft.is_ok());
//! ```
//!
//! # `Form`: any signature, inertia as data
//!
//! Indefinite symmetric forms occur, and must be held without pretending
//! they are rulers: isometry defects, differences of metrics (window₂ −
//! window₁), `Map::grading`'s Krein form (Mara §8.2). A [`Form`] is a
//! symmetric matrix over ℚ, and its [`Inertia`] (n₊, n₋, n₀) is computed by
//! **Sylvester's law** through an LDLᵀ with **symmetric pivoting**:
//!
//! - a nonzero diagonal entry is a 1×1 pivot, moved to the front by a
//!   symmetric permutation; its sign counts once;
//! - when the whole remaining diagonal is zero but an entry b = aᵢⱼ isn't,
//!   rows and columns i, j form a 2×2 pivot [[0, b], [b, 0]], whose
//!   determinant −b² < 0 counts one positive and one negative;
//! - when everything left is zero, it counts as n₀.
//!
//! Each step is a congruence A ≅ E ⊕ (A/E) (the Schur complement), so the
//! inertias add (Haynsworth), and by Sylvester's law the count is the
//! form's. **Not leading minors**: Sylvester's *criterion* (every leading
//! minor positive) certifies positive definite only. [[0, 1], [1, 0]] has leading
//! minor 0 and inertia (1, 1, 0), and [[0, 0], [0, −1]] has minors ≥ 0 and
//! isn't semidefinite (P11a).
//!
//! A `Form` becomes a `Metrix` only through a draft (certificate) and a seal
//! (signature), never by default and never by clipping.
//!
//! # Isometry defects and relative spectra
//!
//! - [`isometry_defect`]`(T, G_A, G_B)` = Tᵀ·G_B·T − G_A, for T reading
//!   frame A into frame B (Mara §8.1): the pull-back of B's ruler along T,
//!   minus A's. It is zero iff T is an isometry, and indefinite in general
//!   (P11j: eigenvalues 3 and −3/4).
//! - [`relative_spectrum`]`(G₁, G₂)` = the eigenvalues of G₁⁻¹G₂, the roots
//!   of det(G₂ − λG₁), as an exact algebraic multiset: the characteristic
//!   polynomial over ℚ, factored by [`poly`](crate::poly), each root an
//!   [`Algebraic`] (minimal polynomial, index among its real roots) with
//!   its multiplicity. Both rulers must be on one frame: the eigenvalues
//!   mean something only in one basis. Basis-invariant: a simultaneous
//!   congruence acts on G₁⁻¹G₂ by similarity (P11e), so per-axis units drop
//!   out (Mara §9.1, change detection between windows).
//!
//! **Caveat (Mara §8.3).** The relative eigenvalues are a **complete**
//! invariant of the pair only **over ℝ, with one form definite**
//! (simultaneous diagonalisation). Over ℚ they are not: (1) and (2) on ℚ¹
//! share relative eigenvalue 1, but are not simultaneously congruent to
//! (1), (1) over ℚ (p² = 2, P11f): rational classes carry Hasse–Minkowski
//! data the eigenvalues don't. For indefinite pairs they are not complete
//! either (P11g, P11h). Here both are `Metrix`, so the spectrum is complete
//! for congruence by GL_n(ℝ), and necessary, not complete, for GL_n(ℚ).
//!
//! # Not here
//!
//! - **`Measured<Metrix>`** (D6: an estimate is measured until a window
//!   ruler signs it): [`crate::measured`], exact over ℚ from integer
//!   counts, a `Metrix` only through a draft and a seal.
//! - **A metric slot on `hypothesis::Frame`.** Composition needs no metric
//!   (Mara §4.2, §6.4). A `Metrix` names its frame by address instead, and
//!   the frame stays untouched, so nothing on the composition path can
//!   reference a metric.

use num_traits::Zero;

use crate::algedonic::Signer;
use crate::gl::{rational, Charpoly, Gl, Scalar};
use crate::hypothesis::Frame;
use crate::oid::{Oid, Scope};
use crate::poly::{Algebraic, Budget, Factored, QPoly, ZPoly};
use crate::room::{Coefficient, Entry, Q, Z};

mod sealed {
    pub trait Sealed {}
    impl Sealed for crate::room::Q {}
    impl Sealed for crate::room::Z {}
}

/// Entries an ordered field reads exactly: ℚ, and ℤ inside it. Not ℤ/n,
/// which has no order, so "positive" means nothing there.
pub trait Ordered: Clone + sealed::Sealed {
    /// This entry in ℚ.
    fn in_q(&self) -> Q;
}

impl Ordered for Q {
    fn in_q(&self) -> Q {
        self.clone()
    }
}

impl Ordered for Z {
    fn in_q(&self) -> Q {
        Scalar::rational(self)
    }
}

/// Why a matrix isn't a `Metrix` (or a `Form`), or two can't be read
/// together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetrixError {
    /// Ragged, or not square.
    NotSquare,
    /// Square, but not the size the frame (or the other operand) has.
    WrongSize {
        /// The size wanted.
        expected: usize,
        /// The size given.
        found: usize,
    },
    /// gᵢⱼ ≠ gⱼᵢ at this entry (the first, row-major, below the diagonal).
    NotSymmetric {
        /// Its row, from 0.
        row: usize,
        /// Its column, from 0.
        col: usize,
    },
    /// The LDLᵀ pivot dₖ is ≤ 0: not positive definite. `pivot` counts
    /// from 1, so it names the leading k×k block that fails.
    NotDefinite {
        /// k, from 1.
        pivot: usize,
        /// dₖ, exactly.
        value: Q,
    },
    /// Two rulers on two frames, where one frame is needed.
    OtherFrame {
        /// The first operand's frame.
        expected: Oid,
        /// The second operand's frame.
        found: Oid,
    },
}

/// The certificate of definiteness: G = L·D·Lᵀ, L unit lower triangular,
/// every entry of D > 0.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Certificate {
    l: Vec<Vec<Q>>,
    d: Vec<Q>,
}

impl Certificate {
    /// L, unit lower triangular, row by row.
    pub fn l(&self) -> &[Vec<Q>] {
        &self.l
    }

    /// The pivots, the diagonal of D.
    pub fn d(&self) -> &[Q] {
        &self.d
    }

    /// Whether this certifies `g`: L is n×n unit lower triangular, every
    /// pivot is > 0, and L·D·Lᵀ = g exactly.
    pub fn verifies(&self, g: &[Vec<Q>]) -> bool {
        let n = self.d.len();
        let (zero, one) = (Q::from(0), Q::from(1));
        let shaped = g.len() == n
            && g.iter().all(|r| r.len() == n)
            && self.l.len() == n
            && self.l.iter().enumerate().all(|(i, r)| {
                r.len() == n
                    && r.iter().enumerate().all(|(j, x)| match j.cmp(&i) {
                        std::cmp::Ordering::Equal => *x == one,
                        std::cmp::Ordering::Greater => *x == zero,
                        std::cmp::Ordering::Less => true,
                    })
            });
        shaped
            && self.d.iter().all(|x| *x > zero)
            && (0..n).all(|i| {
                (0..n).all(|j| {
                    let s = (0..n).fold(Q::from(0), |s, k| {
                        s.add(&self.l[i][k].mul(&self.d[k]).mul(&self.l[j][k]))
                    });
                    s == g[i][j]
                })
            })
    }
}

/// An unsigned metric on a frame: certified positive definite, not yet
/// anyone's judgment. No default.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetrixDraft {
    frame: Oid,
    g: Vec<Vec<Q>>,
    certificate: Certificate,
}

impl MetrixDraft {
    /// The metric on `frame` with these rows, if it is one: square, of the
    /// frame's dimension, symmetric, and positive definite by an exact LDLᵀ.
    pub fn new<R: Ordered>(frame: &Frame, rows: Vec<Vec<R>>) -> Result<Self, MetrixError> {
        let g = square(rows)?;
        if g.len() != frame.dim() {
            return Err(MetrixError::WrongSize {
                expected: frame.dim(),
                found: g.len(),
            });
        }
        symmetric(&g)?;
        let certificate = ldl(&g)?;
        Ok(MetrixDraft {
            frame: frame.oid().clone(),
            g,
            certificate,
        })
    }

    /// The Euclidean identity on `frame`, declared: a draft like any other,
    /// a `Metrix` only once someone seals it.
    pub fn identity(frame: &Frame) -> Self {
        let n = frame.dim();
        let eye: Vec<Vec<Q>> = (0..n)
            .map(|i| (0..n).map(|j| Q::from(i64::from(i == j))).collect())
            .collect();
        MetrixDraft {
            frame: frame.oid().clone(),
            g: eye.clone(),
            certificate: Certificate {
                l: eye,
                d: vec![Q::from(1); n],
            },
        }
    }

    /// Its certificate (L, D).
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    /// The canonical preimage: `"metrix\0" ‖ u64le(len) ‖ frame oid ‖
    /// u64le(n) ‖ rational(gᵢⱼ)` row by row. Neither the signer nor the
    /// certificate (a function of G) is in it.
    pub fn preimage(&self) -> Vec<u8> {
        let mut bytes = b"metrix\0".to_vec();
        let frame = self.frame.as_str().as_bytes();
        bytes.extend_from_slice(&(frame.len() as u64).to_le_bytes());
        bytes.extend_from_slice(frame);
        bytes.extend_from_slice(&(self.g.len() as u64).to_le_bytes());
        for x in self.g.iter().flatten() {
            rational(&mut bytes, x.value());
        }
        bytes
    }

    /// Its address: the preimage, in the public scope.
    pub fn oid(&self) -> Oid {
        Oid::hash(&Scope::public(), &self.preimage())
    }

    /// Seal the metric as `by`'s: the only way to a [`Metrix`].
    pub fn seal(self, by: Oid) -> Metrix {
        let oid = self.oid();
        Metrix {
            signer: Signer::mint(by, oid.clone()),
            oid,
            draft: self,
        }
    }
}

/// A signed metric: a frame's ruler for length. A [`MetrixDraft`] sealed
/// by its [`Signer`]; no default, no public constructor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Metrix {
    draft: MetrixDraft,
    oid: Oid,
    signer: Signer,
}

impl Metrix {
    /// Its address (the draft's; the signer is beside it).
    pub fn oid(&self) -> Oid {
        self.oid.clone()
    }

    /// Who signed it, over which address.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The frame whose basis it is written in.
    pub fn frame(&self) -> &Oid {
        &self.draft.frame
    }

    /// n: the frame's dimension.
    pub fn dim(&self) -> usize {
        self.draft.g.len()
    }

    /// G, row by row.
    pub fn rows(&self) -> &[Vec<Q>] {
        &self.draft.g
    }

    /// Its certificate of definiteness.
    pub fn certificate(&self) -> &Certificate {
        &self.draft.certificate
    }
}

/// The inertia (n₊, n₋, n₀) of a symmetric form: how many positive,
/// negative and zero squares it is congruent to. Basis-free (Sylvester).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Inertia {
    /// n₊.
    pub positive: usize,
    /// n₋.
    pub negative: usize,
    /// n₀: the nullity.
    pub zero: usize,
}

/// A symmetric matrix over ℚ of any signature, with its inertia. Unsigned:
/// a computed value (a defect, a difference), not a judgment.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Form {
    g: Vec<Vec<Q>>,
    inertia: Inertia,
}

impl Form {
    /// The form with these rows, if square and symmetric.
    pub fn new<R: Ordered>(rows: Vec<Vec<R>>) -> Result<Self, MetrixError> {
        let g = square(rows)?;
        symmetric(&g)?;
        Ok(Form::computed(g))
    }

    /// A metric, read as a form (inertia (n, 0, 0)).
    pub fn of(metrix: &Metrix) -> Self {
        Form::computed(metrix.rows().to_vec())
    }

    /// `minuend − subtrahend`: two rulers on one frame, compared.
    pub fn difference(minuend: &Metrix, subtrahend: &Metrix) -> Result<Self, MetrixError> {
        same_frame(minuend, subtrahend)?;
        let g = minuend
            .rows()
            .iter()
            .zip(subtrahend.rows())
            .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x.add(&y.neg())).collect())
            .collect();
        Ok(Form::computed(g))
    }

    /// Pᵀ·F·P: the same form in another basis. P acts as the identity past
    /// its size; a P larger than the form is refused.
    pub fn congruent(&self, p: &Gl<Q>) -> Result<Self, MetrixError> {
        let n = self.g.len();
        if p.size() > n {
            return Err(MetrixError::WrongSize {
                expected: n,
                found: p.size(),
            });
        }
        Ok(Form::computed(pull_back(&p.rows(n), &self.g)))
    }

    /// Its rows.
    pub fn rows(&self) -> &[Vec<Q>] {
        &self.g
    }

    /// n.
    pub fn dim(&self) -> usize {
        self.g.len()
    }

    /// (n₊, n₋, n₀), by Sylvester's law.
    pub fn inertia(&self) -> Inertia {
        self.inertia
    }

    /// Whether every entry is 0.
    pub fn is_zero(&self) -> bool {
        self.g.iter().flatten().all(|x| x.value().is_zero())
    }

    fn computed(g: Vec<Vec<Q>>) -> Self {
        let inertia = inertia_of(&g);
        Form { g, inertia }
    }
}

/// Tᵀ·G_B·T − G_A: how far T, reading frame A into frame B, is from an
/// isometry of A's ruler onto B's. Zero iff T is an isometry.
///
/// Both rulers must have one size, and T must act within it (a `Gl` acts as
/// the identity past its size).
pub fn isometry_defect(t: &Gl<Q>, g_a: &Metrix, g_b: &Metrix) -> Result<Form, MetrixError> {
    let n = g_a.dim();
    for (found, fits) in [(g_b.dim(), g_b.dim() == n), (t.size(), t.size() <= n)] {
        if !fits {
            return Err(MetrixError::WrongSize { expected: n, found });
        }
    }
    let pulled = pull_back(&t.rows(n), g_b.rows());
    let g = pulled
        .iter()
        .zip(g_a.rows())
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x.add(&y.neg())).collect())
        .collect();
    Ok(Form::computed(g))
}

/// The eigenvalues of G₁⁻¹G₂ (the roots of det(G₂ − λG₁)), exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelativeSpectrum {
    charpoly: Charpoly,
    eigenvalues: Vec<(Algebraic, usize)>,
    unresolved: Option<ZPoly>,
}

impl RelativeSpectrum {
    /// det(xI − G₁⁻¹G₂) over ℚ, leading coefficient first.
    pub fn charpoly(&self) -> &Charpoly {
        &self.charpoly
    }

    /// Each eigenvalue found, as (minimal polynomial, index among its real
    /// roots), with its multiplicity; in the canonical order of the
    /// irreducible factors, then by index. All of them when
    /// [`is_certified`](Self::is_certified).
    pub fn eigenvalues(&self) -> &[(Algebraic, usize)] {
        &self.eigenvalues
    }

    /// Whether the factorization is certified, so every eigenvalue is in
    /// [`eigenvalues`](Self::eigenvalues).
    pub fn is_certified(&self) -> bool {
        self.unresolved.is_none()
    }

    /// When the factoring budget ran out: the part of the charpoly not yet
    /// split into irreducibles (primitive, with its multiplicities), its
    /// roots unnamed. Never a guess.
    pub fn unresolved(&self) -> Option<&ZPoly> {
        self.unresolved.as_ref()
    }
}

/// The relative spectrum of two rulers on one frame: the eigenvalues of
/// G₁⁻¹G₂ as an exact algebraic multiset.
///
/// A complete invariant of the pair only over ℝ with one form definite
/// (Mara §8.3): here, under congruence by GL_n(ℝ), not GL_n(ℚ). Factoring
/// runs against `budget` ([`ZPoly::factor`]); running out leaves the rest
/// in [`RelativeSpectrum::unresolved`].
pub fn relative_spectrum(
    g1: &Metrix,
    g2: &Metrix,
    budget: Budget,
) -> Result<RelativeSpectrum, MetrixError> {
    same_frame(g1, g2)?;
    let n = g1.dim();
    // G₁⁻¹G₂, column by column, through G₁'s certificate.
    let columns: Vec<Vec<Q>> = (0..n)
        .map(|j| {
            let b: Vec<Q> = g2.rows().iter().map(|r| r[j].clone()).collect();
            solve(g1.certificate(), &b)
        })
        .collect();
    let a: Vec<Q> = (0..n * n)
        .map(|at| columns[at % n][at / n].clone())
        .collect();
    let charpoly = Charpoly::of(&a, n);
    let p = QPoly::from_descending(charpoly.coefficients().iter().map(|c| c.value().clone()))
        .primitive();
    // The charpoly is monic, so p ≠ 0, and the square-free part of a nonzero
    // polynomial is square-free: `factor` has nothing to refuse.
    let (irreducible, rest) = match p
        .square_free()
        .factor(budget)
        .expect("the square-free part of a nonzero polynomial is square-free")
    {
        Factored::Certified(f) => (f.factors().to_vec(), None),
        Factored::Uncertified { irreducible, .. } => (irreducible, Some(())),
    };
    let mut left = p.to_rational();
    let mut eigenvalues = Vec::new();
    for f in &irreducible {
        let fq = f.to_rational();
        let mut multiplicity = 0;
        while let Some((quotient, remainder)) = left.div_rem(&fq) {
            if !remainder.is_zero() {
                break;
            }
            left = quotient;
            multiplicity += 1;
        }
        for index in 0..f.real_roots().len() {
            eigenvalues.push((Algebraic::new(f.clone(), index), multiplicity));
        }
    }
    Ok(RelativeSpectrum {
        charpoly,
        eigenvalues,
        unresolved: rest.map(|()| left.primitive()),
    })
}

// --- Exact linear algebra over ℚ ----------------------------------------------

/// Square rows, read in ℚ.
fn square<R: Ordered>(rows: Vec<Vec<R>>) -> Result<Vec<Vec<Q>>, MetrixError> {
    let n = rows.len();
    if rows.iter().any(|r| r.len() != n) {
        return Err(MetrixError::NotSquare);
    }
    Ok(rows
        .iter()
        .map(|r| r.iter().map(Ordered::in_q).collect())
        .collect())
}

fn symmetric(g: &[Vec<Q>]) -> Result<(), MetrixError> {
    for (row, r) in g.iter().enumerate() {
        for col in 0..row {
            if r[col] != g[col][row] {
                return Err(MetrixError::NotSymmetric { row, col });
            }
        }
    }
    Ok(())
}

fn same_frame(a: &Metrix, b: &Metrix) -> Result<(), MetrixError> {
    if a.frame() == b.frame() {
        Ok(())
    } else {
        Err(MetrixError::OtherFrame {
            expected: a.frame().clone(),
            found: b.frame().clone(),
        })
    }
}

/// x / d, for a pivot d already known to be nonzero.
fn over(x: &Q, d: &Q) -> Q {
    Q::from(x.value() / d.value())
}

/// LDLᵀ without pivoting: dₖ = gₖₖ − Σⱼ₍ⱼ₍ₖ₎ lₖⱼ²dⱼ, refused at the first
/// dₖ ≤ 0; lᵢₖ = (gᵢₖ − Σⱼ₍ⱼ₍ₖ₎ lᵢⱼlₖⱼdⱼ) / dₖ.
fn ldl(g: &[Vec<Q>]) -> Result<Certificate, MetrixError> {
    let n = g.len();
    let zero = Q::from(0);
    let mut l: Vec<Vec<Q>> = (0..n)
        .map(|i| (0..n).map(|j| Q::from(i64::from(i == j))).collect())
        .collect();
    let mut d: Vec<Q> = Vec::with_capacity(n);
    for k in 0..n {
        let dk = (0..k).fold(g[k][k].clone(), |s, j| {
            s.add(&l[k][j].mul(&l[k][j]).mul(&d[j]).neg())
        });
        if dk <= zero {
            return Err(MetrixError::NotDefinite {
                pivot: k + 1,
                value: dk,
            });
        }
        for i in k + 1..n {
            let s = (0..k).fold(g[i][k].clone(), |s, j| {
                s.add(&l[i][j].mul(&l[k][j]).mul(&d[j]).neg())
            });
            l[i][k] = over(&s, &dk);
        }
        d.push(dk);
    }
    Ok(Certificate { l, d })
}

/// x with G·x = b, for G = L·D·Lᵀ certified: forward, scale, back.
fn solve(c: &Certificate, b: &[Q]) -> Vec<Q> {
    let n = b.len();
    let mut y: Vec<Q> = Vec::with_capacity(n);
    for (i, bi) in b.iter().enumerate() {
        let s = (0..i).fold(bi.clone(), |s, j| s.add(&c.l[i][j].mul(&y[j]).neg()));
        y.push(s);
    }
    let z: Vec<Q> = y.iter().zip(&c.d).map(|(y, d)| over(y, d)).collect();
    let mut x = vec![Q::from(0); n];
    for i in (0..n).rev() {
        x[i] = (i + 1..n).fold(z[i].clone(), |s, j| s.add(&c.l[j][i].mul(&x[j]).neg()));
    }
    x
}

/// Tᵀ·G·T, all n×n.
fn pull_back(t: &[Vec<Q>], g: &[Vec<Q>]) -> Vec<Vec<Q>> {
    let n = g.len();
    let dot = |f: &dyn Fn(usize) -> Q, h: &dyn Fn(usize) -> Q| {
        (0..n).fold(Q::from(0), |s, k| s.add(&f(k).mul(&h(k))))
    };
    // G·T, then Tᵀ·(G·T).
    let gt: Vec<Vec<Q>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| dot(&|k| g[i][k].clone(), &|k| t[k][j].clone()))
                .collect()
        })
        .collect();
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| dot(&|k| t[k][i].clone(), &|k| gt[k][j].clone()))
                .collect()
        })
        .collect()
}

/// Sylvester's law through an LDLᵀ with symmetric pivoting (module docs):
/// a 1×1 pivot on any nonzero diagonal entry, else a 2×2 pivot
/// [[0, b], [b, 0]] on a nonzero off-diagonal entry, else the rest is n₀.
/// Each step replaces the form by its Schur complement.
fn inertia_of(g: &[Vec<Q>]) -> Inertia {
    let zero = Q::from(0);
    let mut a = g.to_vec();
    let mut count = Inertia {
        positive: 0,
        negative: 0,
        zero: 0,
    };
    while !a.is_empty() {
        let n = a.len();
        if let Some(k) = (0..n).find(|&k| a[k][k] != zero) {
            let d = a[k][k].clone();
            if d > zero {
                count.positive += 1;
            } else {
                count.negative += 1;
            }
            let rest: Vec<usize> = (0..n).filter(|&i| i != k).collect();
            a = rest
                .iter()
                .map(|&r| {
                    rest.iter()
                        .map(|&s| a[r][s].add(&over(&a[r][k].mul(&a[k][s]), &d).neg()))
                        .collect()
                })
                .collect();
        } else if let Some((i, j)) = (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .find(|&(i, j)| a[i][j] != zero)
        {
            // E = [[0, b], [b, 0]], E⁻¹ = [[0, 1/b], [1/b, 0]]:
            // a′ᵣₛ = aᵣₛ − (aᵣᵢ·aⱼₛ + aᵣⱼ·aᵢₛ) / b.
            let b = a[i][j].clone();
            count.positive += 1;
            count.negative += 1;
            let rest: Vec<usize> = (0..n).filter(|&r| r != i && r != j).collect();
            a = rest
                .iter()
                .map(|&r| {
                    rest.iter()
                        .map(|&s| {
                            let cross = a[r][i].mul(&a[j][s]).add(&a[r][j].mul(&a[i][s]));
                            a[r][s].add(&over(&cross, &b).neg())
                        })
                        .collect()
                })
                .collect();
        } else {
            count.zero += n;
            break;
        }
    }
    count
}

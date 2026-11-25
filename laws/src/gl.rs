//! `prism_laws::gl` — invertible exact transports: GL over ℚ, or over ℤ
//! with det ±1 (prism-021; Mara 54c88f72 §11 "For prism").
//!
//! A [`Gl<R>`] is an invertible square matrix with exact entries. It is a
//! [`Group`] under path-order composition and [`Exact`], so
//! `Field<Graph, Gl<R>>`, [`check_flat`](crate::field::check_flat),
//! `Curved::loops` and faces work unchanged: the holonomy of a loop of
//! frames is an exact `Gl`, compared exactly, never by trace (a shear has
//! the identity's trace and charpoly).
//!
//! **One identity at every size**, after [`Perm`](crate::field::Perm): a
//! transport acts as the identity past its size, and a trailing identity
//! block is trimmed, so I₂ = I₁ = the empty transport = [`Group::identity`].
//! That keeps `compose` total: two transports of different sizes compose as
//! block matrices. A frame's dimension lives on the frame, not here.
//!
//! **Refused at construction, never later** ([`GlError`]): a ragged or
//! non-square matrix, a singular one (det 0), and over ℤ one whose
//! determinant isn't a unit (±1), whose inverse isn't integral. The inverse
//! is computed once, over ℚ, and kept beside the matrix, so
//! [`Group::inverse`] can't fail.
//!
//! Not a room [`Map`](crate::room::Map): a `Map`'s kinds are the cochain
//! rooms of a graph (C⁰, C¹, C²), sealed, and a frame's stalk is none of
//! them. `Gl` uses room's exact rings ([`Q`], [`Z`]) and their traits.

use std::fmt::Debug;
use std::hash::Hash;

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::Zero;

use crate::field::{Exact, Group};
use crate::room::{Coefficient, Entry, Exactly, Q, Z};

mod sealed {
    pub trait Sealed {}
    impl Sealed for crate::room::Q {}
    impl Sealed for crate::room::Z {}
}

/// The exact rings a transport's entries can live in: [`Q`] (invertible
/// means det ≠ 0) and [`Z`] (invertible means det = ±1).
pub trait Scalar: Entry<Ring = ()> + Exactly + Eq + Ord + Hash + sealed::Sealed {
    /// This entry in ℚ.
    fn rational(&self) -> Q;
    /// The entry of this ring equal to `q`, if there is one.
    fn of_rational(q: &Q) -> Option<Self>;
    /// A basis of the kernel {x : M·x = 0} of a `rows` × `cols` matrix,
    /// over this ring: over ℚ the free-column basis of its reduced echelon
    /// shape, over ℤ a ℤ-basis of the kernel lattice (unimodular column
    /// operations). Canonical: a function of the matrix alone.
    fn kernel(rows: &[Vec<Self>], cols: usize) -> Vec<Vec<Self>>;
}

impl Scalar for Q {
    fn rational(&self) -> Q {
        self.clone()
    }
    fn of_rational(q: &Q) -> Option<Self> {
        Some(q.clone())
    }
    fn kernel(rows: &[Vec<Q>], cols: usize) -> Vec<Vec<Q>> {
        let (reduced, pivots) = echelon(rows.to_vec(), cols);
        let zero = Q::from(0);
        (0..cols)
            .filter(|c| !pivots.contains(c))
            .map(|free| {
                let mut x = vec![zero.clone(); cols];
                x[free] = Q::from(1);
                for (row, &p) in pivots.iter().enumerate() {
                    x[p] = reduced[row][free].neg();
                }
                x
            })
            .collect()
    }
}

impl Scalar for Z {
    fn rational(&self) -> Q {
        Q::from(BigRational::from_integer(self.value().clone()))
    }
    fn of_rational(q: &Q) -> Option<Self> {
        q.value()
            .is_integer()
            .then(|| Z::from(q.value().to_integer()))
    }
    fn kernel(rows: &[Vec<Z>], cols: usize) -> Vec<Vec<Z>> {
        let mut a: Vec<Vec<BigInt>> = rows
            .iter()
            .map(|r| r.iter().map(|x| x.value().clone()).collect())
            .collect();
        // V starts as the identity; column operations on A are mirrored on V.
        let mut v: Vec<Vec<BigInt>> = (0..cols)
            .map(|i| (0..cols).map(|j| BigInt::from(u8::from(i == j))).collect())
            .collect();
        let mut rank = 0;
        for i in 0..a.len() {
            for j in rank + 1..cols {
                if a[i][j].is_zero() {
                    continue;
                }
                let (p, q) = (a[i][rank].clone(), a[i][j].clone());
                let e = p.extended_gcd(&q);
                let (s, t) = (&p / &e.gcd, &q / &e.gcd);
                // [col_r, col_j] ← [x·col_r + y·col_j, −t·col_r + s·col_j]: det 1.
                let mix = |m: &mut Vec<Vec<BigInt>>| {
                    for row in m.iter_mut() {
                        let (r, c) = (row[rank].clone(), row[j].clone());
                        row[rank] = &e.x * &r + &e.y * &c;
                        row[j] = -(&t * &r) + &s * &c;
                    }
                };
                mix(&mut a);
                mix(&mut v);
            }
            if !a[i].get(rank).is_none_or(Zero::is_zero) {
                rank += 1;
            }
        }
        (rank..cols)
            .map(|c| v.iter().map(|row| Z::from(row[c].clone())).collect())
            .collect()
    }
}

/// Reduced row echelon shape over ℚ: the rows (pivot rows only) and the
/// pivot columns, ascending.
fn echelon(mut m: Vec<Vec<Q>>, cols: usize) -> (Vec<Vec<Q>>, Vec<usize>) {
    let mut pivots = Vec::new();
    let mut row = 0;
    for col in 0..cols {
        // The pivot is the first entry with an inverse: over ℚ, exactly the
        // nonzero ones, so every pivot found scales to 1.
        let Some((p, inv)) = (row..m.len()).find_map(|i| m[i][col].recip().map(|inv| (i, inv)))
        else {
            continue;
        };
        m.swap(row, p);
        m[row] = m[row].iter().map(|x| x.mul(&inv)).collect();
        for i in 0..m.len() {
            if i != row && m[i][col] != Q::from(0) {
                let f = m[i][col].clone();
                m[i] = m[i]
                    .iter()
                    .zip(&m[row])
                    .map(|(x, y)| x.add(&f.mul(y).neg()))
                    .collect();
            }
        }
        pivots.push(col);
        row += 1;
    }
    m.truncate(row);
    (m, pivots)
}

/// Why a matrix isn't a transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlError {
    /// Ragged, or not square.
    NotSquare,
    /// det = 0: no transport back.
    Singular,
    /// det ≠ 0 but not a unit of the ring (over ℤ: not ±1), so the
    /// inverse leaves the ring.
    NotUnit,
}

/// An invertible square matrix over an exact ring: a transport between two
/// frames' stalks. Trimmed: no trailing identity block, so every transport
/// has one shape and the identity is the empty one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Gl<R: Scalar> {
    /// The size after trimming.
    n: usize,
    /// Row-major, n × n.
    m: Vec<R>,
    /// Its inverse, row-major, n × n.
    back: Vec<R>,
}

impl<R: Scalar> Gl<R> {
    /// The transport with these rows, if it is one: square, and invertible
    /// over `R`. The empty matrix is the identity.
    pub fn new(rows: Vec<Vec<R>>) -> Result<Self, GlError> {
        let n = rows.len();
        if rows.iter().any(|r| r.len() != n) {
            return Err(GlError::NotSquare);
        }
        let inverse = invert(&rows).ok_or(GlError::Singular)?;
        let back = inverse
            .iter()
            .flatten()
            .map(R::of_rational)
            .collect::<Option<Vec<R>>>()
            .ok_or(GlError::NotUnit)?;
        let m = rows.into_iter().flatten().collect();
        Ok(Gl { n, m, back }.trimmed())
    }

    /// Its size: the smallest n it acts within (identity past it).
    pub fn size(&self) -> usize {
        self.n
    }

    /// Its rows as an `n` × `n` matrix, the identity past its size (at least
    /// its size, if `n` is smaller).
    pub fn rows(&self, n: usize) -> Vec<Vec<R>> {
        rows(&padded(&self.m, self.n, n.max(self.n)), n.max(self.n))
    }

    /// M·v, the identity past its size (v padded with zeros to its size).
    pub fn apply(&self, v: &[R]) -> Vec<R> {
        let n = v.len().max(self.n);
        let mut x = v.to_vec();
        x.resize(n, R::zero(&()));
        let m = padded(&self.m, self.n, n);
        (0..n)
            .map(|i| (0..n).fold(R::zero(&()), |s, j| s.add(&m[i * n + j].mul(&x[j]))))
            .collect()
    }

    /// det(xI − M) at size `n` (at least its size), over ℚ, by
    /// Faddeev–LeVerrier. Exact: two transports with one charpoly have one
    /// encoding.
    pub fn charpoly(&self, n: usize) -> Charpoly {
        let n = n.max(self.n);
        let a: Vec<Q> = padded(&self.m, self.n, n).iter().map(R::rational).collect();
        Charpoly::of(&a, n)
    }

    /// Its canonical bytes: `u64le(n)`, then the n² entries row by row,
    /// each a rational as `docs/spec/ruler.md` encodes one.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&(self.n as u64).to_le_bytes());
        for x in &self.m {
            rational(out, x.rational().value());
        }
    }

    /// Drop the trailing identity block, from both the matrix and its
    /// inverse (whose block is the same).
    fn trimmed(mut self) -> Self {
        let unit = |m: &[R], n: usize| {
            let k = n - 1;
            (0..n).all(|i| {
                let want = if i == k { R::one(&()) } else { R::zero(&()) };
                m[k * n + i] == want && m[i * n + k] == want
            })
        };
        while self.n > 0 && unit(&self.m, self.n) {
            let n = self.n - 1;
            self.m = padded(&self.m, self.n, n);
            self.back = padded(&self.back, self.n, n);
            self.n = n;
        }
        self
    }
}

impl<R: Scalar> Group for Gl<R> {
    fn identity() -> Self {
        Gl {
            n: 0,
            m: Vec::new(),
            back: Vec::new(),
        }
    }
    /// First `self`, then `other`: other·self on columns.
    fn compose(&self, other: &Self) -> Self {
        let n = self.n.max(other.n);
        let (a, b) = (padded(&self.m, self.n, n), padded(&other.m, other.n, n));
        let (ai, bi) = (
            padded(&self.back, self.n, n),
            padded(&other.back, other.n, n),
        );
        Gl {
            n,
            m: product(&b, &a, n),
            back: product(&ai, &bi, n),
        }
        .trimmed()
    }
    fn inverse(&self) -> Self {
        Gl {
            n: self.n,
            m: self.back.clone(),
            back: self.m.clone(),
        }
    }
    /// Exactly: trimmed, the identity is the empty transport.
    fn is_identity(&self) -> bool {
        self.n == 0
    }
}

impl<R: Scalar> Exact for Gl<R> {}

/// A characteristic polynomial det(xI − M) over ℚ, leading coefficient
/// first: `coefficients()[k]` is the coefficient of xⁿ⁻ᵏ.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Charpoly {
    coefficients: Vec<Q>,
}

impl Charpoly {
    /// det(xI − A) of the n × n row-major matrix `a` over ℚ, by
    /// Faddeev–LeVerrier: any square matrix, invertible or not.
    pub(crate) fn of(a: &[Q], n: usize) -> Charpoly {
        let mut coefficients = vec![Q::from(1)];
        let mut m = vec![Q::from(0); n * n];
        for k in 1..=n {
            // M_k = A·M_{k−1} + c_{n−k+1}·I; c_{n−k} = −tr(A·M_k)/k.
            let mut next = product(a, &m, n);
            let c = coefficients[k - 1].clone();
            for i in 0..n {
                next[i * n + i] = next[i * n + i].add(&c);
            }
            let am = product(a, &next, n);
            let trace = (0..n).fold(Q::from(0), |s, i| s.add(&am[i * n + i]));
            let k = Q::from(k as i64);
            coefficients.push(trace.neg().mul(&k.recip().unwrap_or(k)));
            m = next;
        }
        Charpoly { coefficients }
    }

    /// The n + 1 coefficients, leading first (the first is 1).
    pub fn coefficients(&self) -> &[Q] {
        &self.coefficients
    }

    /// Its canonical bytes: `u64le(n + 1)`, then each coefficient as a
    /// rational (`docs/spec/ruler.md`). One polynomial, one encoding.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = (self.coefficients.len() as u64).to_le_bytes().to_vec();
        for c in &self.coefficients {
            rational(&mut out, c.value());
        }
        out
    }
}

/// `sign ‖ bytes(|numerator|) ‖ bytes(denominator)`, as `ruler.md`.
pub(crate) fn rational(out: &mut Vec<u8>, r: &BigRational) {
    let (sign, numer) = r.numer().to_bytes_le();
    out.push(match sign {
        Sign::Minus => 2,
        Sign::NoSign => 0,
        Sign::Plus => 1,
    });
    for bytes in [numer, r.denom().to_bytes_le().1] {
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&bytes);
    }
}

/// The n × n row-major matrix `m` of size `k`, cut or padded with the
/// identity to size `n`.
fn padded<R: Scalar>(m: &[R], k: usize, n: usize) -> Vec<R> {
    (0..n * n)
        .map(|at| {
            let (i, j) = (at / n, at % n);
            if i < k && j < k {
                m[i * k + j].clone()
            } else if i == j {
                R::one(&())
            } else {
                R::zero(&())
            }
        })
        .collect()
}

fn rows<R: Clone>(m: &[R], n: usize) -> Vec<Vec<R>> {
    m.chunks(n.max(1)).take(n).map(<[R]>::to_vec).collect()
}

/// a·b, both n × n row-major.
fn product<R: Entry<Ring = ()>>(a: &[R], b: &[R], n: usize) -> Vec<R> {
    (0..n * n)
        .map(|at| {
            let (i, j) = (at / n, at % n);
            (0..n).fold(R::zero(&()), |s, k| s.add(&a[i * n + k].mul(&b[k * n + j])))
        })
        .collect()
}

/// The inverse over ℚ by Gauss–Jordan, or `None` when singular.
fn invert<R: Scalar>(rows: &[Vec<R>]) -> Option<Vec<Vec<Q>>> {
    let n = rows.len();
    let augmented: Vec<Vec<Q>> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            r.iter()
                .map(R::rational)
                .chain((0..n).map(|j| Q::from(i64::from(i == j))))
                .collect()
        })
        .collect();
    let (reduced, pivots) = echelon(augmented, 2 * n);
    (pivots.len() >= n && pivots[..n].iter().copied().eq(0..n))
        .then(|| reduced.into_iter().map(|r| r[n..].to_vec()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: i64) -> Q {
        Q::from(n)
    }

    #[test]
    fn the_z_kernel_is_a_lattice_basis_not_a_rational_one() {
        // 2x − 4y = 0 over ℤ: the kernel is ℤ·(2, 1), not ℤ·(1, 1/2).
        let k = Z::kernel(&[vec![Z::from(2), Z::from(-4)]], 2);
        assert_eq!(k.len(), 1);
        let v = &k[0];
        assert_eq!(
            Z::from(2).mul(&v[0]).add(&Z::from(-4).mul(&v[1])),
            Z::from(0)
        );
        assert!(v
            .iter()
            .all(|x| *x.value() <= BigInt::from(2) && *x.value() >= BigInt::from(-2)));
        assert_eq!(Z::kernel(&[], 2).len(), 2);
    }

    #[test]
    fn the_q_kernel_of_nothing_is_everything() {
        assert_eq!(Q::kernel(&[], 2), vec![vec![q(1), q(0)], vec![q(0), q(1)]]);
        assert!(Q::kernel(&[vec![q(1), q(0)], vec![q(0), q(1)]], 2).is_empty());
    }

    #[test]
    fn q_is_one_ring_and_times_is_repeated_addition() {
        // ℚ needs no ring data: every Q's ring is the one unit value.
        assert_eq!(Q::new(3, 4).expect("3/4").ring(), &());
        let x = Q::new(3, 4).expect("3/4");
        assert_eq!(x.times(&BigInt::from(3)), x.add(&x).add(&x));
        assert_eq!(x.times(&BigInt::from(-2)), Q::new(-3, 2).expect("-3/2"));
        assert_eq!(x.times(&BigInt::from(0)), Q::zero(&()));
    }

    #[test]
    fn the_charpoly_of_the_identity_is_a_power_of_x_minus_one() {
        let p = Gl::<Q>::identity().charpoly(2);
        assert_eq!(p.coefficients(), &[q(1), q(-2), q(1)]);
        let one = BigInt::from(1);
        assert_eq!(Z::of_rational(&Q::new(one, 2).expect("1/2")), None);
    }
}

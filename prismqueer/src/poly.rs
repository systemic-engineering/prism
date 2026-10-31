//! `prismqueer::poly` — exact polynomials over ℤ and ℚ, and the real roots
//! of an integer polynomial, isolated exactly.
//!
//! Nothing here is a float until the one place a measurement comes in,
//! [`Roots::locate`], and even there the float is read as the exact dyadic
//! rational it is. Coefficients are `num-bigint` integers ([`ZPoly`]) or
//! reduced `BigInt` ratios ([`QPoly`]), so no step rounds or overflows.
//!
//! What `poly` keeps from collapsing early:
//! - **A root is an interval and an index, not a decimal.** [`Roots`] holds
//!   disjoint rational intervals `(lo, hi]`, one root each, in increasing
//!   order. A root's name is its index there; refining an interval never
//!   changes it.
//! - **A measurement is matched, or it refuses.** [`Roots::locate`] says
//!   which exact root a measured eigenvalue is, only when exactly one root is
//!   in its window. Two or more is [`Located::Ambiguous`], with the
//!   candidates: what is known, not a guess.
//! - **Repeated roots are kept once.** Isolation runs on the square-free
//!   part p / gcd(p, p′) ([`ZPoly::square_free`]); an eigenvalue's
//!   multiplicity is the charpoly's business, its identity the root's.
//!
//! - **A factorization is certified, or it says it isn't.**
//!   [`ZPoly::factor`] (Zassenhaus: a good prime, Berlekamp mod p, a Hensel
//!   lift past the Mignotte bound, subset recombination) returns
//!   [`Factored::Certified`] only with every irreducible factor in hand. The
//!   subset search is exponential in the worst case, so it runs against a
//!   [`Budget`] of trials, and running out is [`Factored::Uncertified`]:
//!   the factors found, and the rest, never a guess.
//!
//! For mirror's exact algebraic tag (`0x06`: the minimal polynomial, then
//! the root's index among its real roots): the minimal polynomial of an
//! eigenvalue is its *irreducible* factor over ℚ, and the square-free part
//! is only the product of those factors. [`Factorization::identify`] reads
//! a measured value against every factor's isolated roots and returns the
//! tag, [`Algebraic`] (minimal, index), or refuses. [`Roots::index_of`] on
//! a square-free part that isn't irreducible is an index among *its* roots,
//! not the tag.
//!
//! Polynomials are stored lowest degree first: `coefficients()[k]` is the
//! coefficient of xᵏ, and there are no trailing zeros, so the zero
//! polynomial has none and [`ZPoly::degree`] is `None`.

use std::ops::{Add, Mul, Neg, Range, Sub};

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

// --- Shared coefficient arithmetic -------------------------------------------

/// Drop trailing zeros: the leading coefficient, if any, is nonzero.
fn trim<T: Zero>(mut c: Vec<T>) -> Vec<T> {
    while c.last().is_some_and(Zero::is_zero) {
        c.pop();
    }
    c
}

fn add_coefficients<T>(a: &[T], b: &[T]) -> Vec<T>
where
    T: Clone + Zero,
    for<'x> &'x T: Add<&'x T, Output = T>,
{
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut c = long.to_vec();
    for (x, y) in c.iter_mut().zip(short) {
        *x = &*x + y;
    }
    trim(c)
}

fn negate<T>(a: &[T]) -> Vec<T>
where
    for<'x> &'x T: Neg<Output = T>,
{
    a.iter().map(|x| -x).collect()
}

fn mul_coefficients<T>(a: &[T], b: &[T]) -> Vec<T>
where
    T: Clone + Zero,
    for<'x> &'x T: Add<&'x T, Output = T> + Mul<&'x T, Output = T>,
{
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut c = vec![T::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            c[i + j] = &c[i + j] + &(x * y);
        }
    }
    trim(c)
}

fn derive<T>(a: &[T]) -> Vec<T>
where
    T: Zero + From<BigInt>,
    for<'x> &'x T: Mul<&'x T, Output = T>,
{
    let c = a
        .iter()
        .enumerate()
        .skip(1)
        .map(|(k, x)| x * &T::from(BigInt::from(k)))
        .collect();
    trim(c)
}

// --- ℤ[x] --------------------------------------------------------------------

/// A polynomial with integer coefficients, exactly: an element of ℤ[x].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ZPoly(Vec<BigInt>);

impl ZPoly {
    /// From coefficients lowest degree first (c₀, c₁, …). Trailing zeros are
    /// dropped.
    pub fn from_ascending(coefficients: impl IntoIterator<Item = BigInt>) -> Self {
        ZPoly(trim(coefficients.into_iter().collect()))
    }

    /// From coefficients leading first (cₙ, …, c₀), the way a polynomial is
    /// written. Leading zeros are dropped.
    pub fn from_descending(coefficients: impl IntoIterator<Item = BigInt>) -> Self {
        let mut c: Vec<BigInt> = coefficients.into_iter().collect();
        c.reverse();
        ZPoly(trim(c))
    }

    /// The coefficients, lowest degree first; none for the zero polynomial.
    pub fn coefficients(&self) -> &[BigInt] {
        &self.0
    }

    /// The degree; `None` for the zero polynomial.
    pub fn degree(&self) -> Option<usize> {
        self.0.len().checked_sub(1)
    }

    /// The leading coefficient; `None` for the zero polynomial.
    pub fn leading(&self) -> Option<&BigInt> {
        self.0.last()
    }

    /// Whether this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// The content: the gcd of the coefficients, ≥ 0 (0 for the zero
    /// polynomial).
    pub fn content(&self) -> BigInt {
        self.0.iter().fold(BigInt::zero(), |g, c| g.gcd(c))
    }

    /// The primitive part: divided by its content, and signed so the leading
    /// coefficient is positive. The zero polynomial's is itself.
    pub fn primitive(&self) -> ZPoly {
        let content = self.content();
        if content.is_zero() {
            return self.clone();
        }
        let content = if self.0.last().is_some_and(Signed::is_negative) {
            -content
        } else {
            content
        };
        ZPoly(self.0.iter().map(|c| c / &content).collect())
    }

    /// Divided by its content, sign kept: what a Sturm sequence may do.
    fn shrink(&self) -> ZPoly {
        let content = self.content();
        if content.is_zero() {
            return self.clone();
        }
        ZPoly(self.0.iter().map(|c| c / &content).collect())
    }

    /// The derivative p′.
    pub fn derivative(&self) -> ZPoly {
        ZPoly(derive(&self.0))
    }

    /// p(x) at an exact rational, by Horner's rule.
    pub fn eval(&self, x: &BigRational) -> BigRational {
        self.0.iter().rev().fold(BigRational::zero(), |acc, c| {
            acc * x + BigRational::from_integer(c.clone())
        })
    }

    /// The sign of p(x): −1, 0 or 1.
    fn sign_at(&self, x: &BigRational) -> i8 {
        sign(&self.eval(x))
    }

    /// Pseudo-division by `d`: (q, r) with lc(d)^(δ+1)·self = q·d + r and
    /// deg r < deg d, where δ = deg self − deg d. Every step stays in ℤ[x].
    /// When deg self < deg d, (0, self). `None` when `d` is zero.
    pub fn pseudo_div(&self, d: &ZPoly) -> Option<(ZPoly, ZPoly)> {
        let n = d.degree()?;
        let lc = d.leading().expect("d ≠ 0");
        let Some(m) = self.degree().filter(|&m| m >= n) else {
            return Some((ZPoly(Vec::new()), self.clone()));
        };
        let mut q = vec![BigInt::zero(); m - n + 1];
        let mut r = self.0.clone();
        let mut spare = m - n + 1;
        while r.len() > n {
            let k = r.len() - 1 - n;
            let s = r.last().expect("r ≠ 0").clone();
            for x in q.iter_mut() {
                *x *= lc;
            }
            q[k] += &s;
            for x in r.iter_mut() {
                *x *= lc;
            }
            for (i, di) in d.0.iter().enumerate() {
                r[i + k] -= &s * di;
            }
            r = trim(r);
            spare -= 1;
        }
        let scale = num_traits::pow(lc.clone(), spare);
        let scaled = |v: Vec<BigInt>| ZPoly(trim(v.into_iter().map(|x| x * &scale).collect()));
        Some((scaled(q), scaled(r)))
    }

    /// gcd over ℤ[x], by primitive pseudo-remainders: primitive, with a
    /// positive leading coefficient. gcd(p, 0) is p's primitive part, and
    /// gcd(0, 0) is 0. The integer content is not part of it.
    pub fn gcd(&self, other: &ZPoly) -> ZPoly {
        let (mut a, mut b) = (self.primitive(), other.primitive());
        if a.degree() < b.degree() {
            std::mem::swap(&mut a, &mut b);
        }
        while !b.is_zero() {
            let (_, r) = a.pseudo_div(&b).expect("b ≠ 0");
            a = b;
            b = r.primitive();
        }
        a
    }

    /// The square-free part p / gcd(p, p′), primitive with a positive
    /// leading coefficient: every root of p, each once. A nonzero constant's
    /// is 1, and 0's is 0.
    pub fn square_free(&self) -> ZPoly {
        if self.is_zero() {
            return self.clone();
        }
        let g = self.gcd(&self.derivative());
        let (quotient, _) = self.pseudo_div(&g).expect("g ≠ 0");
        quotient.primitive()
    }

    /// The same polynomial over ℚ.
    pub fn to_rational(&self) -> QPoly {
        QPoly(
            self.0
                .iter()
                .map(|c| BigRational::from_integer(c.clone()))
                .collect(),
        )
    }

    /// The real roots, isolated: of the square-free part, so each once.
    pub fn real_roots(&self) -> Roots {
        Roots::isolate(self.square_free())
    }
}

impl Add for &ZPoly {
    type Output = ZPoly;
    fn add(self, other: &ZPoly) -> ZPoly {
        ZPoly(add_coefficients(&self.0, &other.0))
    }
}

impl Sub for &ZPoly {
    type Output = ZPoly;
    fn sub(self, other: &ZPoly) -> ZPoly {
        ZPoly(add_coefficients(&self.0, &negate(&other.0)))
    }
}

impl Mul for &ZPoly {
    type Output = ZPoly;
    fn mul(self, other: &ZPoly) -> ZPoly {
        ZPoly(mul_coefficients(&self.0, &other.0))
    }
}

impl Neg for &ZPoly {
    type Output = ZPoly;
    fn neg(self) -> ZPoly {
        ZPoly(negate(&self.0))
    }
}

// --- ℚ[x] --------------------------------------------------------------------

/// A polynomial with rational coefficients, exactly: an element of ℚ[x], a
/// Euclidean domain.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QPoly(Vec<BigRational>);

impl QPoly {
    /// From coefficients lowest degree first. Trailing zeros are dropped.
    pub fn from_ascending(coefficients: impl IntoIterator<Item = BigRational>) -> Self {
        QPoly(trim(coefficients.into_iter().collect()))
    }

    /// From coefficients leading first. Leading zeros are dropped.
    pub fn from_descending(coefficients: impl IntoIterator<Item = BigRational>) -> Self {
        let mut c: Vec<BigRational> = coefficients.into_iter().collect();
        c.reverse();
        QPoly(trim(c))
    }

    /// The constant polynomial `c`.
    pub fn constant(c: BigRational) -> Self {
        QPoly(trim(vec![c]))
    }

    /// The coefficients, lowest degree first; none for the zero polynomial.
    pub fn coefficients(&self) -> &[BigRational] {
        &self.0
    }

    /// The degree; `None` for the zero polynomial.
    pub fn degree(&self) -> Option<usize> {
        self.0.len().checked_sub(1)
    }

    /// The leading coefficient; `None` for the zero polynomial.
    pub fn leading(&self) -> Option<&BigRational> {
        self.0.last()
    }

    /// Whether this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// Divided by its leading coefficient. The zero polynomial's is itself.
    pub fn monic(&self) -> QPoly {
        match self.0.last() {
            None => self.clone(),
            Some(lc) => QPoly(self.0.iter().map(|c| c / lc).collect()),
        }
    }

    /// The derivative p′.
    pub fn derivative(&self) -> QPoly {
        QPoly(derive(&self.0))
    }

    /// p(x) at an exact rational, by Horner's rule.
    pub fn eval(&self, x: &BigRational) -> BigRational {
        self.0
            .iter()
            .rev()
            .fold(BigRational::zero(), |acc, c| acc * x + c)
    }

    /// Division with remainder: (q, r) with self = q·d + r and
    /// deg r < deg d. `None` when `d` is zero.
    pub fn div_rem(&self, d: &QPoly) -> Option<(QPoly, QPoly)> {
        let n = d.degree()?;
        let lc = d.leading().expect("d ≠ 0");
        let Some(m) = self.degree().filter(|&m| m >= n) else {
            return Some((QPoly(Vec::new()), self.clone()));
        };
        let mut q = vec![BigRational::zero(); m - n + 1];
        let mut r = self.0.clone();
        while r.len() > n {
            let k = r.len() - 1 - n;
            let s = r.last().expect("r ≠ 0") / lc;
            for (i, di) in d.0.iter().enumerate() {
                r[i + k] -= &s * di;
            }
            q[k] = s;
            r = trim(r);
        }
        Some((QPoly(trim(q)), QPoly(r)))
    }

    /// The monic gcd, by Euclid's algorithm. gcd(p, 0) is p made monic.
    pub fn gcd(&self, other: &QPoly) -> QPoly {
        let (mut a, mut b) = (self.clone(), other.clone());
        while !b.is_zero() {
            let (_, r) = a.div_rem(&b).expect("b ≠ 0");
            a = b;
            b = r;
        }
        a.monic()
    }

    /// The primitive part over ℤ: denominators cleared, divided by the
    /// content, leading coefficient positive.
    pub fn primitive(&self) -> ZPoly {
        let lcm = self.0.iter().fold(BigInt::one(), |l, c| l.lcm(c.denom()));
        ZPoly::from_ascending(self.0.iter().map(|c| (c * &lcm).to_integer())).primitive()
    }
}

impl Add for &QPoly {
    type Output = QPoly;
    fn add(self, other: &QPoly) -> QPoly {
        QPoly(add_coefficients(&self.0, &other.0))
    }
}

impl Sub for &QPoly {
    type Output = QPoly;
    fn sub(self, other: &QPoly) -> QPoly {
        QPoly(add_coefficients(&self.0, &negate(&other.0)))
    }
}

impl Mul for &QPoly {
    type Output = QPoly;
    fn mul(self, other: &QPoly) -> QPoly {
        QPoly(mul_coefficients(&self.0, &other.0))
    }
}

impl Neg for &QPoly {
    type Output = QPoly;
    fn neg(self) -> QPoly {
        QPoly(negate(&self.0))
    }
}

// --- Real roots --------------------------------------------------------------

fn sign(x: &BigRational) -> i8 {
    match x.numer().sign() {
        Sign::Minus => -1,
        Sign::NoSign => 0,
        Sign::Plus => 1,
    }
}

/// A half-open rational interval (lo, hi]: where one real root lies.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    lo: BigRational,
    hi: BigRational,
}

impl Interval {
    /// The open end.
    pub fn lo(&self) -> &BigRational {
        &self.lo
    }

    /// The closed end.
    pub fn hi(&self) -> &BigRational {
        &self.hi
    }

    /// Whether lo < x ≤ hi.
    pub fn contains(&self, x: &BigRational) -> bool {
        &self.lo < x && x <= &self.hi
    }
}

/// What a measured value is, against the exact roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Located {
    /// Exactly one root is in the window: its index.
    Root(usize),
    /// Several roots are in the window: the match refuses, and names the
    /// candidates' indices rather than guessing among them.
    Ambiguous(Range<usize>),
    /// No root is in the window.
    Absent,
    /// The value or the radius isn't a finite measurement (a NaN, an
    /// infinity, a negative radius).
    Unmeasured,
}

/// The real roots of a square-free integer polynomial, isolated by a Sturm
/// sequence: disjoint rational intervals, one root each, in increasing
/// order. A root's index is its place in that order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roots {
    poly: ZPoly,
    sturm: Vec<ZPoly>,
    intervals: Vec<Interval>,
}

impl Roots {
    /// Isolate the real roots of `poly`, which must be square-free (every
    /// [`ZPoly::square_free`] is).
    fn isolate(poly: ZPoly) -> Roots {
        let sturm = sturm(&poly);
        let mut roots = Roots {
            poly,
            sturm,
            intervals: Vec::new(),
        };
        if roots.poly.degree().unwrap_or(0) == 0 {
            return roots;
        }
        // Cauchy: every root has |r| < 1 + max |cᵢ / cₙ| ≤ bound.
        let c = &roots.poly.0;
        let lc = c.last().expect("degree ≥ 1").abs();
        let largest = c.iter().map(Signed::abs).max().expect("nonempty");
        let bound = BigRational::from_integer(BigInt::one() + largest.div_ceil(&lc));
        let mut stack = vec![(-bound.clone(), bound)];
        // Depth first, upper half pushed first: intervals come out ascending.
        while let Some((lo, hi)) = stack.pop() {
            match roots.count_in(&lo, &hi) {
                0 => {}
                1 => roots.intervals.push(Interval { lo, hi }),
                _ => {
                    let mid = (&lo + &hi) / BigRational::from_integer(BigInt::from(2));
                    stack.push((mid.clone(), hi));
                    stack.push((lo, mid));
                }
            }
        }
        roots
    }

    /// The square-free polynomial whose roots these are.
    pub fn polynomial(&self) -> &ZPoly {
        &self.poly
    }

    /// The isolating intervals, in increasing order.
    pub fn intervals(&self) -> &[Interval] {
        &self.intervals
    }

    /// How many real roots.
    pub fn len(&self) -> usize {
        self.intervals.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Sign changes along the Sturm sequence at x, zeros skipped.
    fn variations(&self, signs: impl Iterator<Item = i8>) -> usize {
        let signs: Vec<i8> = signs.filter(|&s| s != 0).collect();
        signs.windows(2).filter(|w| w[0] != w[1]).count()
    }

    fn variations_at(&self, x: &BigRational) -> usize {
        self.variations(self.sturm.iter().map(|p| p.sign_at(x)))
    }

    fn variations_at_minus_infinity(&self) -> usize {
        self.variations(self.sturm.iter().map(|p| {
            let lc = sign(&BigRational::from_integer(
                p.leading().expect("Sturm terms ≠ 0").clone(),
            ));
            if p.degree().expect("≠ 0") % 2 == 0 {
                lc
            } else {
                -lc
            }
        }))
    }

    /// How many roots lie in (lo, hi]: V(lo) − V(hi), Sturm's theorem. At a
    /// root r the count equals the one just right of r, so the half-open
    /// interval is exact whether or not its ends are roots.
    pub fn count_in(&self, lo: &BigRational, hi: &BigRational) -> usize {
        if lo >= hi {
            return 0;
        }
        self.variations_at(lo) - self.variations_at(hi)
    }

    /// How many roots are strictly below x, and whether x is one.
    fn below(&self, x: &BigRational) -> (usize, bool) {
        // The zero polynomial vanishes everywhere, but isolates nothing.
        let at = !self.poly.is_zero() && self.poly.sign_at(x) == 0;
        let up_to = self.variations_at_minus_infinity() - self.variations_at(x);
        (up_to - usize::from(at), at)
    }

    /// The index of `x` among the roots, if `x` is one exactly.
    pub fn index_of(&self, x: &BigRational) -> Option<usize> {
        match self.below(x) {
            (index, true) => Some(index),
            _ => None,
        }
    }

    /// Which root a measured value is: the roots in the closed window
    /// [measured − radius, measured + radius], counted exactly (each float
    /// is the dyadic rational it is). One root: [`Located::Root`] with its
    /// index. Several: [`Located::Ambiguous`], a refusal naming them. None:
    /// [`Located::Absent`].
    pub fn locate(&self, measured: f64, radius: f64) -> Located {
        let (Some(m), Some(r)) = (
            BigRational::from_float(measured),
            BigRational::from_float(radius),
        ) else {
            return Located::Unmeasured;
        };
        if r.is_negative() {
            return Located::Unmeasured;
        }
        let (lo, hi) = (&m - &r, &m + &r);
        let (below, on_edge) = self.below(&lo);
        let count = self.count_in(&lo, &hi) + usize::from(on_edge);
        match count {
            0 => Located::Absent,
            1 => Located::Root(below),
            _ => Located::Ambiguous(below..below + count),
        }
    }
}

/// The Sturm sequence p, p′, −rem(p, p′), …: each term the negated
/// remainder of the two before, up to a positive factor (a pseudo-remainder
/// scaled back to the true remainder's sign, then divided by its content).
fn sturm(p: &ZPoly) -> Vec<ZPoly> {
    if p.is_zero() {
        return Vec::new();
    }
    let mut seq = vec![p.clone()];
    let mut next = p.derivative().shrink();
    while !next.is_zero() {
        seq.push(next);
        let (a, b) = (&seq[seq.len() - 2], &seq[seq.len() - 1]);
        let (_, r) = a.pseudo_div(b).expect("b ≠ 0");
        // prem = lc(b)^(δ+1) · rem: flip back when that factor is negative.
        let odd = (a.degree().expect("≠ 0") - b.degree().expect("≠ 0") + 1) % 2 == 1;
        let negative = b.leading().is_some_and(Signed::is_negative) && odd;
        next = if negative { r } else { -&r }.shrink();
    }
    seq
}

// --- Similarity: the Smith form of xI − A over ℚ[x] ---------------------------

/// The rows of a matrix aren't all as long as there are rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotSquare;

/// The invariant factors of a square matrix A: the Smith form of xI − A
/// over ℚ[x], a Euclidean domain. Monic, each dividing the next, and their
/// product is det(xI − A).
///
/// They are A's complete similarity invariant over ℚ: two matrices are
/// similar exactly when their invariant factors agree. The charpoly is only
/// their product, so it can't tell the shear [[1, 1], [0, 1]], with factors
/// (1, (x − 1)²), from I₂, with (x − 1, x − 1). The last factor is the
/// minimal polynomial; the nontrivial ones give the rational canonical form
/// ([`Invariants::rational_canonical`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Invariants {
    factors: Vec<QPoly>,
}

impl Invariants {
    /// The invariant factors of an integer matrix, given by rows.
    pub fn of(a: &[Vec<BigInt>]) -> Result<Self, NotSquare> {
        let a: Vec<Vec<BigRational>> = a
            .iter()
            .map(|r| {
                r.iter()
                    .map(|x| BigRational::from_integer(x.clone()))
                    .collect()
            })
            .collect();
        Self::of_rational(&a)
    }

    /// The invariant factors of a rational matrix, given by rows: by
    /// elimination on xI − A over ℚ[x]. Pivot on an entry of least degree,
    /// clear its row and column by division with remainder (a remainder
    /// becomes the next, lower-degree pivot), and when the pivot doesn't
    /// divide an entry left, add that entry's row to the pivot's and go
    /// again. Each pass lowers the pivot's degree, so it ends.
    pub fn of_rational(a: &[Vec<BigRational>]) -> Result<Self, NotSquare> {
        let n = a.len();
        if a.iter().any(|r| r.len() != n) {
            return Err(NotSquare);
        }
        let mut m: Vec<Vec<QPoly>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        let mut c = vec![-&a[i][j]];
                        if i == j {
                            c.push(BigRational::one());
                        }
                        QPoly::from_ascending(c)
                    })
                    .collect()
            })
            .collect();
        let mut factors = Vec::with_capacity(n);
        for t in 0..n {
            loop {
                let smallest = (t..n)
                    .flat_map(|i| (t..n).map(move |j| (i, j)))
                    .filter(|&(i, j)| !m[i][j].is_zero())
                    .min_by_key(|&(i, j)| m[i][j].degree());
                let Some((p, q)) = smallest else { break };
                m.swap(t, p);
                for row in m.iter_mut() {
                    row.swap(t, q);
                }
                let mut clear = true;
                // Rows and columns before t are already zero off the diagonal.
                let pivot_row = m[t].clone();
                for row in m.iter_mut().skip(t + 1) {
                    let (quotient, _) = row[t].div_rem(&pivot_row[t]).expect("pivot ≠ 0");
                    for (x, p) in row[t..].iter_mut().zip(&pivot_row[t..]) {
                        *x = &*x - &(p * &quotient);
                    }
                    clear &= row[t].is_zero();
                }
                for j in t + 1..n {
                    let (quotient, _) = m[t][j].div_rem(&m[t][t]).expect("pivot ≠ 0");
                    for row in m.iter_mut().skip(t) {
                        let x = &row[j] - &(&row[t] * &quotient);
                        row[j] = x;
                    }
                    clear &= m[t][j].is_zero();
                }
                if !clear {
                    continue;
                }
                let pivot = &m[t][t];
                let divides = |x: &QPoly| x.div_rem(pivot).expect("pivot ≠ 0").1.is_zero();
                let stray = (t + 1..n).find(|&i| !m[i][t + 1..].iter().all(divides));
                match stray {
                    Some(i) => {
                        let stray_row = m[i].clone();
                        for (x, s) in m[t][t..].iter_mut().zip(&stray_row[t..]) {
                            *x = &*x + s;
                        }
                    }
                    None => break,
                }
            }
            factors.push(m[t][t].monic());
        }
        Ok(Invariants { factors })
    }

    /// d₁ | d₂ | … | dₙ: one per row, monic, the units (1) first.
    pub fn factors(&self) -> &[QPoly] {
        &self.factors
    }

    /// The factors of degree ≥ 1: the companion blocks of the rational
    /// canonical form. The last is the minimal polynomial.
    pub fn nontrivial(&self) -> &[QPoly] {
        let units = self
            .factors
            .iter()
            .take_while(|f| f.degree() == Some(0))
            .count();
        &self.factors[units..]
    }

    /// d₁ · d₂ ⋯ dₙ = det(xI − A), the characteristic polynomial. The empty
    /// matrix's is 1.
    pub fn product(&self) -> QPoly {
        self.factors
            .iter()
            .fold(QPoly::constant(BigRational::one()), |acc, f| &acc * f)
    }

    /// The rational canonical form, by rows: the companion matrix of each
    /// nontrivial factor, block diagonal in order. The companion of
    /// xᵏ + cₖ₋₁xᵏ⁻¹ + … + c₀ has ones below its diagonal and −c₀, …, −cₖ₋₁
    /// down its last column. It is similar to A, and has A's factors.
    pub fn rational_canonical(&self) -> Vec<Vec<BigRational>> {
        let n = self.factors.len();
        let mut form = vec![vec![BigRational::zero(); n]; n];
        let mut at = n - self
            .nontrivial()
            .iter()
            .map(|f| f.0.len() - 1)
            .sum::<usize>();
        for f in self.nontrivial() {
            let k = f.0.len() - 1;
            for i in 0..k {
                if i + 1 < k {
                    form[at + i + 1][at + i] = BigRational::one();
                }
                form[at + i][at + k - 1] = -&f.0[i];
            }
            at += k;
        }
        form
    }
}

// --- Factoring over ℚ: Zassenhaus --------------------------------------------

/// How many subset trials Zassenhaus recombination may make before it
/// stops and says so ([`Factored::Uncertified`]).
///
/// An **evaluation** budget: it may change the answer from certified to
/// uncertified, so it is a count of trials, the same on every machine, and
/// never a clock or a memory threshold (never `brain::Capacity`). With the
/// `bundle` feature, `flux::pillar::Budget` converts into it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Budget(pub usize);

#[cfg(feature = "bundle")]
impl From<crate::flux::pillar::Budget> for Budget {
    fn from(budget: crate::flux::pillar::Budget) -> Self {
        Budget(budget.0)
    }
}

/// The polynomial is zero or has a repeated root: [`ZPoly::factor`] wants a
/// square-free one ([`ZPoly::square_free`] makes it so).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotSquareFree;

/// What [`ZPoly::factor`] knows, within its budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Factored {
    /// Every irreducible factor over ℚ, and their product is the input.
    Certified(Factorization),
    /// The budget ran out. `irreducible` are true irreducible factors (each
    /// found at the least subset size still possible, so none splits), and
    /// `unresolved` is the rest, of degree ≥ 1, **not** certified either
    /// way. Their product is still the input. Uncertified is never wrong.
    Uncertified {
        /// Irreducible factors found before the budget ran out, in the
        /// canonical order.
        irreducible: Vec<ZPoly>,
        /// The cofactor still to factor: maybe irreducible, maybe not.
        unresolved: ZPoly,
    },
}

/// A certified factorization over ℚ of a square-free integer polynomial:
/// primitive irreducible factors with positive leading coefficients, in
/// canonical order (by degree, then coefficients leading first), whose
/// product is the input's primitive part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Factorization {
    factors: Vec<ZPoly>,
    roots: Vec<Roots>,
}

/// An exact real algebraic number, the way mirror's `0x06` tag reads it:
/// its **minimal polynomial** (primitive, positive leading coefficient)
/// and its **index among that polynomial's real roots**, in increasing
/// order. The byte encoding is mirror's; this is the value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Algebraic {
    minimal: ZPoly,
    index: usize,
}

/// What a measured value is, against a factorization's exact roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Identified {
    /// Exactly one root of one factor is in the window.
    Algebraic(Algebraic),
    /// Several roots are in the window, across one or more factors: the
    /// reading refuses and names them all, in factor order.
    Ambiguous(Vec<Algebraic>),
    /// No root is in the window.
    Absent,
    /// The value or the radius isn't a finite measurement.
    Unmeasured,
}

impl Algebraic {
    /// The tag (minimal, index). Not checked: a tag read off a certified
    /// [`Factorization`] has an irreducible `minimal`; one decoded from
    /// bytes is what the bytes say.
    pub fn new(minimal: ZPoly, index: usize) -> Self {
        Algebraic { minimal, index }
    }

    /// The minimal polynomial.
    pub fn minimal(&self) -> &ZPoly {
        &self.minimal
    }

    /// The index among the minimal polynomial's real roots, ascending.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The isolating interval (lo, hi] of the root: exactly one real root
    /// of the minimal polynomial lies in it. `None` when the index is past
    /// the last real root.
    pub fn isolating(&self) -> Option<Interval> {
        self.minimal.real_roots().intervals.get(self.index).cloned()
    }
}

impl Factorization {
    fn new(factors: Vec<ZPoly>) -> Self {
        let factors = canonical(factors);
        // Irreducible over ℚ, so square-free: isolation runs on each as is.
        let roots = factors.iter().map(|f| Roots::isolate(f.clone())).collect();
        Factorization { factors, roots }
    }

    /// The irreducible factors, in canonical order. A constant's are none.
    pub fn factors(&self) -> &[ZPoly] {
        &self.factors
    }

    /// Their product: the input's primitive part.
    pub fn product(&self) -> ZPoly {
        self.factors
            .iter()
            .fold(ZPoly(vec![BigInt::one()]), |acc, f| &acc * f)
    }

    /// Which exact algebraic number a measured value is: the roots of every
    /// factor in the closed window [measured − radius, measured + radius],
    /// counted exactly ([`Roots::locate`] on each). One root: its tag, the
    /// factor as minimal polynomial and the root's index among that
    /// factor's real roots. Several: a refusal naming them.
    pub fn identify(&self, measured: f64, radius: f64) -> Identified {
        let mut hits = Vec::new();
        for (f, roots) in self.factors.iter().zip(&self.roots) {
            let tag = |index| Algebraic::new(f.clone(), index);
            match roots.locate(measured, radius) {
                Located::Unmeasured => return Identified::Unmeasured,
                Located::Absent => {}
                Located::Root(i) => hits.push(tag(i)),
                Located::Ambiguous(range) => hits.extend(range.map(tag)),
            }
        }
        match hits.len() {
            0 => Identified::Absent,
            1 => Identified::Algebraic(hits.pop().expect("one")),
            _ => Identified::Ambiguous(hits),
        }
    }
}

/// The canonical order of factors: by degree, then coefficients leading
/// first.
fn canonical(mut factors: Vec<ZPoly>) -> Vec<ZPoly> {
    factors.sort_by(|a, b| {
        a.degree()
            .cmp(&b.degree())
            .then_with(|| a.0.iter().rev().cmp(b.0.iter().rev()))
    });
    factors
}

/// How many good primes to try; the one with the fewest factors mod p wins
/// (fewer factors, smaller subset search).
const PRIMES_TRIED: usize = 5;

impl ZPoly {
    /// Factor into irreducibles over ℚ, exactly: Zassenhaus.
    ///
    /// The input must be square-free (else [`NotSquareFree`]). Its content
    /// and sign are units over ℚ, so the primitive part is what gets
    /// factored, and what the factors multiply to.
    ///
    /// 1. **A good prime p**: p ∤ lc(f), and f mod p is still square-free.
    ///    The first five such primes are tried, in order, and
    ///    the one with the fewest factors mod p is kept (the first, on a
    ///    tie). There are finitely many bad primes (those dividing lc(f) or
    ///    the discriminant), so this ends.
    /// 2. **Factor mod p** by Berlekamp: the null space of Q − I, Q the
    ///    matrix of the Frobenius g ↦ gᵖ on 𝔽ₚ[x]/(f), has dimension r, the
    ///    number of irreducible factors; gcd(g, v − s) over its basis
    ///    vectors v and every s ∈ 𝔽ₚ splits them all. Deterministic
    ///    outright: no random choice is made, so no seed is needed.
    /// 3. **Hensel-lift** the monic factors to pᵏ, one p-adic digit at a
    ///    time (linear lifting), with pᵏ > 2B for the Mignotte bound
    ///    B = |lc(f)| · 2ⁿ · ⌈‖f‖₂⌉: a factor g of f over ℤ of degree
    ///    m ≤ n has |gᵢ| ≤ C(m, i)·M(g) ≤ 2ⁿ‖f‖₂ (M the Mahler measure,
    ///    M(g) ≤ M(f) ≤ ‖f‖₂), and the candidate lc(f)/lc(g)·g is at most
    ///    |lc(f)| times that. So every true factor's candidate is exact in
    ///    the symmetric range (−pᵏ/2, pᵏ/2].
    /// 4. **Recombine** (Zassenhaus): for subsets S of the lifted factors,
    ///    by size s = 1, 2, … up to half of what is left, the candidate is
    ///    the primitive part of lc(f)·∏ₛ hᵢ mod pᵏ, symmetric; if it divides
    ///    f exactly over ℤ it is a factor, its subset is removed and f is
    ///    divided by it. What is left when no subset of size ≤ half remains
    ///    is irreducible.
    ///
    /// **Cost, and the budget.** A subset trial is a product mod pᵏ and a
    /// trial division. With r factors mod p there are at most
    /// Σ_{s ≤ r/2} C(r, s) ≤ 2ʳ⁻¹ trials: exponential in r. r is small for
    /// most polynomials, but not all: the Swinnerton-Dyer polynomial Sₘ
    /// (the minimal polynomial of √2 + √3 + … + √pₘ, degree 2ᵐ) splits into
    /// factors of degree ≤ 2 mod *every* prime, so r ≥ 2ᵐ⁻¹ and the search
    /// is ~2^(2ᵐ⁻¹ − 1): fine at m = 3, hopeless by m = 6. So the trials
    /// are counted against `budget`, and running out returns
    /// [`Factored::Uncertified`] with what is known, never a guess. A prime
    /// where f stays irreducible (r = 1) needs no trial at all.
    ///
    /// van Hoeij's algorithm replaces the subset search with lattice
    /// reduction (LLL on the traces of the lifted factors, a knapsack
    /// lattice), which recombines in polynomial time; that is a later step.
    ///
    /// **The certificate.** The product of the factors is the input's
    /// primitive part; that is checked exactly, here, before anything is
    /// returned. Irreducibility has no cheap independent check: the
    /// evidence is the algorithm (every smaller subset was tried and
    /// failed, with the lift past the bound), and the tests confirm it by
    /// exhaustion on small cases.
    pub fn factor(&self, budget: Budget) -> Result<Factored, NotSquareFree> {
        if self.is_zero() || self.gcd(&self.derivative()).degree() != Some(0) {
            return Err(NotSquareFree);
        }
        let f = self.primitive();
        let n = f.degree().expect("nonzero");
        if n == 0 {
            return Ok(Factored::Certified(Factorization::new(Vec::new())));
        }
        if n == 1 {
            return Ok(Factored::Certified(Factorization::new(vec![f])));
        }
        let (p, modular) = good_prime(&f);
        // ⌈‖f‖₂⌉ ≤ ⌊√Σcᵢ²⌋ + 1, then B = |lc|·2ⁿ·⌈‖f‖₂⌉.
        let norm: BigInt = f.0.iter().map(|c| c * c).sum::<BigInt>().sqrt() + 1;
        let bound = f.leading().expect("n ≥ 1").abs() * (BigInt::one() << n) * norm;
        let prime = BigInt::from(p);
        let (mut k, mut modulus) = (1u32, prime.clone());
        while modulus <= &bound * 2 {
            modulus *= &prime;
            k += 1;
        }
        let lifted = hensel(&f, &modular, p, k, &modulus);
        let factored = recombine(f.clone(), lifted, &modulus, budget);
        // The certificate: the product is the input, exactly.
        let product = match &factored {
            Factored::Certified(fs) => fs.product(),
            Factored::Uncertified {
                irreducible,
                unresolved,
            } => irreducible
                .iter()
                .fold(unresolved.clone(), |acc, g| &acc * g),
        };
        assert_eq!(product, f, "the factors' product is the input");
        Ok(factored)
    }

    /// The exact quotient by `d` in ℤ[x], if `d` divides `self` there.
    fn div_exact(&self, d: &ZPoly) -> Option<ZPoly> {
        let n = d.degree()?;
        let lc = d.leading().expect("d ≠ 0");
        let m = match self.degree() {
            None => return Some(ZPoly(Vec::new())),
            Some(m) if m < n => return None,
            Some(m) => m,
        };
        let mut r = self.0.clone();
        let mut q = vec![BigInt::zero(); m - n + 1];
        for k in (0..=m - n).rev() {
            let (s, rest) = r[k + n].div_rem(lc);
            if !rest.is_zero() {
                return None;
            }
            for (i, di) in d.0.iter().enumerate() {
                r[i + k] -= &s * di;
            }
            q[k] = s;
        }
        r.iter().all(Zero::is_zero).then(|| ZPoly(trim(q)))
    }
}

/// The first [`PRIMES_TRIED`] good primes for f, and the one with the
/// fewest factors mod p: (p, f's monic irreducible factors mod p).
fn good_prime(f: &ZPoly) -> (u64, Vec<Vec<u64>>) {
    let lc = f.leading().expect("nonzero");
    let mut best: Option<(u64, Vec<Vec<u64>>)> = None;
    let mut good = 0;
    for p in (2u64..).filter(|&p| (2..p).take_while(|d| d * d <= p).all(|d| p % d != 0)) {
        if (lc % p).is_zero() {
            continue;
        }
        let fp = modp::from_z(&f.0, p);
        if modp::gcd(&fp, &modp::derivative(&fp, p), p).len() > 1 {
            continue;
        }
        let factors = modp::berlekamp(&modp::monic(&fp, p), p);
        let fewer = best.as_ref().is_none_or(|(_, b)| factors.len() < b.len());
        if fewer {
            best = Some((p, factors));
        }
        good += 1;
        if good == PRIMES_TRIED || best.as_ref().is_some_and(|(_, b)| b.len() == 1) {
            break;
        }
    }
    best.expect("a square-free polynomial has a good prime")
}

/// From f ≡ lc(f)·∏ gᵢ (mod p), the gᵢ monic, irreducible and distinct:
/// monic hᵢ ≡ gᵢ (mod p) with f ≡ lc(f)·∏ hᵢ (mod pᵏ), coefficients in
/// [0, pᵏ).
fn hensel(f: &ZPoly, modular: &[Vec<u64>], p: u64, k: u32, modulus: &BigInt) -> Vec<ZPoly> {
    let lc = f.leading().expect("nonzero");
    let inverse = lc.extended_gcd(modulus).x.mod_floor(modulus);
    let target = ZPoly(trim(
        f.0.iter()
            .map(|c| (c * &inverse).mod_floor(modulus))
            .collect(),
    ));
    let mut lifted = Vec::with_capacity(modular.len());
    let mut target = target;
    for (i, g0) in modular.iter().enumerate() {
        if i + 1 == modular.len() {
            lifted.push(target);
            break;
        }
        let h0 = modular[i + 1..]
            .iter()
            .fold(vec![1u64], |acc, g| modp::mul(&acc, g, p));
        let (g, h) = lift(&target, g0, &h0, p, k);
        lifted.push(g);
        target = h;
    }
    lifted
}

/// One two-factor Hensel lift: from monic `target` ≡ g₀·h₀ (mod p), g₀ and
/// h₀ monic and coprime, monic g ≡ g₀, h ≡ h₀ with target ≡ g·h (mod pᵏ).
/// With s·g₀ + t·h₀ = 1 mod p, each step takes the next digit
/// e = (target − g·h)/pʲ mod p and adds pʲ·τ to g and pʲ·σ to h, where
/// τ = t·e mod g₀ and σ = (e − τ·h₀)/g₀: then σ·g₀ + τ·h₀ = e, and
/// deg τ < deg g₀, deg σ < deg h₀ keep both monic.
fn lift(target: &ZPoly, g0: &[u64], h0: &[u64], p: u64, k: u32) -> (ZPoly, ZPoly) {
    let (_, _, t) = modp::ext_gcd(g0, h0, p);
    let up = |v: &[u64]| ZPoly(v.iter().map(|&c| BigInt::from(c)).collect());
    let (mut g, mut h) = (up(g0), up(h0));
    let mut pj = BigInt::from(p);
    for _ in 1..k {
        let diff = target - &(&g * &h);
        let digits: Vec<BigInt> = diff.0.iter().map(|c| c / &pj).collect();
        let e = modp::from_z(&digits, p);
        let tau = modp::div_rem(&modp::mul(&t, &e, p), g0, p).1;
        let sigma = modp::div_rem(&modp::sub(&e, &modp::mul(&tau, h0, p), p), g0, p).0;
        g = &g + &(&up(&tau) * &ZPoly(vec![pj.clone()]));
        h = &h + &(&up(&sigma) * &ZPoly(vec![pj.clone()]));
        pj *= p;
    }
    (g, h)
}

/// c mod m, in the symmetric range (−m/2, m/2].
fn symmetric(c: &BigInt, m: &BigInt) -> BigInt {
    let r = c.mod_floor(m);
    if &r * 2 > *m {
        r - m
    } else {
        r
    }
}

/// Zassenhaus recombination of the lifted factors, within `budget` trials.
fn recombine(mut f: ZPoly, mut lifted: Vec<ZPoly>, modulus: &BigInt, budget: Budget) -> Factored {
    let mut found = Vec::new();
    let mut trials = 0;
    let mut s = 1;
    'size: while 2 * s <= lifted.len() {
        let mut subset: Vec<usize> = (0..s).collect();
        loop {
            if trials == budget.0 {
                return Factored::Uncertified {
                    irreducible: canonical(found),
                    unresolved: f,
                };
            }
            trials += 1;
            let lc = f.leading().expect("deg f ≥ 1").clone();
            let candidate = subset.iter().fold(ZPoly(vec![lc]), |acc, &i| {
                let c = &acc * &lifted[i];
                ZPoly(trim(c.0.iter().map(|x| symmetric(x, modulus)).collect()))
            });
            let g = candidate.primitive();
            if let Some(q) = f.div_exact(&g).filter(|_| g.degree() >= Some(1)) {
                found.push(g);
                f = q;
                for &i in subset.iter().rev() {
                    lifted.remove(i);
                }
                continue 'size;
            }
            if !next_subset(&mut subset, lifted.len()) {
                break;
            }
        }
        s += 1;
    }
    found.push(f);
    Factored::Certified(Factorization::new(found))
}

/// The next s-subset of 0..n in lexicographic order; false after the last.
fn next_subset(subset: &mut [usize], n: usize) -> bool {
    let s = subset.len();
    let Some(i) = (0..s).rev().find(|&i| subset[i] < n - s + i) else {
        return false;
    };
    subset[i] += 1;
    for j in i + 1..s {
        subset[j] = subset[j - 1] + 1;
    }
    true
}

/// 𝔽ₚ[x], for a word-sized prime p < 2³²: coefficients in [0, p), lowest
/// degree first, no trailing zeros.
mod modp {
    use num_bigint::BigInt;
    use num_integer::Integer;
    use num_traits::ToPrimitive;

    fn trim(mut a: Vec<u64>) -> Vec<u64> {
        while a.last() == Some(&0) {
            a.pop();
        }
        a
    }

    pub(super) fn from_z(c: &[BigInt], p: u64) -> Vec<u64> {
        let m = BigInt::from(p);
        trim(
            c.iter()
                .map(|x| x.mod_floor(&m).to_u64().expect("< p"))
                .collect(),
        )
    }

    fn pow(mut a: u64, mut e: u64, p: u64) -> u64 {
        let mut r = 1;
        a %= p;
        while e > 0 {
            if e & 1 == 1 {
                r = r * a % p;
            }
            a = a * a % p;
            e >>= 1;
        }
        r
    }

    fn inv(a: u64, p: u64) -> u64 {
        pow(a, p - 2, p)
    }

    pub(super) fn sub(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
        let n = a.len().max(b.len());
        let at = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
        trim((0..n).map(|i| (at(a, i) + p - at(b, i)) % p).collect())
    }

    pub(super) fn mul(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut c = vec![0; a.len() + b.len() - 1];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                c[i + j] = (c[i + j] + x * y) % p;
            }
        }
        trim(c)
    }

    pub(super) fn monic(a: &[u64], p: u64) -> Vec<u64> {
        match a.last() {
            None => Vec::new(),
            Some(&lc) => {
                let i = inv(lc, p);
                a.iter().map(|x| x * i % p).collect()
            }
        }
    }

    pub(super) fn derivative(a: &[u64], p: u64) -> Vec<u64> {
        trim(
            a.iter()
                .enumerate()
                .skip(1)
                .map(|(k, x)| (k as u64 % p) * x % p)
                .collect(),
        )
    }

    /// (q, r) with a = q·b + r, deg r < deg b; b ≠ 0.
    pub(super) fn div_rem(a: &[u64], b: &[u64], p: u64) -> (Vec<u64>, Vec<u64>) {
        let n = b.len() - 1;
        if a.len() <= n {
            return (Vec::new(), a.to_vec());
        }
        let i = inv(b[n], p);
        let mut r = a.to_vec();
        let mut q = vec![0; a.len() - n];
        for k in (0..q.len()).rev() {
            let s = r[k + n] * i % p;
            q[k] = s;
            for (j, bj) in b.iter().enumerate() {
                r[k + j] = (r[k + j] + p - s * bj % p) % p;
            }
        }
        (trim(q), trim(r))
    }

    /// The monic gcd; gcd(0, 0) = 0.
    pub(super) fn gcd(a: &[u64], b: &[u64], p: u64) -> Vec<u64> {
        let (mut a, mut b) = (a.to_vec(), b.to_vec());
        while !b.is_empty() {
            let r = div_rem(&a, &b, p).1;
            a = b;
            b = r;
        }
        monic(&a, p)
    }

    /// (g, s, t) with s·a + t·b = g, the monic gcd.
    pub(super) fn ext_gcd(a: &[u64], b: &[u64], p: u64) -> (Vec<u64>, Vec<u64>, Vec<u64>) {
        let (mut r0, mut r1) = (a.to_vec(), b.to_vec());
        let (mut s0, mut s1) = (vec![1], Vec::new());
        let (mut t0, mut t1) = (Vec::new(), vec![1]);
        while !r1.is_empty() {
            let (q, r) = div_rem(&r0, &r1, p);
            let s = sub(&s0, &mul(&q, &s1, p), p);
            let t = sub(&t0, &mul(&q, &t1, p), p);
            (r0, r1) = (r1, r);
            (s0, s1) = (s1, s);
            (t0, t1) = (t1, t);
        }
        let i = inv(*r0.last().expect("a or b ≠ 0"), p);
        let scale = |v: &[u64]| trim(v.iter().map(|x| x * i % p).collect());
        (scale(&r0), scale(&s0), scale(&t0))
    }

    /// The left null space of `rows` (n × n), as basis vectors.
    fn null_space(rows: &[Vec<u64>], p: u64) -> Vec<Vec<u64>> {
        let n = rows.len();
        // v·M = 0 ⇔ Mᵀ·v = 0: eliminate on the transpose.
        let mut a: Vec<Vec<u64>> = (0..n)
            .map(|j| (0..n).map(|i| rows[i][j]).collect())
            .collect();
        let mut pivots: Vec<Option<usize>> = vec![None; n];
        let mut row = 0;
        for col in 0..n {
            let Some(r) = (row..n).find(|&r| a[r][col] != 0) else {
                continue;
            };
            a.swap(row, r);
            let i = inv(a[row][col], p);
            for x in a[row].iter_mut() {
                *x = *x * i % p;
            }
            let pivot = a[row].clone();
            for (r, line) in a.iter_mut().enumerate() {
                let f = line[col];
                if r != row && f != 0 {
                    for (x, y) in line.iter_mut().zip(&pivot) {
                        *x = (*x + p - f * y % p) % p;
                    }
                }
            }
            pivots[col] = Some(row);
            row += 1;
        }
        (0..n)
            .filter(|&free| pivots[free].is_none())
            .map(|free| {
                let mut v = vec![0; n];
                v[free] = 1;
                for (col, pivot) in pivots.iter().enumerate() {
                    if let Some(r) = pivot {
                        v[col] = (p - a[*r][free]) % p;
                    }
                }
                v
            })
            .collect()
    }

    /// Berlekamp: the monic irreducible factors of a monic square-free f of
    /// degree ≥ 1 over 𝔽ₚ. The rows of Q are xⁱᵖ mod f; g = Σvᵢxⁱ has
    /// gᵖ ≡ g (mod f) exactly when v·(Q − I) = 0, and that space has
    /// dimension r, the number of factors. For each basis vector v, f's
    /// pieces split as ∏ₛ gcd(piece, v − s), s ∈ 𝔽ₚ.
    pub(super) fn berlekamp(f: &[u64], p: u64) -> Vec<Vec<u64>> {
        let n = f.len() - 1;
        if n == 1 {
            return vec![f.to_vec()];
        }
        let mulmod = |a: &[u64], b: &[u64]| div_rem(&mul(a, b, p), f, p).1;
        let (mut xp, mut base, mut e) = (vec![1], vec![0, 1], p);
        while e > 0 {
            if e & 1 == 1 {
                xp = mulmod(&xp, &base);
            }
            base = mulmod(&base, &base);
            e >>= 1;
        }
        let mut rows = Vec::with_capacity(n);
        let mut power = vec![1];
        for i in 0..n {
            let mut row: Vec<u64> = (0..n).map(|j| power.get(j).copied().unwrap_or(0)).collect();
            row[i] = (row[i] + p - 1) % p;
            rows.push(row);
            power = mulmod(&power, &xp);
        }
        let basis = null_space(&rows, p);
        let r = basis.len();
        let mut factors = vec![f.to_vec()];
        for v in basis.into_iter().map(trim).filter(|v| v.len() > 1) {
            if factors.len() == r {
                break;
            }
            let mut next = Vec::with_capacity(r);
            for g in factors {
                if g.len() <= 2 {
                    next.push(g);
                    continue;
                }
                let mut rest = g;
                for s in 0..p {
                    if rest.len() <= 1 {
                        break;
                    }
                    let h = gcd(&rest, &sub(&v, &[s], p), p);
                    if h.len() > 1 {
                        rest = div_rem(&rest, &h, p).0;
                        next.push(h);
                    }
                }
            }
            factors = next;
        }
        factors
    }
}

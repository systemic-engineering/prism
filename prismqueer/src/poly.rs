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
//! For mirror's exact algebraic tag (`0x06`: the minimal polynomial, then
//! the root's index among its real roots), one step is still missing: the
//! **minimal polynomial** of an eigenvalue is its *irreducible* factor over
//! ℚ, and the square-free part is only the product of those factors. Until
//! factoring over ℚ (Zassenhaus, or van Hoeij with LLL) lands, the index
//! here is among the square-free part's roots, which is the tag only when
//! that part is irreducible, and this module cannot certify that it is.
//! With the factor in hand, its index is the same computation on it.
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

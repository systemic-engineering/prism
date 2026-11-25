//! S3*: the sampled audit of a verdict cache, stated exactly.
//!
//! The sporadic audit: besides the verdicts it is handed, the control
//! function now and then recomputes one itself. In *Brain of the Firm*
//! (Beer, 1981) the audit sits inside S3; the name "3*" is OPEN until
//! *Heart of Enterprise* is read (Mara). For a verdict cache
//! that is Mara's sampled recompute: n of the N unchanged items, uniformly,
//! without replacement. This module is the maths of what such a sample can
//! say, in exact rationals (`docs/spec/audit.md`):
//!
//! - [`miss_probability`]: if k of N items are wrong, the chance a sample of
//!   n misses all of them, C(N−k, n) / C(N, n).
//! - [`bounds_rate`]: whether n clean recomputes establish "the drift rate is
//!   below p at confidence c", which holds iff (1 − p)ⁿ ≤ 1 − c.
//! - [`smallest_sample`]: the least such n.
//! - [`audit`]: the gate's verdict over a sample. A clean sample is
//!   `Partial`, carrying the [`Bound`] it stands on; only a full recompute is
//!   `Success`; a drifted item found is `Failure`.
//!
//! **S3* bounds a drift rate. It doesn't catch a single wrong item:** at a
//! 5 % sample one wrong item is missed 95 % of the time. Changed items are
//! covered by recomputing all of them (Mara's rule), not by this.
//!
//! No float enters: every probability is a reduced `BigRational`, every
//! comparison is exact.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};
use terni::{Imperfect, Loss};

/// An input the audit maths refuses instead of answering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// n > N: a sample without replacement can't be larger than the population.
    SampleExceedsPopulation { sample: u64, population: u64 },
    /// k > N: more bad items than items.
    BadExceedsPopulation { bad: u64, population: u64 },
    /// More drifted items found than were sampled.
    DriftExceedsSample { drifted: u64, sample: u64 },
    /// A claim needs 0 < rate < 1 and 0 < confidence < 1. (The caller holds
    /// both; the refusal doesn't copy them, so it stays small.)
    NotAClaim,
}

/// P(a uniform sample of `n` of `population`, without replacement, misses
/// all `bad` items) = C(N−k, n) / C(N, n), exactly.
///
/// Computed as the product ∏ᵢ₌₀ⁿ⁻¹ (N−k−i)/(N−i), which is the same ratio
/// with the common factorials cancelled; it is 0 as soon as n > N − k (the
/// sample can't avoid every bad item). Refuses n > N and k > N.
pub fn miss_probability(population: u64, bad: u64, n: u64) -> Result<BigRational, Refusal> {
    if n > population {
        return Err(Refusal::SampleExceedsPopulation {
            sample: n,
            population,
        });
    }
    if bad > population {
        return Err(Refusal::BadExceedsPopulation { bad, population });
    }
    let good = population - bad;
    if n > good {
        return Ok(BigRational::zero());
    }
    let mut num = BigInt::one();
    let mut den = BigInt::one();
    for i in 0..n {
        num *= good - i;
        den *= population - i;
    }
    Ok(BigRational::new(num, den))
}

fn claim(p: &BigRational, c: &BigRational) -> Result<(), Refusal> {
    let open = |x: &BigRational| x > &BigRational::zero() && x < &BigRational::one();
    if open(p) && open(c) {
        Ok(())
    } else {
        Err(Refusal::NotAClaim)
    }
}

fn pow(base: &BigInt, mut e: u64) -> BigInt {
    let mut acc = BigInt::one();
    let mut b = base.clone();
    while e > 0 {
        if e & 1 == 1 {
            acc *= &b;
        }
        e >>= 1;
        if e > 0 {
            b = &b * &b;
        }
    }
    acc
}

/// (1 − p)ⁿ ≤ 1 − c, compared exactly. `p`, `c` already checked.
fn holds(n: u64, p: &BigRational, c: &BigRational) -> bool {
    let keep = BigRational::one() - p; // a/b, reduced, 0 < a < b
    let miss = BigRational::one() - c; // m/d, reduced
                                       // (a/b)ⁿ ≤ m/d  ⟺  aⁿ·d ≤ m·bⁿ   (b, d > 0)
    pow(keep.numer(), n) * miss.denom() <= miss.numer() * pow(keep.denom(), n)
}

/// Whether `n_clean` clean recomputes establish "the drift rate is below `p`
/// at confidence `c`": (1 − p)ⁿ ≤ 1 − c, exactly.
///
/// If the rate were p or more, n uniform draws would all come back clean
/// with probability at most (1 − p)ⁿ (sampling with replacement bounds the
/// without-replacement case from above). So when that is ≤ 1 − c, a clean
/// sample rejects "rate ≥ p" at confidence c. Refuses p or c outside (0, 1).
pub fn bounds_rate(n_clean: u64, p: &BigRational, c: &BigRational) -> Result<bool, Refusal> {
    claim(p, c)?;
    Ok(holds(n_clean, p, c))
}

/// The least n for which [`bounds_rate`] holds. (1 − p)ⁿ falls strictly with
/// n, so the set of good n is upward closed: doubling finds an upper end,
/// bisection the least. Each step is one exact comparison.
pub fn smallest_sample(p: &BigRational, c: &BigRational) -> Result<u64, Refusal> {
    claim(p, c)?;
    let mut hi = 1u64;
    while !holds(hi, p, c) {
        hi *= 2;
    }
    let mut lo = hi / 2; // holds(lo) is false, or lo == 0
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if holds(mid, p, c) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(hi)
}

/// The bound a clean sample stands on: `sampled` of `population` recomputed
/// clean, and whether that establishes "drift rate < `rate` at
/// `confidence`". The rate and confidence are a ruler's (a judgment); whether
/// the sample meets them is measured.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bound {
    pub population: u64,
    pub sampled: u64,
    pub rate: BigRational,
    pub confidence: BigRational,
    pub established: bool,
}

/// The loss of a sampled audit: the set of bounds it stands on. Composing
/// two audits unions their bounds (nothing is averaged); `total` is an audit
/// that couldn't be made.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sampled {
    bounds: Vec<Bound>,
    total: bool,
}

impl Sampled {
    /// One bound.
    pub fn of(bound: Bound) -> Self {
        Self {
            bounds: vec![bound],
            total: false,
        }
    }

    /// The bounds, sorted, each once.
    pub fn bounds(&self) -> &[Bound] {
        &self.bounds
    }

    /// Whether this is the absorbing loss (an audit that couldn't be made).
    pub fn is_total(&self) -> bool {
        self.total
    }
}

impl Loss for Sampled {
    fn zero() -> Self {
        Self::default()
    }

    fn total() -> Self {
        Self {
            bounds: Vec::new(),
            total: true,
        }
    }

    fn is_zero(&self) -> bool {
        !self.total && self.bounds.is_empty()
    }

    fn combine(self, other: Self) -> Self {
        if self.total || other.total {
            return Self::total();
        }
        let mut bounds = self.bounds;
        bounds.extend(other.bounds);
        bounds.sort();
        bounds.dedup();
        Self {
            bounds,
            total: false,
        }
    }
}

/// What turns an audit red.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    /// `found` of the `sampled` recomputes disagreed with the cache.
    Drift { found: u64, sampled: u64 },
    /// The audit's own inputs were impossible or not a claim.
    Refused(Refusal),
}

/// The gate's verdict over a sample: `sampled` of `population` unchanged
/// items recomputed, `drifted` of them disagreeing with the cache, judged
/// against the claim "drift rate < `p` at confidence `c`".
///
/// - `drifted > 0`: `Failure(Drift)`. A located wrong item is red.
/// - clean, `sampled == population`: `Success`. Only a full recompute is.
/// - clean, a proper sample: `Partial`, naming its [`Bound`] (with whether
///   the sample establishes the claim). **Never `Success`.**
/// - impossible inputs or a non-claim: `Failure(Refused)`, total loss.
pub fn audit(
    population: u64,
    sampled: u64,
    drifted: u64,
    p: &BigRational,
    c: &BigRational,
) -> Imperfect<(), Finding, Sampled> {
    let refused = |r| Imperfect::Failure(Finding::Refused(r), Sampled::total());
    if sampled > population {
        return refused(Refusal::SampleExceedsPopulation {
            sample: sampled,
            population,
        });
    }
    if drifted > sampled {
        return refused(Refusal::DriftExceedsSample {
            drifted,
            sample: sampled,
        });
    }
    if let Err(r) = claim(p, c) {
        return refused(r);
    }
    if drifted > 0 {
        return Imperfect::Failure(
            Finding::Drift {
                found: drifted,
                sampled,
            },
            Sampled::zero(),
        );
    }
    if sampled == population {
        return Imperfect::Success(());
    }
    Imperfect::Partial(
        (),
        Sampled::of(Bound {
            population,
            sampled,
            rate: p.clone(),
            confidence: c.clone(),
            established: holds(sampled, p, c),
        }),
    )
}

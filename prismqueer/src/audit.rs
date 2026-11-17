//! S3*: the sampled audit of a verdict cache, stated exactly.
//!
//! Red stub: the signatures exist, the bodies don't (docs/spec/audit.md).

use num_rational::BigRational;
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
    /// A claim needs 0 < rate < 1 and 0 < confidence < 1.
    NotAClaim {
        rate: BigRational,
        confidence: BigRational,
    },
}

/// P(a uniform sample of `n` of `population`, without replacement, misses all `bad`).
pub fn miss_probability(_population: u64, _bad: u64, _n: u64) -> Result<BigRational, Refusal> {
    todo!("red")
}

/// Whether `n_clean` clean recomputes establish "drift rate < p at confidence c".
pub fn bounds_rate(_n_clean: u64, _p: &BigRational, _c: &BigRational) -> Result<bool, Refusal> {
    todo!("red")
}

/// The least n for which [`bounds_rate`] holds.
pub fn smallest_sample(_p: &BigRational, _c: &BigRational) -> Result<u64, Refusal> {
    todo!("red")
}

/// The bound a clean sample establishes (or fails to).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bound {
    pub population: u64,
    pub sampled: u64,
    pub rate: BigRational,
    pub confidence: BigRational,
    pub established: bool,
}

/// The loss of a sampled audit: the bounds it stands on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sampled {
    bounds: Vec<Bound>,
    total: bool,
}

impl Sampled {
    pub fn of(_bound: Bound) -> Self {
        todo!("red")
    }
    pub fn bounds(&self) -> &[Bound] {
        &self.bounds
    }
    pub fn is_total(&self) -> bool {
        todo!("red")
    }
}

impl Loss for Sampled {
    fn zero() -> Self {
        Self::default()
    }
    fn total() -> Self {
        todo!("red")
    }
    fn is_zero(&self) -> bool {
        todo!("red")
    }
    fn combine(self, _other: Self) -> Self {
        todo!("red")
    }
}

/// What turns an audit red.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    Drift { found: u64, sampled: u64 },
    Refused(Refusal),
}

/// The gate's verdict over a sample.
pub fn audit(
    _population: u64,
    _sampled: u64,
    _drifted: u64,
    _p: &BigRational,
    _c: &BigRational,
) -> Imperfect<(), Finding, Sampled> {
    todo!("red")
}

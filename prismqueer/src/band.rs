//! `prismqueer::band` — declared measurement bands (prism-006 ask 3).
//!
//! A float reading near zero is a measurement, not a fact. A [`Band`] says
//! how to read it: exact thresholds, a declared [`ScaleLaw`], and three
//! verdicts, `Zero | DontKnow | NonZero`. It is the value type mirror's
//! Resolution rulers wrap: geometry here, governance (address, approval,
//! record) in mirror. Shape from mirror's `docs/specs/mirror-ruler.md` §1.
//!
//! [`zeros`] names every numerical zero prism uses, so they are distinct,
//! comparable rulers instead of disguised constants.

use crate::field::{Group, Ratio};

/// How a band's thresholds scale with the thing measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScaleLaw {
    /// Thresholds as given.
    Absolute,
    /// Thresholds times `max(scale, floor)`.
    TimesScale {
        /// The smallest scale used.
        floor: Ratio,
    },
    /// Thresholds times `k · ε · dim · max(scale, 1)`: a floating-point error
    /// bound (ε = `f64::EPSILON`).
    EpsilonScaled {
        /// The constant in front of ε.
        k: u64,
        /// The dimension the error grows with.
        dim: usize,
    },
}

/// What a band says about a reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// At or below the zero threshold.
    Zero,
    /// Between the thresholds: the band can't tell.
    DontKnow,
    /// At or above the non-zero threshold.
    NonZero,
}

/// A declared reading of "is this zero?".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Band {
    zero_below: Ratio,
    nonzero_from: Ratio,
    scale: ScaleLaw,
}

/// The thresholds were crossed: `zero_below` above `nonzero_from`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crossed;

/// Two bands under different scale laws: no common coarsening, a fork.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fork;

impl Band {
    /// A band. Refused when `zero_below > nonzero_from`.
    pub fn new(zero_below: Ratio, nonzero_from: Ratio, scale: ScaleLaw) -> Result<Self, Crossed> {
        if less(&nonzero_from, &zero_below) {
            return Err(Crossed);
        }
        Ok(Band {
            zero_below,
            nonzero_from,
            scale,
        })
    }

    /// Read `x` measured against an operator of size `scale`.
    pub fn read(&self, x: f64, scale: f64) -> Verdict {
        let unit = match &self.scale {
            ScaleLaw::Absolute => 1.0,
            ScaleLaw::TimesScale { floor } => scale.max(value(floor)),
            ScaleLaw::EpsilonScaled { k, dim } => {
                *k as f64 * f64::EPSILON * *dim as f64 * scale.max(1.0)
            }
        };
        let m = x.abs();
        if m <= value(&self.zero_below) * unit {
            Verdict::Zero
        } else if m >= value(&self.nonzero_from) * unit {
            Verdict::NonZero
        } else {
            Verdict::DontKnow
        }
    }

    /// The coarsest band that reads `Zero` or `DontKnow` wherever either band
    /// does: `(min zero_below, max nonzero_from)`. Only within one scale law;
    /// across laws there's no common reading, and it's a [`Fork`].
    pub fn coarsest(&self, other: &Self) -> Result<Self, Fork> {
        if self.scale != other.scale {
            return Err(Fork);
        }
        let zero_below = if less(&other.zero_below, &self.zero_below) {
            &other.zero_below
        } else {
            &self.zero_below
        };
        let nonzero_from = if less(&self.nonzero_from, &other.nonzero_from) {
            &other.nonzero_from
        } else {
            &self.nonzero_from
        };
        Ok(Band {
            zero_below: zero_below.clone(),
            nonzero_from: nonzero_from.clone(),
            scale: self.scale.clone(),
        })
    }
}

/// The value of a positive ratio as a float.
fn value(r: &Ratio) -> f64 {
    r.exponents()
        .iter()
        .map(|(&p, &e)| (p as f64).powi(e as i32))
        .product()
}

/// `a < b`, read through the quotient `a / b` so equal ratios compare exactly.
fn less(a: &Ratio, b: &Ratio) -> bool {
    let q = a.compose(&b.inverse());
    !q.is_identity() && value(&q) < 1.0
}

/// Every numerical zero prism reads with, as a declared band.
pub mod zeros {
    use super::{Band, ScaleLaw};
    use crate::field::Ratio;

    fn tenth(e: u32) -> Ratio {
        Ratio::of(1, 10u64.pow(e))
    }

    /// `settle`'s kernel certification: `16 · ε · n · scale`, no don't-know region.
    pub fn settle(n: usize) -> Band {
        Band::new(
            Ratio::of(1, 1),
            Ratio::of(1, 1),
            ScaleLaw::EpsilonScaled { k: 16, dim: n },
        )
        .expect("ordered")
    }

    /// `Instrument::kernel_dimension`: the same rule as [`settle`] (since `c7de979`).
    pub fn instrument(dim: usize) -> Band {
        settle(dim)
    }

    /// `Dissonance`: zero below `1e-12`, non-zero from `1e-6`, both times
    /// `max(‖A‖_F · ‖B‖_F, 1)`.
    pub fn commutator() -> Band {
        Band::new(
            tenth(12),
            tenth(6),
            ScaleLaw::TimesScale {
                floor: Ratio::of(1, 1),
            },
        )
        .expect("ordered")
    }

    /// `spectrum.rs`: `1e-12`, absolute.
    pub fn spectrum() -> Band {
        Band::new(tenth(12), tenth(12), ScaleLaw::Absolute).expect("ordered")
    }

    /// `tension.rs`: `1e-9`, absolute.
    pub fn tension() -> Band {
        Band::new(tenth(9), tenth(9), ScaleLaw::Absolute).expect("ordered")
    }

    /// `coincidence.rs` (inside `Oid::hash`): `f64::EPSILON` = 2⁻⁵², absolute.
    pub fn coincidence() -> Band {
        let eps = Ratio::from_primes([(2, -52)]);
        Band::new(eps.clone(), eps, ScaleLaw::Absolute).expect("ordered")
    }
}

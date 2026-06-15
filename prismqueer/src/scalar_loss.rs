//! Core's own scalar loss type — a simple f64 measurement.

use terni::{Loss, Metric};

/// A scalar information loss measured in bits.
///
/// This is core's own Loss implementation, replacing the former
/// terni::ShannonLoss that was removed upstream. It wraps an f64
/// and implements the Loss monoid.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ScalarLoss(pub f64);

impl ScalarLoss {
    /// Construct a loss of `bits` bits. Panics if negative.
    pub fn new(bits: f64) -> Self {
        assert!(bits >= 0.0, "loss must be non-negative");
        ScalarLoss(bits)
    }

    /// Extract the raw f64 value.
    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

impl Loss for ScalarLoss {
    fn zero() -> Self {
        ScalarLoss(0.0)
    }
    fn total() -> Self {
        ScalarLoss(f64::INFINITY)
    }
    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }
    fn combine(self, other: Self) -> Self {
        ScalarLoss(self.0 + other.0)
    }
}

/// `ScalarLoss` is a metric. Its values are bits-of-information, which are
/// non-negative by construction (`ScalarLoss::new` panics on negatives);
/// distance is the absolute difference, which is symmetric; the triangle
/// inequality follows from `|a - c| <= |a - b| + |b - c|` on the reals.
impl Metric for ScalarLoss {
    fn is_non_negative(&self) -> bool {
        self.0 >= 0.0
    }

    fn distance_to(&self, other: &Self) -> Self {
        ScalarLoss((self.0 - other.0).abs())
    }

    fn triangle(&self, b: &Self, c: &Self) -> bool {
        let d_ac = self.distance_to(c).as_f64();
        let d_ab = self.distance_to(b).as_f64();
        let d_bc = b.distance_to(c).as_f64();
        // Allow a tiny f64 slack for floating-point round-off.
        d_ac <= d_ab + d_bc + f64::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_value() {
        let l = ScalarLoss::new(2.5);
        assert_eq!(l.as_f64(), 2.5);
    }

    #[test]
    #[should_panic(expected = "loss must be non-negative")]
    fn new_rejects_negative() {
        ScalarLoss::new(-1.0);
    }

    #[test]
    fn zero_is_zero() {
        let z = ScalarLoss::zero();
        assert!(z.is_zero());
        assert_eq!(z.as_f64(), 0.0);
    }

    #[test]
    fn total_is_infinite() {
        let t = ScalarLoss::total();
        assert!(t.as_f64().is_infinite());
    }

    #[test]
    fn combine_adds() {
        let a = ScalarLoss::new(1.0);
        let b = ScalarLoss::new(2.0);
        assert_eq!(a.combine(b).as_f64(), 3.0);
    }

    #[test]
    fn default_is_zero() {
        let d = ScalarLoss::default();
        assert!(d.is_zero());
    }

    // --- Metric properties (Gap 4 closure) ---

    #[test]
    fn metric_non_negative_for_typical_values() {
        for bits in [0.0, 0.5, 1.0, 2.5, 100.0, 1e9] {
            assert!(ScalarLoss::new(bits).is_non_negative());
        }
    }

    #[test]
    fn metric_self_distance_is_zero() {
        let a = ScalarLoss::new(3.5);
        assert!(a.distance_to(&a).is_zero());
    }

    #[test]
    fn metric_symmetry() {
        // a.distance_to(b) == b.distance_to(a) for a sample grid.
        let samples = [0.0_f64, 0.5, 1.0, 2.5, 4.0, 100.0];
        for &x in &samples {
            for &y in &samples {
                let a = ScalarLoss::new(x);
                let b = ScalarLoss::new(y);
                assert_eq!(
                    a.distance_to(&b).as_f64(),
                    b.distance_to(&a).as_f64(),
                    "symmetry failed for ({}, {})",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn metric_triangle_inequality() {
        // d(a,c) <= d(a,b) + d(b,c) for a sample grid.
        let samples = [0.0_f64, 0.5, 1.0, 2.5, 4.0, 100.0];
        for &x in &samples {
            for &y in &samples {
                for &z in &samples {
                    let a = ScalarLoss::new(x);
                    let b = ScalarLoss::new(y);
                    let c = ScalarLoss::new(z);
                    assert!(
                        a.triangle(&b, &c),
                        "triangle failed for ({}, {}, {})",
                        x,
                        y,
                        z
                    );
                }
            }
        }
    }
}

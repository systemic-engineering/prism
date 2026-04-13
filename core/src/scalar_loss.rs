//! Core's own scalar loss type — a simple f64 measurement.

use terni::Loss;

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
}

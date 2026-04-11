//! Imperfect — Result extended with partial success.
//!
//! Three states: Ok (perfect), Partial (value with loss), Err (failure).
//! Derived from partial successes in PbtA game design.
//!
//! `Loss` is a trait. `ShannonLoss` (information loss in bits) is the
//! default implementation.

/// A measure of what didn't survive a transformation.
pub trait Loss: Clone + Default {
    fn zero() -> Self;
    fn total() -> Self;
    fn is_zero(&self) -> bool;
    fn combine(self, other: Self) -> Self;
}

/// Information loss measured in bits. The default `Loss` implementation.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ShannonLoss(f64);

impl ShannonLoss {
    pub fn new(bits: f64) -> Self {
        ShannonLoss(bits)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }

    pub fn is_lossless(&self) -> bool {
        self.is_zero()
    }
}

impl Default for ShannonLoss {
    fn default() -> Self {
        Self::zero()
    }
}

impl Loss for ShannonLoss {
    fn zero() -> Self {
        ShannonLoss(0.0)
    }

    fn total() -> Self {
        ShannonLoss(f64::INFINITY)
    }

    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    fn combine(self, other: Self) -> Self {
        ShannonLoss(self.0 + other.0)
    }
}

impl std::ops::Add for ShannonLoss {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        ShannonLoss(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for ShannonLoss {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl std::fmt::Display for ShannonLoss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.6} bits", self.0)
    }
}

impl From<f64> for ShannonLoss {
    fn from(v: f64) -> Self {
        ShannonLoss(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shannon_zero() {
        let l = ShannonLoss::zero();
        assert!(l.is_zero());
        assert_eq!(l.as_f64(), 0.0);
    }

    #[test]
    fn shannon_total() {
        let l = ShannonLoss::total();
        assert!(!l.is_zero());
        assert!(l.as_f64().is_infinite());
    }

    #[test]
    fn shannon_new() {
        let l = ShannonLoss::new(1.5);
        assert_eq!(l.as_f64(), 1.5);
        assert!(!l.is_zero());
    }

    #[test]
    fn shannon_combine() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.5);
        let c = a.combine(b);
        assert_eq!(c.as_f64(), 3.5);
    }

    #[test]
    fn shannon_combine_zero_is_identity() {
        let a = ShannonLoss::new(3.0);
        let b = a.clone().combine(ShannonLoss::zero());
        assert_eq!(a, b);
    }

    #[test]
    fn shannon_default_is_zero() {
        let l = ShannonLoss::default();
        assert!(l.is_zero());
    }

    #[test]
    fn shannon_display() {
        let l = ShannonLoss::new(2.0);
        assert_eq!(format!("{}", l), "2.000000 bits");
    }

    #[test]
    fn shannon_from_f64() {
        let l: ShannonLoss = 3.14.into();
        assert_eq!(l.as_f64(), 3.14);
    }

    #[test]
    fn shannon_add_operator() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.5);
        let c = a + b;
        assert_eq!(c.as_f64(), 3.5);
    }

    #[test]
    fn shannon_add_assign() {
        let mut a = ShannonLoss::new(1.0);
        a += ShannonLoss::new(0.5);
        assert_eq!(a.as_f64(), 1.5);
    }

    #[test]
    fn shannon_ordering() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.0);
        assert!(a < b);
    }

    #[test]
    fn shannon_is_lossless() {
        assert!(ShannonLoss::zero().is_lossless());
        assert!(!ShannonLoss::new(0.1).is_lossless());
    }

    // --- Imperfect ---

    #[test]
    fn ok_is_ok() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert!(i.is_ok());
        assert!(!i.is_partial());
        assert!(!i.is_err());
    }

    #[test]
    fn partial_is_partial() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.5));
        assert!(i.is_ok());
        assert!(i.is_partial());
        assert!(!i.is_err());
    }

    #[test]
    fn err_is_err() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert!(!i.is_ok());
        assert!(!i.is_partial());
        assert!(i.is_err());
    }

    #[test]
    fn ok_returns_value() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert_eq!(i.ok(), Some(42));
    }

    #[test]
    fn partial_ok_returns_value() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.0));
        assert_eq!(i.ok(), Some(42));
    }

    #[test]
    fn err_ok_returns_none() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert_eq!(i.ok(), None);
    }

    #[test]
    fn err_returns_error() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert_eq!(i.err(), Some("oops".into()));
    }

    #[test]
    fn ok_err_returns_none() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert_eq!(i.err(), None);
    }

    #[test]
    fn loss_ok_is_zero() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        assert!(i.loss().is_zero());
    }

    #[test]
    fn loss_partial() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.5));
        assert_eq!(i.loss().as_f64(), 1.5);
    }

    #[test]
    fn loss_err_is_total() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        assert!(i.loss().as_f64().is_infinite());
    }

    #[test]
    fn as_ref_ok() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        let r = i.as_ref();
        assert_eq!(r.ok(), Some(&42));
    }

    #[test]
    fn as_ref_partial() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.0));
        let r = i.as_ref();
        assert_eq!(r.ok(), Some(&42));
        assert!(r.is_partial());
    }

    #[test]
    fn as_ref_err() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        let r = i.as_ref();
        assert_eq!(r.err(), Some(&"oops".to_string()));
    }

    #[test]
    fn map_ok() {
        let i: Imperfect<u32, String> = Imperfect::Ok(42);
        let m = i.map(|v| v * 2);
        assert_eq!(m.ok(), Some(84));
    }

    #[test]
    fn map_partial_preserves_loss() {
        let i: Imperfect<u32, String> = Imperfect::Partial(42, ShannonLoss::new(1.0));
        let m = i.map(|v| v * 2);
        assert_eq!(m.ok(), Some(84));
        assert!(m.is_partial());
    }

    #[test]
    fn map_err_is_noop() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        let m = i.map(|v| v * 2);
        assert!(m.is_err());
    }

    #[test]
    fn map_err_transforms_error() {
        let i: Imperfect<u32, String> = Imperfect::Err("oops".into());
        let m = i.map_err(|e| e.len());
        assert_eq!(m.err(), Some(4));
    }
}

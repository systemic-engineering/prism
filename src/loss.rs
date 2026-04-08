/// What didn't survive the projection. Measured in bits.
/// Every prism operation may lose information. The loss records how much.
/// Zero loss = lossless projection = refract.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ShannonLoss(f64);

impl ShannonLoss {
    pub fn new(bits: f64) -> Self {
        ShannonLoss(bits)
    }

    pub fn zero() -> Self {
        ShannonLoss(0.0)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    /// Semantic alias for `is_zero` — true when the projection is lossless.
    /// `is_lossless` reads more naturally in optic-pipeline contexts where
    /// "zero" could be confused with numerical zero.
    pub fn is_lossless(&self) -> bool {
        self.is_zero()
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
    fn zero() {
        let l = ShannonLoss::zero();
        assert_eq!(l.as_f64(), 0.0);
        assert!(l.is_zero());
    }

    #[test]
    fn new() {
        let l = ShannonLoss::new(1.5);
        assert_eq!(l.as_f64(), 1.5);
        assert!(!l.is_zero());
    }

    #[test]
    fn display() {
        let l = ShannonLoss::new(2.0);
        assert_eq!(format!("{}", l), "2.000000 bits");
    }

    #[test]
    fn from_f64() {
        let l: ShannonLoss = 3.14.into();
        assert_eq!(l.as_f64(), 3.14);
    }

    #[test]
    fn ordering() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.0);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn equality() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(1.0);
        assert_eq!(a, b);
    }

    #[test]
    fn add() {
        let a = ShannonLoss::new(1.0);
        let b = ShannonLoss::new(2.5);
        let c = a + b;
        assert_eq!(c.as_f64(), 3.5);
    }

    #[test]
    fn add_assign() {
        let mut a = ShannonLoss::new(1.0);
        a += ShannonLoss::new(0.5);
        assert_eq!(a.as_f64(), 1.5);
    }

    #[test]
    fn add_zero_is_identity() {
        let a = ShannonLoss::new(3.0);
        let b = a.clone() + ShannonLoss::zero();
        assert_eq!(a, b);
    }
}

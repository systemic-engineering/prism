/// Measurement precision. Newtype over f64.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Precision(f64);

impl Precision {
    pub fn new(v: f64) -> Self {
        Precision(v)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

impl std::fmt::Display for Precision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "±{:.6}", self.0)
    }
}

impl From<f64> for Precision {
    fn from(v: f64) -> Self {
        Precision(v)
    }
}

/// Load factor, clamped to [0.0, 1.0].
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Pressure(f64);

impl Pressure {
    pub fn new(v: f64) -> Self {
        Pressure(v.clamp(0.0, 1.0))
    }

    pub fn ratio(&self) -> f64 {
        self.0
    }

    pub fn is_critical(&self) -> bool {
        self.0 >= 0.9
    }
}

impl std::fmt::Display for Pressure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_new() {
        let p = Precision::new(0.01);
        assert_eq!(p.as_f64(), 0.01);
    }

    #[test]
    fn precision_display() {
        let p = Precision::new(0.001);
        assert_eq!(format!("{}", p), "±0.001000");
    }

    #[test]
    fn precision_from_f64() {
        let p: Precision = 0.5_f64.into();
        assert_eq!(p.as_f64(), 0.5);
    }

    #[test]
    fn precision_ordering() {
        let a = Precision::new(0.1);
        let b = Precision::new(0.2);
        assert!(a < b);
    }

    #[test]
    fn pressure_clamps_high() {
        let p = Pressure::new(1.5);
        assert_eq!(p.ratio(), 1.0);
    }

    #[test]
    fn pressure_clamps_low() {
        let p = Pressure::new(-0.5);
        assert_eq!(p.ratio(), 0.0);
    }

    #[test]
    fn pressure_preserves_valid() {
        let p = Pressure::new(0.75);
        assert_eq!(p.ratio(), 0.75);
    }

    #[test]
    fn pressure_display() {
        let p = Pressure::new(0.75);
        assert_eq!(format!("{}", p), "75.0%");
    }

    #[test]
    fn pressure_critical() {
        assert!(Pressure::new(0.9).is_critical());
        assert!(Pressure::new(1.0).is_critical());
        assert!(!Pressure::new(0.89).is_critical());
    }

    #[test]
    fn pressure_zero() {
        let p = Pressure::new(0.0);
        assert_eq!(p.ratio(), 0.0);
        assert!(!p.is_critical());
    }
}

//! Luminosity — how much signal is getting through.

/// How much signal is getting through.
/// The ternary state of a beam or crystal.
#[derive(Debug, Clone, PartialEq)]
pub enum Luminosity {
    /// Full signal. Zero holonomy. Crystal.
    Light,
    /// Partial signal. Nonzero holonomy. Oscillating.
    Dimmed(f64),
    /// No signal. Error and loss propagate. Fixpoint.
    Dark,
}

impl Luminosity {
    pub fn is_light(&self) -> bool { matches!(self, Luminosity::Light) }
    pub fn is_dimmed(&self) -> bool { matches!(self, Luminosity::Dimmed(_)) }
    pub fn is_dark(&self) -> bool { matches!(self, Luminosity::Dark) }

    pub fn holonomy(&self) -> Option<f64> {
        match self {
            Luminosity::Light => Some(0.0),
            Luminosity::Dimmed(h) => Some(*h),
            Luminosity::Dark => None,
        }
    }

    pub fn from_holonomy(h: f64) -> Self {
        if h == 0.0 {
            Luminosity::Light
        } else if h.is_finite() {
            Luminosity::Dimmed(h)
        } else {
            Luminosity::Dark
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminosity_states() {
        let light = Luminosity::Light;
        let dimmed = Luminosity::Dimmed(0.3);
        let dark = Luminosity::Dark;

        assert!(light.is_light());
        assert!(!light.is_dark());
        assert!(dimmed.is_dimmed());
        assert_eq!(dimmed.holonomy(), Some(0.3));
        assert!(dark.is_dark());
        assert_eq!(dark.holonomy(), None);
    }

    #[test]
    fn luminosity_from_holonomy() {
        assert!(Luminosity::from_holonomy(0.0).is_light());
        assert!(Luminosity::from_holonomy(0.5).is_dimmed());
        assert!(Luminosity::from_holonomy(f64::INFINITY).is_dark());
    }
}

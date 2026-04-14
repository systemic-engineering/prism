//! Luminosity — how much signal is getting through.

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

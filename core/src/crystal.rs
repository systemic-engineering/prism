//! Crystal — a settled Prism.

use crate::luminosity::Luminosity;
use crate::oid::{Oid, Addressable};

/// A settled Prism. The Prism IS the value.
/// The Crystal IS the Prism at rest.
///
/// Crystal(prism, luminosity) — the shape and its state.
/// The Oid comes from the Prism. The Luminosity comes from the holonomy.
#[derive(Debug, Clone, PartialEq)]
pub struct Crystal<P>(pub P, pub Luminosity);

impl<P> Crystal<P> {
    pub fn prism(&self) -> &P {
        &self.0
    }

    pub fn luminosity(&self) -> &Luminosity {
        &self.1
    }

    pub fn into_prism(self) -> P {
        self.0
    }

    pub fn is_settled(&self) -> bool {
        self.1.is_light()
    }
}

impl<P: Addressable> Addressable for Crystal<P> {
    fn oid(&self) -> Oid {
        self.0.oid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal Prism for testing
    #[derive(Debug, Clone, PartialEq)]
    struct IdPrism(u32);

    impl Addressable for IdPrism {
        fn oid(&self) -> Oid {
            Oid::hash(&self.0.to_le_bytes())
        }
    }

    #[test]
    fn crystal_light() {
        let crystal = Crystal(IdPrism(42), Luminosity::Light);
        assert!(crystal.luminosity().is_light());
        assert_eq!(crystal.prism(), &IdPrism(42));
    }

    #[test]
    fn crystal_addressable_from_prism() {
        let crystal = Crystal(IdPrism(42), Luminosity::Light);
        let prism = IdPrism(42);
        assert_eq!(crystal.oid(), prism.oid());
    }

    #[test]
    fn crystal_dimmed_carries_holonomy() {
        let crystal = Crystal(IdPrism(1), Luminosity::Dimmed(0.23));
        assert!(crystal.luminosity().is_dimmed());
        assert_eq!(crystal.luminosity().holonomy(), Some(0.23));
    }

    #[test]
    fn crystal_dark() {
        let crystal = Crystal(IdPrism(0), Luminosity::Dark);
        assert!(crystal.luminosity().is_dark());
    }

    #[test]
    fn same_prism_same_oid_regardless_of_luminosity() {
        let light = Crystal(IdPrism(42), Luminosity::Light);
        let dimmed = Crystal(IdPrism(42), Luminosity::Dimmed(0.5));
        assert_eq!(light.oid(), dimmed.oid());
    }
}

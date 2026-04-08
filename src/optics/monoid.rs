//! Monoid structure of Prism.
//!
//! A Prism with `type Crystal = Self` is closed under refract composition.
//! The set of such prisms forms a monoid: composition is associative,
//! IdPrism is the identity element, and endofunction composition is the
//! operation.

use crate::{Beam, Prism};

// Layer 1 types go here — see tests for the expected API.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Stage;

    #[test]
    fn id_prism_refracts_beam_unchanged() {
        let id: IdPrism<String> = IdPrism::new();
        let input = Beam::new("hello".to_string());
        let out = id.refract(input);
        assert_eq!(out.result.marker(), "id");
        assert_eq!(out.stage, Stage::Refracted);
    }

    #[test]
    fn id_prism_is_its_own_crystal() {
        // Compile-time check: IdPrism<T>::Crystal = IdPrism<T>.
        // This test asserts the fixed-point property holds structurally.
        fn require_self_crystal<P: Prism<Crystal = P>>() {}
        require_self_crystal::<IdPrism<String>>();
    }
}

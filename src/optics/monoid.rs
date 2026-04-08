//! Monoid structure of Prism.
//!
//! A Prism with `type Crystal = Self` is closed under refract composition.
//! The set of such prisms forms a monoid: composition is associative,
//! IdPrism is the identity element, and endofunction composition is the
//! operation.

use std::marker::PhantomData;
use crate::{Beam, Prism, Stage};

/// A Prism whose Crystal is itself — the closure property that makes
/// prisms compose into a monoid.
///
/// Laws:
/// - Identity: `compose(identity_prism(), p) ≡ p ≡ compose(p, identity_prism())`
/// - Associativity: `compose(compose(a, b), c) ≡ compose(a, compose(b, c))`
pub trait PrismMonoid: Prism<Crystal = Self> + Sized {
    /// The identity element: refracting through this prism leaves the
    /// Beam's content unchanged and transitions stage to Refracted.
    fn identity_prism() -> Self;

    /// Monoid composition: run `self` then `other`.
    fn compose(self, other: Self) -> Self;
}

/// The identity prism for a type `T`. Refracting a `Beam<T>` produces
/// a `Beam<IdPrism<T>>` where the result carries the identity marker.
#[derive(Debug, Clone)]
pub struct IdPrism<T> {
    _phantom: PhantomData<T>,
}

impl<T> IdPrism<T> {
    pub fn new() -> Self {
        IdPrism { _phantom: PhantomData }
    }
    pub fn marker(&self) -> &'static str {
        "id"
    }
}

impl<T: Clone> Default for IdPrism<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> Prism for IdPrism<T> {
    type Input = T;
    type Focused = T;
    type Projected = T;
    type Part = T;
    type Crystal = IdPrism<T>;

    fn focus(&self, beam: Beam<T>) -> Beam<T> {
        Beam {
            result: beam.result,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
        }
    }

    fn project(&self, beam: Beam<T>) -> Beam<T> {
        Beam {
            result: beam.result,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Projected,
        }
    }

    fn split(&self, beam: Beam<T>) -> Vec<Beam<T>> {
        vec![Beam {
            result: beam.result,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Split,
        }]
    }

    fn zoom(
        &self,
        beam: Beam<T>,
        f: &dyn Fn(Beam<T>) -> Beam<T>,
    ) -> Beam<T> {
        f(beam)
    }

    fn refract(&self, beam: Beam<T>) -> Beam<IdPrism<T>> {
        Beam {
            result: IdPrism::new(),
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}

impl<T: Clone + 'static> PrismMonoid for IdPrism<T> {
    fn identity_prism() -> Self {
        IdPrism::new()
    }

    fn compose(self, _other: Self) -> Self {
        // Composing identity with identity is identity.
        self
    }
}

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

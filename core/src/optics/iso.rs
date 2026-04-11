//! Iso — the total invertible optic.
//!
//! An Iso<A, B> is a pair of functions (forward: A → B, backward: B → A)
//! such that `backward(forward(a)) = a` and `forward(backward(b)) = b`.
//! This is the only optic where refract is genuinely lossless and the
//! round-trip holds as a law.

use crate::{Beam, Prism, Stage};

/// A total invertible pair (A → B, B → A).
///
/// Laws:
/// - `backward(forward(a)) ≡ a` (left inverse)
/// - `forward(backward(b)) ≡ b` (right inverse)
///
/// As a Prism: focus applies forward, refract crystallizes the Iso itself
/// since fn pointers are Copy and the optic IS the lossless fixed point.
#[derive(Clone, Copy)]
pub struct Iso<A, B> {
    forward_fn: fn(A) -> B,
    backward_fn: fn(B) -> A,
}

impl<A: 'static, B: 'static> Iso<A, B> {
    /// Construct an Iso from a forward and backward fn pointer.
    ///
    /// # Laws
    ///
    /// The caller is responsible for ensuring the round-trip laws hold:
    ///
    /// - `backward(forward(a)) ≡ a` for all `a: A` (left inverse)
    /// - `forward(backward(b)) ≡ b` for all `b: B` (right inverse)
    ///
    /// These cannot be enforced by the type system. An Iso constructed from
    /// non-inverse functions will compile and run, but will produce results
    /// that are meaningless under the Iso laws — downstream consumers that
    /// depend on round-trip equality will see silent corruption.
    ///
    /// If you cannot prove the laws hold for arbitrary inputs, prefer one of
    /// the more permissive optics (`Lens` for partial inverses, `Setter` for
    /// write-only, `Fold` for read-only).
    pub fn new(forward: fn(A) -> B, backward: fn(B) -> A) -> Self {
        Iso { forward_fn: forward, backward_fn: backward }
    }

    pub fn forward(&self, a: A) -> B {
        (self.forward_fn)(a)
    }

    pub fn backward(&self, b: B) -> A {
        (self.backward_fn)(b)
    }
}

impl<A: Clone + 'static, B: Clone + 'static> Prism for Iso<A, B> {
    type Input = A;
    type Focused = B;
    type Projected = B;
    type Part = B;
    type Crystal = Iso<A, B>;

    fn focus(&self, beam: Beam<A>) -> Beam<B> {
        let forward = (self.forward_fn)(beam.result);
        Beam {
            result: forward,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
            connection: beam.connection,
        }
    }

    fn project(&self, beam: Beam<B>) -> Beam<B> {
        // Iso's project is lossless pass-through — no precision cut.
        Beam {
            stage: Stage::Projected,
            ..beam
        }
    }

    fn split(&self, beam: Beam<B>) -> Vec<Beam<B>> {
        vec![Beam {
            stage: Stage::Split,
            ..beam
        }]
    }

    fn zoom(
        &self,
        beam: Beam<B>,
        f: &dyn Fn(Beam<B>) -> Beam<B>,
    ) -> Beam<B> {
        f(beam)
    }

    fn refract(&self, beam: Beam<B>) -> Beam<Iso<A, B>> {
        // fn pointers are Copy — the optic itself IS the lossless fixed point.
        Beam {
            result: self.clone(),
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
            connection: beam.connection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn str_to_chars(s: String) -> Vec<char> {
        s.chars().collect()
    }

    fn chars_to_str(v: Vec<char>) -> String {
        v.into_iter().collect()
    }

    #[test]
    fn iso_round_trip() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);

        let forward = iso.forward("hello".to_string());
        assert_eq!(forward, vec!['h', 'e', 'l', 'l', 'o']);
        let backward = iso.backward(forward);
        assert_eq!(backward, "hello");
    }

    #[test]
    fn iso_refract_is_lossless() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);

        let beam = Beam::new("test".to_string());
        let projected = iso.project(iso.focus(beam));
        assert_eq!(projected.result, vec!['t', 'e', 's', 't']);
        assert!(projected.loss.is_zero());

        let refracted = iso.refract(projected);
        assert_eq!(refracted.stage, Stage::Refracted);
    }

    #[test]
    fn iso_is_clone_and_copy() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let iso2 = iso; // Copy
        let iso3 = iso2.clone(); // Clone
        let _ = iso3.forward("ok".to_string());
    }
}

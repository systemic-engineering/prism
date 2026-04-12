//! Iso — the total invertible optic.
//!
//! An Iso<A, B> is a pair of functions (forward: A → B, backward: B → A)
//! such that `backward(forward(a)) = a` and `forward(backward(b)) = b`.
//!
//! As a Prism: focus applies forward, project is identity, refract applies backward.
//! Round-trip is genuinely lossless — the only optic with this property.

use crate::{Beam, Prism, PureBeam};
use std::convert::Infallible;
use terni::ShannonLoss;

/// A total invertible pair (A → B, B → A).
///
/// Laws:
/// - `backward(forward(a)) ≡ a` (left inverse)
/// - `forward(backward(b)) ≡ b` (right inverse)
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
    /// - `backward(forward(a)) ≡ a` for all `a: A`
    /// - `forward(backward(b)) ≡ b` for all `b: B`
    pub fn new(forward: fn(A) -> B, backward: fn(B) -> A) -> Self {
        Iso {
            forward_fn: forward,
            backward_fn: backward,
        }
    }

    pub fn forward(&self, a: A) -> B {
        (self.forward_fn)(a)
    }

    pub fn backward(&self, b: B) -> A {
        (self.backward_fn)(b)
    }
}

/// Iso implements Prism with PureBeam.
///
/// Pipeline flow:
/// - focus: applies forward (A → B)
/// - project: identity pass-through (B → B)
/// - refract: applies backward (B → A)
impl<A: Clone + 'static, B: Clone + 'static> Prism for Iso<A, B> {
    type Input = PureBeam<(), A, Infallible, ShannonLoss>;
    type Focused = PureBeam<A, B, Infallible, ShannonLoss>;
    type Projected = PureBeam<B, B, Infallible, ShannonLoss>;
    type Refracted = PureBeam<B, A, Infallible, ShannonLoss>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let a = beam.result().ok().expect("focus: Err beam").clone();
        let b = (self.forward_fn)(a);
        beam.next(b)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let b = beam.result().ok().expect("project: Err beam").clone();
        beam.next(b)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let b = beam.result().ok().expect("refract: Err beam").clone();
        let a = (self.backward_fn)(b);
        beam.next(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Beam as BeamTrait;

    fn str_to_chars(s: String) -> Vec<char> {
        s.chars().collect()
    }

    fn chars_to_str(v: Vec<char>) -> String {
        v.into_iter().collect()
    }

    // --- Inherent method tests ---

    #[test]
    fn iso_forward() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let result = iso.forward("hello".to_string());
        assert_eq!(result, vec!['h', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn iso_backward() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let result = iso.backward(vec!['h', 'i']);
        assert_eq!(result, "hi");
    }

    #[test]
    fn iso_round_trip_forward_backward() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let original = "hello".to_string();
        let round_tripped = iso.backward(iso.forward(original.clone()));
        assert_eq!(round_tripped, original);
    }

    #[test]
    fn iso_round_trip_backward_forward() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let original = vec!['a', 'b', 'c'];
        let round_tripped = iso.forward(iso.backward(original.clone()));
        assert_eq!(round_tripped, original);
    }

    // --- Prism trait tests ---

    fn seed<T: Clone>(v: T) -> PureBeam<(), T, Infallible, ShannonLoss> {
        PureBeam::ok((), v)
    }

    #[test]
    fn iso_prism_focus_applies_forward() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let beam = seed("hi".to_string());
        let focused = iso.focus(beam);
        assert_eq!(focused.result().ok(), Some(&vec!['h', 'i']));
        assert_eq!(focused.input(), &"hi".to_string());
    }

    #[test]
    fn iso_prism_project_is_identity() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let focused = iso.focus(seed("ab".to_string()));
        let projected = iso.project(focused);
        assert_eq!(projected.result().ok(), Some(&vec!['a', 'b']));
    }

    #[test]
    fn iso_prism_refract_applies_backward() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let focused = iso.focus(seed("hi".to_string()));
        let projected = iso.project(focused);
        let refracted = iso.refract(projected);
        assert_eq!(refracted.result().ok(), Some(&"hi".to_string()));
    }

    #[test]
    fn iso_prism_full_pipeline_round_trips() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let beam = seed("test".to_string());
        let focused = iso.focus(beam);
        let projected = iso.project(focused);
        let refracted = iso.refract(projected);
        // Iso round-trips perfectly: refract(project(focus(a))) == a
        assert_eq!(refracted.result().ok(), Some(&"test".to_string()));
        assert!(refracted.is_ok());
        assert!(!refracted.is_partial());
    }

    #[test]
    fn iso_prism_is_lossless() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let focused = iso.focus(seed("x".to_string()));
        assert!(!focused.is_partial());
        let projected = iso.project(focused);
        assert!(!projected.is_partial());
        let refracted = iso.refract(projected);
        assert!(!refracted.is_partial());
    }

    #[test]
    fn iso_is_clone_and_copy() {
        let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
        let iso2 = iso; // Copy
        let iso3 = iso2.clone(); // Clone
        let _ = iso3.forward("ok".to_string());
    }
}

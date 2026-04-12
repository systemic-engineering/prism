//! Setter — the write-only optic.
//!
//! A Setter<S, A> provides a way to modify A within S by applying a function,
//! without giving observational access to the part.
//!
//! As a Prism: all stages are identity on S. The modify operation is available
//! as an inherent method. The pipeline preserves S through all stages.

use crate::{Beam, Prism, PureBeam};
use std::convert::Infallible;
use terni::ShannonLoss;

#[derive(Clone, Copy)]
pub struct Setter<S, A> {
    modify_fn: fn(S, &dyn Fn(A) -> A) -> S,
}

impl<S: 'static, A: 'static> Setter<S, A> {
    /// Construct a Setter from a modify fn pointer.
    ///
    /// # Laws
    /// - `modify(s, |a| a) ≡ s` (identity law)
    /// - `modify(modify(s, f), g) ≡ modify(s, g ∘ f)` (composition law)
    pub fn new(modify: fn(S, &dyn Fn(A) -> A) -> S) -> Self {
        Setter { modify_fn: modify }
    }

    pub fn modify<F>(&self, s: S, f: F) -> S
    where
        F: Fn(A) -> A + 'static,
    {
        (self.modify_fn)(s, &f)
    }
}

/// Setter implements Prism with PureBeam.
///
/// Pipeline flow:
/// - focus: identity (S → S) — no read access
/// - project: identity (S → S)
/// - refract: applies modify with identity to witness the closure, returns S
impl<S: Clone + 'static, A: Clone + 'static> Prism for Setter<S, A> {
    type Input = PureBeam<(), S, Infallible, ShannonLoss>;
    type Focused = PureBeam<S, S, Infallible, ShannonLoss>;
    type Projected = PureBeam<S, S, Infallible, ShannonLoss>;
    type Refracted = PureBeam<S, S, Infallible, ShannonLoss>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let s = beam.result().ok().expect("focus: Err beam").clone();
        beam.next(s)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let s = beam.result().ok().expect("project: Err beam").clone();
        beam.next(s)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let s = beam.result().ok().expect("refract: Err beam").clone();
        // Witness: run modify with identity to prove the closure is reachable
        let witnessed = (self.modify_fn)(s, &|a| a);
        beam.next(witnessed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Beam as BeamTrait;

    #[derive(Clone, Debug, PartialEq)]
    struct Box2 {
        label: String,
        count: i32,
    }

    fn box2_modify_count(b: Box2, f: &dyn Fn(i32) -> i32) -> Box2 {
        Box2 {
            count: f(b.count),
            ..b
        }
    }

    // --- Inherent method tests ---

    #[test]
    fn setter_modifies_field() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let b = Box2 {
            label: "test".to_string(),
            count: 5,
        };
        let b2 = s.modify(b, |c| c + 10);
        assert_eq!(b2.count, 15);
        assert_eq!(b2.label, "test");
    }

    #[test]
    fn setter_identity_law() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let b = Box2 {
            label: "x".to_string(),
            count: 7,
        };
        let b2 = s.modify(b.clone(), |a| a);
        assert_eq!(b2, b);
    }

    // --- Prism trait tests ---

    fn seed<T: Clone>(v: T) -> PureBeam<(), T, Infallible, ShannonLoss> {
        PureBeam::ok((), v)
    }

    #[test]
    fn setter_prism_focus_passes_through() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let b = Box2 {
            label: "x".to_string(),
            count: 5,
        };
        let focused = s.focus(seed(b.clone()));
        assert_eq!(focused.result().ok(), Some(&b));
    }

    #[test]
    fn setter_prism_project_passes_through() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let b = Box2 {
            label: "x".to_string(),
            count: 5,
        };
        let focused = s.focus(seed(b.clone()));
        let projected = s.project(focused);
        assert_eq!(projected.result().ok(), Some(&b));
    }

    #[test]
    fn setter_prism_refract_witnesses_modify() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let b = Box2 {
            label: "x".to_string(),
            count: 5,
        };
        let focused = s.focus(seed(b.clone()));
        let projected = s.project(focused);
        let refracted = s.refract(projected);
        // refract applies identity modify, so value is unchanged
        assert_eq!(refracted.result().ok(), Some(&b));
    }

    #[test]
    fn setter_prism_is_lossless() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let b = Box2 {
            label: "t".to_string(),
            count: 3,
        };
        let focused = s.focus(seed(b));
        assert!(!focused.is_partial());
    }

    #[test]
    fn setter_is_clone_and_copy() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let s2 = s; // Copy
        let s3 = s2.clone(); // Clone
        let b = Box2 {
            label: "t".to_string(),
            count: 3,
        };
        assert_eq!(s3.modify(b, |c| c * 2).count, 6);
    }
}

//! OpticPrism — the semidet bidirectional optic.
//!
//! Named OpticPrism (not Prism) to avoid collision with our crate's
//! central Prism trait. Represents a sum-type case: preview may fail
//! (the case doesn't match), but review always reconstructs the whole.
//!
//! As a Prism: focus extracts (using Failure on non-match), project is identity,
//! refract returns the extracted value.

use std::convert::Infallible;
use crate::{Beam, Prism, PureBeam};
use imperfect::{Imperfect, ShannonLoss};

#[derive(Clone, Copy)]
pub struct OpticPrism<S, A> {
    match_fn: fn(&S) -> bool,
    extract_fn: fn(&S) -> A,
    review_fn: fn(A) -> S,
}

impl<S: 'static, A: 'static> OpticPrism<S, A> {
    /// Construct an OpticPrism from a match, extract, and review function.
    ///
    /// # Laws
    /// - When `matches(s)`: `review(extract(s)) ≡ s`
    /// - For any `a`: `matches(review(a))` is true
    /// - When `!matches(s)`: extract returns a sentinel; the beam will be Partial with infinite loss.
    pub fn new(matches: fn(&S) -> bool, extract: fn(&S) -> A, review: fn(A) -> S) -> Self {
        OpticPrism {
            match_fn: matches,
            extract_fn: extract,
            review_fn: review,
        }
    }

    /// Returns `true` if `s` is the variant this prism targets.
    pub fn matches(&self, s: &S) -> bool {
        (self.match_fn)(s)
    }

    /// Returns `Some(A)` on a matching `s`, `None` otherwise.
    pub fn extract(&self, s: &S) -> Option<A> {
        if (self.match_fn)(s) {
            Some((self.extract_fn)(s))
        } else {
            None
        }
    }

    pub fn review(&self, a: A) -> S {
        (self.review_fn)(a)
    }
}

/// OpticPrism implements Prism with PureBeam.
///
/// Pipeline flow:
/// - focus: extract (S → A), using Partial with infinite loss on non-match
/// - project: identity pass-through
/// - refract: identity pass-through
///
/// Non-matching inputs produce a Partial beam with ShannonLoss::total()
/// (infinite loss), signaling refutation through the loss channel rather
/// than the error channel.
impl<S: Clone + 'static, A: Clone + 'static> Prism for OpticPrism<S, A> {
    type Input     = PureBeam<(), S, Infallible, ShannonLoss>;
    type Focused   = PureBeam<S, A, Infallible, ShannonLoss>;
    type Projected = PureBeam<A, A, Infallible, ShannonLoss>;
    type Refracted = PureBeam<A, A, Infallible, ShannonLoss>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let s = beam.result().ok().expect("focus: Err beam").clone();
        if (self.match_fn)(&s) {
            let a = (self.extract_fn)(&s);
            beam.next(a)
        } else {
            // Non-match: extract the sentinel and mark with infinite loss.
            let sentinel = (self.extract_fn)(&s);
            beam.tick(Imperfect::Partial(sentinel, ShannonLoss::new(f64::INFINITY)))
        }
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let a = beam.result().ok().expect("project: Err beam").clone();
        beam.next(a)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let a = beam.result().ok().expect("refract: Err beam").clone();
        beam.next(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Beam as BeamTrait;

    #[derive(Clone, Debug, PartialEq)]
    enum Shape {
        Circle(i32),
        Square(i32),
    }

    fn shape_is_circle(s: &Shape) -> bool {
        matches!(s, Shape::Circle(_))
    }

    fn shape_extract_circle(s: &Shape) -> i32 {
        if let Shape::Circle(r) = s { *r } else { -1 }
    }

    fn shape_review_circle(r: i32) -> Shape {
        Shape::Circle(r)
    }

    fn circle_prism() -> OpticPrism<Shape, i32> {
        OpticPrism::new(shape_is_circle, shape_extract_circle, shape_review_circle)
    }

    // --- Inherent method tests ---

    #[test]
    fn optic_prism_matches() {
        let p = circle_prism();
        assert!(p.matches(&Shape::Circle(5)));
        assert!(!p.matches(&Shape::Square(3)));
    }

    #[test]
    fn optic_prism_extract_some_on_match() {
        let p = circle_prism();
        assert_eq!(p.extract(&Shape::Circle(5)), Some(5));
    }

    #[test]
    fn optic_prism_extract_none_on_mismatch() {
        let p = circle_prism();
        assert_eq!(p.extract(&Shape::Square(3)), None);
    }

    #[test]
    fn optic_prism_review() {
        let p = circle_prism();
        assert_eq!(p.review(7), Shape::Circle(7));
    }

    // --- Prism trait tests ---

    fn seed<T: Clone>(v: T) -> PureBeam<(), T, Infallible, ShannonLoss> {
        PureBeam::ok((), v)
    }

    #[test]
    fn optic_prism_focus_matching_is_lossless() {
        let p = circle_prism();
        let beam = seed(Shape::Circle(42));
        let focused = p.focus(beam);
        assert_eq!(focused.result().ok(), Some(&42));
        assert!(!focused.is_partial());
    }

    #[test]
    fn optic_prism_focus_nonmatch_produces_infinite_loss() {
        let p = circle_prism();
        let beam = seed(Shape::Square(3));
        let focused = p.focus(beam);
        // Should be partial with infinite loss
        assert!(focused.is_partial());
        // The sentinel value is accessible
        assert_eq!(focused.result().ok(), Some(&-1));
    }

    #[test]
    fn optic_prism_full_pipeline_matching() {
        let p = circle_prism();
        let focused = p.focus(seed(Shape::Circle(10)));
        let projected = p.project(focused);
        let refracted = p.refract(projected);
        assert_eq!(refracted.result().ok(), Some(&10));
        assert!(!refracted.is_partial());
    }

    #[test]
    fn optic_prism_full_pipeline_nonmatch_carries_loss() {
        let p = circle_prism();
        let focused = p.focus(seed(Shape::Square(3)));
        assert!(focused.is_partial());
        let projected = p.project(focused);
        // Loss propagates through the pipeline
        assert!(projected.is_partial());
        let refracted = p.refract(projected);
        assert!(refracted.is_partial());
    }

    #[test]
    fn optic_prism_is_clone_and_copy() {
        let p = circle_prism();
        let p2 = p; // Copy
        let p3 = p2.clone(); // Clone
        assert!(p3.matches(&Shape::Circle(1)));
    }
}

//! OpticPrism — the semidet bidirectional optic.
//!
//! Named OpticPrism (not Prism) to avoid collision with our crate's
//! central Prism trait. Represents a sum-type case: preview may fail
//! (the case doesn't match), but review always reconstructs the whole.

use crate::{Beam, Prism, ShannonLoss, Stage};
use std::marker::PhantomData;

pub struct OpticPrism<S, A> {
    match_fn: Box<dyn Fn(&S) -> bool>,
    extract_fn: Box<dyn Fn(&S) -> A>,
    review_fn: Box<dyn Fn(A) -> S>,
    _phantom: PhantomData<(S, A)>,
}

impl<S: 'static, A: 'static> OpticPrism<S, A> {
    /// Construct a Prism from three closures:
    ///
    /// - `matches`: returns `true` when `S` is the variant this prism targets.
    /// - `extract`: given any `&S`, returns an `A`. Called regardless of whether
    ///   `matches` returned true — the closure author is responsible for returning
    ///   a safe sentinel value on non-matching inputs. The sentinel is the author's
    ///   choice; the framework does not impose `A::default()`.
    /// - `review`: reconstructs an `S` from an `A` (always succeeds).
    ///
    /// Refutation (non-matching input) is encoded as `ShannonLoss::INFINITY` in the
    /// resulting `Beam<A>`. The `Beam::result` field is the sentinel from `extract`;
    /// downstream consumers MUST check `beam.loss` before reading `beam.result`.
    pub fn new<M, E, R>(matches: M, extract: E, review: R) -> Self
    where
        M: Fn(&S) -> bool + 'static,
        E: Fn(&S) -> A + 'static,
        R: Fn(A) -> S + 'static,
    {
        OpticPrism {
            match_fn: Box::new(matches),
            extract_fn: Box::new(extract),
            review_fn: Box::new(review),
            _phantom: PhantomData,
        }
    }

    /// Returns `true` if `s` is the variant this prism targets.
    pub fn matches(&self, s: &S) -> bool {
        (self.match_fn)(s)
    }

    /// Returns `Some(A)` on a matching `s`, `None` otherwise.
    /// Inherent convenience method — `Option` is internal API, not on the trait surface.
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

impl<S: Clone + 'static, A: Clone + 'static> Prism for OpticPrism<S, A> {
    type Input = S;
    type Focused = A;      // refutation lives in ShannonLoss, never in Option
    type Projected = A;
    type Part = A;
    type Crystal = OpticPrismCrystal<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<A> {
        // Always call extract_fn. When the input doesn't match, the closure
        // author's sentinel is used as the result value and loss is set to
        // infinity. Downstream consumers must check loss before using result.
        let a = (self.extract_fn)(&beam.result);
        let loss = if (self.match_fn)(&beam.result) {
            beam.loss
        } else {
            ShannonLoss::new(f64::INFINITY)
        };
        Beam {
            result: a,
            path: beam.path,
            loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
        }
    }

    fn project(&self, beam: Beam<A>) -> Beam<A> {
        Beam { stage: Stage::Projected, ..beam }
    }

    fn split(&self, beam: Beam<A>) -> Vec<Beam<A>> {
        vec![Beam { stage: Stage::Split, ..beam }]
    }

    fn zoom(&self, beam: Beam<A>, f: &dyn Fn(Beam<A>) -> Beam<A>) -> Beam<A> {
        f(beam)
    }

    fn refract(&self, beam: Beam<A>) -> Beam<OpticPrismCrystal<S, A>> {
        Beam {
            result: OpticPrismCrystal { _phantom: PhantomData },
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}

pub struct OpticPrismCrystal<S, A> {
    _phantom: PhantomData<(S, A)>,
}

impl<S: Clone + 'static, A: Clone + 'static> Prism for OpticPrismCrystal<S, A> {
    type Input = A;
    type Focused = A;
    type Projected = A;
    type Part = A;
    type Crystal = OpticPrismCrystal<S, A>;

    fn focus(&self, beam: Beam<A>) -> Beam<A> { Beam { stage: Stage::Focused, ..beam } }
    fn project(&self, beam: Beam<A>) -> Beam<A> { Beam { stage: Stage::Projected, ..beam } }
    fn split(&self, beam: Beam<A>) -> Vec<Beam<A>> { vec![Beam { stage: Stage::Split, ..beam }] }
    fn zoom(&self, beam: Beam<A>, f: &dyn Fn(Beam<A>) -> Beam<A>) -> Beam<A> { f(beam) }
    fn refract(&self, beam: Beam<A>) -> Beam<OpticPrismCrystal<S, A>> {
        Beam {
            result: OpticPrismCrystal { _phantom: PhantomData },
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shape has NO Default derive — verifying A: Default is not required.
    #[derive(Clone, Debug, PartialEq)]
    enum Shape {
        Circle(i32),
        Square(i32),
        Empty,
    }

    fn circle_prism() -> OpticPrism<Shape, i32> {
        OpticPrism::new(
            |s: &Shape| matches!(s, Shape::Circle(_)),            // match_fn
            |s: &Shape| if let Shape::Circle(r) = s { *r } else { -1 }, // extract_fn: sentinel -1 on no-match
            |r: i32| Shape::Circle(r),                            // review_fn
        )
    }

    #[test]
    fn optic_prism_matches_reports_true_for_circle() {
        let p = circle_prism();
        assert!(p.matches(&Shape::Circle(5)));
        assert!(!p.matches(&Shape::Square(3)));
        assert!(!p.matches(&Shape::Empty));
    }

    #[test]
    fn optic_prism_extract_returns_some_on_match_none_on_mismatch() {
        let p = circle_prism();
        assert_eq!(p.extract(&Shape::Circle(5)), Some(5));
        assert_eq!(p.extract(&Shape::Square(3)), None);
    }

    #[test]
    fn optic_prism_review_reconstructs() {
        let p = circle_prism();
        assert_eq!(p.review(7), Shape::Circle(7));
    }

    // Prism trait: focus on a MATCHING input → lossless Beam<A> (no Option in type).
    #[test]
    fn optic_prism_focus_matching_case_is_lossless() {
        let p = circle_prism();
        let beam: Beam<i32> = p.focus(Beam::new(Shape::Circle(42)));
        assert_eq!(beam.result, 42);
        assert!(beam.loss.is_zero());
        assert_eq!(beam.stage, Stage::Focused);
    }

    // Prism trait: focus on a NON-MATCHING input → infinite-loss Beam<A>,
    // result is the sentinel the closure author chose (not A::default()).
    #[test]
    fn optic_prism_focus_nonmatch_produces_infinite_loss_without_default() {
        let p = circle_prism();
        let beam: Beam<i32> = p.focus(Beam::new(Shape::Square(3)));
        assert!(beam.loss.as_f64().is_infinite(), "loss must be infinite on refutation");
        // The sentinel is -1, which proves we did NOT call A::default().
        // (i32::default() would be 0, not -1.)
        assert_eq!(beam.result, -1, "extract_fn sentinel must be -1, not A::default()");
        assert_eq!(beam.stage, Stage::Focused);
    }

    // Prism trait: project on a matched beam is a simple stage transition.
    #[test]
    fn optic_prism_project_matching_case_is_lossless() {
        let p = circle_prism();
        let focused = p.focus(Beam::new(Shape::Circle(42)));
        let projected = p.project(focused);
        assert_eq!(projected.result, 42);
        assert!(projected.loss.is_zero());
        assert_eq!(projected.stage, Stage::Projected);
    }

    // Prism trait: project on a refuted beam preserves infinite loss.
    #[test]
    fn optic_prism_project_preserves_infinite_loss() {
        let p = circle_prism();
        let focused = p.focus(Beam::new(Shape::Square(3)));
        assert!(focused.loss.as_f64().is_infinite());
        let projected = p.project(focused);
        assert!(projected.loss.as_f64().is_infinite());
        assert_eq!(projected.stage, Stage::Projected);
    }
}

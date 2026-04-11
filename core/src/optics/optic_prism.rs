//! OpticPrism — the semidet bidirectional optic.
//!
//! Named OpticPrism (not Prism) to avoid collision with our crate's
//! central Prism trait. Represents a sum-type case: preview may fail
//! (the case doesn't match), but review always reconstructs the whole.

use crate::{Beam, Prism, ShannonLoss, Stage};

#[derive(Clone, Copy)]
pub struct OpticPrism<S, A> {
    match_fn: fn(&S) -> bool,
    extract_fn: fn(&S) -> A,
    review_fn: fn(A) -> S,
}

impl<S: 'static, A: 'static> OpticPrism<S, A> {
    /// Construct an OpticPrism (the semidet bidirectional optic) from a
    /// match function, extract function, and review function.
    ///
    /// # Laws
    ///
    /// The caller is responsible for ensuring the prism laws hold:
    ///
    /// - When `matches(s)` is true: `review(extract(s)) ≡ s` (review undoes extract)
    /// - For any `a: A`: `matches(review(a))` is true (review produces a valid case)
    /// - When `matches(s)` is true: `extract(s)` returns the embedded `A` (not a sentinel)
    /// - When `matches(s)` is false: `extract(s)` may return any sentinel `A` of
    ///   the closure author's choice. The framework will mark the resulting beam
    ///   with `ShannonLoss::infinite()`. Downstream consumers MUST check `loss`
    ///   before reading `result`. The sentinel value is undefined behavior at
    ///   the value level on infinite-loss beams.
    ///
    /// These cannot be enforced by the type system.
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
    ///
    /// This is an INHERENT CONVENIENCE method and returns `Option<A>` by
    /// design — it is NOT part of the `Prism` trait surface and does NOT
    /// participate in the spectral framework's loss-based refutation channel.
    /// The `Option` here is the standard Rust idiom for "did the case match,"
    /// unrelated to the Prism pipeline's encoding of refutation.
    ///
    /// For the Prism pipeline, use `focus` → `project` → `refract` via the
    /// `apply` free function or explicit calls. The trait surface encodes
    /// refutation as `ShannonLoss::infinite()` on the returned Beam, never
    /// as `Option` or `Result`. See N1 in
    /// `docs/seam-taut-rereview-2026-04-08.md`.
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
    type Crystal = OpticPrism<S, A>;

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
            connection: beam.connection,
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

    fn refract(&self, beam: Beam<A>) -> Beam<OpticPrism<S, A>> {
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

    // Shape has NO Default derive — verifying A: Default is not required.
    #[derive(Clone, Debug, PartialEq)]
    enum Shape {
        Circle(i32),
        Square(i32),
        Empty,
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

    #[test]
    fn optic_prism_is_clone_and_copy() {
        let p = circle_prism();
        let p2 = p; // Copy
        let p3 = p2.clone(); // Clone
        assert!(p3.matches(&Shape::Circle(1)));
    }
}

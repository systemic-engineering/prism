//! OpticPrism — the semidet bidirectional optic.
//!
//! Named OpticPrism (not Prism) to avoid collision with our crate's
//! central Prism trait. Represents a sum-type case: preview may fail
//! (the case doesn't match), but review always reconstructs the whole.

use crate::{Beam, Prism, ShannonLoss, Stage};
use std::marker::PhantomData;

pub struct OpticPrism<S, A> {
    preview_fn: Box<dyn Fn(&S) -> Option<A>>,
    review_fn: Box<dyn Fn(A) -> S>,
    _phantom: PhantomData<(S, A)>,
}

impl<S: 'static, A: 'static> OpticPrism<S, A> {
    pub fn new<P, R>(preview: P, review: R) -> Self
    where
        P: Fn(&S) -> Option<A> + 'static,
        R: Fn(A) -> S + 'static,
    {
        OpticPrism {
            preview_fn: Box::new(preview),
            review_fn: Box::new(review),
            _phantom: PhantomData,
        }
    }

    pub fn preview(&self, s: &S) -> Option<A> {
        (self.preview_fn)(s)
    }

    pub fn review(&self, a: A) -> S {
        (self.review_fn)(a)
    }
}

impl<S: Clone + Default + 'static, A: Clone + Default + 'static> Prism for OpticPrism<S, A> {
    type Input = S;
    type Focused = Option<A>;
    type Projected = A;
    type Part = A;
    type Crystal = OpticPrismCrystal<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<Option<A>> {
        todo!()
    }

    fn project(&self, beam: Beam<Option<A>>) -> Beam<A> {
        todo!()
    }

    fn split(&self, beam: Beam<A>) -> Vec<Beam<A>> {
        todo!()
    }

    fn zoom(&self, beam: Beam<A>, f: &dyn Fn(Beam<A>) -> Beam<A>) -> Beam<A> {
        todo!()
    }

    fn refract(&self, beam: Beam<A>) -> Beam<OpticPrismCrystal<S, A>> {
        todo!()
    }
}

pub struct OpticPrismCrystal<S, A> {
    _phantom: PhantomData<(S, A)>,
}

impl<S: Clone + Default + 'static, A: Clone + Default + 'static> Prism for OpticPrismCrystal<S, A> {
    type Input = A;
    type Focused = A;
    type Projected = A;
    type Part = A;
    type Crystal = OpticPrismCrystal<S, A>;

    fn focus(&self, beam: Beam<A>) -> Beam<A> { todo!() }
    fn project(&self, beam: Beam<A>) -> Beam<A> { todo!() }
    fn split(&self, beam: Beam<A>) -> Vec<Beam<A>> { todo!() }
    fn zoom(&self, beam: Beam<A>, f: &dyn Fn(Beam<A>) -> Beam<A>) -> Beam<A> { todo!() }
    fn refract(&self, beam: Beam<A>) -> Beam<OpticPrismCrystal<S, A>> { todo!() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default, Debug, PartialEq)]
    enum Shape {
        Circle(i32),
        Square(i32),
        #[default]
        Empty,
    }

    #[test]
    fn optic_prism_preview_succeeds_for_matching_case() {
        let circle_prism: OpticPrism<Shape, i32> = OpticPrism::new(
            |s: &Shape| if let Shape::Circle(r) = s { Some(*r) } else { None },
            |r: i32| Shape::Circle(r),
        );

        assert_eq!(circle_prism.preview(&Shape::Circle(5)), Some(5));
        assert_eq!(circle_prism.preview(&Shape::Square(3)), None);
    }

    #[test]
    fn optic_prism_review_reconstructs() {
        let circle_prism: OpticPrism<Shape, i32> = OpticPrism::new(
            |s: &Shape| if let Shape::Circle(r) = s { Some(*r) } else { None },
            |r: i32| Shape::Circle(r),
        );

        assert_eq!(circle_prism.review(7), Shape::Circle(7));
    }

    #[test]
    fn optic_prism_project_encodes_refutation_as_infinite_loss() {
        let circle_prism: OpticPrism<Shape, i32> = OpticPrism::new(
            |s: &Shape| if let Shape::Circle(r) = s { Some(*r) } else { None },
            |r: i32| Shape::Circle(r),
        );

        // Focus a Square — preview yields None.
        let beam = Beam::new(Shape::Square(3));
        let focused = circle_prism.focus(beam);
        assert_eq!(focused.result, None);

        // Project the None — loss becomes infinite.
        let projected = circle_prism.project(focused);
        assert!(projected.loss.as_f64().is_infinite());
    }

    #[test]
    fn optic_prism_project_matching_case_is_lossless() {
        let circle_prism: OpticPrism<Shape, i32> = OpticPrism::new(
            |s: &Shape| if let Shape::Circle(r) = s { Some(*r) } else { None },
            |r: i32| Shape::Circle(r),
        );

        let beam = Beam::new(Shape::Circle(42));
        let focused = circle_prism.focus(beam);
        let projected = circle_prism.project(focused);
        assert_eq!(projected.result, 42);
        assert!(projected.loss.is_zero());
    }
}

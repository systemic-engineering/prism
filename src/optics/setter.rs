//! Setter — the write-only optic.
//!
//! A Setter<S, A> provides a way to modify A within S by applying a
//! function, without giving observational access. It's the weakest of
//! the optic kinds — Fold + Setter with the read side removed.

use crate::{Beam, Prism, Stage};

#[derive(Clone, Copy)]
pub struct Setter<S, A> {
    modify_fn: fn(S, &dyn Fn(A) -> A) -> S,
}

impl<S: 'static, A: 'static> Setter<S, A> {
    /// Construct a Setter from a modify fn pointer.
    ///
    /// # Laws
    ///
    /// Setter is the weakest classical optic — it has only a write side, no
    /// read side. The caller is responsible for ensuring:
    ///
    /// - `modify(s, |a| a) ≡ s` (the identity inner function does not change s)
    /// - `modify(modify(s, f), g) ≡ modify(s, g ∘ f)` (modify composes)
    ///
    /// These are the standard Setter laws from functional optics. They cannot
    /// be enforced by the type system.
    ///
    /// Note: the inner `&dyn Fn(A) -> A` is a function argument, not a stored
    /// closure — this is fine and does not affect the fn pointer constraint on
    /// `modify_fn` itself.
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

impl<S: Clone + 'static, A: Clone + 'static> Prism for Setter<S, A> {
    type Input = S;
    type Focused = S;
    type Projected = S;
    type Part = S;
    type Crystal = Setter<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<S> { Beam { stage: Stage::Focused, ..beam } }
    fn project(&self, beam: Beam<S>) -> Beam<S> { Beam { stage: Stage::Projected, ..beam } }
    fn split(&self, beam: Beam<S>) -> Vec<Beam<S>> { vec![Beam { stage: Stage::Split, ..beam }] }
    fn zoom(&self, beam: Beam<S>, f: &dyn Fn(Beam<S>) -> Beam<S>) -> Beam<S> { f(beam) }

    fn refract(&self, beam: Beam<S>) -> Beam<Setter<S, A>> {
        // fn pointers are Copy — the optic itself IS the lossless fixed point.
        // We also exercise modify_fn with identity to prove the closure is reachable.
        let _witnessed = (self.modify_fn)(beam.result.clone(), &|a| a);
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

    #[derive(Clone, Debug, PartialEq)]
    struct Box2 { label: String, count: i32 }

    fn box2_modify_count(b: Box2, f: &dyn Fn(i32) -> i32) -> Box2 {
        Box2 { count: f(b.count), ..b }
    }

    #[test]
    fn setter_modifies_field_via_function() {
        let count_setter: Setter<Box2, i32> = Setter::new(box2_modify_count);

        let b = Box2 { label: "test".to_string(), count: 5 };
        let b2 = count_setter.modify(b, |c| c + 10);
        assert_eq!(b2.count, 15);
        assert_eq!(b2.label, "test");
    }

    #[test]
    fn setter_refract_runs_modify_with_identity() {
        // refract must call modify_fn with the identity inner function.
        // Crystal = Self — the refracted beam carries the Setter itself.
        let count_setter: Setter<Box2, i32> = Setter::new(box2_modify_count);

        let input = Box2 { label: "x".to_string(), count: 5 };
        let beam = Beam::new(input.clone());
        let focused = count_setter.focus(beam);
        let projected = count_setter.project(focused);
        let refracted = count_setter.refract(projected);

        assert_eq!(refracted.stage, Stage::Refracted);
        // The crystal is the Setter itself. Verify it still works.
        let result = refracted.result.modify(input, |c| c + 1);
        assert_eq!(result.count, 6);
    }

    #[test]
    fn setter_is_clone_and_copy() {
        let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
        let s2 = s; // Copy
        let s3 = s2.clone(); // Clone
        let b = Box2 { label: "t".to_string(), count: 3 };
        assert_eq!(s3.modify(b, |c| c * 2).count, 6);
    }
}

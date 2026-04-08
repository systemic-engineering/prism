//! Setter — the write-only optic.
//!
//! A Setter<S, A> provides a way to modify A within S by applying a
//! function, without giving observational access. It's the weakest of
//! the optic kinds — Fold + Setter with the read side removed.

use crate::{Beam, Prism, Stage};
use std::marker::PhantomData;

pub struct Setter<S, A> {
    modify_fn: Box<dyn Fn(S, &dyn Fn(A) -> A) -> S>,
    _phantom: PhantomData<(S, A)>,
}

impl<S: 'static, A: 'static> Setter<S, A> {
    /// Construct a Setter from a modify function.
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
    pub fn new<M>(modify: M) -> Self
    where
        M: Fn(S, &dyn Fn(A) -> A) -> S + 'static,
    {
        Setter {
            modify_fn: Box::new(modify),
            _phantom: PhantomData,
        }
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
    type Crystal = SetterCrystal<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<S> { Beam { stage: Stage::Focused, ..beam } }
    fn project(&self, beam: Beam<S>) -> Beam<S> { Beam { stage: Stage::Projected, ..beam } }
    fn split(&self, beam: Beam<S>) -> Vec<Beam<S>> { vec![Beam { stage: Stage::Split, ..beam }] }
    fn zoom(&self, beam: Beam<S>, f: &dyn Fn(Beam<S>) -> Beam<S>) -> Beam<S> { f(beam) }
    fn refract(&self, beam: Beam<S>) -> Beam<SetterCrystal<S, A>> {
        // Call modify_fn with the identity inner function. This exercises the
        // closure and proves it's reachable from the Prism trait surface.
        // With identity, the result is semantically equal to the input S, but
        // the closure has run. The crystal records the witnessed S.
        let modified = (self.modify_fn)(beam.result, &|a| a);
        Beam {
            result: SetterCrystal { witnessed: modified, _phantom: PhantomData },
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}

pub struct SetterCrystal<S, A> {
    pub witnessed: S,
    _phantom: PhantomData<A>,
}

impl<S: Clone + 'static, A: Clone + 'static> Prism for SetterCrystal<S, A> {
    type Input = S;
    type Focused = S;
    type Projected = S;
    type Part = S;
    type Crystal = SetterCrystal<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<S> { Beam { stage: Stage::Focused, ..beam } }
    fn project(&self, beam: Beam<S>) -> Beam<S> { Beam { stage: Stage::Projected, ..beam } }
    fn split(&self, beam: Beam<S>) -> Vec<Beam<S>> { vec![Beam { stage: Stage::Split, ..beam }] }
    fn zoom(&self, beam: Beam<S>, f: &dyn Fn(Beam<S>) -> Beam<S>) -> Beam<S> { f(beam) }
    fn refract(&self, beam: Beam<S>) -> Beam<SetterCrystal<S, A>> {
        Beam {
            result: SetterCrystal { witnessed: beam.result.clone(), _phantom: PhantomData },
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

    #[derive(Clone, Debug, PartialEq)]
    struct Box2 { label: String, count: i32 }

    #[test]
    fn setter_modifies_field_via_function() {
        let count_setter: Setter<Box2, i32> = Setter::new(
            |b: Box2, f: &dyn Fn(i32) -> i32| Box2 { count: f(b.count), ..b },
        );

        let b = Box2 { label: "test".to_string(), count: 5 };
        let b2 = count_setter.modify(b, |c| c + 10);
        assert_eq!(b2.count, 15);
        assert_eq!(b2.label, "test");
    }

    #[test]
    fn setter_refract_runs_modify_with_identity_and_witnesses_value() {
        // refract must call modify_fn with the identity inner function.
        // Because the inner fn is identity, the witnessed S equals the input.
        // This proves the closure is reachable from the Prism trait surface.
        let count_setter: Setter<Box2, i32> = Setter::new(
            |b: Box2, f: &dyn Fn(i32) -> i32| Box2 { count: f(b.count), ..b },
        );

        let input = Box2 { label: "x".to_string(), count: 5 };
        let beam = Beam::new(input.clone());
        let focused = count_setter.focus(beam);
        let projected = count_setter.project(focused);
        let refracted = count_setter.refract(projected);

        assert_eq!(refracted.stage, Stage::Refracted);
        // Identity modify is a no-op at the A level, so witnessed == input.
        assert_eq!(refracted.result.witnessed, input);
    }
}

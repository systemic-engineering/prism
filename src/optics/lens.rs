//! Lens — total bidirectional single-focus optic.
//!
//! A Lens<S, A> gives total access to a part A within a whole S.
//! The view function extracts A; the modify function updates A
//! within S. Laws:
//! - `view(set(s, a)) = a`        (set-view)
//! - `set(s, view(s)) = s`        (view-set)
//! - `set(set(s, a1), a2) = set(s, a2)`  (set-set)

use crate::{Beam, Prism, Stage};
use crate::optics::phantom_crystal::PhantomCrystal;
use std::marker::PhantomData;

pub struct Lens<S, A> {
    view_fn: Box<dyn Fn(&S) -> A>,
    set_fn: Box<dyn Fn(S, A) -> S>,
    _phantom: PhantomData<(S, A)>,
}

/// Type-level marker for Lens<S, A> crystals.
#[derive(Clone)]
pub struct LensMarker<S, A> {
    _phantom: PhantomData<(S, A)>,
}

impl<S: 'static, A: 'static> Lens<S, A> {
    /// Construct a Lens from a view and set function.
    ///
    /// # Laws
    ///
    /// The caller is responsible for ensuring the lens laws hold:
    ///
    /// - `view(set(s, a)) ≡ a` (set-view: setting then viewing returns what was set)
    /// - `set(s, view(s)) ≡ s` (view-set: viewing then setting returns the original)
    /// - `set(set(s, a1), a2) ≡ set(s, a2)` (set-set: setting twice equals setting once)
    ///
    /// These cannot be enforced by the type system. A Lens constructed from
    /// functions that violate any of these laws will compile and run but will
    /// produce results that are meaningless under the Lens laws.
    pub fn new<V, U>(view: V, set: U) -> Self
    where
        V: Fn(&S) -> A + 'static,
        U: Fn(S, A) -> S + 'static,
    {
        Lens {
            view_fn: Box::new(view),
            set_fn: Box::new(set),
            _phantom: PhantomData,
        }
    }

    pub fn view(&self, s: &S) -> A {
        (self.view_fn)(s)
    }

    pub fn set(&self, s: S, a: A) -> S {
        (self.set_fn)(s, a)
    }

    pub fn modify<F>(&self, s: S, f: F) -> S
    where
        F: Fn(A) -> A,
    {
        let a = self.view(&s);
        self.set(s, f(a))
    }
}

impl<S: Clone + 'static, A: Clone + 'static> Prism for Lens<S, A> {
    type Input = S;
    type Focused = A;
    type Projected = A;
    type Part = A;
    type Crystal = PhantomCrystal<LensMarker<S, A>>;

    fn focus(&self, beam: Beam<S>) -> Beam<A> {
        let a = (self.view_fn)(&beam.result);
        Beam {
            result: a,
            path: beam.path,
            loss: beam.loss,
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

    fn zoom(
        &self,
        beam: Beam<A>,
        f: &dyn Fn(Beam<A>) -> Beam<A>,
    ) -> Beam<A> {
        f(beam)
    }

    fn refract(&self, beam: Beam<A>) -> Beam<PhantomCrystal<LensMarker<S, A>>> {
        Beam {
            result: PhantomCrystal::new(),
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
    struct Point { x: i32, y: i32 }

    #[test]
    fn lens_views_and_sets_field() {
        let x_lens: Lens<Point, i32> = Lens::new(
            |p: &Point| p.x,
            |p: Point, new_x: i32| Point { x: new_x, ..p },
        );

        let p = Point { x: 3, y: 5 };
        assert_eq!(x_lens.view(&p), 3);
        let p2 = x_lens.set(p, 10);
        assert_eq!(p2.x, 10);
        assert_eq!(p2.y, 5);
    }

    #[test]
    fn lens_view_set_law() {
        let x_lens: Lens<Point, i32> = Lens::new(
            |p: &Point| p.x,
            |p: Point, new_x: i32| Point { x: new_x, ..p },
        );
        let p = Point { x: 3, y: 5 };
        let viewed = x_lens.view(&p);
        let restored = x_lens.set(p.clone(), viewed);
        assert_eq!(restored, p);
    }

    #[test]
    fn lens_set_view_law() {
        let x_lens: Lens<Point, i32> = Lens::new(
            |p: &Point| p.x,
            |p: Point, new_x: i32| Point { x: new_x, ..p },
        );
        let p = Point { x: 3, y: 5 };
        let set_p = x_lens.set(p, 99);
        assert_eq!(x_lens.view(&set_p), 99);
    }

    #[test]
    fn lens_refract_as_prism() {
        let x_lens: Lens<Point, i32> = Lens::new(
            |p: &Point| p.x,
            |p: Point, new_x: i32| Point { x: new_x, ..p },
        );
        let beam = Beam::new(Point { x: 3, y: 5 });
        let focused = x_lens.focus(beam);
        assert_eq!(focused.result, 3);
        assert_eq!(focused.stage, Stage::Focused);
    }
}

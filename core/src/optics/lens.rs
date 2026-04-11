//! Lens — total bidirectional single-focus optic.
//!
//! A Lens<S, A> gives total access to a part A within a whole S.
//! The view function extracts A; the modify function updates A
//! within S. Laws:
//! - `view(set(s, a)) = a`        (set-view)
//! - `set(s, view(s)) = s`        (view-set)
//! - `set(set(s, a1), a2) = set(s, a2)`  (set-set)

use crate::{Beam, Prism, Stage};

#[derive(Clone, Copy)]
pub struct Lens<S, A> {
    view_fn: fn(&S) -> A,
    set_fn: fn(S, A) -> S,
}

impl<S: 'static, A: 'static> Lens<S, A> {
    /// Construct a Lens from a view and set fn pointer.
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
    pub fn new(view: fn(&S) -> A, set: fn(S, A) -> S) -> Self {
        Lens { view_fn: view, set_fn: set }
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
    type Crystal = Lens<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<A> {
        let a = (self.view_fn)(&beam.result);
        Beam {
            result: a,
            path: beam.path,
            loss: beam.loss,
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

    fn zoom(
        &self,
        beam: Beam<A>,
        f: &dyn Fn(Beam<A>) -> Beam<A>,
    ) -> Beam<A> {
        f(beam)
    }

    fn refract(&self, beam: Beam<A>) -> Beam<Lens<S, A>> {
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

    #[derive(Clone, Debug, PartialEq)]
    struct Point { x: i32, y: i32 }

    fn point_view_x(p: &Point) -> i32 { p.x }
    fn point_set_x(p: Point, new_x: i32) -> Point { Point { x: new_x, ..p } }

    #[test]
    fn lens_views_and_sets_field() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);

        let p = Point { x: 3, y: 5 };
        assert_eq!(x_lens.view(&p), 3);
        let p2 = x_lens.set(p, 10);
        assert_eq!(p2.x, 10);
        assert_eq!(p2.y, 5);
    }

    #[test]
    fn lens_view_set_law() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let p = Point { x: 3, y: 5 };
        let viewed = x_lens.view(&p);
        let restored = x_lens.set(p.clone(), viewed);
        assert_eq!(restored, p);
    }

    #[test]
    fn lens_set_view_law() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let p = Point { x: 3, y: 5 };
        let set_p = x_lens.set(p, 99);
        assert_eq!(x_lens.view(&set_p), 99);
    }

    #[test]
    fn lens_refract_as_prism() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let beam = Beam::new(Point { x: 3, y: 5 });
        let focused = x_lens.focus(beam);
        assert_eq!(focused.result, 3);
        assert_eq!(focused.stage, Stage::Focused);
    }

    #[test]
    fn lens_is_clone_and_copy() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let x_lens2 = x_lens; // Copy
        let x_lens3 = x_lens2.clone(); // Clone
        assert_eq!(x_lens3.view(&Point { x: 7, y: 0 }), 7);
    }
}

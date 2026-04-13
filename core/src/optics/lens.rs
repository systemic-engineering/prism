//! Lens — total bidirectional single-focus optic.
//!
//! A Lens<S, A> gives total access to a part A within a whole S.
//! Laws:
//! - `view(set(s, a)) = a`        (set-view)
//! - `set(s, view(s)) = s`        (view-set)
//! - `set(set(s, a1), a2) = set(s, a2)`  (set-set)
//!
//! As a Prism: focus views (S→A), project is identity, refract returns A.
//! The original S is available as the beam's input chain for reconstruction.

use crate::ScalarLoss;
use crate::{Beam, Optic, Prism};
use std::convert::Infallible;

#[derive(Clone, Copy)]
pub struct Lens<S, A> {
    view_fn: fn(&S) -> A,
    set_fn: fn(S, A) -> S,
}

impl<S: 'static, A: 'static> Lens<S, A> {
    /// Construct a Lens from a view and set fn pointer.
    ///
    /// # Laws
    /// - `view(set(s, a)) ≡ a` (set-view)
    /// - `set(s, view(s)) ≡ s` (view-set)
    /// - `set(set(s, a1), a2) ≡ set(s, a2)` (set-set)
    pub fn new(view: fn(&S) -> A, set: fn(S, A) -> S) -> Self {
        Lens {
            view_fn: view,
            set_fn: set,
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

/// Lens implements Prism with Optic.
///
/// Pipeline flow:
/// - focus: view (S → A)
/// - project: identity (A → A)
/// - refract: identity (A → A) — the focused value is the final output
impl<S: Clone + 'static, A: Clone + 'static> Prism for Lens<S, A> {
    type Input = Optic<(), S, Infallible, ScalarLoss>;
    type Focused = Optic<S, A, Infallible, ScalarLoss>;
    type Projected = Optic<A, A, Infallible, ScalarLoss>;
    type Refracted = Optic<A, A, Infallible, ScalarLoss>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let s = beam.result().ok().expect("focus: Err beam").clone();
        let a = (self.view_fn)(&s);
        beam.next(a)
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
    struct Point {
        x: i32,
        y: i32,
    }

    fn point_view_x(p: &Point) -> i32 {
        p.x
    }
    fn point_set_x(p: Point, new_x: i32) -> Point {
        Point { x: new_x, ..p }
    }

    // --- Inherent method tests ---

    #[test]
    fn lens_view() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let p = Point { x: 3, y: 5 };
        assert_eq!(x_lens.view(&p), 3);
    }

    #[test]
    fn lens_set() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let p = Point { x: 3, y: 5 };
        let p2 = x_lens.set(p, 10);
        assert_eq!(p2.x, 10);
        assert_eq!(p2.y, 5);
    }

    #[test]
    fn lens_modify() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let p = Point { x: 3, y: 5 };
        let p2 = x_lens.modify(p, |x| x * 2);
        assert_eq!(p2.x, 6);
        assert_eq!(p2.y, 5);
    }

    // --- Lens laws ---

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
    fn lens_set_set_law() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let p = Point { x: 3, y: 5 };
        let set_once = x_lens.set(p.clone(), 10);
        let set_twice = x_lens.set(set_once, 20);
        let set_direct = x_lens.set(p, 20);
        assert_eq!(set_twice, set_direct);
    }

    // --- Prism trait tests ---

    fn seed<T: Clone>(v: T) -> Optic<(), T, Infallible, ScalarLoss> {
        Optic::ok((), v)
    }

    #[test]
    fn lens_prism_focus_views() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let beam = seed(Point { x: 42, y: 0 });
        let focused = x_lens.focus(beam);
        assert_eq!(focused.result().ok(), Some(&42));
        // The input of the focused beam is the original S
        assert_eq!(focused.input(), &Point { x: 42, y: 0 });
    }

    #[test]
    fn lens_prism_project_passes_through() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let focused = x_lens.focus(seed(Point { x: 7, y: 3 }));
        let projected = x_lens.project(focused);
        assert_eq!(projected.result().ok(), Some(&7));
    }

    #[test]
    fn lens_prism_refract_returns_value() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let focused = x_lens.focus(seed(Point { x: 5, y: 2 }));
        let projected = x_lens.project(focused);
        let refracted = x_lens.refract(projected);
        assert_eq!(refracted.result().ok(), Some(&5));
    }

    #[test]
    fn lens_prism_is_lossless() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let focused = x_lens.focus(seed(Point { x: 1, y: 2 }));
        assert!(!focused.is_partial());
        let projected = x_lens.project(focused);
        assert!(!projected.is_partial());
        let refracted = x_lens.refract(projected);
        assert!(!refracted.is_partial());
    }

    #[test]
    fn lens_is_clone_and_copy() {
        let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
        let x_lens2 = x_lens; // Copy
        let x_lens3 = x_lens2.clone(); // Clone
        assert_eq!(x_lens3.view(&Point { x: 7, y: 0 }), 7);
    }
}

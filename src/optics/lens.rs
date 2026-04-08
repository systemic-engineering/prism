//! Lens — total bidirectional single-focus optic.
//!
//! A Lens<S, A> gives total access to a part A within a whole S.
//! The view function extracts A; the modify function updates A
//! within S. Laws:
//! - `view(set(s, a)) = a`        (set-view)
//! - `set(s, view(s)) = s`        (view-set)
//! - `set(set(s, a1), a2) = set(s, a2)`  (set-set)

use crate::{Beam, Prism, Stage};
use std::marker::PhantomData;

pub struct Lens<S, A> {
    view_fn: Box<dyn Fn(&S) -> A>,
    set_fn: Box<dyn Fn(S, A) -> S>,
    _phantom: PhantomData<(S, A)>,
}

impl<S: 'static, A: 'static> Lens<S, A> {
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
    type Crystal = LensCrystal<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<A> {
        todo!()
    }

    fn project(&self, beam: Beam<A>) -> Beam<A> {
        todo!()
    }

    fn split(&self, beam: Beam<A>) -> Vec<Beam<A>> {
        todo!()
    }

    fn zoom(
        &self,
        beam: Beam<A>,
        f: &dyn Fn(Beam<A>) -> Beam<A>,
    ) -> Beam<A> {
        todo!()
    }

    fn refract(&self, beam: Beam<A>) -> Beam<LensCrystal<S, A>> {
        todo!()
    }
}

pub struct LensCrystal<S, A> {
    _phantom: PhantomData<(S, A)>,
}

impl<S: Clone + 'static, A: Clone + 'static> Prism for LensCrystal<S, A> {
    type Input = A;
    type Focused = A;
    type Projected = A;
    type Part = A;
    type Crystal = LensCrystal<S, A>;

    fn focus(&self, beam: Beam<A>) -> Beam<A> { todo!() }
    fn project(&self, beam: Beam<A>) -> Beam<A> { todo!() }
    fn split(&self, beam: Beam<A>) -> Vec<Beam<A>> { todo!() }
    fn zoom(&self, beam: Beam<A>, f: &dyn Fn(Beam<A>) -> Beam<A>) -> Beam<A> { todo!() }
    fn refract(&self, beam: Beam<A>) -> Beam<LensCrystal<S, A>> { todo!() }
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

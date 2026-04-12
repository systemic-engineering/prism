//! Traversal — the multi-focus optic.
//!
//! A Traversal lifts a per-element function over a container.
//! Given a mapping fn(A) -> B and a Vec<A>, produces Vec<B>.
//!
//! As a Prism: focus maps elements (Vec<A> → Vec<B>), project is identity,
//! refract returns the mapped collection.

use crate::{Beam, Prism, PureBeam};
use std::convert::Infallible;
use terni::ShannonLoss;

#[derive(Clone, Copy)]
pub struct Traversal<A, B> {
    map_fn: fn(A) -> B,
}

impl<A: 'static, B: 'static> Traversal<A, B> {
    /// Construct a Traversal from a per-element mapping fn pointer.
    ///
    /// # Laws
    /// - `map(a)` returns the same `B` for the same `a` (purity)
    /// - `map(a)` does not panic for any well-typed `a` (totality)
    pub fn new(map: fn(A) -> B) -> Self {
        Traversal { map_fn: map }
    }

    pub fn traverse(&self, input: Vec<A>) -> Vec<B> {
        input.into_iter().map(|a| (self.map_fn)(a)).collect()
    }
}

/// Traversal implements Prism with PureBeam.
///
/// Pipeline flow:
/// - focus: map each element (Vec<A> → Vec<B>)
/// - project: identity (Vec<B> → Vec<B>)
/// - refract: identity (Vec<B> → Vec<B>)
impl<A: Clone + 'static, B: Clone + 'static> Prism for Traversal<A, B> {
    type Input = PureBeam<(), Vec<A>, Infallible, ShannonLoss>;
    type Focused = PureBeam<Vec<A>, Vec<B>, Infallible, ShannonLoss>;
    type Projected = PureBeam<Vec<B>, Vec<B>, Infallible, ShannonLoss>;
    type Refracted = PureBeam<Vec<B>, Vec<B>, Infallible, ShannonLoss>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let items = beam.result().ok().expect("focus: Err beam").clone();
        let mapped: Vec<B> = items.into_iter().map(|a| (self.map_fn)(a)).collect();
        beam.next(mapped)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let v = beam.result().ok().expect("project: Err beam").clone();
        beam.next(v)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let v = beam.result().ok().expect("refract: Err beam").clone();
        beam.next(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Beam as BeamTrait;

    fn double(x: i32) -> i32 {
        x * 2
    }

    // --- Inherent method tests ---

    #[test]
    fn traversal_maps_over_vec() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        assert_eq!(t.traverse(vec![1, 2, 3]), vec![2, 4, 6]);
    }

    #[test]
    fn traversal_empty_vec() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        assert_eq!(t.traverse(vec![]), Vec::<i32>::new());
    }

    // --- Prism trait tests ---

    fn seed<T: Clone>(v: T) -> PureBeam<(), T, Infallible, ShannonLoss> {
        PureBeam::ok((), v)
    }

    #[test]
    fn traversal_prism_focus_maps_elements() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        let beam = seed(vec![1, 2, 3]);
        let focused = t.focus(beam);
        assert_eq!(focused.result().ok(), Some(&vec![2, 4, 6]));
    }

    #[test]
    fn traversal_prism_project_passes_through() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        let focused = t.focus(seed(vec![10]));
        let projected = t.project(focused);
        assert_eq!(projected.result().ok(), Some(&vec![20]));
    }

    #[test]
    fn traversal_prism_refract_returns_mapped() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        let focused = t.focus(seed(vec![5, 10]));
        let projected = t.project(focused);
        let refracted = t.refract(projected);
        assert_eq!(refracted.result().ok(), Some(&vec![10, 20]));
    }

    #[test]
    fn traversal_prism_is_lossless() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        let focused = t.focus(seed(vec![1]));
        assert!(!focused.is_partial());
        let projected = t.project(focused);
        assert!(!projected.is_partial());
        let refracted = t.refract(projected);
        assert!(!refracted.is_partial());
    }

    #[test]
    fn traversal_prism_with_type_change() {
        fn to_string(x: i32) -> String {
            x.to_string()
        }
        let t: Traversal<i32, String> = Traversal::new(to_string);
        let focused = t.focus(seed(vec![1, 2, 3]));
        assert_eq!(
            focused.result().ok(),
            Some(&vec!["1".to_string(), "2".to_string(), "3".to_string()])
        );
    }

    #[test]
    fn traversal_is_clone_and_copy() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        let t2 = t; // Copy
        let t3 = t2.clone(); // Clone
        assert_eq!(t3.traverse(vec![1]), vec![2]);
    }

    /// Traversal's multi-focus split is expressed via smap in the new API.
    #[test]
    fn traversal_split_via_smap() {
        let t: Traversal<i32, i32> = Traversal::new(double);
        let focused = t.focus(seed(vec![1, 2, 3]));
        // smap can decompose the Vec into individual processing
        let first_element =
            focused.smap(|v| terni::Imperfect::Success(v.first().cloned().unwrap_or(0)));
        assert_eq!(first_element.result().ok(), Some(&2));
    }
}

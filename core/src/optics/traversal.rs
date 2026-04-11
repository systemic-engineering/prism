//! Traversal — the multi-focus optic.
//!
//! A Traversal lifts a per-element function over a container. In our
//! setting: given an inner operation `f: A -> B` and a Vec<A>, produce
//! a Vec<B>. This is the classical Traversal from functional optics,
//! specialized to Vec for simplicity.

use crate::{Beam, Prism, Stage};

#[derive(Clone, Copy)]
pub struct Traversal<A, B> {
    map_fn: fn(A) -> B,
}

impl<A: 'static, B: 'static> Traversal<A, B> {
    /// Construct a Traversal from a per-element mapping fn pointer.
    ///
    /// # Laws
    ///
    /// Traversal is the multi-focus optic. The caller is responsible for
    /// ensuring the per-element mapping is:
    ///
    /// - Pure: `map(a)` returns the same `B` for the same `a`
    /// - Total: `map(a)` does not panic for any well-typed `a`
    ///
    /// Traversal does not have set/get laws because it is shape-preserving:
    /// it transforms elements but not their position in the container.
    /// Recombination via a Gather strategy will preserve the original order
    /// (assuming the iterator order was preserved).
    pub fn new(map: fn(A) -> B) -> Self {
        Traversal { map_fn: map }
    }

    pub fn traverse(&self, input: Vec<A>) -> Vec<B> {
        input.into_iter().map(|a| (self.map_fn)(a)).collect()
    }
}

impl<A: Clone + 'static, B: Clone + 'static> Prism for Traversal<A, B> {
    type Input = Vec<A>;
    type Focused = Vec<B>;
    type Projected = Vec<B>;
    type Part = B;
    type Crystal = Traversal<A, B>;

    fn focus(&self, beam: Beam<Vec<A>>) -> Beam<Vec<B>> {
        let mapped: Vec<B> = beam.result.into_iter().map(|a| (self.map_fn)(a)).collect();
        Beam {
            result: mapped,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
            connection: beam.connection,
        }
    }

    fn project(&self, beam: Beam<Vec<B>>) -> Beam<Vec<B>> {
        Beam { stage: Stage::Projected, ..beam }
    }

    fn split(&self, beam: Beam<Vec<B>>) -> Vec<Beam<B>> {
        beam.result
            .into_iter()
            .enumerate()
            .map(|(i, b)| Beam {
                result: b,
                path: {
                    let mut p = beam.path.clone();
                    p.push(crate::Oid::new(format!("{}", i)));
                    p
                },
                loss: beam.loss.clone(),
                precision: beam.precision.clone(),
                recovered: beam.recovered.clone(),
                stage: Stage::Split,
                connection: beam.connection.clone(),
            })
            .collect()
    }

    fn zoom(
        &self,
        beam: Beam<Vec<B>>,
        f: &dyn Fn(Beam<Vec<B>>) -> Beam<Vec<B>>,
    ) -> Beam<Vec<B>> {
        f(beam)
    }

    fn refract(&self, beam: Beam<Vec<B>>) -> Beam<Traversal<A, B>> {
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

    #[test]
    fn traversal_maps_over_vec() {
        fn double(x: i32) -> i32 { x * 2 }
        let double_t: Traversal<i32, i32> = Traversal::new(double);
        let result = double_t.traverse(vec![1, 2, 3]);
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn traversal_as_prism_focus_maps() {
        fn to_upper(s: String) -> String { s.to_uppercase() }
        let to_upper_t: Traversal<String, String> = Traversal::new(to_upper);
        let beam = Beam::new(vec!["hello".to_string(), "world".to_string()]);
        let focused = to_upper_t.focus(beam);
        assert_eq!(focused.result, vec!["HELLO", "WORLD"]);
        assert_eq!(focused.stage, Stage::Focused);
    }

    #[test]
    fn traversal_split_yields_individual_beams_with_shared_path() {
        fn identity(x: i32) -> i32 { x }
        let id: Traversal<i32, i32> = Traversal::new(identity);
        let beam = Beam::new(vec![10, 20, 30]);
        let focused = id.focus(beam);
        let projected = id.project(focused);
        let parts = id.split(projected);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].result, 10);
        assert_eq!(parts[1].result, 20);
        assert_eq!(parts[2].result, 30);
        for p in &parts {
            assert_eq!(p.stage, Stage::Split);
        }
    }

    #[test]
    fn traversal_split_indexes_children() {
        use crate::Oid;
        fn identity(x: i32) -> i32 { x }
        let id: Traversal<i32, i32> = Traversal::new(identity);
        let beam = Beam::new(vec![10, 20, 30]);
        let focused = id.focus(beam);
        let projected = id.project(focused);
        let parts = id.split(projected);

        assert_eq!(parts.len(), 3);
        // Each child should have a path entry with its index
        assert_eq!(parts[0].path.last(), Some(&Oid::new("0")));
        assert_eq!(parts[1].path.last(), Some(&Oid::new("1")));
        assert_eq!(parts[2].path.last(), Some(&Oid::new("2")));
    }

    #[test]
    fn traversal_is_clone_and_copy() {
        fn double(x: i32) -> i32 { x * 2 }
        let t: Traversal<i32, i32> = Traversal::new(double);
        let t2 = t; // Copy
        let t3 = t2.clone(); // Clone
        assert_eq!(t3.traverse(vec![1]), vec![2]);
    }
}

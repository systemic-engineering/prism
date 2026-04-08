//! Traversal — the multi-focus optic.
//!
//! A Traversal lifts a per-element function over a container. In our
//! setting: given an inner operation `f: A -> B` and a Vec<A>, produce
//! a Vec<B>. This is the classical Traversal from functional optics,
//! specialized to Vec for simplicity.

use crate::{Beam, Prism, Stage};
use std::marker::PhantomData;

pub struct Traversal<A, B> {
    map_fn: Box<dyn Fn(A) -> B>,
    _phantom: PhantomData<(A, B)>,
}

impl<A: 'static, B: 'static> Traversal<A, B> {
    pub fn new<F>(map: F) -> Self
    where
        F: Fn(A) -> B + 'static,
    {
        Traversal {
            map_fn: Box::new(map),
            _phantom: PhantomData,
        }
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
    type Crystal = TraversalCrystal<A, B>;

    fn focus(&self, beam: Beam<Vec<A>>) -> Beam<Vec<B>> {
        let mapped: Vec<B> = beam.result.into_iter().map(|a| (self.map_fn)(a)).collect();
        Beam {
            result: mapped,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
        }
    }

    fn project(&self, beam: Beam<Vec<B>>) -> Beam<Vec<B>> {
        Beam { stage: Stage::Projected, ..beam }
    }

    fn split(&self, beam: Beam<Vec<B>>) -> Vec<Beam<B>> {
        beam.result
            .into_iter()
            .map(|b| Beam {
                result: b,
                path: beam.path.clone(),
                loss: beam.loss.clone(),
                precision: beam.precision.clone(),
                recovered: beam.recovered.clone(),
                stage: Stage::Split,
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

    fn refract(&self, beam: Beam<Vec<B>>) -> Beam<TraversalCrystal<A, B>> {
        Beam {
            result: TraversalCrystal { _phantom: PhantomData },
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}

pub struct TraversalCrystal<A, B> {
    _phantom: PhantomData<(A, B)>,
}

impl<A: Clone + 'static, B: Clone + 'static> Prism for TraversalCrystal<A, B> {
    type Input = Vec<B>;
    type Focused = Vec<B>;
    type Projected = Vec<B>;
    type Part = B;
    type Crystal = TraversalCrystal<A, B>;

    fn focus(&self, beam: Beam<Vec<B>>) -> Beam<Vec<B>> { Beam { stage: Stage::Focused, ..beam } }
    fn project(&self, beam: Beam<Vec<B>>) -> Beam<Vec<B>> { Beam { stage: Stage::Projected, ..beam } }
    fn split(&self, beam: Beam<Vec<B>>) -> Vec<Beam<B>> {
        beam.result
            .into_iter()
            .map(|b| Beam {
                result: b,
                path: beam.path.clone(),
                loss: beam.loss.clone(),
                precision: beam.precision.clone(),
                recovered: beam.recovered.clone(),
                stage: Stage::Split,
            })
            .collect()
    }
    fn zoom(&self, beam: Beam<Vec<B>>, f: &dyn Fn(Beam<Vec<B>>) -> Beam<Vec<B>>) -> Beam<Vec<B>> {
        f(beam)
    }
    fn refract(&self, beam: Beam<Vec<B>>) -> Beam<TraversalCrystal<A, B>> {
        Beam {
            result: TraversalCrystal { _phantom: PhantomData },
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

    #[test]
    fn traversal_maps_over_vec() {
        let double: Traversal<i32, i32> = Traversal::new(|x| x * 2);
        let result = double.traverse(vec![1, 2, 3]);
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn traversal_as_prism_focus_maps() {
        let to_upper: Traversal<String, String> = Traversal::new(|s: String| s.to_uppercase());
        let beam = Beam::new(vec!["hello".to_string(), "world".to_string()]);
        let focused = to_upper.focus(beam);
        assert_eq!(focused.result, vec!["HELLO", "WORLD"]);
        assert_eq!(focused.stage, Stage::Focused);
    }

    #[test]
    fn traversal_split_yields_individual_beams_with_shared_path() {
        let id: Traversal<i32, i32> = Traversal::new(|x: i32| x);
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
}

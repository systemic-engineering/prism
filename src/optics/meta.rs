//! MetaPrism — operates on populations of beams.
//!
//! A base prism's `split` produces `Vec<Beam<Part>>`. To work with that
//! population as a unit, you wrap it in a MetaPrism parameterized by a
//! Gather strategy. The MetaPrism's refract collapses the population
//! back into a single Beam using the strategy.
//!
//! This is where the inter-level movement happens: base prisms live at
//! level 0 (`Beam<T>`), meta prisms live at level 1 (`Vec<Beam<T>>`).

use std::marker::PhantomData;
use crate::{Beam, Prism, Stage};
use super::gather::{Gather, SumGather};

/// MetaPrism lifts a Gather strategy into a full Prism that operates
/// on populations of beams. Its Input is `Vec<Beam<T>>`; its refract
/// collapses the population to a `Beam<T>` via the Gather strategy.
///
/// Type parameters:
/// - `T`: the element type inside each child beam
/// - `G`: the gather strategy (implements `Gather<T>`)
pub struct MetaPrism<T, G: Gather<T>> {
    gather: G,
    _phantom: PhantomData<T>,
}

impl<T, G: Gather<T>> MetaPrism<T, G> {
    pub fn new(gather: G) -> Self {
        MetaPrism {
            gather,
            _phantom: PhantomData,
        }
    }
}

impl<T: Clone + 'static, G: Gather<T> + Clone + 'static> Prism for MetaPrism<T, G> {
    type Input = Vec<Beam<T>>;
    type Focused = Vec<Beam<T>>;
    type Projected = T;
    type Part = Beam<T>;
    type Crystal = MetaPrism<T, G>;

    fn focus(&self, beam: Beam<Vec<Beam<T>>>) -> Beam<Vec<Beam<T>>> {
        Beam {
            result: beam.result,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
        }
    }

    fn project(&self, beam: Beam<Vec<Beam<T>>>) -> Beam<T> {
        let mut gathered = self.gather.gather(beam.result);
        gathered.stage = Stage::Projected;
        gathered
    }

    fn split(&self, beam: Beam<T>) -> Vec<Beam<Beam<T>>> {
        vec![Beam {
            result: beam,
            path: Vec::new(),
            loss: crate::ShannonLoss::new(0.0),
            precision: crate::Precision::new(1.0),
            recovered: None,
            stage: Stage::Split,
        }]
    }

    fn zoom(
        &self,
        beam: Beam<T>,
        f: &dyn Fn(Beam<T>) -> Beam<T>,
    ) -> Beam<T> {
        f(beam)
    }

    fn refract(&self, beam: Beam<T>) -> Beam<MetaPrism<T, G>> {
        Beam {
            result: MetaPrism {
                gather: self.gather.clone(),
                _phantom: PhantomData,
            },
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
    use crate::Precision;

    #[test]
    fn meta_prism_project_gathers_to_single_beam() {
        let meta: MetaPrism<String, SumGather> = MetaPrism::new(SumGather);
        let population = vec![
            Beam::new("foo".to_string()),
            Beam::new("bar".to_string()),
            Beam::new("baz".to_string()),
        ];
        let input = Beam::new(population);
        let focused = meta.focus(input);
        let projected = meta.project(focused);
        assert_eq!(projected.result, "foobarbaz");
        assert_eq!(projected.stage, Stage::Projected);
    }

    #[test]
    fn meta_prism_full_pipeline_ends_at_refracted() {
        let meta: MetaPrism<String, SumGather> = MetaPrism::new(SumGather);
        let population = vec![
            Beam::new("a".to_string()),
            Beam::new("b".to_string()),
        ];
        let out = crate::apply(&meta, population);
        assert_eq!(out.stage, Stage::Refracted);
    }

    #[test]
    fn meta_prism_crystal_is_self() {
        fn require_prism<P: Prism>() {}
        require_prism::<MetaPrism<String, SumGather>>();
    }
}

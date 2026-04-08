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
    use crate::{Oid, Precision, ShannonLoss};

    // -------------------------------------------------------------------------
    // WordsPrism — a test prism that splits a String into individual words.
    // Tests the spec-correct MetaPrism<P, G> shape where P is an inner prism
    // that supplies the split. MetaPrism::focus must call inner.split.
    // -------------------------------------------------------------------------

    #[derive(Clone)]
    struct WordsPrism;

    impl Prism for WordsPrism {
        type Input = String;
        type Focused = String;
        type Projected = String;
        type Part = String;
        type Crystal = WordsPrism;

        fn focus(&self, beam: Beam<String>) -> Beam<String> {
            Beam { stage: Stage::Focused, ..beam }
        }

        fn project(&self, beam: Beam<String>) -> Beam<String> {
            Beam { stage: Stage::Projected, ..beam }
        }

        fn split(&self, beam: Beam<String>) -> Vec<Beam<String>> {
            beam.result
                .split_whitespace()
                .enumerate()
                .map(|(i, w)| Beam {
                    result: w.to_string(),
                    path: {
                        let mut p = beam.path.clone();
                        p.push(Oid::new(format!("word/{}", i)));
                        p
                    },
                    loss: beam.loss.clone(),
                    precision: beam.precision.clone(),
                    recovered: beam.recovered.clone(),
                    stage: Stage::Split,
                })
                .collect()
        }

        fn zoom(
            &self,
            beam: Beam<String>,
            f: &dyn Fn(Beam<String>) -> Beam<String>,
        ) -> Beam<String> {
            f(beam)
        }

        fn refract(&self, beam: Beam<String>) -> Beam<WordsPrism> {
            Beam {
                result: WordsPrism,
                path: beam.path,
                loss: beam.loss,
                precision: beam.precision,
                recovered: beam.recovered,
                stage: Stage::Refracted,
            }
        }
    }

    // -------------------------------------------------------------------------
    // Spec-correct tests against MetaPrism<P: Prism, G: Gather<P::Part>>.
    // These tests define the required shape. They fail to compile against the
    // current MetaPrism<T, G> because MetaPrism::new takes only a gather
    // strategy, not an inner prism.
    // -------------------------------------------------------------------------

    /// C1 regression: MetaPrism must delegate to inner.split.
    #[test]
    fn meta_prism_uses_inner_split_and_gathers() {
        // This requires MetaPrism::new(inner, gather) — two arguments.
        // The current impl only takes one argument (the gather strategy).
        let meta = MetaPrism::new(WordsPrism, SumGather);

        let input = Beam::new("hello world test".to_string());
        let focused = meta.focus(input);

        assert_eq!(focused.result.len(), 3);
        assert_eq!(focused.result[0].result, "hello");
        assert_eq!(focused.result[1].result, "world");
        assert_eq!(focused.result[2].result, "test");
        assert_eq!(focused.stage, Stage::Focused);

        let projected = meta.project(focused);
        assert_eq!(projected.result, "helloworldtest");
        assert_eq!(projected.stage, Stage::Projected);
    }

    /// Full pipeline test.
    #[test]
    fn meta_prism_full_pipeline_via_apply() {
        let meta = MetaPrism::new(WordsPrism, SumGather);
        let out = crate::apply(&meta, "alpha beta gamma".to_string());
        assert_eq!(out.stage, Stage::Refracted);
    }

    /// M2 regression: split must carry parent path/loss/precision/recovered.
    #[test]
    fn meta_prism_split_carries_parent_provenance() {
        let meta = MetaPrism::new(WordsPrism, SumGather);

        let parent_beam = Beam {
            result: "x".to_string(),
            path: vec![Oid::new("parent")],
            loss: ShannonLoss::new(2.5),
            precision: Precision::new(0.7),
            recovered: None,
            stage: Stage::Projected,
        };

        let parts = meta.split(parent_beam);
        assert_eq!(parts.len(), 1);

        let outer = &parts[0];
        assert_eq!(
            outer.path,
            vec![Oid::new("parent")],
            "outer path must be inherited from parent (M2 fix)"
        );
        assert_eq!(outer.loss.as_f64(), 2.5, "outer loss must be inherited");
        assert_eq!(outer.precision.as_f64(), 0.7, "outer precision must be inherited");
        assert_eq!(outer.stage, Stage::Split);
    }

    /// Crystal type closure.
    #[test]
    fn meta_prism_crystal_is_self() {
        fn require_prism<P: Prism>() {}
        require_prism::<MetaPrism<WordsPrism, SumGather>>();
    }

    /// focus preserves the outer beam's path/loss/precision.
    #[test]
    fn meta_prism_focus_preserves_outer_envelope() {
        let meta = MetaPrism::new(WordsPrism, SumGather);

        let input = Beam {
            result: "one two".to_string(),
            path: vec![Oid::new("root")],
            loss: ShannonLoss::new(1.0),
            precision: Precision::new(0.8),
            recovered: None,
            stage: Stage::Initial,
        };

        let focused = meta.focus(input);
        assert_eq!(focused.path, vec![Oid::new("root")]);
        assert_eq!(focused.loss.as_f64(), 1.0);
        assert_eq!(focused.precision.as_f64(), 0.8);
        assert_eq!(focused.stage, Stage::Focused);
    }

    /// project carries outer envelope forward (M2 fix for project).
    #[test]
    fn meta_prism_project_carries_outer_envelope() {
        let meta = MetaPrism::new(WordsPrism, SumGather);

        let input = Beam {
            result: "one two".to_string(),
            path: vec![Oid::new("root")],
            loss: ShannonLoss::new(1.0),
            precision: Precision::new(0.8),
            recovered: None,
            stage: Stage::Initial,
        };

        let focused = meta.focus(input);
        let projected = meta.project(focused);

        assert_eq!(projected.path, vec![Oid::new("root")]);
        assert_eq!(projected.loss.as_f64(), 1.0);
        assert_eq!(projected.precision.as_f64(), 0.8);
        assert_eq!(projected.stage, Stage::Projected);
    }
}

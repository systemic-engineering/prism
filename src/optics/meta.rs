//! MetaPrism — lifts an inner prism's split into a first-class Prism.
//!
//! A `MetaPrism<P, G>` takes a beam whose content is the inner prism `P`'s
//! projected type. Its `focus` calls `inner.split(beam)` to produce a
//! population of child beams. Its `project` gathers that population back into
//! a single beam via the `G: Gather` strategy. Its `refract` closes the loop
//! by emitting a fresh MetaPrism as the crystal.
//!
//! This is where cross-level movement is made explicit: the inner prism lives
//! at Level 0 (`Beam<T>`). The meta-prism operates at Level 1 by wrapping the
//! inner's split output (`Vec<Beam<T>>`) and gathering it back via strategy G.
//!
//! # Type parameters
//! - `P`: the inner prism that supplies `focus`, `project`, and `split`
//! - `G`: the gather strategy for collapsing the split population

use crate::{Beam, Prism, Stage};
use super::gather::Gather;

/// A meta-prism parameterized by an inner prism `P` and a gather strategy `G`.
///
/// The headline use case: lift `P::split` into a full Prism whose `focus`
/// delegates to `inner.split`, whose `project` collapses via `gather.gather`,
/// and whose `refract` produces a crystal of the same meta-prism type.
///
/// Type flow through the pipeline:
/// ```text
/// focus:   Beam<P::Projected>       → Beam<Vec<Beam<P::Part>>>
/// project: Beam<Vec<Beam<P::Part>>> → Beam<P::Part>
/// split:   Beam<P::Part>            → Vec<Beam<Beam<P::Part>>>
/// zoom:    Beam<P::Part>            → Beam<P::Part>
/// refract: Beam<P::Part>            → Beam<MetaPrism<P, G>>
/// ```
pub struct MetaPrism<P: Prism, G: Gather<P::Part>> {
    pub inner: P,
    pub gather: G,
}

impl<P: Prism, G: Gather<P::Part>> MetaPrism<P, G> {
    pub fn new(inner: P, gather: G) -> Self {
        MetaPrism { inner, gather }
    }
}

impl<P, G> Prism for MetaPrism<P, G>
where
    P: Prism + Clone + 'static,
    G: Gather<P::Part> + Clone + 'static,
    P::Projected: Clone + 'static,
    P::Part: Clone + 'static,
{
    /// The input is whatever the inner prism's `split` expects: `P::Projected`.
    type Input = P::Projected;
    /// Focus produces the inner prism's population: `Vec<Beam<P::Part>>`.
    type Focused = Vec<Beam<P::Part>>;
    /// Project collapses the population to a single part: `P::Part`.
    type Projected = P::Part;
    /// Each part is a child beam from the population.
    type Part = Beam<P::Part>;
    /// Crystal is itself — the meta-prism is its own fixed point.
    type Crystal = MetaPrism<P, G>;

    /// Focus: call `inner.split` on the incoming beam to produce the population.
    ///
    /// The outer beam's path/loss/precision/recovered are carried into the
    /// returned `Beam<Vec<...>>` so provenance is not silently dropped.
    fn focus(&self, beam: Beam<P::Projected>) -> Beam<Vec<Beam<P::Part>>> {
        let path = beam.path.clone();
        let loss = beam.loss.clone();
        let precision = beam.precision.clone();
        let recovered = beam.recovered.clone();

        // Hand a "projected" beam to the inner prism's split.
        let inner_beam = Beam {
            result: beam.result,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Projected,
        };
        let parts = self.inner.split(inner_beam);

        Beam {
            result: parts,
            path,
            loss,
            precision,
            recovered,
            stage: Stage::Focused,
        }
    }

    /// Project: gather the population back to a single `Beam<P::Part>`.
    ///
    /// The outer envelope's path/loss/precision/recovered are carried forward
    /// (M2 fix: the gather strategy doesn't know about the outer envelope).
    fn project(&self, beam: Beam<Vec<Beam<P::Part>>>) -> Beam<P::Part> {
        let mut gathered = self.gather.gather(beam.result);
        // Gather strategies emit Projected (M4 fix: Stage::Joined was removed).
        // We re-set it explicitly here as defense-in-depth and to mark this
        // as the meta-prism's project step.
        gathered.stage = Stage::Projected;
        // M2 fix: carry outer envelope's accumulated state forward so
        // provenance is not silently reset by the gather strategy.
        gathered.path = beam.path;
        gathered.loss = beam.loss;
        gathered.precision = beam.precision;
        gathered.recovered = beam.recovered;
        gathered
    }

    /// Split: re-emit a singleton population from a `Beam<P::Part>`.
    ///
    /// M2 fix: the outer wrapper beam carries the parent's path/loss/
    /// precision/recovered so provenance is not reset to fresh values.
    fn split(&self, beam: Beam<P::Part>) -> Vec<Beam<Beam<P::Part>>> {
        vec![Beam {
            result: Beam {
                result: beam.result,
                path: beam.path.clone(),
                loss: beam.loss.clone(),
                precision: beam.precision.clone(),
                recovered: beam.recovered.clone(),
                stage: Stage::Projected,
            },
            path: beam.path,           // M2: carry parent path
            loss: beam.loss,           // M2: carry parent loss
            precision: beam.precision, // M2: carry parent precision
            recovered: beam.recovered, // M2: carry parent recovered
            stage: Stage::Split,
        }]
    }

    fn zoom(
        &self,
        beam: Beam<P::Part>,
        f: &dyn Fn(Beam<P::Part>) -> Beam<P::Part>,
    ) -> Beam<P::Part> {
        f(beam)
    }

    fn refract(&self, beam: Beam<P::Part>) -> Beam<MetaPrism<P, G>> {
        Beam {
            result: MetaPrism {
                inner: self.inner.clone(),
                gather: self.gather.clone(),
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
    use super::super::gather::ConcatGather;

    // -------------------------------------------------------------------------
    // WordsPrism — a test prism that splits a String into individual words.
    // This exercises the inner-prism path: a non-trivial split with multiple
    // parts, making it possible to verify MetaPrism delegates to inner.split.
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
    // Tests
    // -------------------------------------------------------------------------

    /// The core C1 regression: MetaPrism must delegate to inner.split.
    ///
    /// Verifies that `focus` calls `inner.split` and that the resulting
    /// population has the correct count, content, and stage.
    #[test]
    fn meta_prism_uses_inner_split_and_gathers() {
        let meta = MetaPrism::new(WordsPrism, ConcatGather);

        let input = Beam::new("hello world test".to_string());
        let focused = meta.focus(input);

        // inner.split should have produced three word beams
        assert_eq!(focused.result.len(), 3, "focus should produce 3 word beams");
        assert_eq!(focused.result[0].result, "hello");
        assert_eq!(focused.result[1].result, "world");
        assert_eq!(focused.result[2].result, "test");
        assert_eq!(focused.stage, Stage::Focused);

        let projected = meta.project(focused);
        assert_eq!(projected.result, "helloworldtest");
        assert_eq!(projected.stage, Stage::Projected);
    }

    /// Full pipeline test: apply() chains focus → project → refract.
    #[test]
    fn meta_prism_full_pipeline_via_apply() {
        let meta = MetaPrism::new(WordsPrism, ConcatGather);
        let out = crate::apply(&meta, "alpha beta gamma".to_string());
        assert_eq!(out.stage, Stage::Refracted);
    }

    /// M2 regression: split must carry parent path/loss/precision/recovered.
    ///
    /// The old implementation used FRESH values (`Vec::new()`, `ShannonLoss::new(0.0)`,
    /// `Precision::new(1.0)`), silently dropping parent provenance. This test
    /// asserts the fix: the outer wrapper beam in the split result inherits
    /// the parent beam's envelope.
    #[test]
    fn meta_prism_split_carries_parent_provenance() {
        let meta = MetaPrism::new(WordsPrism, ConcatGather);

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
        assert_eq!(
            outer.loss.as_f64(),
            2.5,
            "outer loss must be inherited from parent (M2 fix)"
        );
        assert_eq!(
            outer.precision.as_f64(),
            0.7,
            "outer precision must be inherited from parent (M2 fix)"
        );
        assert_eq!(outer.stage, Stage::Split);
    }

    /// Verify the crystal type is itself — the fixed-point closure property.
    #[test]
    fn meta_prism_crystal_is_self() {
        fn require_prism<P: Prism>() {}
        require_prism::<MetaPrism<WordsPrism, ConcatGather>>();
    }

    /// Verify that focus preserves the outer beam's path/loss/precision.
    #[test]
    fn meta_prism_focus_preserves_outer_envelope() {
        let meta = MetaPrism::new(WordsPrism, ConcatGather);

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

    /// Verify that project carries outer envelope forward (M2 fix for project).
    #[test]
    fn meta_prism_project_carries_outer_envelope() {
        let meta = MetaPrism::new(WordsPrism, ConcatGather);

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

        // The outer path/loss/precision should be carried forward, not
        // overwritten by the gather strategy's output.
        assert_eq!(projected.path, vec![Oid::new("root")]);
        assert_eq!(projected.loss.as_f64(), 1.0);
        assert_eq!(projected.precision.as_f64(), 0.8);
        assert_eq!(projected.stage, Stage::Projected);
    }
}

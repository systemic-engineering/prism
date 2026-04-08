//! Integration test: compose multiple optics end-to-end.
//!
//! Builds a small "words pipeline" using Traversal + MetaPrism +
//! ConcatGather, verifying the optic layers compose as expected.

#![cfg(feature = "optics")]

use prism::{apply, Beam, Oid, Prism, Stage};
use prism::optics::gather::ConcatGather;
use prism::optics::meta::MetaPrism;
use prism::optics::traversal::Traversal;

// ---------------------------------------------------------------------------
// WordsPrism — a test inner prism that splits a String into individual words.
// Needed here because MetaPrism<P, G> now requires a real inner prism P.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn traversal_lifts_string_transform_over_vec() {
    let to_upper: Traversal<String, String> = Traversal::new(|s: String| s.to_uppercase());
    let beam = Beam::new(vec![
        "alpha".to_string(),
        "beta".to_string(),
        "gamma".to_string(),
    ]);
    let focused = to_upper.focus(beam);
    assert_eq!(focused.result, vec!["ALPHA", "BETA", "GAMMA"]);
    assert_eq!(focused.stage, Stage::Focused);
}

/// MetaPrism now takes an inner prism (WordsPrism) plus a gather strategy.
/// focus delegates to inner.split; project gathers the population.
#[test]
fn meta_prism_over_sum_gather_collapses_population() {
    let meta = MetaPrism::new(WordsPrism, ConcatGather);
    let input = Beam::new("alpha beta gamma".to_string());
    let focused = meta.focus(input);
    assert_eq!(focused.result.len(), 3);
    let projected = meta.project(focused);
    assert_eq!(projected.result, "alphabetagamma");
    assert_eq!(projected.stage, Stage::Projected);
}

/// Full pipeline: apply chains focus → project → refract.
#[test]
fn gather_then_apply_full_pipeline() {
    let meta = MetaPrism::new(WordsPrism, ConcatGather);
    let out = apply(&meta, "one two".to_string());
    assert_eq!(out.stage, Stage::Refracted);
}

// ---------------------------------------------------------------------------
// Compose integration tests — verify the boot-fold pattern from the spec
// ---------------------------------------------------------------------------

/// Compose<IdPrism<String>, IdPrism<IdPrism<String>>> chains two identity
/// prisms across a type boundary (String → IdPrism<String>). This invokes
/// the full Compose::refract pipeline: first prism's refract produces a
/// Beam<IdPrism<String>>, which the second prism consumes and produces
/// Beam<IdPrism<IdPrism<String>>>.
#[test]
fn compose_two_idprisms_end_to_end_through_full_pipeline() {
    use prism::optics::monoid::{Compose, IdPrism};

    let first: IdPrism<String> = IdPrism::new();
    let second: IdPrism<IdPrism<String>> = IdPrism::new();
    let composed = Compose::new(first, second);

    // Run the full Compose pipeline: focus → project → refract
    let input = Beam::new("hello".to_string());
    let focused = composed.focus(input);
    assert_eq!(focused.stage, Stage::Focused);

    let projected = composed.project(focused);
    assert_eq!(projected.stage, Stage::Projected);

    let out = composed.refract(projected);

    assert_eq!(out.stage, Stage::Refracted);
    // The beam's path/loss/precision should be preserved through the
    // entire chain (both prisms are identity on those fields).
    assert!(out.path.is_empty());
    assert!(out.loss.is_lossless());
}

/// Compose preserves path entries and beam metadata through both prisms
/// in the composition chain.
#[test]
fn compose_preserves_path_through_both_idprisms() {
    use prism::optics::monoid::{Compose, IdPrism};

    let first: IdPrism<String> = IdPrism::new();
    let second: IdPrism<IdPrism<String>> = IdPrism::new();
    let composed = Compose::new(first, second);

    let input = Beam {
        result: "world".to_string(),
        path: vec![Oid::new("origin"), Oid::new("step1")],
        loss: prism::ShannonLoss::new(0.0),
        precision: prism::Precision::new(1.0),
        recovered: None,
        stage: Stage::Initial,
    };

    let focused = composed.focus(input);
    let projected = composed.project(focused);
    let out = composed.refract(projected);

    assert_eq!(out.stage, Stage::Refracted);
    assert_eq!(out.path.len(), 2, "path must survive through Compose");
    assert_eq!(out.path[0].as_str(), "origin");
    assert_eq!(out.path[1].as_str(), "step1");
    assert_eq!(out.loss.as_f64(), 0.0);
    assert_eq!(out.precision.as_f64(), 1.0);
}

/// The spec's headline use case: MetaPrism<WordsPrism, ConcatGather> splits
/// a string into words, gathers them back via ConcatGather. This is the
/// Layer 3 integration that justifies the optics layer's existence.
#[test]
fn meta_prism_full_pipeline_splits_gathers_and_refracts() {
    let meta = MetaPrism::new(WordsPrism, ConcatGather);

    // Run the full pipeline through apply: focus → project → refract
    let out = apply(&meta, "alpha beta gamma".to_string());

    // Should reach Refracted stage after the full pipeline.
    assert_eq!(out.stage, Stage::Refracted);

    // Meta's refract must return Beam<WordsPrism>, not a transformed type.
    // The beam carries the path from the split operations.
    // (WordsPrism::refract produces a Beam<WordsPrism>.)
}

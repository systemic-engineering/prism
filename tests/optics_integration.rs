//! Integration test: compose multiple optics end-to-end.
//!
//! Builds a small "words pipeline" using Traversal + MetaPrism +
//! SumGather, verifying the optic layers compose as expected.

#![cfg(feature = "optics")]

use prism::{apply, Beam, Prism, Stage};
use prism::optics::gather::SumGather;
use prism::optics::meta::MetaPrism;
use prism::optics::traversal::Traversal;

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

#[test]
fn meta_prism_over_sum_gather_collapses_population() {
    let meta: MetaPrism<String, SumGather> = MetaPrism::new(SumGather);
    let population = vec![
        Beam::new("alpha ".to_string()),
        Beam::new("beta ".to_string()),
        Beam::new("gamma".to_string()),
    ];
    let input = Beam::new(population);
    let focused = meta.focus(input);
    let projected = meta.project(focused);
    assert_eq!(projected.result, "alpha beta gamma");
    assert_eq!(projected.stage, Stage::Projected);
}

#[test]
fn gather_then_apply_full_pipeline() {
    let meta: MetaPrism<String, SumGather> = MetaPrism::new(SumGather);
    let population = vec![
        Beam::new("one".to_string()),
        Beam::new("two".to_string()),
    ];
    let out = apply(&meta, population);
    assert_eq!(out.stage, Stage::Refracted);
}

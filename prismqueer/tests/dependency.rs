//! prism-008 item 3: the engine's dependency graph, the part prism owns.
//!
//! Directed (premise → head), signed (positive, or through a `not`), and a
//! multigraph: parallel edges are kept, so reuse is visible in b₁. Plus
//! reachability. mirror computes R⁺, the removal seed and N1 on top.

use std::collections::BTreeSet;

use prismqueer::dependency::{DependencyGraph, Sign, Through};

fn set(xs: &[&'static str]) -> BTreeSet<&'static str> {
    xs.iter().copied().collect()
}

#[test]
fn parallel_edges_are_kept_and_reuse_shows_in_the_cycle_rank() {
    let mut g = DependencyGraph::new();
    g.add_edge("a", "b", Sign::Positive); // rule r1
    g.add_edge("a", "b", Sign::Positive); // rule r2, same premise and head
    assert_eq!(g.multiplicity(&"a", &"b"), 2);
    assert_eq!(g.edge_count(), 2);
    assert_eq!(
        g.cycle_rank(),
        1,
        "two routes a→b: one cycle a simple graph would lose"
    );
}

#[test]
fn a_self_dependency_is_a_cycle() {
    let mut g = DependencyGraph::new();
    g.add_edge("p", "p", Sign::Negative);
    assert_eq!(g.cycle_rank(), 1);
}

#[test]
fn reach_through_positive_edges_or_through_any() {
    let mut g = DependencyGraph::new();
    g.add_edge("a", "b", Sign::Positive);
    g.add_edge("b", "c", Sign::Negative);
    g.add_edge("c", "d", Sign::Positive);
    g.add_node("e");
    assert_eq!(
        g.reach(&set(&["a"]), Through::Positive),
        set(&["a", "b"]),
        "stops at the not"
    );
    assert_eq!(
        g.reach(&set(&["a"]), Through::Any),
        set(&["a", "b", "c", "d"])
    );
    assert_eq!(
        g.reach(&set(&["e"]), Through::Any),
        set(&["e"]),
        "an isolated node reaches itself"
    );
}

#[test]
fn negative_edges_are_listed_in_insertion_order() {
    let mut g = DependencyGraph::new();
    g.add_edge("x", "y", Sign::Negative);
    g.add_edge("a", "b", Sign::Positive);
    g.add_edge("b", "c", Sign::Negative);
    let negs: Vec<(&str, &str)> = g.negative_edges().map(|e| (*e.from(), *e.to())).collect();
    assert_eq!(negs, vec![("x", "y"), ("b", "c")]);
}

#[test]
fn components_and_cycle_rank_on_a_disconnected_multigraph() {
    let mut g = DependencyGraph::new();
    g.add_edge("a", "b", Sign::Positive);
    g.add_edge("b", "a", Sign::Positive); // a 2-cycle, as two directed edges
    g.add_edge("c", "d", Sign::Positive);
    assert_eq!(g.components(), 2);
    assert_eq!(g.cycle_rank(), 1, "b₁ = |E| − |V| + components = 3 − 4 + 2");
}

//! Edge multiplicity in field's graph base (prism-008, Ask 2).
//!
//! Two edges between the same pair are two edges: each has its own identity
//! and its own transport. Cycle space and b₁ count both, so a doubled pair
//! is one independent loop, of length 2. Going out along one edge and back
//! along the other leaves `g·h⁻¹`, in the group's own terms. A loop is a
//! hole; filled by a 2-cell (a bigon) it's curvature on that face.

#![cfg(feature = "lapack")]

use prismqueer::field::{check_flat, Edge, Field, FieldError, Flatness, Graph, Group, Perm, Ratio};
use prismqueer::spectral::graph_from_edges;

/// A pure fifth and a grave fifth (40/27, just D–A) on the same pair.
fn pure_and_grave() -> Field<Graph, Ratio> {
    Field::multigraph([((0, 1), Ratio::of(3, 2)), ((0, 1), Ratio::of(40, 27))])
}

#[test]
fn two_edges_on_one_pair_are_two_edges() {
    let field = pure_and_grave();
    let graph = field.graph();
    assert_eq!(
        graph.edges().collect::<Vec<_>>(),
        [Edge::between(0, 1, 0), Edge::between(1, 0, 1)],
        "each parallel edge keeps its own identity, numbered in the order given"
    );
    assert_eq!(graph.multiplicity((0, 1)), 2);
    assert_eq!(
        graph.cycle_rank(),
        1,
        "a doubled pair is one independent loop"
    );
    assert_eq!(graph.vertices().collect::<Vec<_>>(), [0, 1]);
}

#[test]
fn a_two_loop_holds_the_first_map_times_the_inverse_of_the_second() {
    match check_flat(&pure_and_grave()) {
        Flatness::Curved(c) => {
            assert!(c.faces().is_empty(), "no faces declared");
            assert_eq!(c.loops().len(), 1, "one hole");
            let hole = &c.loops()[0];
            assert_eq!(hole.vertices(), &[0, 1, 0], "out to 1 and back");
            assert_eq!(
                hole.edges(),
                &[Edge::between(0, 1, 0), Edge::between(0, 1, 1)],
                "out along the first edge, back along the second"
            );
            assert_eq!(
                *hole.holonomy(),
                Ratio::of(81, 80),
                "3/2 · (40/27)⁻¹: the syntonic comma, exactly"
            );
        }
        Flatness::Flat(_) => panic!("a pure and a grave fifth don't agree"),
    }
}

#[test]
fn in_a_group_that_does_not_commute_the_residue_is_g_then_h_inverse() {
    let g = Perm::new(vec![0, 1, 3, 2]);
    let h = Perm::new(vec![1, 2, 0, 3]);
    let field = Field::multigraph([((0, 1), g.clone()), ((0, 1), h.clone())]);
    let want = g.compose(&h.inverse());
    assert_ne!(want, h.inverse().compose(&g), "the order is visible");
    assert_ne!(want, g.compose(&h), "the inverse is visible");
    assert_ne!(want, want.inverse(), "the direction is visible");
    match check_flat(&field) {
        Flatness::Curved(c) => assert_eq!(*c.loops()[0].holonomy(), want),
        Flatness::Flat(_) => panic!("two different permutations on one pair"),
    }
}

#[test]
fn equal_parallel_maps_close() {
    // The same fifth twice, once given backwards (stored inverted).
    let field = Field::multigraph([((0, 1), Ratio::of(3, 2)), ((1, 0), Ratio::of(2, 3))]);
    assert_eq!(
        field.graph().cycle_rank(),
        1,
        "still a loop, just a closed one"
    );
    match check_flat(&field) {
        Flatness::Flat(flat) => assert_eq!(flat.potential()[&1], Ratio::of(3, 2)),
        Flatness::Curved(_) => panic!("equal maps close"),
    }
}

#[test]
fn a_filled_bigon_that_does_not_close_is_curvature_on_that_face() {
    let field = pure_and_grave()
        .with_bigons([[Edge::between(0, 1, 1), Edge::between(0, 1, 0)]])
        .unwrap();
    match check_flat(&field) {
        Flatness::Curved(c) => {
            assert!(c.loops().is_empty(), "the 2-loop bounds a face: not a hole");
            assert_eq!(c.faces().len(), 1);
            let face = &c.faces()[0];
            assert_eq!(face.vertices(), &[0, 1, 0]);
            assert_eq!(
                face.edges(),
                &[Edge::between(0, 1, 0), Edge::between(0, 1, 1)]
            );
            assert_eq!(*face.holonomy(), Ratio::of(81, 80));
        }
        Flatness::Flat(_) => panic!("the bigon is curved"),
    }
}

#[test]
fn a_flat_filled_bigon_is_flat() {
    let field = Field::multigraph([((0, 1), Ratio::of(3, 2)), ((0, 1), Ratio::of(3, 2))])
        .with_bigons([[Edge::between(0, 1, 0), Edge::between(0, 1, 1)]])
        .unwrap();
    assert!(matches!(check_flat(&field), Flatness::Flat(_)));
}

#[test]
fn a_doubled_side_of_a_triangle_adds_one_loop_and_locates_the_comma() {
    // The triangle closes through the pure fifth; only the grave one is off.
    let field = Field::multigraph([
        ((0, 1), Ratio::of(3, 2)),
        ((0, 1), Ratio::of(40, 27)),
        ((1, 2), Ratio::of(4, 3)),
        ((2, 0), Ratio::of(1, 2)),
    ]);
    assert_eq!(field.graph().cycle_rank(), 2);
    match check_flat(&field) {
        Flatness::Curved(c) => {
            assert_eq!(c.loops().len(), 1, "the triangle closes; the bigon doesn't");
            assert_eq!(c.loops()[0].vertices(), &[0, 1, 0]);
            assert_eq!(*c.loops()[0].holonomy(), Ratio::of(81, 80));
        }
        Flatness::Flat(_) => panic!("the grave fifth is off"),
    }
    assert_eq!(
        field.with_faces([[0, 1, 2]]),
        Err(FieldError::Parallel((0, 1))),
        "a triangle named by its corners can't choose between parallel sides"
    );
}

#[test]
fn a_bigon_off_the_graph_or_on_two_pairs_is_refused() {
    let doubled = Field::multigraph([
        ((0, 1), Ratio::of(3, 2)),
        ((0, 1), Ratio::of(40, 27)),
        ((1, 2), Ratio::of(4, 3)),
    ]);
    assert_eq!(
        doubled
            .clone()
            .with_bigons([[Edge::between(0, 1, 0), Edge::between(0, 1, 2)]]),
        Err(FieldError::Missing((0, 1))),
        "there is no third edge on 0–1"
    );
    assert_eq!(
        doubled.with_bigons([[Edge::between(0, 1, 0), Edge::between(1, 2, 0)]]),
        Err(FieldError::Apart((0, 1), (1, 2))),
        "two edges on different pairs bound no bigon"
    );
}

#[test]
fn a_simple_graph_has_one_edge_per_pair() {
    let field = Field::transports(
        &graph_from_edges(&[(0, 1), (1, 2), (0, 2)]),
        [
            ((0, 1), Ratio::of(5, 4)),
            ((1, 2), Ratio::of(5, 4)),
            ((2, 0), Ratio::of(5, 8)),
        ],
    )
    .unwrap();
    assert_eq!(field.graph().cycle_rank(), 1);
    assert!(field.graph().edges().all(|e| e.nth() == 0));
    assert_eq!(field.graph().multiplicity((0, 2)), 1);
    match check_flat(&field) {
        Flatness::Curved(c) => assert_eq!(
            c.loops()[0].edges(),
            &[
                Edge::between(0, 1, 0),
                Edge::between(1, 2, 0),
                Edge::between(0, 2, 0)
            ],
            "a triangle's loop names its three edges"
        ),
        Flatness::Flat(_) => panic!("(5/4)³ is not an octave"),
    }
}

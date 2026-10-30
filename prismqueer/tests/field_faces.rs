//! prism-008 (Mara's navigation note): a failure on one triple overlap is
//! curvature on that face, not holonomy around a hole (the navigator's
//! cocked hat). Faces are declared 2-cells; a loop that bounds faces isn't a
//! hole.

#![cfg(feature = "lapack")]

use prismqueer::field::{check_flat, Field, Flatness, Ratio};
use prismqueer::spectral::graph_from_edges;

fn thirds() -> [((u64, u64), Ratio); 3] {
    [
        ((0, 1), Ratio::of(5, 4)),
        ((1, 2), Ratio::of(5, 4)),
        ((2, 0), Ratio::of(5, 8)),
    ]
}

#[test]
fn an_empty_triangle_that_does_not_close_is_a_hole() {
    let t = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
    match check_flat(&Field::transports(&t, thirds()).unwrap()) {
        Flatness::Curved(c) => {
            assert_eq!(c.loops().len(), 1, "one hole");
            assert!(c.faces().is_empty(), "no faces declared");
        }
        Flatness::Flat(_) => panic!("it doesn't close"),
    }
}

#[test]
fn a_filled_triangle_that_does_not_close_is_curvature_on_that_face() {
    let t = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
    let field = Field::transports(&t, thirds())
        .unwrap()
        .with_faces([[0, 1, 2]])
        .unwrap();
    match check_flat(&field) {
        Flatness::Curved(c) => {
            assert!(c.loops().is_empty(), "the loop bounds a face: not a hole");
            assert_eq!(c.faces().len(), 1);
            assert_eq!(c.faces()[0].vertices(), &[0, 1, 2, 0]);
            assert_eq!(
                *c.faces()[0].holonomy(),
                Ratio::of(125, 128),
                "the cocked hat, exactly"
            );
        }
        Flatness::Flat(_) => panic!("the face is curved"),
    }
}

#[test]
fn a_flat_filled_triangle_is_flat() {
    let t = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
    let just = [
        ((0, 1), Ratio::of(5, 4)),
        ((1, 2), Ratio::of(6, 5)),
        ((2, 0), Ratio::of(2, 3)),
    ];
    let field = Field::transports(&t, just)
        .unwrap()
        .with_faces([[0, 1, 2]])
        .unwrap();
    assert!(matches!(check_flat(&field), Flatness::Flat(_)));
}

#[test]
fn filling_one_of_two_triangles_leaves_the_other_a_hole() {
    // A square with diagonal 0-2: triangles {0,1,2} and {0,2,3}. Fill the
    // first, flat; the comma lives only on the second, unfilled.
    let g = graph_from_edges(&[(0, 1), (1, 2), (2, 3), (0, 3), (0, 2)]);
    let field = Field::transports(
        &g,
        [
            ((0, 1), Ratio::of(5, 4)),
            ((1, 2), Ratio::of(6, 5)),
            ((0, 2), Ratio::of(3, 2)),
            ((2, 3), Ratio::of(4, 3)),
            ((3, 0), Ratio::of(1, 2)),
        ],
    )
    .unwrap()
    .with_faces([[0, 1, 2]])
    .unwrap();
    // 0->2->3->0 = 3/2 · 4/3 · 1/2 = 1: also flat. Now detune 2->3.
    assert!(matches!(check_flat(&field), Flatness::Flat(_)));
    let detuned = Field::transports(
        &g,
        [
            ((0, 1), Ratio::of(5, 4)),
            ((1, 2), Ratio::of(6, 5)),
            ((0, 2), Ratio::of(3, 2)),
            ((2, 3), Ratio::of(81, 64)),
            ((3, 0), Ratio::of(1, 2)),
        ],
    )
    .unwrap()
    .with_faces([[0, 1, 2]])
    .unwrap();
    match check_flat(&detuned) {
        Flatness::Curved(c) => {
            assert!(c.faces().is_empty(), "the filled face is still flat");
            assert_eq!(c.loops().len(), 1, "one hole, around the unfilled triangle");
            assert_eq!(
                *c.loops()[0].holonomy(),
                Ratio::of(3, 2)
                    .compose_with(&Ratio::of(81, 64))
                    .compose_with(&Ratio::of(1, 2))
            );
        }
        Flatness::Flat(_) => panic!("the unfilled triangle is detuned"),
    }
}

#[test]
fn a_face_on_a_missing_edge_is_refused() {
    let g = graph_from_edges(&[(0, 1), (1, 2)]);
    assert!(
        Field::transports(&g, [((0, 1), Ratio::of(3, 2)), ((1, 2), Ratio::of(3, 2))])
            .unwrap()
            .with_faces([[0, 1, 2]])
            .is_err()
    );
}

trait ComposeWith {
    fn compose_with(&self, other: &Self) -> Self;
}

impl ComposeWith for Ratio {
    fn compose_with(&self, other: &Self) -> Self {
        prismqueer::field::Group::compose(self, other)
    }
}

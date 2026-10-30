//! prism-005: single- and multi-truth fields as first-class.
//!
//! A scalar is a field on a point. A vector of criteria is a field on k
//! unconnected points. An intonation is a group-valued field on a graph: a
//! transport per edge. Whether a field can be collapsed depends on its value
//! algebra and its curvature (Mara, ruler-and-pact §10–§11):
//! - lattice-valued: a lawful collapse, the meet;
//! - group-valued and flat: collapses up to gauge (a potential), losing the transports;
//! - group-valued and curved: no lawful collapse. The holonomy is the witness.
//!
//! Exact groups give exact holonomy. A float field can only be read through a
//! declared noise band.

#![cfg(feature = "lapack")]

use prismqueer::field::{
    check_flat, check_flat_within, Discrete, Exact, Field, Flatness, Group, Lattice, Least, Log2,
    Perm, Point, Ratio, Zn,
};
use prismqueer::spectral::graph_from_edges;
use prismqueer::spectral::settle::Noise;

fn laws<G: Group>(samples: &[G]) {
    let e = G::identity();
    for a in samples {
        assert_eq!(a.compose(&e), *a, "right identity");
        assert_eq!(e.compose(a), *a, "left identity");
        assert!(a.compose(&a.inverse()).is_identity(), "inverse");
        for b in samples {
            for c in samples {
                assert_eq!(
                    a.compose(b).compose(c),
                    a.compose(&b.compose(c)),
                    "associative"
                );
            }
        }
    }
}

// --- A. The value algebra ---

#[test]
fn exact_groups_keep_the_group_laws() {
    laws(&[
        Ratio::of(3, 2),
        Ratio::of(4, 3),
        Ratio::of(81, 80),
        Ratio::identity(),
    ]);
    laws(&[Zn::<12>::new(7), Zn::<12>::new(5), Zn::<12>::new(0)]);
    laws(&[
        Perm::new(vec![1, 0, 2]),
        Perm::new(vec![0, 2, 1]),
        Perm::new(vec![2, 0, 1]),
    ]);
}

#[test]
fn permutations_do_not_commute() {
    let a = Perm::new(vec![1, 0, 2]);
    let b = Perm::new(vec![0, 2, 1]);
    assert_ne!(a.compose(&b), b.compose(&a));
}

#[test]
fn ratios_are_exact_prime_exponent_vectors() {
    let fifth = Ratio::of(3, 2);
    let octave_down = Ratio::of(1, 2);
    let mut twelve = Ratio::identity();
    for _ in 0..12 {
        twelve = twelve.compose(&fifth);
    }
    for _ in 0..7 {
        twelve = twelve.compose(&octave_down);
    }
    assert_eq!(twelve, Ratio::of(531441, 524288));
    assert_eq!(twelve, Ratio::from_primes([(2, -19), (3, 12)]));
    assert!(
        !twelve.is_identity(),
        "the Pythagorean comma is never the identity"
    );
}

#[test]
fn a_lattice_field_collapses_by_its_meet() {
    let criteria = Field::discrete(vec![Least(7), Least(3), Least(9)]);
    assert_eq!(criteria.meet(), Least(3));
    let checks = Field::discrete(vec![true, true, false]);
    assert!(!checks.meet());
    let scalar: Field<Point, Least> = Field::point(Least(4));
    assert_eq!(scalar.meet(), Least(4), "a scalar is a field on a point");
    assert_eq!(
        Field::<Discrete, Least>::discrete(vec![]).meet(),
        Least::top(),
        "the empty meet is top"
    );
}

// --- B and C. Group fields on graphs: flat or curved ---

/// The circle of fifths: twelve steps i -> i+1 (and 11 -> 0), each a fifth,
/// seven of them reduced by an octave so every pitch stays in one octave.
fn circle_of_fifths<V: Group>(
    up: V,
    reduced: V,
) -> (
    prismqueer::spectral::Graph,
    Field<prismqueer::field::Graph, V>,
) {
    let mut edges: Vec<(u64, u64)> = (0..11).map(|i| (i, i + 1)).collect();
    edges.push((11, 0));
    let graph = graph_from_edges(&edges);
    let steps = edges
        .iter()
        .enumerate()
        .map(|(k, &e)| (e, if k < 5 { up.clone() } else { reduced.clone() }));
    let field = Field::transports(&graph, steps).expect("every edge has a transport");
    (graph, field)
}

#[test]
fn twelve_fifths_hold_the_pythagorean_comma_exactly() {
    let (_, field) = circle_of_fifths(Ratio::of(3, 2), Ratio::of(3, 4));
    match check_flat(&field) {
        Flatness::Curved(curved) => {
            let loops = curved.loops();
            assert_eq!(loops.len(), 1, "one independent cycle");
            assert_eq!(
                loops[0].vertices(),
                (0..12).chain([0]).collect::<Vec<u64>>(),
                "loop 0 -> 1 -> ... -> 11 -> 0"
            );
            assert_eq!(*loops[0].holonomy(), Ratio::of(531441, 524288));
        }
        Flatness::Flat(_) => panic!("the circle of fifths does not close"),
    }
}

#[test]
fn a_tree_is_flat_and_its_potential_absorbs_every_transport() {
    let graph = graph_from_edges(&[(0, 1), (1, 2), (1, 3)]);
    let field = Field::transports(
        &graph,
        [
            ((0, 1), Ratio::of(3, 2)),
            ((2, 1), Ratio::of(5, 4)),
            ((1, 3), Ratio::of(9, 8)),
        ],
    )
    .unwrap();
    match check_flat(&field) {
        Flatness::Flat(flat) => {
            for ((u, v), t) in [
                ((0, 1), Ratio::of(3, 2)),
                ((2, 1), Ratio::of(5, 4)),
                ((1, 3), Ratio::of(9, 8)),
            ] {
                let p = flat.potential();
                assert_eq!(
                    p[&u].inverse().compose(&p[&v]),
                    t,
                    "transport {u}->{v} = p(u)^-1 p(v)"
                );
            }
        }
        Flatness::Curved(_) => panic!("a tree has no cycles"),
    }
}

#[test]
fn a_curved_permutation_field_has_a_non_identity_holonomy() {
    let graph = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
    let swap01 = Perm::new(vec![1, 0, 2]);
    let swap12 = Perm::new(vec![0, 2, 1]);
    let field = Field::transports(
        &graph,
        [
            ((0, 1), swap01.clone()),
            ((1, 2), swap12.clone()),
            ((2, 0), Perm::identity()),
        ],
    )
    .unwrap();
    match check_flat(&field) {
        Flatness::Curved(curved) => {
            let h = curved.loops()[0].holonomy();
            assert_eq!(
                *h,
                swap01.compose(&swap12),
                "0 -> 1 -> 2 -> 0, composed in path order"
            );
            assert!(!h.is_identity());
        }
        Flatness::Flat(_) => panic!("two transpositions around a triangle don't close"),
    }
}

#[test]
fn a_float_field_is_read_only_through_a_declared_band() {
    // 12-TET: every fifth is 7/12 of an octave; reduced ones lose an octave.
    let (_, tempered) = circle_of_fifths(Log2(7.0 / 12.0), Log2(7.0 / 12.0 - 1.0));
    assert!(
        matches!(check_flat_within(&tempered, Noise(1e-9)), Flatness::Flat(_)),
        "tempered closes within the band"
    );
    // Pythagorean fifths as floats: open by 12 log2(3/2) - 7 ≈ 0.0235.
    let (_, pythagorean) = circle_of_fifths(Log2((1.5f64).log2()), Log2((0.75f64).log2()));
    assert!(matches!(
        check_flat_within(&pythagorean, Noise(1e-9)),
        Flatness::Curved(_)
    ));
}

#[test]
fn a_field_missing_a_transport_is_refused() {
    let graph = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
    assert!(Field::transports(
        &graph,
        [((0, 1), Zn::<12>::new(7)), ((1, 2), Zn::<12>::new(7))]
    )
    .is_err());
    assert!(
        Field::transports(
            &graph,
            [
                ((0, 1), Zn::<12>::new(1)),
                ((1, 2), Zn::<12>::new(1)),
                ((0, 2), Zn::<12>::new(1)),
                ((5, 6), Zn::<12>::new(1))
            ]
        )
        .is_err(),
        "an edge off the graph"
    );
}

/// Loki's napkin (prism-007): 45/32 × 64/45 = 2 closes, but two tritones
/// up and an octave back leave the diaschisma, 2025/2048, every lap. A
/// characterization test: the capability exists since prism-005.
#[test]
fn two_tritones_and_an_octave_leave_the_diaschisma() {
    let graph = graph_from_edges(&[(0, 1), (1, 2), (0, 2)]);
    let closes = Field::transports(
        &graph,
        [
            ((0, 1), Ratio::of(45, 32)),
            ((1, 2), Ratio::of(64, 45)),
            ((2, 0), Ratio::of(1, 2)),
        ],
    )
    .unwrap();
    assert!(
        matches!(check_flat(&closes), Flatness::Flat(_)),
        "45/32 × 64/45 × 1/2 = 1"
    );
    let twice = Field::transports(
        &graph,
        [
            ((0, 1), Ratio::of(45, 32)),
            ((1, 2), Ratio::of(45, 32)),
            ((2, 0), Ratio::of(1, 2)),
        ],
    )
    .unwrap();
    match check_flat(&twice) {
        Flatness::Curved(c) => {
            assert_eq!(c.loops()[0].vertices(), &[0, 1, 2, 0]);
            assert_eq!(
                *c.loops()[0].holonomy(),
                Ratio::of(2025, 2048),
                "the diaschisma, exactly"
            );
        }
        Flatness::Flat(_) => panic!("(45/32)² doesn't reach the octave"),
    }
}

/// Compile-level: only exact groups get `check_flat`; this just names the bound.
fn _exact_bound<V: Exact>() {}

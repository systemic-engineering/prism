//! One graph under `room` and `field` (prism-011, step 2).
//!
//! `room` and `field` are built on one multigraph, `field::Graph`, so an exact
//! solve in the room and a holonomy in the field are about the same graph.
//! The laws:
//!
//! 1. **Room over a multigraph.** Parallel edges are distinct C¹ basis
//!    vectors: δ has one row per edge, and L₀ = δ*δ counts multiplicity (a
//!    double edge adds 2 to each end's diagonal and −2 off it). A self-loop's
//!    row stays zero. A 2-loop is harmonic, so dim ker D = b₀ + b₁ counts it.
//! 2. **Room and field agree on one graph.** field's `cycle_rank` is room's
//!    b₁, read from the spectrum of L₁↓ = δδ*. `check_flat`'s verdict on a
//!    1-cochain c is whether c is in the image of room's δ, and its witnesses
//!    check in the room, exactly: a potential p has δp = c, and a curved loop
//!    is a cycle z (δ*z = 0) with ⟨z, c⟩ its holonomy ≠ 0, so no g has δg = c.

#![cfg(feature = "lapack")]

use prismqueer::field::{check_flat, Edge, Field, Flatness, Graph, Ratio};
use prismqueer::music::{Atom, Note, Rule, Tuning};
use prismqueer::room::{coboundary, dirac, laplacian, Edges, Map, Room, Vertices, Z};
use prismqueer::spectral::graph_from_edges;

/// dim ker of a symmetric exact map: x's multiplicity as a root of its
/// charpoly, which for a symmetric map is the kernel's dimension. Exact.
fn kernel<K: prismqueer::room::Kind>(map: &Map<K, K, Z>) -> usize {
    map.clone()
        .symmetric()
        .expect("symmetric")
        .charpoly()
        .coefficients()
        .iter()
        .rev()
        .take_while(|c| **c == Z::from(0))
        .count()
}

/// The measured eigenvalues of a symmetric exact map within `band` of 0.
fn measured_zeros<K: prismqueer::room::Kind>(map: &Map<K, K, Z>, band: f64) -> usize {
    map.clone()
        .symmetric()
        .expect("symmetric")
        .measure()
        .spectrum()
        .expect("measured")
        .iter()
        .filter(|l| l.abs() <= band)
        .count()
}

fn ints(xs: &[i64]) -> Vec<Z> {
    xs.iter().map(|&x| Z::from(x)).collect()
}

fn l1_down(g: &Graph) -> Map<Edges, Edges, Z> {
    let delta = coboundary(g);
    delta.adjoint().then(&delta).expect("C¹ → C⁰ → C¹")
}

// --- Law 1: room over a multigraph ------------------------------------------

/// Two parallel edges a–b: two rows of δ, both −1 at a and +1 at b. L₀ is
/// [[2, −2], [−2, 2]]. The 2-loop is a cycle, so b₁ = 1 and D's kernel is
/// b₀ + b₁ = 2.
#[test]
fn a_two_loop_is_two_rows_of_delta_and_one_harmonic_cycle() {
    let g = Graph::new([], [(0, 1), (0, 1)]);
    let delta = coboundary(&g);
    assert_eq!((delta.to().dim(), delta.from().dim()), (2, 2), "δ is 2 × 2");
    for row in 0..2 {
        assert_eq!(delta.entry(row, 0), Some(&Z::from(-1)), "δ[{row}, a]");
        assert_eq!(delta.entry(row, 1), Some(&Z::from(1)), "δ[{row}, b]");
    }

    let l0 = delta.then(&delta.adjoint()).expect("C⁰ → C¹ → C⁰");
    let expected = [[2, -2], [-2, 2]];
    for (i, row) in expected.iter().enumerate() {
        for (j, &x) in row.iter().enumerate() {
            assert_eq!(l0.entry(i, j), Some(&Z::from(x)), "L₀[{i}, {j}]");
        }
    }
    assert_eq!(
        laplacian(&g),
        l0.clone().symmetric().unwrap(),
        "laplacian is δ*δ"
    );

    assert_eq!(kernel(&l1_down(&g)), 1, "b₁ = 1: the 2-loop is harmonic");
    assert_eq!(g.cycle_rank(), 1, "field counts the same loop");
    let d = dirac(&g);
    assert_eq!(kernel(&d), 2, "dim ker D = b₀ + b₁ = 1 + 1");
    // D² = L₀ ⊕ L₁↓, both with spectrum {0, 4}: D's is {−2, 0, 0, 2}.
    let spectrum = d.measure().symmetric().unwrap().spectrum().unwrap();
    for (got, want) in spectrum.iter().zip([-2.0, 0.0, 0.0, 2.0]) {
        assert!((got - want).abs() < 1e-12, "D's spectrum {spectrum:?}");
    }
}

/// A doubled edge is its own C¹: two dimensions, its own address, over the
/// same C⁰ as the single edge.
#[test]
fn a_doubled_edge_is_another_edge_room_over_the_same_vertex_room() {
    let single = Graph::new([], [(0, 1)]);
    let double = Graph::new([], [(0, 1), (0, 1)]);
    let (one, two) = (Room::edges(&single), Room::edges(&double));
    assert_eq!((one.dim(), two.dim()), (1, 2));
    assert_ne!(
        one.complex(),
        two.complex(),
        "multiplicity is in the address"
    );
    assert_eq!(one.skeleton(), two.skeleton(), "one C⁰ under both");
    assert_eq!(Room::<Vertices>::vertices(&single), Room::vertices(&double));
}

/// Three parallel edges and a self-loop: L₀ counts the multiplicity (3 on
/// the diagonal, −3 off it), the loop's row is zero, and b₁ = 4 − 2 + 1 = 3.
#[test]
fn laplacian_counts_multiplicity_and_a_self_loop_stays_zero() {
    let g = Graph::new([], [(0, 1), (1, 1), (1, 0), (0, 1)]);
    let delta = coboundary(&g);
    assert_eq!(delta.to().dim(), 4, "one row per edge");
    // Rows in canonical order: (0, 1) three times, then (1, 1).
    for col in 0..2 {
        assert_eq!(delta.entry(3, col), Some(&Z::from(0)), "the loop's row");
    }
    let l0 = delta.then(&delta.adjoint()).unwrap();
    assert_eq!(l0.entry(0, 0), Some(&Z::from(3)));
    assert_eq!(l0.entry(0, 1), Some(&Z::from(-3)));
    assert_eq!(l0.entry(1, 1), Some(&Z::from(3)));
    assert_eq!(kernel(&l1_down(&g)), 3);
    assert_eq!(g.cycle_rank(), 3);
    assert_eq!(kernel(&dirac(&g)), 1 + 3, "dim ker D = b₀ + b₁");
}

// --- Law 2: room and field agree on one graph -------------------------------

/// The cycle z of a loop, as a vector of C¹: +1 on an edge walked from its
/// smaller end, −1 against it.
fn cycle(g: &Graph, vertices: &[u64], edges: &[Edge]) -> Vec<Z> {
    let basis: Vec<Edge> = g.edges().collect();
    let mut z = vec![Z::from(0); basis.len()];
    for (k, e) in edges.iter().enumerate() {
        let i = basis.iter().position(|b| b == e).expect("an edge of g");
        let sign = if vertices[k] == e.ends().0 { 1 } else { -1 };
        z[i] = Z::from(z[i].value() + sign);
    }
    z
}

fn dot(x: &[Z], y: &[Z]) -> Z {
    Z::from(
        x.iter()
            .zip(y)
            .map(|(a, b)| a.value() * b.value())
            .sum::<num_bigint::BigInt>(),
    )
}

/// Whether c is in the image of δ, read in the room alone: im δ = (ker δ*)⊥
/// = (ker L₁↓)⊥, so c is a coboundary iff its projection on ker L₁↓ is zero.
/// The kernel's dimension is exact (the charpoly); the projection is
/// measured, through a declared band.
fn in_image(g: &Graph, c: &[Z]) -> bool {
    let l1 = l1_down(g);
    let k = kernel(&l1);
    let modes = l1.symmetric().unwrap().measure().decompose().unwrap();
    let c: Vec<f64> = c
        .iter()
        .map(|x| x.value().to_string().parse().unwrap())
        .collect();
    let projection: f64 = (0..k)
        .map(|i| {
            let v = modes.vector(i).unwrap();
            v.iter().zip(&c).map(|(a, b)| a * b).sum::<f64>().powi(2)
        })
        .sum();
    projection <= 1e-12
}

/// check_flat's verdict on `field` (a Z-valued field, values `c` in C¹
/// order) against room's δ on the same graph: verdict, b₁, and both
/// witnesses checked exactly in the room.
fn agree(name: &str, field: &Field<Graph, Z>, c: &[Z]) -> bool {
    let g = field.graph();
    let delta = coboundary(g);
    assert_eq!(delta.to().dim(), c.len(), "{name}: c is a 1-cochain of g");

    let l1 = l1_down(g);
    assert_eq!(kernel(&l1), g.cycle_rank(), "{name}: b₁, exactly");
    assert_eq!(
        measured_zeros(&l1, 1e-9),
        g.cycle_rank(),
        "{name}: b₁, measured"
    );
    let b0 = kernel(&delta.then(&delta.adjoint()).unwrap());
    assert_eq!(kernel(&dirac(g)), b0 + g.cycle_rank(), "{name}: ker D");

    let image = in_image(g, c);
    match check_flat(field) {
        Flatness::Flat(flat) => {
            assert!(image, "{name}: Flat, but c isn't in im δ");
            let p: Vec<Z> = g.vertices().map(|v| flat.potential()[&v].clone()).collect();
            assert_eq!(delta.apply(&p).unwrap(), c, "{name}: δp = c");
            true
        }
        Flatness::Curved(curved) => {
            assert!(!image, "{name}: Curved, but c is in im δ");
            assert!(curved.faces().is_empty(), "{name}: no faces declared");
            assert!(!curved.loops().is_empty(), "{name}: a curved loop");
            for l in curved.loops() {
                let z = cycle(g, l.vertices(), l.edges());
                assert_eq!(
                    delta.adjoint().apply(&z).unwrap(),
                    vec![Z::from(0); g.vertex_count()],
                    "{name}: the loop is a cycle, δ*z = 0"
                );
                assert_eq!(&dot(&z, c), l.holonomy(), "{name}: ⟨z, c⟩ is the holonomy");
                assert_ne!(l.holonomy(), &Z::from(0), "{name}: and it isn't 0");
            }
            false
        }
    }
}

/// An edge and its integer transport.
type Transport = ((u64, u64), i64);

/// A Z-valued field from transports given in canonical order (smaller end
/// first, by ends, parallels adjacent), so its C¹ order is the given order.
fn field(given: &[Transport]) -> (Field<Graph, Z>, Vec<Z>) {
    let f = Field::multigraph(given.iter().map(|&(e, x)| (e, Z::from(x))));
    let ends: Vec<(u64, u64)> = f.graph().edges().map(|e| e.ends()).collect();
    let given_ends: Vec<(u64, u64)> = given.iter().map(|&(e, _)| e).collect();
    assert_eq!(ends, given_ends, "given in C¹ order");
    let c = given.iter().map(|&(_, x)| Z::from(x)).collect();
    (f, c)
}

#[test]
fn check_flat_agrees_with_the_image_of_delta_on_multigraphs() {
    let cases: &[(&str, &[Transport], bool)] = &[
        ("2-loop, equal", &[((0, 1), 3), ((0, 1), 3)], true),
        ("2-loop, apart", &[((0, 1), 3), ((0, 1), 5)], false),
        ("self-loop at 0", &[((2, 2), 0), ((2, 3), 7)], true),
        ("self-loop twisted", &[((2, 2), 1), ((2, 3), 7)], false),
        (
            "triangle, a doubled side and a loop, flat",
            &[
                ((0, 1), 4),
                ((0, 1), 4),
                ((0, 2), 9),
                ((1, 2), 5),
                ((2, 2), 0),
            ],
            true,
        ),
        (
            "triangle, a doubled side and a loop, the triangle curved",
            &[
                ((0, 1), 4),
                ((0, 1), 4),
                ((0, 2), 8),
                ((1, 2), 5),
                ((2, 2), 0),
            ],
            false,
        ),
        (
            "two components off 0: a 2-loop and a triple edge",
            &[
                ((10, 11), -2),
                ((10, 11), -2),
                ((20, 30), 6),
                ((20, 30), 6),
                ((20, 30), 6),
            ],
            true,
        ),
        (
            "two components, the triple edge apart",
            &[
                ((10, 11), -2),
                ((10, 11), -2),
                ((20, 30), 6),
                ((20, 30), 6),
                ((20, 30), 7),
            ],
            false,
        ),
    ];
    for &(name, given, flat) in cases {
        let (f, c) = field(given);
        assert_eq!(agree(name, &f, &c), flat, "{name}: verdict");
    }
}

/// A simple graph, through `graph_from_edges` and `Field::transports`: the
/// same room as the multigraph it is.
#[test]
fn a_simple_graph_is_the_same_room_through_either_constructor() {
    let k4 = graph_from_edges(&[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
    let pairs: Vec<(u64, u64)> = k4.edges().collect();
    assert_eq!(k4.multigraph(), &Graph::new([], pairs.clone()));
    let p = [0i64, 1, 3, 7];
    let flat = Field::transports(
        &k4,
        pairs
            .iter()
            .map(|&(u, v)| ((u, v), Z::from(p[v as usize] - p[u as usize]))),
    )
    .unwrap();
    let c = ints(
        &pairs
            .iter()
            .map(|&(u, v)| p[v as usize] - p[u as usize])
            .collect::<Vec<_>>(),
    );
    assert!(agree("K4, a potential", &flat, &c));
    let mut twisted = c.clone();
    twisted[5] = Z::from(5);
    let curved = Field::transports(
        &k4,
        pairs.iter().zip(&twisted).map(|(&e, x)| (e, x.clone())),
    )
    .unwrap();
    assert!(!agree("K4, one edge twisted", &curved, &twisted));
}

fn note(head: &str, rule: &str, premise: &str) -> Note {
    Note::new(Atom::new(head), Rule::new(rule), [Atom::new(premise)])
}

/// tests/music.rs's two-Note loop: a pure and a grave fifth on a → e. A
/// Ratio is a vector of prime exponents, so the tuning is a ℤᵏ cochain, one
/// Z cochain per prime: it is flat iff every prime's cochain is in im δ.
#[test]
fn the_two_note_loop_is_curved_in_the_room_too() {
    let fifths = |grave: Ratio| {
        Tuning::new([
            (note("e", "pure fifth", "a"), Ratio::of(3, 2)),
            (note("e", "grave fifth", "a"), grave),
        ])
        .unwrap()
    };
    for (grave, flat) in [(Ratio::of(40, 27), false), (Ratio::of(3, 2), true)] {
        let tuning = fifths(grave.clone());
        let g = tuning.field().graph();
        assert_eq!(g.multiplicity((0, 1)), 2, "two Notes, two edges");
        assert_eq!(kernel(&l1_down(g)), g.cycle_rank(), "b₁ = 1");
        let primes = [2u64, 3, 5];
        let exponent = |r: &Ratio, p: u64| Z::from(*r.exponents().get(&p).unwrap_or(&0));
        let cochains: Vec<Vec<Z>> = primes
            .iter()
            .map(|&p| vec![exponent(&Ratio::of(3, 2), p), exponent(&grave, p)])
            .collect();
        let every_prime_exact = cochains.iter().all(|c| in_image(g, c));
        match check_flat(tuning.field()) {
            Flatness::Flat(_) => assert!(flat && every_prime_exact),
            Flatness::Curved(curved) => {
                assert!(!flat && !every_prime_exact);
                let l = &curved.loops()[0];
                let z = cycle(g, l.vertices(), l.edges());
                for (&p, c) in primes.iter().zip(&cochains) {
                    assert_eq!(
                        dot(&z, c),
                        exponent(l.holonomy(), p),
                        "the comma's {p}-exponent"
                    );
                }
            }
        }
    }
}

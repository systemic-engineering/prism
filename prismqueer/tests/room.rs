//! Rooms: the spaces operators move between.
//!
//! C⁰ (the vertex room) and C¹ (the edge room) of one graph; D lives in
//! C⁰ ⊕ C¹; δ is a door from C⁰ into C¹ and δ* the door back. The laws:
//!
//! 1. δ is exact: entries ±1 or 0, one +1 and one −1 per edge row; a
//!    self-loop's row is zero.
//! 2. D² = L₀ ⊕ L₁↓ exactly, and L₀ is the Laplacian prism already builds.
//!    L₁↓ = δδ* is the down-Laplacian: a graph has no 2-cells, so C¹ has no
//!    up part, and the lower block is L₁↓, not the full Hodge L₁.
//! 3. An eigenbasis is read by columns, and every column is an eigenvector.
//! 4. `Instrument`'s spectrum is that of a D built independently, flat,
//!    from the edge list.
//! 6. The exact charpoly: det(xI − M) over Z, monic, leading first, and D's
//!    follows from D² = L₀ ⊕ L₁↓.
//! 7. The measured spectrum roots the exact charpoly, within a declared band.
//! 8. The triple is even: γ (+1 on C⁰, −1 on C¹) anticommutes with D exactly.
//! 9. Identity is per skeleton: C⁰ is its vertex set alone, C¹ sits over a
//!    C⁰, and Laplacians over one C⁰ compose exactly (their commutator
//!    too), while rooms over different vertex sets still refuse.
//!
//! (Law 5, wrong-kind composition is a compile error, is a doctest on
//! `prismqueer::room`.)
//!
//! Expected values come from outside `room`: the graph's own edge list, a
//! hand-checked Laplacian (`laplacian_from_edges`), `Instrument`, and a
//! naive flat D built in this file.

#![cfg(feature = "lapack")]

use std::collections::BTreeSet;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
use prismqueer::band::{zeros, Verdict};
use prismqueer::room::{
    coboundary, coboundary_over, dirac, laplacian, Edges, Map, Room, RoomError, Sum, Vertices, Z,
};
use prismqueer::spectral::instrument::Instrument;
use prismqueer::spectral::{graph_from_edges, Graph};
use prismqueer::spectral_dimension::laplacian_from_edges;

fn triangle() -> Graph {
    graph_from_edges(&[(0, 1), (1, 2), (0, 2)])
}

/// Vertex ids that aren't 0..n, so position and id can't be confused.
fn path() -> Graph {
    graph_from_edges(&[(10, 20), (20, 30), (30, 40)])
}

fn square_with_diagonal() -> Graph {
    graph_from_edges(&[(0, 1), (1, 2), (2, 3), (0, 3), (0, 2)])
}

fn graphs() -> [(&'static str, Graph); 3] {
    [
        ("triangle", triangle()),
        ("path", path()),
        ("square with diagonal", square_with_diagonal()),
    ]
}

// --- Law 1: δ is exact ------------------------------------------------------

#[test]
fn coboundary_is_exact_signed_incidence() {
    for (name, g) in graphs() {
        let delta = coboundary(&g);
        let vertices: Vec<u64> = g.vertices().collect();
        let edges: Vec<(u64, u64)> = g.edges().collect();
        assert_eq!(delta.from().dim(), vertices.len(), "{name}: C⁰");
        assert_eq!(delta.to().dim(), edges.len(), "{name}: C¹");
        for (row, &(u, v)) in edges.iter().enumerate() {
            let mut plus = 0;
            let mut minus = 0;
            for (col, &w) in vertices.iter().enumerate() {
                let x = delta.entry(row, col).expect("inside the map").clone();
                assert!(
                    x == Z::from(1) || x == Z::from(-1) || x == Z::from(0),
                    "{name}: δ[{row}, {col}] = {x:?}"
                );
                // (δs)(u, v) = s(v) − s(u), edges in canonical orientation.
                let expected = if w == v {
                    Z::from(1)
                } else if w == u {
                    Z::from(-1)
                } else {
                    Z::from(0)
                };
                assert_eq!(x, expected, "{name}: δ[{row}, {col}] on edge ({u}, {v})");
                plus += usize::from(x == Z::from(1));
                minus += usize::from(x == Z::from(-1));
            }
            assert_eq!((plus, minus), (1, 1), "{name}: edge row {row}");
        }
        assert_eq!(delta.entry(edges.len(), 0), None, "{name}: past the rows");
        assert_eq!(
            delta.entry(0, vertices.len()),
            None,
            "{name}: past the columns"
        );
    }
}

/// A self-loop (v, v) is an edge whose two ends are one vertex: (δs)(v, v) =
/// s(v) − s(v) = 0, so its row of δ is zero, whatever s. It adds nothing to
/// L₀ = δ*δ, and as a 1-cochain it's a cycle: harmonic, in ker D.
#[test]
fn a_self_loops_coboundary_row_is_zero() {
    let g = graph_from_edges(&[(0, 1), (1, 1), (1, 2)]);
    let edges: Vec<(u64, u64)> = g.edges().collect();
    assert_eq!(edges, [(0, 1), (1, 1), (1, 2)], "Graph holds the loop");
    let delta = coboundary(&g);
    for col in 0..3 {
        assert_eq!(delta.entry(1, col), Some(&Z::from(0)), "δ[(1, 1), {col}]");
    }
    // L₀ is P₃'s: the loop changes no degree.
    let l0 = delta.then(&delta.adjoint()).expect("C⁰ → C¹ → C⁰");
    let p3 = laplacians(&graph_from_edges(&[(0, 1), (1, 2)])).0;
    for (i, j) in (0..3).flat_map(|i| (0..3).map(move |j| (i, j))) {
        assert_eq!(l0.entry(i, j), p3.entry(i, j), "L₀[{i}, {j}]");
    }
    // D's kernel: one component, one cycle (the loop).
    let p = dirac(&g).symmetric().expect("D = D*").charpoly();
    let zeros_at_0 = p
        .coefficients()
        .iter()
        .rev()
        .take_while(|c| **c == Z::from(0))
        .count();
    assert_eq!(zeros_at_0, 2, "dim ker D = b₀ + b₁ = 1 + 1");
}

// --- Law 2: D² = L₀ ⊕ L₁↓, exactly -----------------------------------------

#[test]
fn dirac_squared_is_l0_plus_the_down_laplacian() {
    for (name, g) in graphs() {
        let delta = coboundary(&g);
        let star = delta.adjoint();
        let l0 = delta.then(&star).expect("δ then δ*: C⁰ → C⁰");
        let l1_down = star.then(&delta).expect("δ* then δ: C¹ → C¹");

        let d = dirac(&g);
        let d2 = d.then(&d).expect("D then D");
        let blocks = Map::diagonal(&l0, &l1_down).expect("one complex");
        assert_eq!(d2, blocks, "{name}: D² = L₀ ⊕ L₁↓");

        // And D is δ below the diagonal, δ* above it.
        let room = d.from().clone();
        let below = Map::inject_left(&room)
            .then(&d)
            .and_then(|m| m.then(&Map::project_right(&room)))
            .expect("C⁰ → C⁰⊕C¹ → C¹");
        assert_eq!(below, delta, "{name}: the lower block of D is δ");

        // L₀ is the Laplacian prism already builds (spectral_dimension).
        let index = |x: u64| g.vertices().position(|v| v == x).unwrap();
        let edges: Vec<(usize, usize)> = g.edges().map(|(u, v)| (index(u), index(v))).collect();
        let n = g.vertex_count();
        let known = laplacian_from_edges(n, &edges);
        let measured = l0.measure();
        for i in 0..n {
            for j in 0..n {
                // Integers in f64 are exact: equality, not a tolerance.
                assert_eq!(
                    *measured.entry(i, j).unwrap(),
                    known[i * n + j],
                    "{name}: L₀[{i}, {j}]"
                );
            }
        }
    }
}

// --- Law 3: eigenpairs read through the type --------------------------------

/// ‖A v − λ v‖∞ for one column, read against the declared `settle` band at
/// the operator's scale (its largest |eigenvalue|).
fn residual_is_zero(a: &Map<Vertices, Vertices, f64>, scale: f64, value: f64, v: &[f64]) -> bool {
    let av = a.apply(v).expect("v lives in A's room");
    let r = av
        .iter()
        .zip(v)
        .map(|(x, y)| (x - value * y).abs())
        .fold(0.0, f64::max);
    zeros::settle(v.len()).read(r, scale) == Verdict::Zero
}

#[test]
fn eigenvectors_are_columns() {
    let room = coboundary(&path()).from().clone();
    // Distinct eigenvalues and an eigenbasis that isn't symmetric: reading a
    // row as an eigenvector gives a wrong answer (checked below).
    let rows = [
        vec![2.0, 1.0, 0.0, 0.0],
        vec![1.0, 3.0, 1.0, 0.0],
        vec![0.0, 1.0, 4.0, 1.0],
        vec![0.0, 0.0, 1.0, 7.0],
    ];
    let a = Map::from_rows(&room, &room, &rows).expect("4×4 on C⁰");
    let modes = a
        .clone()
        .symmetric()
        .expect("symmetric")
        .decompose()
        .expect("measured");

    let spectrum = modes.spectrum();
    assert_eq!(spectrum.len(), 4);
    assert!(
        spectrum.windows(2).all(|w| w[0] <= w[1]),
        "sorted ascending"
    );
    let trace: f64 = spectrum.iter().sum();
    let scale = spectrum.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    assert!((trace - 16.0).abs() < 1e-12, "trace is preserved: {trace}");

    for (k, &value) in spectrum.iter().enumerate() {
        let v = modes.vector(k).expect("column k");
        assert!(
            residual_is_zero(&a, scale, value, v),
            "A v_{k} = λ_{k} v_{k}"
        );
    }
    assert!(modes.vector(4).is_none(), "no column past the end");
    assert_eq!(modes.vectors().count(), 4);

    // The test can tell rows from columns: the transpose's columns (the rows
    // a row-major reader would take) are not all eigenvectors.
    let transposed: Vec<Vec<f64>> = (0..4)
        .map(|k| modes.vectors().map(|col| col[k]).collect())
        .collect();
    assert!(
        spectrum
            .iter()
            .zip(&transposed)
            .any(|(&value, row)| !residual_is_zero(&a, scale, value, row)),
        "a row/column mix-up must be visible"
    );
}

// --- Law 4: the same spectrum as Instrument, built independently ---------

/// D, deliberately naive and flat: straight from an edge list, row-major,
/// with no `room`, no `Graph` and no shared code. Vertices sorted, edges
/// canonical (lower id first) and sorted; rows 0..n₀ are vertices, then one
/// row per edge, which holds δ: −1 at its lower end, +1 at its upper end.
/// D = δ + δ*, so the same entries mirror above the diagonal.
fn naive_dirac(edges: &[(u64, u64)]) -> (usize, Vec<f64>) {
    let mut canonical: Vec<(u64, u64)> = edges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
    canonical.sort_unstable();
    canonical.dedup();
    let mut vertices: Vec<u64> = canonical.iter().flat_map(|&(u, v)| [u, v]).collect();
    vertices.sort_unstable();
    vertices.dedup();
    let at = |x: u64| vertices.binary_search(&x).expect("an endpoint");
    let n0 = vertices.len();
    let n = n0 + canonical.len();
    let mut d = vec![0.0_f64; n * n];
    for (k, &(u, v)) in canonical.iter().enumerate() {
        let e = n0 + k;
        // Summed, so a self-loop's two ends cancel.
        d[e * n + at(u)] -= 1.0;
        d[at(u) * n + e] -= 1.0;
        d[e * n + at(v)] += 1.0;
        d[at(v) * n + e] += 1.0;
    }
    (n, d)
}

/// Graphs for the cross-check: a cycle, a path, a chordal square, two
/// components, a cycle with a tail, and a self-loop.
fn reference_graphs() -> [(&'static str, Vec<(u64, u64)>); 6] {
    [
        ("triangle", vec![(0, 1), (1, 2), (0, 2)]),
        ("path", vec![(10, 20), (20, 30), (30, 40)]),
        (
            "square with diagonal",
            vec![(0, 1), (1, 2), (2, 3), (0, 3), (0, 2)],
        ),
        (
            "triangle and an edge (disconnected)",
            vec![(0, 1), (1, 2), (0, 2), (5, 6)],
        ),
        (
            "cycle with a tail",
            vec![(0, 1), (1, 2), (2, 3), (3, 0), (3, 4), (4, 5)],
        ),
        ("path with a self-loop", vec![(0, 1), (1, 1), (1, 2)]),
    ]
}

/// The independent cross-check: `Instrument` (room-built D, column-major,
/// handed to dsyev as stored) against a flat D built here from the edge list
/// (row-major, through `ffi`'s own layout conversion).
///
/// - Against `ffi::eigensystem`, the same dsyev job (`'V'`) on the flat
///   matrix: bit-equal. Same matrix, same basis order, same arithmetic.
/// - Against `ffi::eigenvalues` (dsyev `'N'`): within `settle`'s band only.
///   `'N'` and `'V'` tridiagonalize alike but then run different QR
///   iterations (dsterf vs dsteqr), so their last bits differ, measured.
#[test]
fn dirac_spectrum_matches_an_independent_flat_build() {
    let bits = |xs: &[f64]| xs.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    for (name, edges) in reference_graphs() {
        let (n, flat) = naive_dirac(&edges);
        // dsyev reads one triangle only: a sign flipped in the other would
        // pass unseen, so the reference's symmetry is checked here.
        for (i, j) in (0..n).flat_map(|i| (0..n).map(move |j| (i, j))) {
            assert_eq!(flat[i * n + j], flat[j * n + i], "{name}: D[{i}, {j}]");
        }
        let instrument = Instrument::of(&graph_from_edges(&edges)).expect("measured");
        let measured = instrument.eigenvalues();

        let (same_job, _) = prismqueer::ffi::eigensystem(n, &flat).expect("measured");
        assert_eq!(
            bits(measured),
            bits(&same_job),
            "{name}: {measured:?} vs {same_job:?}"
        );

        let values_only = prismqueer::ffi::eigenvalues(n, &flat).expect("measured");
        let scale = values_only.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
        assert_eq!(values_only.len(), measured.len(), "{name}");
        for (a, b) in measured.iter().zip(&values_only) {
            assert_eq!(
                zeros::settle(n).read((a - b).abs(), scale),
                Verdict::Zero,
                "{name}: {a} vs {b}"
            );
        }
    }
}

// --- Runtime mismatches are checked errors ----------------------------------

#[test]
fn composing_across_complexes_is_an_error() {
    let a = coboundary(&triangle());
    let b = coboundary(&path());
    match a.then(&b.adjoint()) {
        Err(RoomError::Mismatch { expected, found }) => {
            assert_eq!(expected.dim, 3);
            assert_eq!(found.dim, 3, "same dimension, different complex");
            assert_ne!(expected.complex, found.complex);
        }
        other => panic!("expected a mismatch, got {other:?}"),
    }
    assert!(matches!(
        Room::sum(a.from(), b.to()),
        Err(RoomError::Mismatch { .. })
    ));
    assert!(matches!(
        a.measure().plus(&b.measure()),
        Err(RoomError::Mismatch { .. })
    ));
}

#[test]
fn wrong_lengths_are_errors() {
    let delta = coboundary(&triangle());
    assert_eq!(
        delta.apply(&[Z::from(1), Z::from(2)]),
        Err(RoomError::Length {
            expected: 3,
            found: 2
        })
    );
    assert_eq!(
        delta.apply(&[Z::from(1), Z::from(2), Z::from(4)]),
        Ok(vec![Z::from(1), Z::from(3), Z::from(2)]),
        "δs(u, v) = s(v) − s(u) on (0,1), (0,2), (1,2)"
    );
    let room = delta.from().clone();
    assert_eq!(
        Map::<Vertices, Vertices, Z>::from_rows(&room, &room, &vec![vec![Z::from(1); 3]; 2]),
        Err(RoomError::Length {
            expected: 3,
            found: 2
        })
    );
    assert_eq!(
        Map::<Vertices, Vertices, Z>::from_rows(
            &room,
            &room,
            &[
                vec![Z::from(1); 3],
                vec![Z::from(1); 2],
                vec![Z::from(1); 3]
            ]
        ),
        Err(RoomError::Length {
            expected: 3,
            found: 2
        })
    );
}

#[test]
fn asymmetric_maps_have_no_spectrum() {
    let room = coboundary(&triangle()).from().clone();
    let mut rows = vec![vec![Z::from(0); 3]; 3];
    rows[0][2] = Z::from(1);
    let a = Map::from_rows(&room, &room, &rows).unwrap();
    assert_eq!(
        a.symmetric().map(|_| ()),
        Err(RoomError::Asymmetric { row: 0, col: 2 })
    );
}

#[test]
fn exact_arithmetic_never_wraps() {
    let room = coboundary(&triangle()).from().clone();
    let filled = |z: BigInt| Map::from_rows(&room, &room, &vec![vec![Z::from(z); 3]; 3]).unwrap();
    let max = BigInt::from(i64::MAX);
    let big = filled(max.clone());
    // Where i64 would wrap, the exact answer: 3·MAX² and 2·MAX.
    assert_eq!(big.then(&big), Ok(filled(3 * &max * &max)));
    assert_eq!(big.plus(&big), Ok(filled(2 * &max)));
    let min = filled(BigInt::from(i64::MIN));
    assert_eq!(min.adjoint().then(&Map::identity(&room)), Ok(min.clone()));
    // −i64::MIN = 2⁶³, one past i64::MAX.
    assert_eq!(min.negate(), filled(max + 1));
    assert_eq!(min.negate().negate(), min);
}

/// `map` composed with itself `k` times (k ≥ 1).
fn power<K: prismqueer::room::Kind>(map: &Map<K, K, Z>, k: usize) -> Map<K, K, Z> {
    (1..k).fold(map.clone(), |p, i| {
        p.then(map)
            .unwrap_or_else(|e| panic!("power {}: {e:?}", i + 1))
    })
}

/// Three times `map`, by addition.
fn thrice<K: prismqueer::room::Kind>(map: &Map<K, K, Z>) -> Map<K, K, Z> {
    map.plus(map).and_then(|m| m.plus(map)).expect("one room")
}

/// δ-built maps compose past i64. On the triangle L₀ = 3I − J, so L₀² = 3L₀
/// and L₀ᵏ = 3ᵏ⁻¹L₀: at k = 50 its entries are −3⁴⁹ and 2·3⁴⁹, about 10²³.
/// D's minimal polynomial there is x(x² − 3), so D³ = 3D and D⁸¹ = 3⁴⁰D, with
/// entries ±3⁴⁰ > 2⁶³. The laws checked are the closed forms, exactly.
#[test]
fn delta_built_maps_compose_past_i64() {
    let delta = coboundary(&triangle());
    let l0 = delta.then(&delta.adjoint()).expect("C⁰ → C¹ → C⁰");
    let l50 = power(&l0, 50);
    assert_eq!(l50.then(&l0), Ok(thrice(&l50)), "L₀⁵¹ = 3·L₀⁵⁰");
    let three49 = BigInt::from(3).pow(49);
    assert_eq!(l50.entry(0, 0), Some(&Z::from(2 * &three49)), "2·3⁴⁹");
    assert_eq!(l50.entry(0, 1), Some(&Z::from(-three49)), "−3⁴⁹");

    let d = dirac(&triangle());
    let d81 = power(&d, 81);
    let three40 = BigInt::from(3).pow(40);
    let n = d.from().dim();
    for (i, j) in (0..n).flat_map(|i| (0..n).map(move |j| (i, j))) {
        let expected = Z::from(d.entry(i, j).unwrap().value() * &three40);
        assert_eq!(d81.entry(i, j), Some(&expected), "D⁸¹[{i}, {j}] = 3⁴⁰·D");
    }
    assert_eq!(power(&d, 83), thrice(&d81), "D⁸³ = 3·D⁸¹");
}

#[test]
fn identity_and_injections_are_exact_and_lawful() {
    let g = square_with_diagonal();
    let d = dirac(&g);
    let room: Room<Sum<Vertices, _>> = d.from().clone();
    assert_eq!(room.dim(), room.left().dim() + room.right().dim());
    let id = Map::identity(&room);
    assert_eq!(d.then(&id), Ok(d.clone()));
    assert_eq!(id.then(&d), Ok(d.clone()));
    // project ∘ inject is the identity on each summand.
    let left = Map::<Vertices, Sum<Vertices, _>, Z>::inject_left(&room)
        .then(&Map::project_left(&room))
        .unwrap();
    assert_eq!(left, Map::identity(room.left()));
    let right = Map::<Edges, _, Z>::inject_right(&room)
        .then(&Map::project_right(&room))
        .unwrap();
    assert_eq!(right, Map::identity(room.right()));
    // D is odd: zero on the diagonal blocks.
    let vv = Map::inject_left(&room)
        .then(&d)
        .and_then(|m| m.then(&Map::project_left(&room)))
        .unwrap();
    assert_eq!(vv, Map::zero(room.left(), room.left()));
    // D = D*: exact self-adjointness.
    assert_eq!(d.adjoint(), d);
    assert_eq!(d.plus(&d.negate()), Ok(Map::zero(&room, &room)));
}

#[test]
fn an_empty_room_decomposes_to_nothing() {
    let g = graph_from_edges(&[]);
    let modes = dirac(&g)
        .measure()
        .symmetric()
        .unwrap()
        .decompose()
        .unwrap();
    assert!(modes.spectrum().is_empty());
    assert_eq!(modes.vectors().count(), 0);
    assert_eq!(modes.room().dim(), 0);
}

/// A spectrum without its modes is dsyev('N')'s, not dsyev('V')'s: the two
/// run different tridiagonal solvers and can differ in the last bits (the
/// triangle's L₀ does, at λ₀). Bit for bit, it's `ffi::eigenvalues` of the hand-checked
/// Laplacian, on every graph here, a self-loop and an empty room included.
#[test]
fn a_spectrum_without_modes_is_dsyev_n() {
    let mut all = more_graphs();
    all.push(("loops", graph_from_edges(&[(0, 1), (1, 1), (1, 2), (2, 2)])));
    all.push(("empty", graph_from_edges(&[])));
    for (name, g) in all {
        let delta = coboundary(&g);
        let l0 = delta.then(&delta.adjoint()).expect("C⁰ → C¹ → C⁰");
        let spectrum = l0
            .measure()
            .symmetric()
            .expect("δ*δ is symmetric")
            .spectrum()
            .expect("measured");

        let index = |x: u64| g.vertices().position(|v| v == x).unwrap();
        let edges: Vec<(usize, usize)> = g.edges().map(|(u, v)| (index(u), index(v))).collect();
        let n = g.vertex_count();
        let known = prismqueer::ffi::eigenvalues(n, &laplacian_from_edges(n, &edges))
            .expect("dsyev 'N' converges");
        let bits = |xs: &[f64]| xs.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&spectrum), bits(&known), "{name}");
    }
}

#[test]
fn z_is_an_exact_group_in_field_terms() {
    use prismqueer::field::{Exact, Group};
    fn exact<G: Exact>(g: G) -> G {
        g
    }
    let two = exact(Z::from(2));
    assert_eq!(Z::identity(), Z::from(0));
    assert_eq!(two.compose(&Z::from(3)), Z::from(5));
    assert!(two.compose(&two.inverse()).is_identity());
}

// --- Law 6: the exact characteristic polynomial ------------------------------

fn ints(coefficients: &[i64]) -> Vec<Z> {
    coefficients.iter().map(|&c| Z::from(c)).collect()
}

/// P₃, the path on three vertices.
fn p3() -> Graph {
    graph_from_edges(&[(0, 1), (1, 2)])
}

/// Graphs for the laws: cycles, trees, a complete graph, a disconnected one.
fn more_graphs() -> Vec<(&'static str, Graph)> {
    let mut all: Vec<_> = graphs().into_iter().collect();
    all.push(("P₃", p3()));
    all.push((
        "K₅",
        graph_from_edges(&[
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (1, 2),
            (1, 3),
            (1, 4),
            (2, 3),
            (2, 4),
            (3, 4),
        ]),
    ));
    all.push(("star", graph_from_edges(&[(0, 1), (0, 2), (0, 3), (0, 4)])));
    all.push((
        "triangle and an edge",
        graph_from_edges(&[(0, 1), (1, 2), (0, 2), (5, 6)]),
    ));
    all
}

/// L₀ = δ*δ on C⁰ and L₁↓ = δδ* on C¹, exactly.
fn laplacians(g: &Graph) -> (Map<Vertices, Vertices, Z>, Map<Edges, Edges, Z>) {
    let delta = coboundary(g);
    let star = delta.adjoint();
    (
        delta.then(&star).expect("C⁰ → C¹ → C⁰"),
        star.then(&delta).expect("C¹ → C⁰ → C¹"),
    )
}

#[test]
fn the_charpoly_of_p3s_laplacian_is_x3_minus_4x2_plus_3x() {
    let (l0, _) = laplacians(&p3());
    let p = l0.symmetric().expect("L₀ is symmetric").charpoly();
    // Eigenvalues 0, 1, 3: x(x − 1)(x − 3).
    assert_eq!(p.coefficients(), ints(&[1, -4, 3, 0]).as_slice());
    assert_eq!(p.degree(), 3);
}

#[test]
fn the_charpoly_of_the_triangles_laplacian_is_x3_minus_6x2_plus_9x() {
    let (l0, _) = laplacians(&triangle());
    let p = l0.symmetric().expect("L₀ is symmetric").charpoly();
    // Eigenvalues 0, 3, 3: x(x − 3)².
    assert_eq!(p.coefficients(), ints(&[1, -6, 9, 0]).as_slice());
}

/// A symmetric map that isn't a graph's, with distinct entries everywhere on
/// its band. Tridiagonal, so det(xI − M) follows the three-term recurrence
/// pₖ = (x − aₖ)pₖ₋₁ − bₖ²pₖ₋₂, worked by hand: x⁴ − 16x³ + 86x² − 181x + 121.
#[test]
fn the_charpoly_of_a_tridiagonal_map_matches_its_recurrence() {
    let room = coboundary(&path()).from().clone();
    let rows = [
        ints(&[2, 1, 0, 0]),
        ints(&[1, 3, 1, 0]),
        ints(&[0, 1, 4, 1]),
        ints(&[0, 0, 1, 7]),
    ];
    let m = Map::from_rows(&room, &room, &rows).expect("4×4 on C⁰");
    let p = m.symmetric().expect("symmetric").charpoly();
    assert_eq!(p.coefficients(), ints(&[1, -16, 86, -181, 121]).as_slice());
}

/// K₂₀'s Laplacian is 20I − J: eigenvalues 0 and 20 (19 times), so its
/// charpoly is x(x − 20)¹⁹. The middle coefficients, C(19, k)·20ᵏ, pass 2⁶³
/// (20¹⁹ alone is about 5·10²⁴): the charpoly is a bigint computation.
#[test]
fn the_charpoly_of_k20_needs_bigints() {
    let n = 20u64;
    let edges: Vec<(u64, u64)> = (0..n)
        .flat_map(|u| (u + 1..n).map(move |v| (u, v)))
        .collect();
    let (l0, _) = laplacians(&graph_from_edges(&edges));
    let p = l0.symmetric().expect("symmetric").charpoly();
    // x(x − 20)¹⁹ = Σₖ C(19, k)(−20)ᵏ x²⁰⁻ᵏ, then the constant 0.
    let mut expected = Vec::new();
    let mut binomial = BigInt::from(1);
    for k in 0..=19u32 {
        expected.push(Z::from(&binomial * BigInt::from(-20).pow(k)));
        binomial = binomial * (19 - k) / (k + 1);
    }
    expected.push(Z::from(0));
    assert_eq!(p.coefficients(), expected.as_slice());
    assert!(
        p.coefficients()
            .iter()
            .any(|c| i64::try_from(c.value()).is_err()),
        "some coefficient is past i64"
    );
}

#[test]
fn an_empty_room_has_charpoly_one() {
    let g = graph_from_edges(&[]);
    let p = dirac(&g).symmetric().expect("symmetric").charpoly();
    assert_eq!(p.coefficients(), ints(&[1]).as_slice());
    assert_eq!(p.degree(), 0);
}

/// The product of two polynomials, coefficients leading first.
fn times(a: &[Z], b: &[Z]) -> Vec<BigInt> {
    let mut out = vec![BigInt::from(0); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x.value() * y.value();
        }
    }
    out
}

/// p(x) ↦ p(x²), coefficients leading first.
fn in_squares(p: &[Z]) -> Vec<Z> {
    let mut out = Vec::new();
    for (i, c) in p.iter().enumerate() {
        if i > 0 {
            out.push(Z::from(0));
        }
        out.push(c.clone());
    }
    out
}

/// D's charpoly, derived from D² = L₀ ⊕ L₁↓. The grading γ anticommutes with
/// D, so D's spectrum is symmetric about 0 and p_D(−x) = (−1)ⁿp_D(x). Then
/// p_D(x)² = (−1)ⁿp_D(x)p_D(−x) = det(x²I − D²) = p_L₀(x²)·p_L₁↓(x²).
///
/// On the triangle L₀ and L₁↓ both have x³ − 6x² + 9x, so p_D(x)² =
/// (x⁶ − 6x⁴ + 9x²)² and p_D = x⁶ − 6x⁴ + 9x² (monic, and its square root
/// among monic polynomials is unique): eigenvalues 0, 0, ±√3, ±√3.
#[test]
fn the_charpoly_of_dirac_follows_from_its_square() {
    let p = dirac(&triangle()).symmetric().expect("D = D*").charpoly();
    assert_eq!(p.coefficients(), ints(&[1, 0, -6, 0, 9, 0, 0]).as_slice());

    for (name, g) in more_graphs() {
        let (l0, l1_down) = laplacians(&g);
        let pd = dirac(&g).symmetric().expect("D = D*").charpoly();
        let p0 = l0.symmetric().expect("L₀").charpoly();
        let p1 = l1_down.symmetric().expect("L₁↓").charpoly();
        assert_eq!(
            times(pd.coefficients(), pd.coefficients()),
            times(
                &in_squares(p0.coefficients()),
                &in_squares(p1.coefficients())
            ),
            "{name}: p_D(x)² = p_L₀(x²)·p_L₁↓(x²)"
        );
    }
}

// --- Law 7: the measured spectrum against the exact charpoly ----------------

/// `x` as `m · 2ᵉ`, exactly.
fn dyadic(x: f64) -> (BigInt, i32) {
    let bits = x.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1 << 52) - 1);
    let mantissa = if exponent == 0 {
        fraction << 1
    } else {
        fraction | (1 << 52)
    };
    let m = BigInt::from(mantissa);
    (if bits >> 63 == 1 { -m } else { m }, exponent - 1075)
}

/// `num / den` as a float, for any size of either.
fn quotient(num: &BigInt, den: &BigInt) -> f64 {
    let shift = |b: &BigInt| b.bits().saturating_sub(64);
    let (sn, sd) = (shift(num), shift(den));
    let n = (num >> sn).to_f64().expect("a float");
    let d = (den >> sd).to_f64().expect("a float");
    n / d * 2f64.powi(sn as i32 - sd as i32)
}

/// How far a measured eigenvalue `x` is from being a root of `p`, read in
/// eigenvalue units: |p(x)| / S, where S = Σₖ k·|cₖ|·Rᵏ⁻¹ bounds |p′| on
/// [−R, R]. p(x) is evaluated exactly: `x` is a dyadic rational.
fn root_residual(p: &[Z], x: f64, radius: &BigInt) -> f64 {
    let n = p.len() - 1;
    let (m, e) = dyadic(x);
    // x = m / 2ˢ with s ≥ 0; then 2ˢⁿ·p(x) = Σᵢ cᵢ mⁿ⁻ⁱ 2ˢⁱ, an integer.
    let (m, s) = if e >= 0 {
        (m << e as u32, 0u32)
    } else {
        (m, (-e) as u32)
    };
    let mut scaled = BigInt::from(0);
    for (i, c) in p.iter().enumerate() {
        scaled += (c.value() * m.pow((n - i) as u32)) << (s as usize * i);
    }
    let mut slope = BigInt::from(0);
    for (i, c) in p.iter().enumerate() {
        let k = n - i;
        if k > 0 {
            slope += BigInt::from(k) * c.value().abs() * radius.pow(k as u32 - 1);
        }
    }
    quotient(&scaled.abs(), &(slope << (s as usize * n)))
}

/// Every measured eigenvalue of `map`, put back into its exact charpoly,
/// reads `Zero` in `settle`'s band; and so does nothing shifted by `nudge`
/// when `nudge` is given.
fn spectrum_certifies<K: prismqueer::room::Kind>(
    name: &str,
    map: &Map<K, K, Z>,
    nudge: Option<f64>,
) {
    let p = map.clone().symmetric().expect("symmetric").charpoly();
    let modes = map
        .measure()
        .symmetric()
        .expect("symmetric")
        .decompose()
        .expect("measured");
    let n = modes.spectrum().len();
    let scale = modes.spectrum().iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    // R bounds every true and measured eigenvalue: ρ̂ plus at least 1, far
    // more than dsyev's error.
    let radius = BigInt::from(scale.ceil() as u64 + 1);
    for &value in modes.spectrum() {
        let r = root_residual(p.coefficients(), value, &radius);
        assert_eq!(
            zeros::settle(n).read(r, scale),
            Verdict::Zero,
            "{name}: p({value}) residual {r:e}"
        );
        if let Some(delta) = nudge {
            let off = value + delta * scale.max(1.0);
            let r = root_residual(p.coefficients(), off, &radius);
            assert_eq!(
                zeros::settle(n).read(r, scale),
                Verdict::NonZero,
                "{name}: {off} is not a root (residual {r:e})"
            );
        }
    }
}

/// The cross-check: `Modes` (measured, dsyev) against the exact charpoly.
///
/// The residual is |p(λ̂)| / S, read through `zeros::settle(n)`, the same
/// declared band (16·ε·n·max(ρ, 1)) that certifies kernels. Why it's honest:
/// - p(λ̂) is exact: λ̂ is a dyadic rational, so no evaluation error enters.
/// - By the mean value theorem |p(λ̂)| = |p(λ̂) − p(λ)| ≤ S·|λ̂ − λ| for the
///   true eigenvalue λ nearest λ̂, since S bounds |p′| on [−R, R] and both
///   lie inside. So the residual is at most |λ̂ − λ|.
/// - dsyev is backward stable: λ̂ are the exact eigenvalues of M + E with
///   ‖E‖₂ ≤ c·n·ε·‖M‖₂, and by Weyl |λ̂ − λ| ≤ ‖E‖₂, whatever the
///   multiplicities. `settle` declares c = 16, ‖M‖₂ = ρ for symmetric M.
///
/// The bound is one-sided (S overestimates |p′|), so a negative control checks
/// the band still sees a miss: on graphs with simple spectra, a shift of
/// 10⁻⁹·ρ reads `NonZero`.
#[test]
fn the_measured_spectrum_roots_the_exact_charpoly() {
    for (name, g) in more_graphs() {
        let (l0, l1_down) = laplacians(&g);
        spectrum_certifies(&format!("{name} L₀"), &l0, None);
        spectrum_certifies(&format!("{name} L₁↓"), &l1_down, None);
        spectrum_certifies(&format!("{name} D"), &dirac(&g), None);
    }
    // Simple spectra: P₃'s L₀ (0, 1, 3) and the path's D (0, ±√λ for
    // λ = 2 − √2, 2, 2 + √2).
    spectrum_certifies("P₃ L₀, nudged", &laplacians(&p3()).0, Some(1e-9));
    spectrum_certifies("path D, nudged", &dirac(&path()), Some(1e-9));
}

// --- Law 8: the triple is even ----------------------------------------------

/// γ = +1 on C⁰, −1 on C¹: a grading (γ² = 1, γ = γ*) that anticommutes with
/// D exactly, because D only moves between the rooms.
#[test]
fn the_grading_anticommutes_with_dirac() {
    for (name, g) in more_graphs() {
        let d = dirac(&g);
        let room = d.from().clone();
        let gamma = Map::<Sum<Vertices, Edges>, _, Z>::grading(&room);
        let n0 = room.left().dim();
        for i in 0..room.dim() {
            for j in 0..room.dim() {
                let expected = match (i == j, i < n0) {
                    (false, _) => Z::from(0),
                    (true, true) => Z::from(1),
                    (true, false) => Z::from(-1),
                };
                assert_eq!(gamma.entry(i, j), Some(&expected), "{name}: γ[{i}, {j}]");
            }
        }
        assert_eq!(
            gamma.then(&gamma),
            Ok(Map::identity(&room)),
            "{name}: γ² = 1"
        );
        assert_eq!(gamma.adjoint(), gamma, "{name}: γ = γ*");
        let gd = gamma.then(&d).expect("one room");
        let dg = d.then(&gamma).expect("one room");
        assert_eq!(gd, dg.negate(), "{name}: γD = −Dγ");
        assert_ne!(gd, dg, "{name}: and not γD = Dγ, unless D = 0");
    }
}

// --- Law 9: identity is per skeleton -----------------------------------------
//
// A room's identity is per skeleton level, not the whole complex: C⁰ is its
// vertex set alone, and C¹ sits over a C⁰ by relation. Rewiring the edges
// changes the complex over a fixed 0-skeleton, not the vertex space.

fn set(vertices: &[u64]) -> BTreeSet<u64> {
    vertices.iter().copied().collect()
}

/// The exact map with these rows, from `from` to `to`.
fn exact<A: prismqueer::room::Kind, B: prismqueer::room::Kind>(
    from: &Room<A>,
    to: &Room<B>,
    rows: &[&[i64]],
) -> Map<A, B, Z> {
    let rows: Vec<Vec<Z>> = rows.iter().map(|row| ints(row)).collect();
    Map::from_rows(from, to, &rows).expect("rows fit the rooms")
}

/// L₀ = δ*δ of `g`, out of C⁰(V).
fn laplacian_over(v: &Room<Vertices>, g: &Graph) -> Map<Vertices, Vertices, Z> {
    let delta = coboundary_over(v, g).expect("V(g) ⊆ V");
    delta.then(&delta.adjoint()).expect("C⁰(V) → C¹ → C⁰(V)")
}

#[test]
fn a_vertex_room_is_its_vertex_set() {
    let v = set(&[0, 1, 2]);
    assert_eq!(
        Room::vertices(&triangle()),
        Room::vertices(&p3()),
        "two graphs on one vertex set share one C⁰"
    );
    assert_eq!(Room::vertices(&triangle()), Room::over(&v));
    assert_eq!(Room::over(&v).dim(), 3);
    assert_ne!(
        Room::over(&v),
        Room::over(&set(&[0, 1, 3])),
        "same dimension, different vertex set"
    );
    assert_ne!(Room::over(&v), Room::over(&set(&[])));
}

#[test]
fn an_edge_room_sits_over_its_vertex_room() {
    // Three edges each over {0, 1, 2}: same V, same dimension, other wiring.
    let looped = graph_from_edges(&[(0, 1), (1, 2), (2, 2)]);
    assert_eq!(
        Room::edges(&triangle()),
        Room::edges(&triangle()),
        "one graph, one C¹"
    );
    assert_ne!(Room::edges(&triangle()), Room::edges(&looped));
    assert_ne!(Room::edges(&triangle()), Room::edges(&p3()));
    let c0 = Room::vertices(&triangle());
    for g in [triangle(), looped, p3()] {
        assert_eq!(Room::edges(&g).skeleton(), c0.complex(), "C¹ sits over C⁰");
        assert_eq!(coboundary(&g).from(), &c0, "one C⁰ under every wiring");
    }
    assert!(
        Room::sum(&c0, &Room::edges(&p3())).is_ok(),
        "rewiring keeps the 0-skeleton"
    );
}

#[test]
fn a_coboundary_out_of_a_larger_vertex_room_has_zero_columns() {
    let v = Room::over(&set(&[0, 1, 2, 7]));
    let delta = coboundary_over(&v, &p3()).expect("{0, 1, 2} ⊆ V");
    assert_eq!(delta.from(), &v);
    assert_eq!(delta.to().skeleton(), v.complex());
    assert_eq!(
        delta,
        exact(&v, delta.to(), &[&[-1, 1, 0, 0], &[0, -1, 1, 0]]),
        "vertex 7 is outside the graph: its column is zero"
    );
    assert_eq!(
        coboundary_over(&Room::vertices(&p3()), &p3()),
        Ok(coboundary(&p3())),
        "over its own vertices, δ is the graph's own"
    );
}

/// L_tri = 3I − J kills constants and L_P₃ has zero column sums, so
/// L_tri · L_P₃ = 3 L_P₃, exactly.
#[test]
fn laplacians_over_one_vertex_set_compose_exactly() {
    let (l_tri, _) = laplacians(&triangle());
    let (l_p3, _) = laplacians(&p3());
    let c0 = Room::vertices(&p3());
    // Path order: L_P₃ first, then L_tri, is the matrix L_tri · L_P₃.
    let product = l_p3.then(&l_tri).expect("one C⁰");
    assert_eq!(
        product,
        exact(&c0, &c0, &[&[3, -3, 0], &[-3, 6, -3], &[0, -3, 3]])
    );
}

/// Over V = {0, 1, 2}: L_a = [[1, −1, 0], [−1, 1, 0], [0, 0, 0]] for the
/// edge (0, 1) and L_b = [[0, 0, 0], [0, 1, −1], [0, −1, 1]] for (1, 2).
/// L_a L_b = [[0, −1, 1], [0, 1, −1], [0, 0, 0]] and L_b L_a is its
/// transpose, so [L_a, L_b] = [[0, −1, 1], [1, 0, −1], [−1, 1, 0]].
#[test]
fn the_commutator_over_a_union_of_vertex_sets_is_exact() {
    let v = Room::over(&set(&[0, 1, 2]));
    let l_a = laplacian_over(&v, &graph_from_edges(&[(0, 1)]));
    let l_b = laplacian_over(&v, &graph_from_edges(&[(1, 2)]));
    let ab = l_b.then(&l_a).expect("one C⁰(V)");
    let ba = l_a.then(&l_b).expect("one C⁰(V)");
    let commutator = ab.plus(&ba.negate()).expect("one C⁰(V)");
    assert_eq!(
        commutator,
        exact(&v, &v, &[&[0, -1, 1], &[1, 0, -1], &[-1, 1, 0]])
    );
    assert_eq!(commutator.adjoint(), commutator.negate(), "antisymmetric");
}

#[test]
fn composing_across_vertex_sets_is_still_an_error() {
    let (l_a, _) = laplacians(&graph_from_edges(&[(0, 1)]));
    let (l_b, _) = laplacians(&graph_from_edges(&[(1, 2)]));
    match l_b.then(&l_a) {
        Err(RoomError::Mismatch { expected, found }) => {
            assert_eq!((expected.dim, found.dim), (2, 2));
            assert_ne!(expected.complex, found.complex);
        }
        other => panic!("expected a mismatch, got {other:?}"),
    }
    assert!(
        matches!(
            coboundary_over(&Room::over(&set(&[0, 1])), &p3()),
            Err(RoomError::Mismatch { .. })
        ),
        "P₃'s vertex 2 isn't in V"
    );
    let wider = coboundary_over(&Room::over(&set(&[0, 1, 2, 7])), &p3()).expect("⊆");
    assert!(
        matches!(
            Room::sum(&Room::vertices(&p3()), wider.to()),
            Err(RoomError::Mismatch { .. })
        ),
        "an edge room over another 0-skeleton"
    );
}

/// `laplacian(g)` is δ*δ composed by hand (law 2 holds that one to the
/// independent flat build), already checked symmetric, and measures to it.
#[test]
fn the_laplacian_is_delta_star_delta_symmetric_by_construction() {
    for (name, g) in more_graphs() {
        let (l0, _) = laplacians(&g);
        let measured = l0.measure().symmetric().expect("δ*δ is symmetric");
        let l0 = l0.symmetric().expect("δ*δ is symmetric");
        assert_eq!(laplacian(&g), l0, "{name}: L₀ = δ*δ, exactly");
        assert_eq!(laplacian(&g).measure(), measured, "{name}: measured");
    }
}

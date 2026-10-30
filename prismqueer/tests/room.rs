//! Rooms: the spaces operators move between.
//!
//! C⁰ (the vertex room) and C¹ (the edge room) of one graph; D lives in
//! C⁰ ⊕ C¹; δ is a door from C⁰ into C¹ and δ* the door back. The laws:
//!
//! 1. δ is exact: entries ±1 or 0, one +1 and one −1 per edge row.
//! 2. D² = L₀ ⊕ L₁ exactly, and L₀ is the Laplacian prism already builds.
//! 3. An eigenbasis is read by columns, and every column is an eigenvector.
//! 4. A room-built D has `Instrument`'s spectrum.
//!
//! (Law 5, wrong-kind composition is a compile error, is a doctest on
//! `prismqueer::room`.)
//!
//! Expected values come from outside `room`: the graph's own edge list, a
//! hand-checked Laplacian (`laplacian_from_edges`) and `Instrument`.

#![cfg(feature = "lapack")]

use num_bigint::BigInt;
use prismqueer::band::{zeros, Verdict};
use prismqueer::room::{coboundary, dirac, Edges, Map, Room, RoomError, Sum, Vertices, Z};
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

// --- Law 2: D² = L₀ ⊕ L₁, exactly ------------------------------------------

#[test]
fn dirac_squared_is_the_laplacians() {
    for (name, g) in graphs() {
        let delta = coboundary(&g);
        let star = delta.adjoint();
        let l0 = delta.then(&star).expect("δ then δ*: C⁰ → C⁰");
        let l1 = star.then(&delta).expect("δ* then δ: C¹ → C¹");

        let d = dirac(&g);
        let d2 = d.then(&d).expect("D then D");
        let blocks = Map::diagonal(&l0, &l1).expect("one complex");
        assert_eq!(d2, blocks, "{name}: D² = L₀ ⊕ L₁");

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

// --- Law 4: the same spectrum as Instrument ---------------------------------

#[test]
fn dirac_spectrum_matches_instrument() {
    for (name, g) in graphs() {
        let modes = dirac(&g)
            .measure()
            .symmetric()
            .expect("D is self-adjoint")
            .decompose()
            .expect("measured");
        let instrument = Instrument::of(&g).expect("measured");
        // Same matrix, same basis order, same dsyev: the same numbers.
        assert_eq!(modes.spectrum(), instrument.eigenvalues(), "{name}");
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

//! Faces on `room`: a C² room, δ₁: C¹ → C², and torsion (prism-011, step 4).
//!
//! A graph's faces (field's filled triangles and bigons, [`Face`]) are a third
//! room, C², one dimension per face. Its identity sits over C¹ the way C¹ sits
//! over C⁰. δ₁ is the door into it; a face's row is its boundary ∂f as a chain,
//! so ∂₂ = δ₁*. The laws:
//!
//! 1. **δ₁ ∘ δ₀ = 0, exactly**, over ℤ and over ℤ/n: every face is a closed
//!    walk, so a potential's differences cancel around it.
//! 2. **C² sits over C¹.** Its `one_skeleton` is C¹'s address, its `skeleton`
//!    C⁰'s; refilling the faces changes C² over a fixed C¹.

#![cfg(feature = "lapack")]

use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use prismqueer::field::{Edge, Face, Graph};
use prismqueer::room::{
    coboundary, coboundary1, coboundary1_in, coboundary_in, Kind, Map, Modulus, Room, RoomError,
    Vertices, ZMod, Z,
};

fn modulus(n: i64) -> Modulus {
    Modulus::new(n).expect("n ≥ 2")
}

fn ints(xs: &[i64]) -> Vec<Z> {
    xs.iter().map(|&x| Z::from(x)).collect()
}

fn triangles(g: &Graph, corners: &[[u64; 3]]) -> Vec<Face> {
    corners
        .iter()
        .map(|&c| Face::triangle(g, c).expect("a triangle of g"))
        .collect()
}

/// One triangle, filled.
fn triangle() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (1, 2), (0, 2)]);
    let faces = triangles(&g, &[[0, 1, 2]]);
    (g, faces)
}

/// A square with diagonal 0–2, both triangles filled: a disk.
fn square() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (1, 2), (2, 3), (0, 3), (0, 2)]);
    let faces = triangles(&g, &[[0, 1, 2], [0, 2, 3]]);
    (g, faces)
}

/// Two parallel edges on 0–1, the bigon between them filled.
fn bigon() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (0, 1)]);
    let face = Face::bigon(&g, [Edge::between(0, 1, 0), Edge::between(0, 1, 1)]).unwrap();
    (g, vec![face])
}

fn fixtures() -> Vec<(&'static str, Graph, Vec<Face>)> {
    let (t, tf) = triangle();
    let (s, sf) = square();
    let (b, bf) = bigon();
    vec![("triangle", t, tf), ("square", s, sf), ("bigon", b, bf)]
}

// --- Law 1: δ₁ ∘ δ₀ = 0 ------------------------------------------------------

#[test]
fn a_face_row_is_its_boundary_in_canonical_orientation() {
    // Edges in canonical order: (0,1), (0,2), (1,2). The triangle walks
    // 0 → 1 → 2 → 0: with (0,1), with (1,2), against (0,2).
    let (g, faces) = triangle();
    assert_eq!(faces[0].vertices(), &[0, 1, 2, 0]);
    let d1 = coboundary1(&g, &faces).unwrap();
    assert_eq!(d1.to().dim(), 1);
    assert_eq!(d1.from().dim(), 3);
    let row: Vec<Z> = (0..3).map(|e| d1.entry(0, e).unwrap().clone()).collect();
    assert_eq!(row, ints(&[1, -1, 1]));

    // The bigon: out along the first edge, back along the second.
    let (g, faces) = bigon();
    let d1 = coboundary1(&g, &faces).unwrap();
    assert_eq!(faces[0].vertices(), &[0, 1, 0]);
    assert_eq!(d1.entry(0, 0), Some(&Z::from(1)));
    assert_eq!(d1.entry(0, 1), Some(&Z::from(-1)));
}

#[test]
fn delta_one_after_delta_is_zero_over_the_integers() {
    for (name, g, faces) in fixtures() {
        let d0 = coboundary(&g);
        let d1 = coboundary1(&g, &faces).unwrap();
        let both = d0.then(&d1).expect("δ₁ leaves the room δ enters");
        assert_eq!(both, Map::zero(d0.from(), d1.to()), "{name}: δ₁δ = 0");
        // And dually, ∂₁∂₂ = 0: every face boundary is a cycle.
        assert_eq!(
            d1.adjoint().then(&d0.adjoint()).unwrap(),
            Map::zero(d1.to(), d0.from()),
            "{name}: ∂₁∂₂ = 0"
        );
    }
}

#[test]
fn delta_one_after_delta_is_zero_over_every_zmod() {
    for n in [2, 3, 12] {
        let n = modulus(n);
        for (name, g, faces) in fixtures() {
            let d0 = coboundary_in::<ZMod>(&n, &g);
            let d1 = coboundary1_in::<ZMod>(&n, &g, &faces).unwrap();
            assert_eq!(
                d0.then(&d1).unwrap(),
                Map::zero_in(&n, d0.from(), d1.to()),
                "{name}: δ₁δ = 0 over ℤ/{}",
                n.value()
            );
            // δ₁ over ℤ/n is δ₁ over ℤ, reduced.
            assert_eq!(d1, coboundary1(&g, &faces).unwrap().reduce(&n), "{name}");
        }
    }
}

// --- Law 2: C² sits over C¹ ----------------------------------------------------

#[test]
fn the_face_room_sits_over_the_edge_room() {
    let (g, faces) = square();
    let one = Room::faces(&g, &faces[..1]).unwrap();
    let two = Room::faces(&g, &faces).unwrap();
    for room in [&one, &two] {
        assert_eq!(room.one_skeleton(), Room::edges(&g).complex());
        assert_eq!(room.skeleton(), Room::vertices(&g).complex());
    }
    assert_eq!((one.dim(), two.dim()), (1, 2));
    assert_ne!(one.complex(), two.complex(), "refilled: another C²");
    assert_eq!(&two, coboundary1(&g, &faces).unwrap().to());
    assert_eq!(
        coboundary1(&g, &faces).unwrap().from(),
        coboundary(&g).to(),
        "δ₁ leaves δ's C¹"
    );
    // Orientation is canonical: the bigon's edges named in either order are
    // one face, so one C².
    let (b, bf) = bigon();
    let swapped = Face::bigon(&b, [Edge::between(0, 1, 1), Edge::between(0, 1, 0)]).unwrap();
    assert_eq!(bf, vec![swapped.clone()]);
    assert_eq!(Room::faces(&b, &bf).unwrap(), Room::faces(&b, &[swapped]).unwrap());
}

#[test]
fn a_face_off_the_graph_is_refused() {
    let (_, faces) = triangle();
    let path = Graph::new([], [(0, 1), (1, 2)]);
    let refused = Err(RoomError::Outside(Edge::between(0, 2, 0)));
    assert_eq!(Room::faces(&path, &faces), refused);
    assert_eq!(coboundary1(&path, &faces).map(|_| ()), refused.map(|_: Room<_>| ()));
}

// --- Law 3: Smith normal form, exactly -----------------------------------------

/// The integer matrix `rows` as a map between vertex rooms of its sizes.
fn matrix(rows: &[&[i64]], cols: usize) -> Map<Vertices, Vertices, Z> {
    let from = Room::over(&(0..cols as u64).collect());
    let to = Room::over(&(0..rows.len() as u64).collect());
    let rows: Vec<Vec<Z>> = rows.iter().map(|r| ints(r)).collect();
    Map::from_rows(&from, &to, &rows).expect("a matrix")
}

/// Smith form of `m`, checked exactly, and its invariant factors returned:
/// d₁ | d₂ | … (zeros last, everything divides 0); U and V unimodular
/// (det ±1, exactly); U·M·V = diag(d), with diag(d) built here from the
/// factors alone.
fn smith_checked<A: Kind, B: Kind>(name: &str, m: &Map<A, B, Z>) -> Vec<Z> {
    let s = m.smith();
    let d = s.factors();
    let (rows, cols) = (m.to().dim(), m.from().dim());
    assert_eq!(d.len(), rows.min(cols), "{name}: min(m, n) factors");
    for pair in d.windows(2) {
        let (a, b) = (pair[0].value(), pair[1].value());
        if a.is_zero() {
            assert!(b.is_zero(), "{name}: zeros last");
        } else {
            assert!((b % a).is_zero(), "{name}: {a} | {b}");
        }
    }
    assert!(d.iter().all(|x| x.value().sign() != Sign::Minus), "{name}: dᵢ ≥ 0");
    assert_eq!(s.rank(), d.iter().filter(|x| !x.value().is_zero()).count());
    let unit = |x: Z| x == Z::from(1) || x == Z::from(-1);
    assert!(unit(s.left().determinant()), "{name}: det U = ±1");
    assert!(unit(s.right().determinant()), "{name}: det V = ±1");
    assert_eq!(s.left().from(), m.to(), "{name}: U acts on M's rows");
    assert_eq!(s.right().from(), m.from(), "{name}: V acts on M's columns");

    let mut diagonal = vec![vec![Z::from(0); cols]; rows];
    for (i, di) in d.iter().enumerate() {
        diagonal[i][i] = di.clone();
    }
    let diagonal = Map::from_rows(m.from(), m.to(), &diagonal).unwrap();
    let umv = s.right().then(m).unwrap().then(s.left()).unwrap();
    assert_eq!(umv, diagonal, "{name}: U·M·V = diag(d), exactly");
    assert_eq!(s.diagonal(), diagonal, "{name}: Smith::diagonal is diag(d)");
    d.to_vec()
}

#[test]
fn smith_form_of_integer_matrices() {
    let two60 = BigInt::from(1i64 << 60);
    let cases: Vec<(&str, Map<Vertices, Vertices, Z>, Vec<Z>)> = vec![
        // A large entry stays exact: diag(2⁶⁰, 2⁶¹ + 2) has gcd 2 (2⁶⁰ + 1
        // is odd) and lcm 2⁶⁰·(2⁶⁰ + 1), past i64.
        (
            "big",
            matrix(&[&[1 << 60, 0], &[0, (1 << 61) + 2]], 2),
            vec![Z::from(2), Z::from(&two60 * (&two60 + 1))],
        ),
    ];
    let small: Vec<(&str, Map<Vertices, Vertices, Z>, Vec<i64>)> = vec![
        // The classic 3 × 3, every factor a non-unit: 2 | 6 | 12.
        (
            "classic",
            matrix(&[&[2, 4, 4], &[-6, 6, 12], &[10, -4, -16]], 3),
            vec![2, 6, 12],
        ),
        // diag(2, 3) isn't Smith: 2 ∤ 3. gcd 1, lcm 6.
        ("coprime", matrix(&[&[2, 0], &[0, 3]], 2), vec![1, 6]),
        // diag(4, 6): gcd 2, lcm 12.
        ("4 and 6", matrix(&[&[4, 0], &[0, 6]], 2), vec![2, 12]),
        ("wide", matrix(&[&[1, 2, 3], &[4, 5, 6]], 3), vec![1, 3]),
        ("tall", matrix(&[&[1, 4], &[2, 5], &[3, 6]], 2), vec![1, 3]),
        ("rank one", matrix(&[&[2, 4], &[3, 6]], 2), vec![1, 0]),
        ("zero", matrix(&[&[0, 0], &[0, 0], &[0, 0]], 2), vec![0, 0]),
        ("a two", matrix(&[&[-2]], 1), vec![2]),
        ("swap", matrix(&[&[0, 2], &[2, 0]], 2), vec![2, 2]),
        ("empty", matrix(&[], 3), vec![]),
    ];
    let cases = small
        .into_iter()
        .map(|(name, m, d)| (name, m, ints(&d)))
        .chain(cases);
    for (name, m, expected) in cases {
        assert_eq!(smith_checked(name, &m), expected, "{name}");
    }
}

#[test]
fn smith_form_of_the_faces_of_a_disk_is_all_units() {
    // ∂₂ of a disk has no torsion: every invariant factor is 1.
    for (name, g, faces) in fixtures() {
        let d2 = coboundary1(&g, &faces).unwrap().adjoint();
        let d = smith_checked(name, &d2);
        assert_eq!(d, ints(&vec![1; faces.len()]), "{name}");
    }
}

#[test]
fn determinant_exactly() {
    assert_eq!(
        matrix(&[&[2, 4, 4], &[-6, 6, 12], &[10, -4, -16]], 3).determinant(),
        Z::from(-144)
    );
    assert_eq!(matrix(&[&[0, 1], &[1, 0]], 2).determinant(), Z::from(-1));
    assert_eq!(matrix(&[&[2, 4], &[3, 6]], 2).determinant(), Z::from(0));
    assert_eq!(matrix(&[&[0, 0, 1], &[0, 1, 0], &[1, 0, 0]], 3).determinant(), Z::from(-1));
    assert_eq!(matrix(&[&[0, 2, 1], &[3, 0, 0], &[1, 1, 1]], 3).determinant(), Z::from(-3));
    assert_eq!(matrix(&[], 0).determinant(), Z::from(1), "the empty product");
}

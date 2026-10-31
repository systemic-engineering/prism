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
use num_traits::{ToPrimitive, Zero};
use prismqueer::field::{check_flat, Edge, Face, Field, Flatness, Graph, Zn};
use prismqueer::room::{
    bounds, classify, coboundary, coboundary1, coboundary1_in, coboundary_in, curvature, pair,
    solve, Boundary, Bounded, Coefficient, Exactly, Exactness, Kind, Map, Modulus, QModOne, Room,
    RoomError, Twist, Vertices, ZMod, Z,
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
    assert_eq!(
        Room::faces(&b, &bf).unwrap(),
        Room::faces(&b, &[swapped]).unwrap()
    );
}

#[test]
fn a_face_off_the_graph_is_refused() {
    let (_, faces) = triangle();
    let path = Graph::new([], [(0, 1), (1, 2)]);
    let refused = Err(RoomError::Outside(Edge::between(0, 2, 0)));
    assert_eq!(Room::faces(&path, &faces), refused);
    assert_eq!(
        coboundary1(&path, &faces).map(|_| ()),
        refused.map(|_: Room<_>| ())
    );
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
    assert!(
        d.iter().all(|x| x.value().sign() != Sign::Minus),
        "{name}: dᵢ ≥ 0"
    );
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
    type Case<D> = (&'static str, Map<Vertices, Vertices, Z>, Vec<D>);
    let two60 = BigInt::from(1i64 << 60);
    let cases: Vec<Case<Z>> = vec![
        // A large entry stays exact: diag(2⁶⁰, 2⁶¹ + 2) has gcd 2 (2⁶⁰ + 1
        // is odd) and lcm 2⁶⁰·(2⁶⁰ + 1), past i64.
        (
            "big",
            matrix(&[&[1 << 60, 0], &[0, (1 << 61) + 2]], 2),
            vec![Z::from(2), Z::from(&two60 * (&two60 + 1))],
        ),
    ];
    let small: Vec<Case<i64>> = vec![
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
    assert_eq!(
        matrix(&[&[0, 0, 1], &[0, 1, 0], &[1, 0, 0]], 3).determinant(),
        Z::from(-1)
    );
    assert_eq!(
        matrix(&[&[0, 2, 1], &[3, 0, 0], &[1, 1, 1]], 3).determinant(),
        Z::from(-3)
    );
    assert_eq!(
        matrix(&[], 0).determinant(),
        Z::from(1),
        "the empty product"
    );
}

// --- Laws 4–6: bounds, curvature and the three kinds -----------------------------

/// Decide whether `z` (an integer chain, read in R by `into`) bounds faces
/// in R, and check the verdict's witness exactly: a 2-chain w with ∂₂w = z
/// in R, or a cocycle c in R's dual (δ₁c = 0) with ⟨z, c⟩ ≠ 0, so z isn't
/// any ∂₂w: ⟨∂₂w, c⟩ = ⟨w, δ₁c⟩ = 0.
fn boundary_checked<R: Bounded>(
    name: &str,
    ring: &R::Ring,
    g: &Graph,
    faces: &[Face],
    z: &[Z],
    into: impl Fn(&Z) -> R,
) -> Boundary<R> {
    let zr: Vec<R> = z.iter().map(into).collect();
    let verdict = bounds(ring, g, faces, &zr).expect("z is a chain of g, in the ring");
    match &verdict {
        Boundary::Bounds(w) => {
            assert_eq!(w.len(), faces.len(), "{name}: w ∈ C₂");
            let d2 = coboundary1_in::<R>(ring, g, faces).unwrap().adjoint();
            assert_eq!(d2.apply(w).unwrap(), zr, "{name}: ∂₂w = z, exactly");
        }
        Boundary::Unbounded(c) => {
            let dual = R::dual(ring);
            assert_eq!(c.len(), z.len(), "{name}: c ∈ C¹");
            let curl = coboundary1(g, faces).unwrap().act(&dual, c).unwrap();
            let zero = <R::Dual as Coefficient>::zero(&dual);
            assert!(curl.iter().all(|x| *x == zero), "{name}: δ₁c = 0");
            assert_ne!(pair(&dual, z, c).unwrap(), zero, "{name}: ⟨z, c⟩ ≠ 0");
        }
    }
    verdict
}

/// Classify the twist c and check each kind's witness exactly.
fn twist_checked<V: Exactly>(
    name: &str,
    ring: &V::Ring,
    g: &Graph,
    faces: &[Face],
    c: &[V],
) -> Twist<V> {
    let zero = V::zero(ring);
    let curl = curvature(ring, g, faces, c).expect("c ∈ C¹");
    assert_eq!(
        curl,
        coboundary1(g, faces).unwrap().act(ring, c).unwrap(),
        "{name}: curvature is δ₁c"
    );
    let verdict = classify(ring, g, faces, c).expect("c ∈ C¹");
    match &verdict {
        Twist::Gradient(potential) => {
            assert_eq!(
                coboundary(g).act(ring, potential).unwrap(),
                c,
                "{name}: δg = c"
            );
        }
        Twist::Harmonic(cycles) => {
            assert!(curl.iter().all(|x| *x == zero), "{name}: closed, δ₁c = 0");
            assert!(!cycles.is_empty(), "{name}: a certificate");
            for cycle in cycles {
                let z = cycle.chain();
                assert_eq!(
                    coboundary(g).adjoint().apply(z).unwrap(),
                    ints(&vec![0; g.vertex_count()]),
                    "{name}: δ*z = 0"
                );
                assert_eq!(&pair(ring, z, c).unwrap(), cycle.pairing(), "{name}");
                assert_ne!(cycle.pairing(), &zero, "{name}: ⟨z, c⟩ ≠ 0");
            }
        }
        Twist::Curvature(curls) => {
            assert!(!curls.is_empty(), "{name}: a curved face");
            let located: Vec<usize> = curls.iter().map(|k| k.index()).collect();
            let nonzero: Vec<usize> = (0..faces.len()).filter(|&f| curl[f] != zero).collect();
            assert_eq!(
                located, nonzero,
                "{name}: exactly the faces with (δ₁c)(f) ≠ 0"
            );
            for k in curls {
                assert_eq!(k.face(), &faces[k.index()], "{name}");
                assert_eq!(k.value(), &curl[k.index()], "{name}: the twist around f");
            }
        }
    }
    verdict
}

fn kind<V>(t: &Twist<V>) -> &'static str {
    match t {
        Twist::Gradient(_) => "gradient",
        Twist::Harmonic(_) => "harmonic",
        Twist::Curvature(_) => "curvature",
    }
}

/// ℝℙ², the 6-vertex minimal triangulation (the hemi-icosahedron: the
/// icosahedron with antipodes identified), on K₆.
///
/// Topology arithmetic, checked in [`rp2_is_a_closed_surface_with_one_twist`]:
/// - V = 6, E = 15 (all of K₆), F = 10; χ = 6 − 15 + 10 = 1 = χ(ℝℙ²).
/// - Every edge lies on exactly two faces: a closed surface.
/// - rank ∂₁ = V − 1 = 5 (connected), so the cycles ker ∂₁ have rank
///   15 − 5 = 10.
/// - ∂₂'s Smith form is 1⁹, 2: rank 10 = F, so H₂(ℤ) = ker ∂₂ = 0 (not
///   orientable), and H₁(ℤ) = ker ∂₁ / im ∂₂ = ℤ¹⁰ / (ℤ⁹ ⊕ 2ℤ) = ℤ/2.
/// - Over ℤ/2 the factor 2 is 0: rank ∂₂ = 9, so H₂(ℤ/2) = ℤ/2 (the
///   fundamental class mod 2) and H₁(ℤ/2) = 10 − 9 = ℤ/2.
/// - Over ℤ/3 the factor 2 is a unit: rank ∂₂ = 10 and H₁(ℤ/3) = 0.
///   b₀ − b₁ + b₂ = 1 − 0 + 0 = 1 = χ again, and over ℤ/2 1 − 1 + 1 = 1.
fn rp2() -> (Graph, Vec<Face>) {
    let corners = [
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 5],
        [0, 5, 1],
        [1, 2, 4],
        [2, 3, 5],
        [3, 4, 1],
        [4, 5, 2],
        [5, 1, 3],
    ];
    let k6 = (0..6u64).flat_map(|a| (a + 1..6).map(move |b| (a, b)));
    let g = Graph::new([], k6);
    let faces = triangles(&g, &corners);
    (g, faces)
}

/// The edge index of (a, b) in g.
fn at(g: &Graph, a: u64, b: u64) -> usize {
    g.edges()
        .position(|e| e == Edge::between(a, b, 0))
        .expect("an edge")
}

/// z = 1 → 2 → 3 → 1, a triangle of K₆ that isn't a face: the loop through
/// the cross-cap. +1 on (1,2) and (2,3), −1 on (1,3).
fn rp2_loop(g: &Graph) -> Vec<Z> {
    let mut z = ints(&[0; 15]);
    z[at(g, 1, 2)] = Z::from(1);
    z[at(g, 2, 3)] = Z::from(1);
    z[at(g, 1, 3)] = Z::from(-1);
    z
}

#[test]
fn rp2_is_a_closed_surface_with_one_twist() {
    let (g, faces) = rp2();
    assert_eq!(
        (g.vertex_count(), g.edges().count(), faces.len()),
        (6, 15, 10)
    );
    let d1 = coboundary1(&g, &faces).unwrap();
    for e in 0..15 {
        let on: usize = (0..10)
            .filter(|&f| !d1.entry(f, e).unwrap().value().is_zero())
            .count();
        assert_eq!(on, 2, "edge {e} lies on two faces");
    }
    assert_eq!(g.cycle_rank(), 10, "rank ker ∂₁ = 15 − 5");
    let smith = smith_checked("ℝℙ²", &d1.adjoint());
    let mut expected = ints(&[1; 9]);
    expected.push(Z::from(2));
    assert_eq!(smith, expected, "H₁(ℝℙ²) = ℤ/2: one invariant factor 2");
    assert_eq!(d1.adjoint().smith().rank(), 10, "H₂(ℝℙ²; ℤ) = 0");
    assert_eq!(
        coboundary(&g).then(&d1).unwrap(),
        Map::zero(coboundary(&g).from(), d1.to()),
        "δ₁δ = 0"
    );
}

#[test]
fn over_the_integers_the_loop_is_not_a_boundary_but_twice_it_is() {
    let (g, faces) = rp2();
    let z = rp2_loop(&g);
    assert_eq!(
        coboundary(&g).adjoint().apply(&z).unwrap(),
        ints(&[0; 6]),
        "z is a cycle"
    );
    // z isn't ∂₂ of any integer 2-chain: the certificate is a ℚ/ℤ cocycle,
    // and ⟨z, c⟩ = 1/2 is the linking of the cross-cap with itself.
    match boundary_checked("z over ℤ", &(), &g, &faces, &z, Z::clone) {
        Boundary::Unbounded(c) => {
            assert_eq!(pair(&(), &z, &c).unwrap(), QModOne::new(1, 2).unwrap());
        }
        Boundary::Bounds(_) => panic!("z bounds no integer 2-chain"),
    }
    // 2z = ∂₂w, w = ±1 on every face: the half-sum ½Σ±∂f is z.
    let twice: Vec<Z> = z.iter().map(|x| x.times(&BigInt::from(2))).collect();
    match boundary_checked("2z over ℤ", &(), &g, &faces, &twice, Z::clone) {
        Boundary::Bounds(w) => {
            assert!(
                w.iter().all(|x| *x == Z::from(1) || *x == Z::from(-1)),
                "w = ±1"
            );
        }
        Boundary::Unbounded(_) => panic!("2z bounds"),
    }
}

#[test]
fn over_z_mod_two_the_loop_is_a_genuine_harmonic_class() {
    let (g, faces) = rp2();
    let two = modulus(2);
    let z = rp2_loop(&g);
    let c = match boundary_checked("z over ℤ/2", &two, &g, &faces, &z, |x| x.reduce(&two)) {
        Boundary::Unbounded(c) => c,
        Boundary::Bounds(_) => panic!("z bounds nothing mod 2"),
    };
    // c is closed, ⟨z, c⟩ = 1: a harmonic twist, a genuine comma.
    assert_eq!(pair(&two, &z, &c).unwrap(), ZMod::new(1, &two));
    match twist_checked("c over ℤ/2", &two, &g, &faces, &c) {
        Twist::Harmonic(cycles) => {
            for cycle in &cycles {
                assert!(
                    matches!(
                        boundary_checked("its cycle", &two, &g, &faces, cycle.chain(), |x| x
                            .reduce(&two)),
                        Boundary::Unbounded(_)
                    ),
                    "a harmonic witness bounds no faces"
                );
            }
        }
        other => panic!("expected harmonic, got {}", kind(&other)),
    }
    // A cocycle by hand: 1 on (0,2), (0,3), (1,2), (1,4), (3,4). Each face
    // meets it in 0 or 2 edges, and z meets it once, on (1,2).
    let mut by_hand = vec![ZMod::new(0, &two); 15];
    for (a, b) in [(0, 2), (0, 3), (1, 2), (1, 4), (3, 4)] {
        by_hand[at(&g, a, b)] = ZMod::new(1, &two);
    }
    assert_eq!(pair(&two, &z, &by_hand).unwrap(), ZMod::new(1, &two));
    assert_eq!(
        kind(&twist_checked("by hand", &two, &g, &faces, &by_hand)),
        "harmonic"
    );
}

#[test]
fn over_z_mod_three_the_loop_is_a_boundary() {
    let (g, faces) = rp2();
    let three = modulus(3);
    let z = rp2_loop(&g);
    let minus: Vec<Z> = z.iter().map(Coefficient::neg).collect();
    // 2 is a unit mod 3: w = 2⁻¹·(±1) = ∓1 on every face. Both z and −z, so
    // both residues of the odd entry of U·z are met.
    for (name, z) in [("z", z), ("−z", minus)] {
        assert!(
            matches!(
                boundary_checked(name, &three, &g, &faces, &z, |x| x.reduce(&three)),
                Boundary::Bounds(_)
            ),
            "{name} bounds over ℤ/3"
        );
    }
}

#[test]
fn the_one_cell_rp2_is_the_same_torsion() {
    // ℝℙ² with one cell in each dimension: a vertex, a self-loop a, and one
    // disk whose boundary walks a twice (field's bigon on [a, a]). ∂₂ = (2),
    // χ = 1 − 1 + 1 = 1. The smallest cell structure the multigraph holds;
    // it leans on two conventions (a self-loop walks forward, an edge walked
    // twice counts twice), so the hemi-icosahedron above is the fixture that
    // needs neither.
    let g = Graph::new([], [(0, 0)]);
    let a = Edge::between(0, 0, 0);
    let faces = vec![Face::bigon(&g, [a, a]).unwrap()];
    let d1 = coboundary1(&g, &faces).unwrap();
    assert_eq!(d1.entry(0, 0), Some(&Z::from(2)));
    assert_eq!(smith_checked("one cell", &d1.adjoint()), ints(&[2]));
    let (two, three) = (modulus(2), modulus(3));
    let z = ints(&[1]);
    assert!(matches!(
        boundary_checked("a over ℤ", &(), &g, &faces, &z, Z::clone),
        Boundary::Unbounded(_)
    ));
    assert!(matches!(
        boundary_checked("2a over ℤ", &(), &g, &faces, &ints(&[2]), Z::clone),
        Boundary::Bounds(_)
    ));
    assert!(matches!(
        boundary_checked("a over ℤ/2", &two, &g, &faces, &z, |x| x.reduce(&two)),
        Boundary::Unbounded(_)
    ));
    assert!(matches!(
        boundary_checked("a over ℤ/3", &three, &g, &faces, &z, |x| x.reduce(&three)),
        Boundary::Bounds(_)
    ));
    let one = vec![ZMod::new(1, &two)];
    assert_eq!(
        kind(&twist_checked("a ↦ 1 over ℤ/2", &two, &g, &faces, &one)),
        "harmonic"
    );
}

/// A square with diagonal 0–2, only the triangle {0, 1, 2} filled: a face,
/// and a hole around {0, 2, 3}.
fn half_filled() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (1, 2), (2, 3), (0, 3), (0, 2)]);
    let faces = triangles(&g, &[[0, 1, 2]]);
    (g, faces)
}

#[test]
fn one_twist_of_each_kind() {
    let (g, faces) = half_filled();
    // Edges: (0,1), (0,2), (0,3), (1,2), (2,3).
    let gradient = coboundary(&g).apply(&ints(&[0, 3, 5, -2])).unwrap();
    let harmonic = ints(&[0, 0, 0, 0, 1]); // around the hole only
    let curved = ints(&[1, 0, 0, 0, 0]); // twists the face
    let twelve = modulus(12);
    let reduce = |c: &[Z]| -> Vec<ZMod> { c.iter().map(|x| x.reduce(&twelve)).collect() };
    for (c, expected) in [
        (&gradient, "gradient"),
        (&harmonic, "harmonic"),
        (&curved, "curvature"),
    ] {
        assert_eq!(
            kind(&twist_checked(expected, &(), &g, &faces, c)),
            expected,
            "over ℤ"
        );
        let c12 = reduce(c);
        assert_eq!(
            kind(&twist_checked(expected, &twelve, &g, &faces, &c12)),
            expected
        );
    }
    // The harmonic witness bounds no faces, over ℤ.
    if let Twist::Harmonic(cycles) = classify(&(), &g, &faces, &harmonic).unwrap() {
        for cycle in cycles {
            assert!(matches!(
                boundary_checked("hole", &(), &g, &faces, cycle.chain(), Z::clone),
                Boundary::Unbounded(_)
            ));
        }
    }
    // The curvature is located on the face, exactly 1.
    match classify(&(), &g, &faces, &curved).unwrap() {
        Twist::Curvature(curls) => {
            assert_eq!(curls.len(), 1);
            assert_eq!(curls[0].value(), &Z::from(1));
            assert_eq!(curls[0].face().vertices(), &[0, 1, 2, 0]);
        }
        other => panic!("expected curvature, got {}", kind(&other)),
    }
    // U(1): 1/3 on the face's first edge is curvature 1/3; three thirds
    // around the face close.
    let third = QModOne::new(1, 3).unwrap();
    let zero = QModOne::new(0, 1).unwrap();
    let mut u1 = vec![zero.clone(); 5];
    u1[0] = third.clone();
    match twist_checked("U(1)", &(), &g, &faces, &u1) {
        Twist::Curvature(curls) => assert_eq!(curls[0].value(), &third),
        other => panic!("expected curvature, got {}", kind(&other)),
    }
    // Around 0 → 1 → 2 → 0: 1/3 + 1/3 − 2/3 = 0, so the face closes; the
    // hole 0 → 2 → 3 → 0 carries 2/3, a genuine U(1) comma.
    let two_thirds = QModOne::new(2, 3).unwrap();
    let closes = vec![third.clone(), two_thirds, zero.clone(), third, zero];
    assert_eq!(
        kind(&twist_checked("U(1) closed", &(), &g, &faces, &closes)),
        "harmonic"
    );
}

#[test]
fn twists_and_bounds_refuse_a_cochain_of_another_room() {
    let (g, faces) = half_filled();
    let short = RoomError::Length {
        expected: 5,
        found: 2,
    };
    let c = ints(&[1, 2]);
    assert_eq!(curvature(&(), &g, &faces, &c), Err(short.clone()));
    assert_eq!(classify(&(), &g, &faces, &c), Err(short.clone()));
    assert_eq!(bounds(&(), &g, &faces, &c), Err(short));
    let (two, three) = (modulus(2), modulus(3));
    let c = vec![ZMod::new(1, &three); 5];
    assert!(matches!(
        classify(&two, &g, &faces, &c),
        Err(RoomError::Modulus { .. })
    ));
    assert!(matches!(
        bounds(&two, &g, &faces, &c),
        Err(RoomError::Modulus { .. })
    ));
}

// --- Law 7: room and field agree ---------------------------------------------------

/// A located loop, read as field's transports: vertices, edges, holonomy.
type Located<G> = (Vec<u64>, Vec<Edge>, G);

/// Every c with values in `values` on each edge of fixture (g, faces):
/// room's verdict against field's check_flat with the same twist, read in
/// field's group by `to_field`. Faces: room's curvature per face is field's
/// face holonomy (field lists the faces that don't close). Loops: field's
/// holes are room's fundamental cycles that don't close and bound no faces
/// in the ring. Kinds: flat is a gradient; curved with no curved face is
/// harmonic, and its witnesses are field's holes.
fn agree<R, G>(
    name: &str,
    ring: &R::Ring,
    (g, faces): (&Graph, &[Face]),
    values: &[i64],
    to_field: impl Fn(&R) -> G,
) where
    R: Bounded,
    G: prismqueer::field::Exact,
{
    let m = g.edges().count();
    let in_ring = |x: &BigInt| R::one(ring).times(x);
    let mut count = 0;
    let mut odometer = vec![0usize; m];
    loop {
        let c: Vec<i64> = odometer.iter().map(|&i| values[i]).collect();
        let room_c: Vec<R> = c.iter().map(|&x| in_ring(&BigInt::from(x))).collect();
        let field_c = room_c.iter().map(&to_field);
        let field = Field::multigraph(g.edges().map(|e| e.ends()).zip(field_c));
        let field = faces.iter().fold(field, |f, face| match face.edges() {
            [e, f2] => f.with_bigons([[*e, *f2]]).unwrap(),
            _ => {
                let v = face.vertices();
                f.with_faces([[v[0], v[1], v[2]]]).unwrap()
            }
        });
        assert_eq!(field.graph(), g, "{name}: one graph");
        assert_eq!(field.faces(), faces, "{name}: one set of faces");

        let curl = curvature(ring, g, faces, &room_c).unwrap();
        let verdict = twist_checked(name, ring, g, faces, &room_c);
        let holes: Vec<Located<G>> = match solve(ring, g, &room_c).unwrap() {
            Exactness::Exact(_) => vec![],
            Exactness::Genuine(cycles) => cycles
                .into_iter()
                .filter(|z| {
                    let zr: Vec<R> = z.chain().iter().map(|x| in_ring(x.value())).collect();
                    matches!(bounds(ring, g, faces, &zr).unwrap(), Boundary::Unbounded(_))
                })
                .map(|z| {
                    (
                        z.vertices().to_vec(),
                        z.edges().to_vec(),
                        to_field(z.pairing()),
                    )
                })
                .collect(),
        };
        let zero = <R as Coefficient>::zero(ring);
        match check_flat(&field) {
            Flatness::Flat(_) => assert_eq!(kind(&verdict), "gradient", "{name}: {c:?}"),
            Flatness::Curved(curved) => {
                let read = |l: &prismqueer::field::Loop<G>| -> Located<G> {
                    (
                        l.vertices().to_vec(),
                        l.edges().to_vec(),
                        l.holonomy().clone(),
                    )
                };
                let field_faces: Vec<Located<G>> = curved.faces().iter().map(read).collect();
                let room_faces: Vec<Located<G>> = faces
                    .iter()
                    .zip(&curl)
                    .filter(|(_, x)| **x != zero)
                    .map(|(f, x)| (f.vertices().to_vec(), f.edges().to_vec(), to_field(x)))
                    .collect();
                assert_eq!(
                    room_faces, field_faces,
                    "{name}: one curvature per face, {c:?}"
                );
                let field_loops: Vec<Located<G>> = curved.loops().iter().map(read).collect();
                assert_eq!(holes, field_loops, "{name}: one set of holes, {c:?}");
                match &verdict {
                    Twist::Gradient(_) => panic!("{name}: field is curved, {c:?}"),
                    Twist::Harmonic(cycles) => {
                        assert!(field_faces.is_empty(), "{name}");
                        let witnesses: Vec<Located<G>> = cycles
                            .iter()
                            .map(|z| {
                                let v = z.vertices().to_vec();
                                (v, z.edges().to_vec(), to_field(z.pairing()))
                            })
                            .collect();
                        assert_eq!(witnesses, field_loops, "{name}: harmonic = field's holes");
                    }
                    Twist::Curvature(_) => assert!(!field_faces.is_empty(), "{name}"),
                }
            }
        }
        count += 1;
        // The next c, in odometer order.
        let Some(i) = odometer.iter().position(|&i| i + 1 < values.len()) else {
            break;
        };
        odometer[i] += 1;
        odometer[..i].iter_mut().for_each(|x| *x = 0);
    }
    assert_eq!(count, values.len().pow(m as u32), "{name}: every c");
}

/// field's fixtures: field_faces' triangles and square, field_multiplicity's
/// bigon and doubled side.
fn field_fixtures() -> Vec<(&'static str, Graph, Vec<Face>)> {
    let (t, tf) = triangle();
    let (s, sf) = square();
    let (h, hf) = half_filled();
    let (b, bf) = bigon();
    // A triangle with a doubled side 0–1: the bigon filled, the triangle a
    // hole.
    let d = Graph::new([], [(0, 1), (0, 1), (1, 2), (0, 2)]);
    let df = vec![Face::bigon(&d, [Edge::between(0, 1, 0), Edge::between(0, 1, 1)]).unwrap()];
    vec![
        ("filled triangle", t, tf),
        ("filled square", s, sf),
        ("half-filled square", h, hf),
        ("filled bigon", b, bf),
        ("doubled side", d, df),
    ]
}

#[test]
fn room_and_field_agree_over_the_integers() {
    for (name, g, faces) in field_fixtures() {
        // Every c ∈ {−1, 0, 1, 2}^E, as field's Z too.
        agree(name, &(), (&g, &faces), &[-1, 0, 1, 2], Z::clone);
    }
}

#[test]
fn room_and_field_agree_over_z_mod_two() {
    let two = modulus(2);
    let gf2 = |x: &ZMod| Zn::<2>::new(x.residue().to_u64().unwrap());
    for (name, g, faces) in field_fixtures() {
        agree(name, &two, (&g, &faces), &[0, 1], gf2);
    }
}

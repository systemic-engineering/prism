//! Holes on `room`: the complex's genuine holes, a basis of H₁ (prism-011,
//! the shape answer).
//!
//! Curvature and holes are reported separately, never as one split.
//! `curvature` and `classify`'s `Curvature(curls)` stay per face and
//! path-free. Holes are a property of the complex, not of any c: each is a
//! cycle that bounds no faces, with its certificate in its own group. A
//! reading of c along a hole names its cycle, (z, ⟨z, c⟩): the number belongs
//! to the path, and the caller chooses the path.
//!
//! The laws:
//!
//! 1. **Every witness is checked exactly.** z is a cycle (∂₁z = 0) that
//!    bounds nothing in the ring; its certificate c is closed (δ₁c = 0, in
//!    the ring's dual) with ⟨z, c⟩ ≠ 0; its order is exact (d·z bounds, k·z
//!    doesn't for 0 < k < d; ∞: no multiple bounds); and the certificates are
//!    dual to the cycles (⟨zᵢ, cⱼ⟩ = 0 for i ≠ j), which makes the cycles
//!    independent with exactly their orders.
//! 2. **Counts and orders are invariants.** The basis is a choice; the tests
//!    assert the multiset of orders, and a specific cycle only where the
//!    cycle is forced.
//! 3. **Universal coefficients.** H₁(X; ℤ/n) ≅ H₁(X; ℤ) ⊗ ℤ/n, since H₀ is
//!    free: a free generator gives ℤ/n, a torsion one ℤ/gcd(d, n), and a
//!    gcd of 1 gives nothing.
//! 4. **The comma.** On a curved c, two cycles around one hole read
//!    differently, by exactly ⟨w, δ₁c⟩ for the w between them.

#![cfg(feature = "lapack")]

use num_bigint::BigInt;
use num_traits::Zero;
use prismqueer::field::{check_flat, Edge, Face, Field, Flatness, Graph};
use prismqueer::room::{
    bounds, classify, coboundary, coboundary1, coboundary1_in, curvature, holes, pair, Boundary,
    Bounded, Coefficient, Hole, Modulus, Order, QModOne, Twist, ZMod, Z,
};

fn modulus(n: i64) -> Modulus {
    Modulus::new(n).expect("n ≥ 2")
}

fn ints(xs: &[i64]) -> Vec<Z> {
    xs.iter().map(|&x| Z::from(x)).collect()
}

fn scaled(z: &[Z], k: &BigInt) -> Vec<Z> {
    z.iter().map(|x| x.times(k)).collect()
}

fn triangles(g: &Graph, corners: &[[u64; 3]]) -> Vec<Face> {
    corners
        .iter()
        .map(|&c| Face::triangle(g, c).expect("a triangle of g"))
        .collect()
}

// --- Fixtures ----------------------------------------------------------------

/// One triangle, filled: a disk.
fn filled_triangle() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (1, 2), (0, 2)]);
    let faces = triangles(&g, &[[0, 1, 2]]);
    (g, faces)
}

/// One triangle, hollow: a circle.
fn hollow_triangle() -> (Graph, Vec<Face>) {
    (Graph::new([], [(0, 1), (1, 2), (0, 2)]), vec![])
}

/// A square with diagonal 0–2, only {0, 1, 2} filled: a hole around
/// {0, 2, 3}. Edges: (0,1), (0,2), (0,3), (1,2), (2,3).
fn half_filled() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (1, 2), (2, 3), (0, 3), (0, 2)]);
    let faces = triangles(&g, &[[0, 1, 2]]);
    (g, faces)
}

/// A square with diagonal 0–2, both triangles filled: a disk.
fn filled_square() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (1, 2), (2, 3), (0, 3), (0, 2)]);
    let faces = triangles(&g, &[[0, 1, 2], [0, 2, 3]]);
    (g, faces)
}

/// Two hollow triangles, apart: two circles.
fn two_hollow() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)]);
    (g, vec![])
}

/// Two parallel edges on 0–1, the bigon between them hollow or filled.
fn bigon(filled: bool) -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (0, 1)]);
    let faces = if filled {
        vec![Face::bigon(&g, [Edge::between(0, 1, 0), Edge::between(0, 1, 1)]).unwrap()]
    } else {
        vec![]
    };
    (g, faces)
}

/// A triangle with a doubled side 0–1: the bigon filled, the triangle a hole.
fn doubled_side() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 1), (0, 1), (1, 2), (0, 2)]);
    let faces = vec![Face::bigon(&g, [Edge::between(0, 1, 0), Edge::between(0, 1, 1)]).unwrap()];
    (g, faces)
}

/// ℝℙ², the hemi-icosahedron on K₆ (room_faces' fixture): H₁(ℤ) = ℤ/2.
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

/// The one-cell ℝℙ²: a self-loop a and the bigon [a, a], ∂₂ = (2).
fn rp2_one_cell() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 0)]);
    let a = Edge::between(0, 0, 0);
    let faces = vec![Face::bigon(&g, [a, a]).unwrap()];
    (g, faces)
}

/// The one-cell ℝℙ² wedged with a hollow triangle at vertex 0: H₁(ℤ) =
/// ℤ/2 ⊕ ℤ, a free part and a torsion part at once.
fn rp2_and_circle() -> (Graph, Vec<Face>) {
    let g = Graph::new([], [(0, 0), (0, 1), (1, 2), (0, 2)]);
    let a = Edge::between(0, 0, 0);
    let faces = vec![Face::bigon(&g, [a, a]).unwrap()];
    (g, faces)
}

fn fixtures() -> Vec<(&'static str, Graph, Vec<Face>)> {
    let all = [
        ("filled triangle", filled_triangle()),
        ("hollow triangle", hollow_triangle()),
        ("half-filled square", half_filled()),
        ("filled square", filled_square()),
        ("two hollow triangles", two_hollow()),
        ("hollow bigon", bigon(false)),
        ("filled bigon", bigon(true)),
        ("doubled side", doubled_side()),
        ("ℝℙ²", rp2()),
        ("one-cell ℝℙ²", rp2_one_cell()),
        ("ℝℙ² ∨ S¹", rp2_and_circle()),
    ];
    all.into_iter().map(|(n, (g, f))| (n, g, f)).collect()
}

// --- Law 1: every witness checked exactly ------------------------------------

/// The holes of (g, faces) over R, every witness checked exactly (law 1).
fn holes_checked<R: Bounded>(
    name: &str,
    ring: &R::Ring,
    g: &Graph,
    faces: &[Face],
    into: impl Fn(&Z) -> R,
) -> Vec<Hole<R>> {
    let found = holes(ring, g, faces).expect("faces of g");
    let m = g.edges().count();
    let dual = R::dual(ring);
    let zero = <R::Dual as Coefficient>::zero(&dual);
    let d1 = coboundary1(g, faces).unwrap();
    let is_bounded = |z: &[Z]| {
        let zr: Vec<R> = z.iter().map(&into).collect();
        let verdict = bounds(ring, g, faces, &zr).expect("a chain of g");
        if let Boundary::Bounds(w) = &verdict {
            let d2 = coboundary1_in::<R>(ring, g, faces).unwrap().adjoint();
            assert_eq!(d2.apply(w).unwrap(), zr, "{name}: ∂₂w = z, exactly");
        }
        matches!(verdict, Boundary::Bounds(_))
    };
    for (i, hole) in found.iter().enumerate() {
        let (z, c) = (hole.cycle(), hole.certificate());
        assert_eq!((z.len(), c.len()), (m, m), "{name}: z ∈ C₁, c ∈ C¹");
        assert_eq!(
            coboundary(g).adjoint().apply(z).unwrap(),
            ints(&vec![0; g.vertex_count()]),
            "{name}: hole {i} is a cycle, ∂₁z = 0"
        );
        let curl = d1.act::<R::Dual>(&dual, c).unwrap();
        assert!(curl.iter().all(|x| *x == zero), "{name}: δ₁c = 0");
        let reading = pair(&dual, z, c).unwrap();
        assert_ne!(reading, zero, "{name}: ⟨z, c⟩ ≠ 0");
        assert!(!is_bounded(z), "{name}: hole {i} bounds nothing");
        match hole.order() {
            Order::Finite(d) => {
                let d = d.value().clone();
                assert!(d > BigInt::from(1), "{name}: a nontrivial order");
                assert!(is_bounded(&scaled(z, &d)), "{name}: {d}·z bounds, hole {i}");
                let mut k = BigInt::from(1);
                while k < d {
                    assert!(!is_bounded(&scaled(z, &k)), "{name}: {k}·z doesn't");
                    k += 1;
                }
                assert_eq!(reading.times(&d), zero, "{name}: d·⟨z, c⟩ = 0");
            }
            Order::Infinite => {
                for k in 2..=4 {
                    let k = BigInt::from(k);
                    assert!(!is_bounded(&scaled(z, &k)), "{name}: {k}·z doesn't bound");
                }
            }
        }
        for (j, other) in found.iter().enumerate() {
            if i != j {
                assert_eq!(
                    pair(&dual, z, other.certificate()).unwrap(),
                    zero,
                    "{name}: ⟨z{i}, c{j}⟩ = 0: the certificates are dual"
                );
            }
        }
    }
    found
}

/// The orders of a list of holes, sorted: the invariant, the basis forgotten.
fn orders<R: Bounded>(found: &[Hole<R>]) -> Vec<Order> {
    let mut o: Vec<Order> = found.iter().map(|h| h.order().clone()).collect();
    o.sort();
    o
}

fn finite(d: i64) -> Order {
    Order::Finite(Z::from(d))
}

/// The orders over ℤ, ℤ/2 and ℤ/3, every witness checked.
fn three(name: &str, (g, faces): &(Graph, Vec<Face>)) -> [Vec<Order>; 3] {
    let (two, three) = (modulus(2), modulus(3));
    [
        orders(&holes_checked(name, &(), g, faces, Z::clone)),
        orders(&holes_checked(name, &two, g, faces, |x| x.reduce(&two))),
        orders(&holes_checked(name, &three, g, faces, |x| x.reduce(&three))),
    ]
}

#[test]
fn a_filled_triangle_has_no_holes() {
    assert_eq!(
        three("filled triangle", &filled_triangle()),
        [vec![], vec![], vec![]]
    );
    assert_eq!(
        three("filled square", &filled_square()),
        [vec![], vec![], vec![]]
    );
}

#[test]
fn a_hollow_triangle_has_one_free_hole() {
    let fixture = hollow_triangle();
    assert_eq!(
        three("hollow triangle", &fixture),
        [vec![Order::Infinite], vec![finite(2)], vec![finite(3)]]
    );
    // The cycle is forced, up to sign: 0 → 1 → 2 → 0 on (0,1), (0,2), (1,2).
    let found = holes::<Z>(&(), &fixture.0, &fixture.1).unwrap();
    let z = found[0].cycle().to_vec();
    assert!(z == ints(&[1, -1, 1]) || z == ints(&[-1, 1, -1]), "{z:?}");
}

#[test]
fn a_square_with_one_triangle_filled_has_one_hole() {
    let (g, faces) = half_filled();
    assert_eq!(
        three("half-filled square", &(g.clone(), faces.clone())),
        [vec![Order::Infinite], vec![finite(2)], vec![finite(3)]]
    );
    // The basis is a choice: the hole is 0 → 2 → 3 → 0 up to a boundary and
    // a sign, whichever representative was chosen.
    let h = holes::<Z>(&(), &g, &faces).unwrap().remove(0);
    let around = ints(&[0, 1, -1, 0, 1]);
    let homologous = [1i64, -1].iter().any(|&s| {
        let diff: Vec<Z> = h
            .cycle()
            .iter()
            .zip(&around)
            .map(|(a, b)| a.add(&b.times(&BigInt::from(s))))
            .collect();
        matches!(bounds(&(), &g, &faces, &diff).unwrap(), Boundary::Bounds(_))
    });
    assert!(homologous, "the hole is the one around {{0, 2, 3}}");
}

#[test]
fn two_disjoint_hollow_triangles_have_two_free_holes() {
    assert_eq!(
        three("two hollow triangles", &two_hollow()),
        [
            vec![Order::Infinite, Order::Infinite],
            vec![finite(2), finite(2)],
            vec![finite(3), finite(3)]
        ]
    );
}

#[test]
fn rp2_has_one_hole_of_order_two() {
    for (name, fixture) in [("ℝℙ²", rp2()), ("one-cell ℝℙ²", rp2_one_cell())] {
        let [z, z2, z3] = three(name, &fixture);
        assert_eq!(z, vec![finite(2)], "{name}: over ℤ, ℤ/2");
        assert_eq!(z2, vec![finite(2)], "{name}: over ℤ/2, one generator");
        assert_eq!(z3, vec![], "{name}: over ℤ/3, none: 2 is a unit");
    }
    // Torsion is certified in ℚ/ℤ: the hole pairs to exactly 1/2.
    let (g, faces) = rp2();
    let h = holes::<Z>(&(), &g, &faces).unwrap().remove(0);
    assert_eq!(
        pair(&(), h.cycle(), h.certificate()).unwrap(),
        QModOne::new(1, 2).unwrap()
    );
    // The one-cell cycle is forced: ±a.
    let (g, faces) = rp2_one_cell();
    let h = holes::<Z>(&(), &g, &faces).unwrap().remove(0);
    assert!(h.cycle() == ints(&[1]) || h.cycle() == ints(&[-1]));
}

#[test]
fn a_free_part_and_a_torsion_part_at_once() {
    assert_eq!(
        three("ℝℙ² ∨ S¹", &rp2_and_circle()),
        [
            vec![Order::Infinite, finite(2)],
            vec![finite(2), finite(2)],
            vec![finite(3)]
        ]
    );
}

#[test]
fn a_bigon_unfilled_has_a_hole_and_filled_has_none() {
    assert_eq!(
        three("hollow bigon", &bigon(false)),
        [vec![Order::Infinite], vec![finite(2)], vec![finite(3)]]
    );
    let (g, faces) = bigon(false);
    let z = holes::<Z>(&(), &g, &faces).unwrap()[0].cycle().to_vec();
    assert!(z == ints(&[1, -1]) || z == ints(&[-1, 1]), "forced: {z:?}");
    assert_eq!(
        three("filled bigon", &bigon(true)),
        [vec![], vec![], vec![]]
    );
}

#[test]
fn a_face_off_the_graph_is_refused() {
    let (_, faces) = filled_triangle();
    let path = Graph::new([], [(0, 1), (1, 2)]);
    assert!(holes::<Z>(&(), &path, &faces).is_err());
}

// --- Laws 2 and 3: invariants, and universal coefficients ---------------------

/// gcd, for the universal-coefficient arithmetic.
fn gcd(a: &BigInt, b: &BigInt) -> BigInt {
    let (mut a, mut b) = (a.clone(), b.clone());
    while !b.is_zero() {
        (a, b) = (b.clone(), a % b);
    }
    a
}

#[test]
fn over_the_integers_the_counts_are_the_smith_factors() {
    // Independently of the cycle basis: ∂₂ in edge coordinates. B₁ ⊆ Z₁ and
    // Z₁ is a summand of C₁ (C₁/Z₁ ≅ im ∂₁ is free), so the torsion of
    // C₁/B₁ is that of H₁: the factors dᵢ > 1 of ∂₂. The free rank is
    // b₁ − rank ∂₂.
    for (name, g, faces) in fixtures() {
        let d2 = coboundary1(&g, &faces).unwrap().adjoint().smith();
        let mut expected: Vec<Order> = d2
            .factors()
            .iter()
            .filter(|d| *d.value() > BigInt::from(1))
            .map(|d| Order::Finite(d.clone()))
            .collect();
        expected.extend(vec![Order::Infinite; g.cycle_rank() - d2.rank()]);
        expected.sort();
        let found = orders(&holes_checked(name, &(), &g, &faces, Z::clone));
        assert_eq!(found, expected, "{name}");
    }
}

#[test]
fn universal_coefficients_agree() {
    // H₁(ℤ/n) = H₁(ℤ) ⊗ ℤ/n ⊕ Tor(H₀(ℤ), ℤ/n), and H₀ is free, so Tor = 0:
    // ℤ ⊗ ℤ/n = ℤ/n, and ℤ/d ⊗ ℤ/n = ℤ/gcd(d, n), nothing when that's 1.
    for (name, g, faces) in fixtures() {
        let over_z = orders(&holes_checked(name, &(), &g, &faces, Z::clone));
        for n in [2i64, 3, 4, 6, 12] {
            let ring = modulus(n);
            let found = orders(&holes_checked(name, &ring, &g, &faces, |x| x.reduce(&ring)));
            let n = BigInt::from(n);
            let mut expected: Vec<Order> = over_z
                .iter()
                .filter_map(|o| match o {
                    Order::Infinite => Some(Order::Finite(Z::from(n.clone()))),
                    Order::Finite(d) => {
                        let g = gcd(d.value(), &n);
                        (g > BigInt::from(1)).then(|| Order::Finite(Z::from(g)))
                    }
                })
                .collect();
            expected.sort();
            assert_eq!(found, expected, "{name} over ℤ/{n}");
        }
    }
}

#[test]
fn over_a_prime_the_count_is_the_euler_arithmetic() {
    // Over a field ℤ/p: dim H₁ = b₀ + b₂ − χ, χ = V − E + F, b₂ = F −
    // rank_p ∂₂, rank_p ∂₂ the factors of ∂₂ that p doesn't divide.
    for (name, g, faces) in fixtures() {
        let (v, e, f) = (g.vertex_count(), g.edges().count(), faces.len());
        let b0 = v - (e - g.cycle_rank());
        let d2 = coboundary1(&g, &faces).unwrap().adjoint().smith();
        for p in [2i64, 3] {
            let ring = modulus(p);
            let rank_p = d2
                .factors()
                .iter()
                .filter(|d| !(d.value() % BigInt::from(p)).is_zero())
                .count();
            let b2 = f - rank_p;
            let b1 = (b0 + b2) as i64 - (v as i64 - e as i64 + f as i64);
            let found = holes_checked(name, &ring, &g, &faces, |x| x.reduce(&ring));
            assert_eq!(found.len() as i64, b1, "{name} over ℤ/{p}");
        }
    }
}

// --- Law 4: the comma, named -----------------------------------------------------

#[test]
fn on_a_curved_field_two_paths_around_one_hole_read_differently() {
    // The half-filled square: one face {0, 1, 2}, one hole around {0, 2, 3}.
    // Edges: (0,1), (0,2), (0,3), (1,2), (2,3).
    let (g, faces) = half_filled();
    let short = ints(&[0, 1, -1, 0, 1]); // 0 → 2 → 3 → 0
    let long = ints(&[1, 0, -1, 1, 1]); // 0 → 1 → 2 → 3 → 0
    let gap: Vec<Z> = long
        .iter()
        .zip(&short)
        .map(|(a, b)| a.add(&b.neg()))
        .collect();
    let w = match bounds(&(), &g, &faces, &gap).unwrap() {
        Boundary::Bounds(w) => w,
        Boundary::Unbounded(_) => panic!("z′ − z bounds the face"),
    };
    let d2 = coboundary1(&g, &faces).unwrap().adjoint();
    assert_eq!(d2.apply(&w).unwrap(), gap, "∂₂w = z′ − z");
    // Both go around the one hole.
    let h = holes::<Z>(&(), &g, &faces).unwrap();
    assert_eq!(h.len(), 1);
    for z in [&short, &long] {
        assert!(matches!(
            bounds(&(), &g, &faces, z).unwrap(),
            Boundary::Unbounded(_)
        ));
    }

    // c twists the face by 1 and the hole by 1.
    let c = ints(&[1, 0, 0, 0, 1]);
    match classify(&(), &g, &faces, &c).unwrap() {
        Twist::Curvature(curls) => {
            assert_eq!(curls.len(), 1, "curvature stays per face, path-free");
            assert_eq!(curls[0].value(), &Z::from(1));
        }
        _ => panic!("c is curved"),
    }
    let (a, b) = (
        pair(&(), &short, &c).unwrap(),
        pair(&(), &long, &c).unwrap(),
    );
    assert_eq!((a.clone(), b.clone()), (Z::from(1), Z::from(2)));
    assert_ne!(a, b, "the reading belongs to the path");
    let curl = curvature(&(), &g, &faces, &c).unwrap();
    assert_eq!(
        b.add(&a.neg()),
        pair(&(), &w, &curl).unwrap(),
        "⟨z′, c⟩ − ⟨z, c⟩ = ⟨w, δ₁c⟩"
    );

    // The same in ℤ/12 and in U(1).
    let twelve = modulus(12);
    let c12: Vec<ZMod> = ints(&[5, 0, 0, 0, 7])
        .iter()
        .map(|x| x.reduce(&twelve))
        .collect();
    let curl12 = curvature(&twelve, &g, &faces, &c12).unwrap();
    let (a, b) = (
        pair(&twelve, &short, &c12).unwrap(),
        pair(&twelve, &long, &c12).unwrap(),
    );
    assert_ne!(a, b);
    assert_eq!(b.add(&a.neg()), pair(&twelve, &w, &curl12).unwrap());
    let zero = QModOne::new(0, 1).unwrap();
    let u1 = vec![
        QModOne::new(1, 3).unwrap(),
        zero.clone(),
        zero.clone(),
        zero,
        QModOne::new(1, 4).unwrap(),
    ];
    let curl_u1 = curvature(&(), &g, &faces, &u1).unwrap();
    let (a, b) = (
        pair(&(), &short, &u1).unwrap(),
        pair(&(), &long, &u1).unwrap(),
    );
    assert_eq!(a, QModOne::new(1, 4).unwrap());
    assert_eq!(b, QModOne::new(7, 12).unwrap());
    assert_eq!(b.add(&a.neg()), pair(&(), &w, &curl_u1).unwrap());

    // The control: a closed c (δ₁c = 0) reads the same along both.
    let closed = ints(&[0, 0, 0, 0, 1]);
    assert!(curvature(&(), &g, &faces, &closed)
        .unwrap()
        .iter()
        .all(|x| *x == Z::from(0)));
    assert_eq!(pair(&(), &short, &closed), pair(&(), &long, &closed));
}

// --- Agreement with field --------------------------------------------------------

#[test]
fn the_count_agrees_with_fields_non_bounding_loops() {
    // c(e) = 2^e: every nonzero cycle twists, so field reports each basis loop
    // outside the faces' GF(2) span as a hole.
    for (name, g, faces) in [
        ("filled triangle", filled_triangle()),
        ("filled square", filled_square()),
        ("half-filled square", half_filled()),
        ("filled bigon", bigon(true)),
        ("doubled side", doubled_side()),
        ("hollow triangle", hollow_triangle()),
        ("two hollow triangles", two_hollow()),
        ("hollow bigon", bigon(false)),
    ]
    .into_iter()
    .map(|(n, (g, f))| (n, g, f))
    {
        let values = (0..g.edges().count()).map(|e| Z::from(1i64 << e));
        let field = Field::multigraph(g.edges().map(|e| e.ends()).zip(values));
        let field = faces.iter().fold(field, |f, face| match face.edges() {
            [e, f2] => f.with_bigons([[*e, *f2]]).unwrap(),
            _ => {
                let v = face.vertices();
                f.with_faces([[v[0], v[1], v[2]]]).unwrap()
            }
        });
        let loops = match check_flat(&field) {
            Flatness::Flat(_) => 0,
            Flatness::Curved(curved) => curved.loops().len(),
        };
        let two = modulus(2);
        assert_eq!(
            holes::<Z>(&(), &g, &faces).unwrap().len(),
            loops,
            "{name} over ℤ"
        );
        assert_eq!(
            holes::<ZMod>(&two, &g, &faces).unwrap().len(),
            loops,
            "{name} over ℤ/2"
        );
    }
}

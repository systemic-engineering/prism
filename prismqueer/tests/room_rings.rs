//! Ring kinds on `room`, and the exact edge solver (prism-011, step 3).
//!
//! "If they all have to be computed in the number system they actually live
//! in then they all gotta be computed in the same `Room`." (Alex)
//!
//! A twist c ∈ C¹(G; R) is exact when c = δg for some g ∈ C⁰(G; R), and a
//! genuine class otherwise. Which one depends on R: the same integers can be
//! a genuine class over ℤ and exact over ℤ/n. The laws:
//!
//! 1. **The solver's verdict carries its witness, checked in the room.**
//!    Exact: a potential g with δg = c, exactly. Genuine: a cycle z, an
//!    integer chain with δ*z = 0, whose pairing ⟨z, c⟩ is nonzero in R, so
//!    no g exists (⟨z, δg⟩ = ⟨δ*z, g⟩ = 0 for every g).
//! 2. **The ring decides.** The tritone pair is genuine over ℤ and exact over
//!    ℤ/12; a `not`-loop is a paradox or a choice by the parity of its
//!    negations over ℤ/2 (Harary's balance); a U(1) twist is exact when its
//!    loop sums to 0 mod 1.
//! 3. **ℚ mod 1 is ℤ/m, m the lcm of the denominators**: the same verdict,
//!    and the witnesses map back by x ↦ x/m.
//! 4. **Room and field agree** wherever both apply: `solve` over ℤ against
//!    `check_flat` over ℤ, and over ℤ/n against field's `Zn<N>`.
//! 5. **Moduli don't mix.** Two moduli in one operation is a checked
//!    `RoomError::Modulus`; two ring kinds is a compile error (the module
//!    doctests).

#![cfg(feature = "lapack")]

use std::collections::BTreeMap;

use prismqueer::dependency::{DependencyGraph, Sign};
use prismqueer::field::{check_flat, Edge, Field, Flatness, Graph, Zn};
use prismqueer::room::{
    clear_denominators, coboundary, coboundary_in, laplacian, pair, solve, Coefficient, Exactly,
    Exactness, Modulus, QModOne, RoomError, ZMod, Z,
};

fn modulus(n: i64) -> Modulus {
    Modulus::new(n).expect("n ≥ 2")
}

fn ints(xs: &[i64]) -> Vec<Z> {
    xs.iter().map(|&x| Z::from(x)).collect()
}

fn residues(xs: &[i64], n: &Modulus) -> Vec<ZMod> {
    xs.iter().map(|&x| ZMod::new(x, n)).collect()
}

fn q(num: i64, den: i64) -> QModOne {
    QModOne::new(num, den).expect("a denominator")
}

/// Solve δg = c over `ring` on `g`, and check the verdict's witness in the
/// room, exactly (law 1).
fn witnessed<V: Exactly>(name: &str, ring: &V::Ring, g: &Graph, c: &[V]) -> Exactness<V> {
    let delta = coboundary(g);
    let verdict = solve(ring, g, c).expect("c is a cochain of g, in the ring");
    match &verdict {
        Exactness::Exact(potential) => {
            assert_eq!(potential.len(), g.vertex_count(), "{name}: g ∈ C⁰");
            assert_eq!(
                delta.act(ring, potential).unwrap(),
                c,
                "{name}: δg = c, exactly"
            );
        }
        Exactness::Genuine(cycles) => {
            assert!(!cycles.is_empty(), "{name}: a certificate");
            assert!(cycles.len() <= g.cycle_rank(), "{name}: basis cycles");
            for cycle in cycles {
                let z = cycle.chain();
                assert_eq!(z.len(), c.len(), "{name}: z ∈ C₁");
                assert_eq!(
                    delta.adjoint().apply(z).unwrap(),
                    vec![Z::from(0); g.vertex_count()],
                    "{name}: z is a cycle, δ*z = 0"
                );
                assert_eq!(
                    &pair(ring, z, c).unwrap(),
                    cycle.pairing(),
                    "{name}: the pairing is ⟨z, c⟩"
                );
                assert_ne!(
                    cycle.pairing(),
                    &V::zero(ring),
                    "{name}: ⟨z, c⟩ ≠ 0 in the ring, so no g has δg = c"
                );
                assert_eq!(
                    cycle.vertices().first(),
                    cycle.vertices().last(),
                    "{name}: closed"
                );
                assert_eq!(cycle.edges().len() + 1, cycle.vertices().len());
            }
        }
    }
    verdict
}

fn potential<V: Clone>(verdict: &Exactness<V>) -> Vec<V> {
    match verdict {
        Exactness::Exact(g) => g.clone(),
        Exactness::Genuine(cycles) => panic!("expected exact, got {cycles:?}"),
    }
}

/// The one certificate of a genuine verdict on a graph with b₁ = 1.
fn certificate<V: Clone + std::fmt::Debug>(verdict: &Exactness<V>) -> (Vec<u64>, Vec<Z>, V) {
    match verdict {
        Exactness::Genuine(cycles) => {
            assert_eq!(cycles.len(), 1, "one loop");
            let c = &cycles[0];
            (
                c.vertices().to_vec(),
                c.chain().to_vec(),
                c.pairing().clone(),
            )
        }
        Exactness::Exact(g) => panic!("expected genuine, got the potential {g:?}"),
    }
}

// --- The tritone pair -------------------------------------------------------

/// Two tritones a → b → a: up 6 semitones, then up 6 more, back at a. The
/// two edges are parallel on (a, b) = (0, 1), and C¹ stores each in its
/// canonical orientation, a → b. The first tritone goes a → b: +6. The
/// second goes b → a, so a → b it is −6. The cochain is c = (6, −6).
///
/// The loop is a → b along the first edge, b → a along the second, so its
/// chain is z = (+1, −1) and ⟨z, c⟩ = 6 − (−6) = 12: the octave.
/// - over ℤ, 12 ≠ 0: a genuine class, no potential;
/// - over ℤ/12, 12 ≡ 0: exact. c ≡ (6, 6), and g = (0, 6) has δg = (6, 6):
///   both tritones are one step of a single pitch class, a → b.
///
/// The trap the fixture avoids: (6, 6) over ℤ is exact already (g = (0, 6)),
/// because both edges then point a → b and ⟨z, c⟩ = 6 − 6 = 0. That is two
/// tritones from a to b, not a loop around the octave.
#[test]
fn the_tritone_pair_is_genuine_over_z_and_exact_mod_12() {
    let g = Graph::new([], [(0, 1), (0, 1)]);
    let c = ints(&[6, -6]);

    let (vertices, z, holonomy) = certificate(&witnessed("tritone over ℤ", &(), &g, &c));
    assert_eq!(vertices, vec![0, 1, 0], "a → b → a");
    assert_eq!(
        z,
        ints(&[1, -1]),
        "out along the first, back along the second"
    );
    assert_eq!(holonomy, Z::from(12), "⟨z, c⟩ = 6 − (−6) = 12, the octave");

    // field reads the same loop from the music: two tritones up, a → b → a.
    let music = Field::multigraph([((0, 1), Z::from(6)), ((1, 0), Z::from(6))]);
    let Flatness::Curved(curved) = check_flat(&music) else {
        panic!("two tritones up don't close over ℤ");
    };
    assert_eq!(curved.loops()[0].holonomy(), &Z::from(12));

    let twelve = modulus(12);
    let c12: Vec<ZMod> = c.iter().map(|x| x.reduce(&twelve)).collect();
    assert_eq!(c12, residues(&[6, 6], &twelve), "−6 ≡ 6 (mod 12)");
    let g12 = potential(&witnessed("tritone over ℤ/12", &twelve, &g, &c12));
    assert_eq!(g12, residues(&[0, 6], &twelve), "a at 0, b a tritone up");
    assert_eq!(
        coboundary_in::<ZMod>(&twelve, &g).apply(&g12).unwrap(),
        c12,
        "δg = c with δ over ℤ/12 too"
    );

    // The ring decides: exact iff n divides 12.
    for (n, exact) in [(2, true), (3, true), (6, true), (5, false), (24, false)] {
        let n = modulus(n);
        let cn: Vec<ZMod> = c.iter().map(|x| x.reduce(&n)).collect();
        let name = format!("tritone over ℤ/{}", n.value());
        match witnessed(&name, &n, &g, &cn) {
            Exactness::Exact(_) => assert!(exact, "{name}"),
            Exactness::Genuine(cycles) => {
                assert!(!exact, "{name}");
                assert_eq!(cycles[0].pairing(), &ZMod::new(12, &n), "{name}: 12 mod n");
            }
        }
    }

    // The trap: both edges a → b, (6, 6), is a gradient over ℤ.
    let gradient = potential(&witnessed("two tritones a → b", &(), &g, &ints(&[6, 6])));
    assert_eq!(gradient, ints(&[0, 6]));
}

// --- Harary's balance -------------------------------------------------------

/// A signed dependency graph as a ℤ/2 cochain: a `not` is a half-turn (1),
/// a plain dependency none (0). The rules are named by head and premise; the
/// edges are `dependency.rs`'s, through its `Sign`. `DependencyGraph` exposes
/// no edge iterator, so the rule list is the source for both it and the
/// cochain, and the two cycle ranks are checked equal.
fn signed(rules: &[(u64, u64, Sign)]) -> (Graph, Vec<ZMod>) {
    let two = modulus(2);
    let mut deps = DependencyGraph::new();
    let mut nth: BTreeMap<(u64, u64), usize> = BTreeMap::new();
    let mut sign = BTreeMap::new();
    for &(head, premise, s) in rules {
        deps.add_edge(premise, head, s);
        let ends = (premise.min(head), premise.max(head));
        let k = nth.entry(ends).or_insert(0);
        sign.insert(Edge::between(premise, head, *k), s);
        *k += 1;
    }
    let g = Graph::new([], rules.iter().map(|&(h, p, _)| (p, h)));
    assert_eq!(deps.cycle_rank(), g.cycle_rank(), "one graph");
    let c = g
        .edges()
        .map(|e| match sign[&e] {
            Sign::Positive => ZMod::new(0, &two),
            Sign::Negative => ZMod::new(1, &two),
        })
        .collect();
    (g, c)
}

/// An odd number of `not`s around a loop is holonomy 1 in ℤ/2: no
/// assignment of truth values satisfies every rule, a paradox. An even
/// number is holonomy 0: exact, and the potential is a consistent choice.
/// Orientation doesn't matter in ℤ/2 (−1 ≡ 1).
#[test]
fn harary_an_odd_not_loop_is_a_paradox_and_an_even_one_a_choice() {
    use Sign::{Negative, Positive};
    let (p, q, r) = (0, 1, 2);
    let two = modulus(2);
    let one = ZMod::new(1, &two);

    // p :- not p. A self-loop: δ's row is zero, so the loop is z = e, and
    // ⟨z, c⟩ = c(e) = 1.
    let (g, c) = signed(&[(p, p, Negative)]);
    let (vertices, z, h) = certificate(&witnessed("p :- not p", &two, &g, &c));
    assert_eq!((vertices, z, h), (vec![p, p], ints(&[1]), one.clone()));

    // p :- not q. q :- not p. Two negations: a choice, p and q opposite.
    let (g, c) = signed(&[(p, q, Negative), (q, p, Negative)]);
    let choice = potential(&witnessed("p :- not q. q :- not p.", &two, &g, &c));
    assert_eq!(choice, residues(&[0, 1], &two), "p and q differ");

    // Three negations around p, q, r: a paradox.
    let (g, c) = signed(&[(p, q, Negative), (q, r, Negative), (r, p, Negative)]);
    let (_, _, h) = certificate(&witnessed("odd 3-loop", &two, &g, &c));
    assert_eq!(h, one);

    // Two negations and one plain dependency: a choice.
    let (g, c) = signed(&[(p, q, Negative), (q, r, Positive), (r, p, Negative)]);
    let choice = potential(&witnessed("even 3-loop", &two, &g, &c));
    assert_eq!(choice, residues(&[0, 1, 1], &two), "p apart from q = r");

    // The 2-loop of one negation and one plain edge: odd, a paradox.
    let (g, c) = signed(&[(p, q, Negative), (q, p, Positive)]);
    let (_, _, h) = certificate(&witnessed("odd 2-loop", &two, &g, &c));
    assert_eq!(h, one);
}

// --- U(1) -------------------------------------------------------------------

/// A triangle 0, 1, 2. C¹ is ordered (0, 1), (0, 2), (1, 2), each edge
/// lower vertex first; the loop is 0 → 1 → 2 → 0, so its chain is
/// z = (+1, −1, +1): (0, 2) is walked against its orientation.
///
/// - Steps 1/3, 1/3, 1/3 around the loop: c(0, 2) = −1/3 ≡ 2/3, so
///   c = (1/3, 2/3, 1/3) and ⟨z, c⟩ = 1/3 − 2/3 + 1/3 = 0 (the loop sums to
///   1 ≡ 0). Exact: g = (0, 1/3, 2/3).
/// - Steps 1/3, 1/3, 1/2: c(0, 2) = −1/2 ≡ 1/2, so c = (1/3, 1/2, 1/3) and
///   ⟨z, c⟩ = 1/3 − 1/2 + 1/3 = 1/6 (the loop sums to 7/6 ≡ 1/6). Genuine.
#[test]
fn a_u1_twist_is_exact_when_its_loop_sums_to_a_whole_turn() {
    let g = Graph::new([], [(0, 1), (1, 2), (0, 2)]);

    let thirds = [q(1, 3), q(-1, 3), q(1, 3)];
    assert_eq!(thirds[1], q(2, 3), "−1/3 ≡ 2/3");
    let exact = potential(&witnessed("1/3 × 3", &(), &g, &thirds));
    assert_eq!(exact, vec![q(0, 1), q(1, 3), q(2, 3)]);

    let twisted = [q(1, 3), q(-1, 2), q(1, 3)];
    let (vertices, z, h) = certificate(&witnessed("1/3, 1/3, 1/2", &(), &g, &twisted));
    assert_eq!(vertices, vec![0, 1, 2, 0]);
    assert_eq!(z, ints(&[1, -1, 1]));
    assert_eq!(h, q(1, 6), "1/3 + 1/3 + 1/2 = 7/6 ≡ 1/6");
}

/// ℚ mod 1 reduces to ℤ/m, m the lcm of the denominators: δg ≡ c (mod 1)
/// is δg′ ≡ c′ (mod m) with c = c′/m. The verdicts agree, and ℤ/m's
/// witnesses map back by x ↦ x/m ([`ZMod::fraction`]) to ℚ mod 1's.
#[test]
fn rationals_mod_one_reduce_to_integers_mod_the_lcm() {
    let g = Graph::new([], [(0, 1), (1, 2), (0, 2)]);
    let cases: [(&str, [QModOne; 3], i64); 4] = [
        ("thirds", [q(1, 3), q(2, 3), q(1, 3)], 3),
        ("thirds and a half", [q(1, 3), q(1, 2), q(1, 3)], 6),
        ("quarters", [q(1, 4), q(3, 4), q(1, 2)], 4),
        ("integers: 0 mod 1", [q(0, 1), q(3, 1), q(-2, 1)], 2),
    ];
    for (name, c, m) in cases {
        let (n, reduced) = clear_denominators(&c);
        assert_eq!(n, modulus(m), "{name}: m = lcm (ℤ/1 is refused; 2 holds 0)");
        let back: Vec<QModOne> = reduced.iter().map(ZMod::fraction).collect();
        assert_eq!(back, c, "{name}: c = c′/m");

        let circle = witnessed(name, &(), &g, &c);
        let integers = witnessed(name, &n, &g, &reduced);
        match (&circle, &integers) {
            (Exactness::Exact(gq), Exactness::Exact(gz)) => {
                let back: Vec<QModOne> = gz.iter().map(ZMod::fraction).collect();
                assert_eq!(&back, gq, "{name}: g = g′/m");
            }
            (Exactness::Genuine(cq), Exactness::Genuine(cz)) => {
                assert_eq!(cq.len(), cz.len(), "{name}");
                for (a, b) in cq.iter().zip(cz) {
                    assert_eq!(a.chain(), b.chain(), "{name}: one cycle");
                    assert_eq!(a.pairing(), &b.pairing().fraction(), "{name}: ⟨z, c⟩");
                }
            }
            _ => panic!("{name}: ℚ mod 1 says {circle:?}, ℤ/{m} says {integers:?}"),
        }
    }
}

// --- Room and field agree ---------------------------------------------------

/// An edge and its integer transport.
type Transport = ((u64, u64), i64);

/// tests/one_graph.rs's multigraphs, given in C¹ order.
const CASES: &[(&str, &[Transport])] = &[
    ("2-loop, equal", &[((0, 1), 3), ((0, 1), 3)]),
    ("2-loop, apart", &[((0, 1), 3), ((0, 1), 5)]),
    ("self-loop at 0", &[((2, 2), 0), ((2, 3), 7)]),
    ("self-loop twisted", &[((2, 2), 1), ((2, 3), 7)]),
    (
        "triangle, a doubled side and a loop, flat",
        &[
            ((0, 1), 4),
            ((0, 1), 4),
            ((0, 2), 9),
            ((1, 2), 5),
            ((2, 2), 0),
        ],
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
    ),
    ("the tritone pair", &[((0, 1), 6), ((0, 1), -6)]),
];

/// check_flat's verdict on `field` against the room's on `c` over `ring`:
/// one verdict, one potential, one set of loops with one holonomy each
/// (`read` takes a field value into the ring). Returns whether exact.
fn agree<G, V>(
    name: &str,
    field: &Field<Graph, G>,
    ring: &V::Ring,
    c: &[V],
    read: impl Fn(&G) -> V,
) -> bool
where
    G: prismqueer::field::Exact,
    V: Exactly,
{
    let g = field.graph();
    let room = witnessed(name, ring, g, c);
    match (check_flat(field), room) {
        (Flatness::Flat(flat), Exactness::Exact(potential)) => {
            let field_potential: Vec<V> = flat.potential().values().map(&read).collect();
            assert_eq!(field_potential, potential, "{name}: one potential");
            true
        }
        (Flatness::Curved(curved), Exactness::Genuine(cycles)) => {
            assert!(curved.faces().is_empty(), "{name}: no faces");
            assert_eq!(
                curved.loops().len(),
                cycles.len(),
                "{name}: one set of loops"
            );
            for (l, z) in curved.loops().iter().zip(&cycles) {
                assert_eq!(l.vertices(), z.vertices(), "{name}: one loop");
                assert_eq!(l.edges(), z.edges(), "{name}: one loop, by edge");
                assert_eq!(&read(l.holonomy()), z.pairing(), "{name}: one holonomy");
            }
            false
        }
        (field, room) => panic!("{name}: field says {field:?}, room says {room:?}"),
    }
}

#[test]
fn the_room_solver_agrees_with_check_flat_over_z() {
    for &(name, given) in CASES {
        let field = Field::multigraph(given.iter().map(|&(e, x)| (e, Z::from(x))));
        let c: Vec<Z> = given.iter().map(|&(_, x)| Z::from(x)).collect();
        agree(name, &field, &(), &c, Z::clone);
    }
}

/// Over ℤ/N, field's const-generic `Zn<N>` against the room's `ZMod` with a
/// runtime modulus N.
fn agree_mod<const N: u64>() -> Vec<bool> {
    let n = modulus(N as i64);
    CASES
        .iter()
        .map(|&(name, given)| {
            let residue = |x: i64| x.rem_euclid(N as i64) as u64;
            let field =
                Field::multigraph(given.iter().map(|&(e, x)| (e, Zn::<N>::new(residue(x)))));
            let c: Vec<ZMod> = given.iter().map(|&(_, x)| ZMod::new(x, &n)).collect();
            let name = format!("{name}, mod {N}");
            agree(&name, &field, &n, &c, |x: &Zn<N>| ZMod::new(x.value(), &n))
        })
        .collect()
}

#[test]
fn the_room_solver_agrees_with_check_flat_over_z_mod_n() {
    // Over ℤ: which cases are exact.
    let over_z = [true, false, true, false, true, false, true, false, false];
    // 2-loop apart differs by 2; the curved triangle by 1; the triple edge
    // by 1; the tritone by 12; the twisted self-loop by 1.
    assert_eq!(
        agree_mod::<2>(),
        [true, true, true, false, true, false, true, false, true]
    );
    assert_eq!(
        agree_mod::<3>(),
        [true, false, true, false, true, false, true, false, true]
    );
    assert_eq!(
        agree_mod::<12>(),
        [true, false, true, false, true, false, true, false, true]
    );
    assert_eq!(agree_mod::<13>(), over_z, "13 divides none of the twists");
}

// --- The ring maps, and moduli that don't mix -------------------------------

#[test]
fn coboundary_is_one_map_in_every_ring() {
    let g = Graph::new([], [(0, 1), (0, 1), (1, 2), (2, 2), (0, 2)]);
    assert_eq!(coboundary_in::<Z>(&(), &g), coboundary(&g), "over ℤ, δ");
    for n in [2, 3, 12, 1 << 40] {
        let n = modulus(n);
        let delta = coboundary_in::<ZMod>(&n, &g);
        assert_eq!(delta, coboundary(&g).reduce(&n), "δ over ℤ/n is δ reduced");
        assert_eq!(delta.ring(), &n);
        // Reduction is a ring map: it commutes with composition.
        let l0 = delta.then(&delta.adjoint()).unwrap();
        let reduced = coboundary(&g)
            .then(&coboundary(&g).adjoint())
            .unwrap()
            .reduce(&n);
        assert_eq!(l0, reduced, "L₀ over ℤ/n is L₀ reduced");
        assert_eq!(
            laplacian(&g).charpoly().coefficients().len(),
            l0.from().dim() + 1,
            "one C⁰ under both"
        );
    }
}

#[test]
fn two_moduli_in_one_operation_are_refused() {
    let g = Graph::new([], [(0, 1), (0, 1)]);
    let (five, twelve) = (modulus(5), modulus(12));
    let mismatch = Err(RoomError::Modulus {
        expected: twelve.clone(),
        found: five.clone(),
    });
    let d12 = coboundary_in::<ZMod>(&twelve, &g);
    let d5 = coboundary_in::<ZMod>(&five, &g);
    assert_eq!(d12.then(&d5.adjoint()).map(|_| ()), mismatch);
    assert_eq!(d12.plus(&d5).map(|_| ()), mismatch);
    assert_eq!(d12.apply(&residues(&[1, 2], &five)).map(|_| ()), mismatch);
    assert_eq!(
        solve(&twelve, &g, &residues(&[6, 6], &five)).map(|_| ()),
        mismatch
    );
    assert_eq!(
        pair(&twelve, &ints(&[1, -1]), &residues(&[6, 6], &five)),
        mismatch.clone().map(|()| ZMod::new(0, &twelve))
    );
    // Mixed within one cochain.
    let mixed = [ZMod::new(6, &twelve), ZMod::new(1, &five)];
    assert_eq!(solve(&twelve, &g, &mixed).map(|_| ()), mismatch);
}

#[test]
fn ring_values_are_reduced_and_checked() {
    assert_eq!(Modulus::new(1), None, "ℤ/1 is the zero ring");
    assert_eq!(Modulus::new(0), None);
    assert_eq!(Modulus::new(-4), None);
    let twelve = modulus(12);
    assert_eq!(
        ZMod::new(-1, &twelve).residue(),
        &num_bigint::BigInt::from(11)
    );
    assert_eq!(ZMod::new(25, &twelve), ZMod::new(1, &twelve));
    assert_eq!(ZMod::new(-1, &twelve).modulus(), &twelve);
    assert_eq!(Z::from(-6).reduce(&twelve), ZMod::new(6, &twelve));

    assert_eq!(QModOne::new(1, 0), None, "no denominator");
    assert_eq!(q(4, 6), q(2, 3), "reduced");
    assert_eq!(q(-1, 3), q(2, 3), "0 ≤ num < den");
    assert_eq!(q(1, -3), q(2, 3));
    assert_eq!(q(7, 7), q(0, 1));
    let x = q(-5, 6);
    assert_eq!(
        (x.numerator(), x.denominator()),
        (&num_bigint::BigInt::from(1), &num_bigint::BigInt::from(6))
    );
    assert_eq!(ZMod::new(2, &modulus(6)).fraction(), q(1, 3));

    // The ℤ-action and the group law, as the solver uses them.
    assert_eq!(q(1, 3).add(&q(2, 3)), q(0, 1));
    assert_eq!(q(1, 3).neg(), q(2, 3));
    assert_eq!(q(1, 3).times(&num_bigint::BigInt::from(-2)), q(1, 3));
    assert_eq!(
        ZMod::new(5, &twelve).times(&num_bigint::BigInt::from(-3)),
        ZMod::new(9, &twelve)
    );
    assert_eq!(QModOne::zero(&()), q(0, 1));

    // A cochain of the wrong length.
    let g = Graph::new([], [(0, 1)]);
    assert_eq!(
        solve(&(), &g, &ints(&[1, 2])).map(|_| ()),
        Err(RoomError::Length {
            expected: 1,
            found: 2
        })
    );
    assert_eq!(
        pair(&(), &ints(&[1]), &ints(&[1, 2])),
        Err(RoomError::Length {
            expected: 1,
            found: 2
        })
    );
}

//! The kernel check (prism-019 ask 4; bullet-up-007 test 5 and new 8).
//!
//! Relations are exact transports on the edges of a multigraph over items;
//! a selection is a signed reading per item. The laws:
//!
//! 1. **In carries its certificate**: every relation, checked, and the
//!    selection's coordinates in the coset of consistent readings, which
//!    rebuild it exactly.
//! 2. **Out locates**: exactly the violated relations, each with what it
//!    required and what it found.
//! 3. **Curved relations need a declared ruling**, with witness cycles
//!    (δ*z = 0, ⟨z, c⟩ ≠ 0): never `Out`, and under every selection never
//!    `In`.
//! 4. **The consistent readings are data, never "settled"**: a particular
//!    reading plus one free direction per component. Two components give two
//!    directions; one direction is still a whole line of readings.
//! 5. **The ring decides**: the tritone pair (room_rings's fixture) needs a
//!    ruling over ℤ and has consistent readings over ℤ/12.

#![cfg(feature = "lapack")]

use std::collections::BTreeMap;

use prismqueer::agreement::{
    check, Checked, Consistent, Direction, Refused, Relations, Selection, SelectionDraft, Verdict,
};
use prismqueer::field::{Edge, Graph};
use prismqueer::oid::Oid;
use prismqueer::room::{coboundary, pair, Coefficient, Cycle, Modulus, RoomError, ZMod, Z};

fn z(x: i64) -> Z {
    Z::from(x)
}

fn signed<V: prismqueer::agreement::Reading>(readings: &[(u64, V)]) -> Selection<V> {
    SelectionDraft::new(readings.iter().cloned()).seal(Oid::new("adjudicator"))
}

fn ints(readings: &[(u64, i64)]) -> Selection<Z> {
    signed(&readings.iter().map(|&(v, x)| (v, z(x))).collect::<Vec<_>>())
}

/// A consistent triangle over ℤ: 0 → 1 is +3, 1 → 2 is +4, 0 → 2 is +7.
fn triangle() -> Relations<Z> {
    Relations::new(&(), [], [(0, 1, z(3)), (1, 2, z(4)), (0, 2, z(7))]).unwrap()
}

/// Law 1, checked independently of the verdict: every relation of the
/// graph appears once, holds (s(b) − s(a) = c(e), a < b), and the
/// coordinates rebuild the selection from the coset.
fn certified<V: prismqueer::agreement::Reading>(
    relations: &Relations<V>,
    selection: &Selection<V>,
) -> Vec<V> {
    let verdict = check(relations, selection).expect("checkable");
    let Verdict::In(certificate) = verdict else {
        panic!("expected In, got {verdict:?}");
    };
    let s = selection.readings();
    let edges: Vec<Edge> = relations.graph().edges().collect();
    let checked: Vec<Edge> = certificate.checked().iter().map(Checked::edge).collect();
    assert_eq!(checked, edges, "every relation, in order, once");
    for (c, required) in certificate.checked().iter().zip(relations.required()) {
        let (a, b) = c.edge().ends();
        assert_eq!(c.required(), required, "what the relation requires");
        assert_eq!(c.found(), &s[&b].add(&s[&a].neg()), "δs on this edge");
        assert_eq!(c.found(), c.required(), "and it holds");
    }
    let Consistent::Readings { particular, free } = relations.consistent().unwrap() else {
        panic!("In, so the consistent readings aren't empty");
    };
    assert_eq!(
        certificate.coordinates().len(),
        free.len(),
        "one per direction"
    );
    let mut rebuilt = particular.clone();
    for (direction, shift) in free.iter().zip(certificate.coordinates()) {
        for item in direction.items() {
            let r = rebuilt.get_mut(item).unwrap();
            *r = r.add(shift);
        }
    }
    assert_eq!(
        &rebuilt, s,
        "particular + Σ coordinates · free = the selection"
    );
    certificate.coordinates().to_vec()
}

/// Law 2: exactly these edges are violated, with their required and found
/// values.
fn violated<V: prismqueer::agreement::Reading>(
    relations: &Relations<V>,
    selection: &Selection<V>,
) -> Vec<(Edge, V, V)> {
    let verdict = check(relations, selection).expect("checkable");
    let Verdict::Out(broken) = verdict else {
        panic!("expected Out, got {verdict:?}");
    };
    assert!(!broken.is_empty(), "Out is never empty");
    broken
        .iter()
        .map(|c| {
            assert_ne!(c.found(), c.required(), "{:?} is violated", c.edge());
            (c.edge(), c.required().clone(), c.found().clone())
        })
        .collect()
}

/// Law 3's witness: each cycle is a cycle of the graph whose pairing with
/// the required values is nonzero.
fn witnesses<V: prismqueer::agreement::Reading>(relations: &Relations<V>) -> Vec<Cycle<V>> {
    let Consistent::Empty(cycles) = relations.consistent().unwrap() else {
        panic!("expected no consistent readings");
    };
    assert!(!cycles.is_empty(), "never empty");
    let g = relations.graph();
    for cycle in &cycles {
        assert_eq!(
            coboundary(g).adjoint().apply(cycle.chain()).unwrap(),
            vec![z(0); g.vertex_count()],
            "δ*z = 0"
        );
        let twist = pair(relations.ring(), cycle.chain(), relations.required()).unwrap();
        assert_eq!(&twist, cycle.pairing(), "⟨z, c⟩");
        assert_ne!(
            twist,
            V::zero(relations.ring()),
            "≠ 0: no reading closes it"
        );
    }
    cycles
}

fn needs_ruling<V: prismqueer::agreement::Reading>(
    relations: &Relations<V>,
    selection: &Selection<V>,
) -> Vec<Cycle<V>> {
    match check(relations, selection).expect("checkable") {
        Verdict::NeedsRuling(cycles) => cycles,
        other => panic!("curved relations need a ruling, got {other:?}"),
    }
}

fn items(directions: &[Direction]) -> Vec<Vec<u64>> {
    directions.iter().map(|d| d.items().to_vec()).collect()
}

// --- A consistent triangle ----------------------------------------------------

#[test]
fn a_selection_in_the_kernel_is_in_with_its_certificate() {
    let relations = triangle();
    let selection = ints(&[(0, 10), (1, 13), (2, 17)]);
    assert_eq!(certified(&relations, &selection), vec![z(10)]);
}

#[test]
fn off_by_one_on_one_item_is_out_with_exactly_its_edges() {
    let relations = triangle();
    let selection = ints(&[(0, 10), (1, 14), (2, 17)]);
    assert_eq!(
        violated(&relations, &selection),
        vec![
            (Edge::between(0, 1, 0), z(3), z(4)),
            (Edge::between(1, 2, 0), z(4), z(3)),
        ],
        "both relations at item 1, and not 0 → 2"
    );
}

#[test]
fn the_consistent_readings_are_a_coset_not_a_point() {
    let Consistent::Readings { particular, free } = triangle().consistent().unwrap() else {
        panic!("a consistent triangle has readings");
    };
    assert_eq!(
        particular,
        BTreeMap::from([(0, z(0)), (1, z(3)), (2, z(7))]),
        "0 at the component's smallest item"
    );
    assert_eq!(items(&free), vec![vec![0, 1, 2]], "one component");
    // One free direction is not "settled": two different selections are both
    // inside.
    let relations = triangle();
    for shift in [-5, 0, 1, 1000] {
        let selection = ints(&[(0, shift), (1, shift + 3), (2, shift + 7)]);
        assert_eq!(certified(&relations, &selection), vec![z(shift)]);
    }
}

// --- Curved relations -----------------------------------------------------------

#[test]
fn curved_relations_over_z_need_a_ruling_under_every_selection() {
    // 3 + 4 ≠ 8: the loop 0 → 1 → 2 → 0 has twist ±1.
    let relations = Relations::new(&(), [], [(0, 1, z(3)), (1, 2, z(4)), (0, 2, z(8))]).unwrap();
    let cycles = witnesses(&relations);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].vertices().first(), cycles[0].vertices().last());
    assert_eq!(cycles[0].edges().len(), 3, "around the triangle");
    let twist = cycles[0].pairing();
    assert!(twist == &z(1) || twist == &z(-1), "{twist:?}");
    for a in -3..=3 {
        for b in -3..=3 {
            for c in -3..=3 {
                let selection = ints(&[(0, a), (1, b), (2, c)]);
                assert_eq!(
                    needs_ruling(&relations, &selection),
                    cycles,
                    "({a}, {b}, {c})"
                );
            }
        }
    }
}

#[test]
fn an_odd_loop_over_z2_needs_a_ruling_under_every_selection() {
    let two = Modulus::new(2).unwrap();
    let one = || ZMod::new(1, &two);
    // Three `not`s around a triangle: Harary's unbalanced loop.
    let relations =
        Relations::new(&two, [], [(0, 1, one()), (1, 2, one()), (2, 0, one())]).unwrap();
    let cycles = witnesses(&relations);
    assert_eq!(cycles[0].pairing(), &one());
    for bits in 0..8 {
        let r = |k: u64| ZMod::new((bits >> k) & 1, &two);
        let selection = signed(&[(0, r(0)), (1, r(1)), (2, r(2))]);
        assert_eq!(needs_ruling(&relations, &selection), cycles, "{bits:03b}");
    }
}

// --- Two components -------------------------------------------------------------

/// The triangle, and apart from it 3 → 4 at +5.
fn two_components() -> Relations<Z> {
    Relations::new(
        &(),
        [],
        [(0, 1, z(3)), (1, 2, z(4)), (0, 2, z(7)), (3, 4, z(5))],
    )
    .unwrap()
}

#[test]
fn two_components_are_two_free_directions_not_unsettled() {
    let relations = two_components();
    let Consistent::Readings { particular, free } = relations.consistent().unwrap() else {
        panic!("consistent");
    };
    assert_eq!(
        items(&free),
        vec![vec![0, 1, 2], vec![3, 4]],
        "two directions"
    );
    assert_eq!(
        particular,
        BTreeMap::from([(0, z(0)), (1, z(3)), (2, z(7)), (3, z(0)), (4, z(5))])
    );

    let selection = ints(&[(0, 10), (1, 13), (2, 17), (3, -2), (4, 3)]);
    assert_eq!(certified(&relations, &selection), vec![z(10), z(-2)]);
    // Each direction shifts on its own: moving the second component leaves
    // the first coordinate.
    let moved = ints(&[(0, 10), (1, 13), (2, 17), (3, 40), (4, 45)]);
    assert_eq!(certified(&relations, &moved), vec![z(10), z(40)]);
}

#[test]
fn a_selection_is_checked_per_component() {
    let relations = two_components();
    let second_off = ints(&[(0, 10), (1, 13), (2, 17), (3, -2), (4, 4)]);
    assert_eq!(
        violated(&relations, &second_off),
        vec![(Edge::between(3, 4, 0), z(5), z(6))],
        "only the second component's relation"
    );
    let first_off = ints(&[(0, 11), (1, 13), (2, 17), (3, -2), (4, 3)]);
    assert_eq!(
        violated(&relations, &first_off),
        vec![
            (Edge::between(0, 1, 0), z(3), z(2)),
            (Edge::between(0, 2, 0), z(7), z(6)),
        ],
        "only the first component's relations at item 0"
    );
}

#[test]
fn an_item_with_no_relations_is_its_own_direction() {
    let relations = Relations::new(&(), [7], [(0, 1, z(3))]).unwrap();
    let Consistent::Readings { particular, free } = relations.consistent().unwrap() else {
        panic!("consistent");
    };
    assert_eq!(items(&free), vec![vec![0, 1], vec![7]]);
    assert_eq!(particular[&7], z(0));
    let selection = ints(&[(0, 1), (1, 4), (7, 99)]);
    assert_eq!(certified(&relations, &selection), vec![z(1), z(99)]);
}

// --- The ring decides: the tritone pair -------------------------------------------

/// room_rings's fixture: two tritones up, a → b → a, on parallel edges
/// (0, 1). In canonical orientation c = (6, −6), and ⟨z, c⟩ = 12, the
/// octave.
fn tritones<V: prismqueer::agreement::Reading>(ring: &V::Ring, six: V) -> Relations<V> {
    Relations::new(ring, [], [(0, 1, six.clone()), (1, 0, six)]).unwrap()
}

#[test]
fn the_tritone_pair_needs_a_ruling_over_z_and_has_readings_mod_12() {
    let over_z = tritones(&(), z(6));
    assert_eq!(over_z.graph(), &Graph::new([], [(0, 1), (0, 1)]));
    assert_eq!(over_z.required(), &[z(6), z(-6)], "room_rings's c");
    let cycles = witnesses(&over_z);
    assert_eq!(cycles[0].pairing(), &z(12), "the octave");
    assert_eq!(
        needs_ruling(&over_z, &ints(&[(0, 0), (1, 6)])),
        cycles,
        "even the reading that is right mod 12"
    );

    let twelve = Modulus::new(12).unwrap();
    let m = |x: i64| ZMod::new(x, &twelve);
    let mod_12 = tritones(&twelve, m(6));
    assert_eq!(mod_12.required(), &[m(6), m(6)], "−6 ≡ 6");
    let Consistent::Readings { particular, free } = mod_12.consistent().unwrap() else {
        panic!("12 ≡ 0: the octave closes");
    };
    assert_eq!(particular, BTreeMap::from([(0, m(0)), (1, m(6))]));
    assert_eq!(items(&free), vec![vec![0, 1]]);
    assert_eq!(
        certified(&mod_12, &signed(&[(0, m(0)), (1, m(6))])),
        vec![m(0)]
    );
    assert_eq!(
        certified(&mod_12, &signed(&[(0, m(3)), (1, m(9))])),
        vec![m(3)]
    );
    assert_eq!(
        violated(&mod_12, &signed(&[(0, m(0)), (1, m(7))])),
        vec![
            (Edge::between(0, 1, 0), m(6), m(7)),
            (Edge::between(0, 1, 1), m(6), m(7)),
        ]
    );
}

// --- Refusals and the address -----------------------------------------------------

#[test]
fn a_selection_must_read_exactly_the_items_in_the_ring() {
    let relations = triangle();
    assert_eq!(
        check(&relations, &ints(&[(0, 0), (1, 3)])),
        Err(Refused::Missing(2))
    );
    assert_eq!(
        check(&relations, &ints(&[(0, 0), (1, 3), (2, 7), (5, 0)])),
        Err(Refused::Unknown(5))
    );
    let twelve = Modulus::new(12).unwrap();
    let five = Modulus::new(5).unwrap();
    let mod_12 = tritones(&twelve, ZMod::new(6, &twelve));
    assert!(matches!(
        check(
            &mod_12,
            &signed(&[(0, ZMod::new(0, &five)), (1, ZMod::new(1, &five))])
        ),
        Err(Refused::Room(RoomError::Modulus { .. }))
    ));
    assert!(matches!(
        Relations::new(&twelve, [], [(0, 1, ZMod::new(1, &five))]),
        Err(RoomError::Modulus { .. })
    ));
}

#[test]
fn the_signer_sits_beside_the_selections_address() {
    let draft = SelectionDraft::new([(0, z(1)), (1, z(4))]);
    let address = draft.oid();
    let selection = draft.clone().seal(Oid::new("alex"));
    assert_eq!(selection.oid(), address);
    assert_eq!(selection.signer().over(), &address);
    assert_eq!(selection.signer().by(), &Oid::new("alex"));
    let other = draft.seal(Oid::new("mara"));
    assert_eq!(
        other.oid(),
        selection.oid(),
        "the signer isn't in the address"
    );
    assert_ne!(other, selection, "but it is beside it");
    assert_ne!(
        SelectionDraft::new([(0, z(1)), (1, z(5))]).oid(),
        address,
        "another reading, another address"
    );
    let twelve = Modulus::new(12).unwrap();
    let ring = SelectionDraft::new([(0, ZMod::new(1, &twelve)), (1, ZMod::new(4, &twelve))]);
    assert_ne!(ring.oid(), address, "the ring is in the address");
    let fractions = SelectionDraft::new([
        (0, prismqueer::room::QModOne::new(1, 12).unwrap()),
        (1, prismqueer::room::QModOne::new(4, 12).unwrap()),
    ]);
    assert_ne!(fractions.oid(), ring.oid());
    assert!(draft_preimage_starts_with_the_domain(&fractions));
}

fn draft_preimage_starts_with_the_domain<V: prismqueer::agreement::Reading>(
    draft: &SelectionDraft<V>,
) -> bool {
    draft.preimage().starts_with(b"selection\0")
}

#[test]
fn readings_over_q_mod_1_are_checked_too() {
    let q = |n: i64, d: i64| prismqueer::room::QModOne::new(n, d).unwrap();
    // Three thirds of a turn close the loop mod 1.
    let relations =
        Relations::new(&(), [], [(0, 1, q(1, 3)), (1, 2, q(1, 3)), (2, 0, q(1, 3))]).unwrap();
    let selection = signed(&[(0, q(0, 1)), (1, q(1, 3)), (2, q(2, 3))]);
    assert_eq!(certified(&relations, &selection), vec![q(0, 1)]);
    // Two halves and a third don't.
    let curved = Relations::new(&(), [], [(0, 1, q(1, 2)), (1, 0, q(1, 3))]).unwrap();
    witnesses(&curved);
    assert!(matches!(
        check(&curved, &signed(&[(0, q(0, 1)), (1, q(1, 2))])),
        Ok(Verdict::NeedsRuling(_))
    ));
}

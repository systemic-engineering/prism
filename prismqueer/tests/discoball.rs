//! prism-012 Ask 3: the disco ball, a small instrument over `Room`.
//!
//! Facets are Prisms' readings on one room H over ℤ. The ball rotates by a
//! basis change over that fixed ring (a signed permutation: the unitaries
//! of ℤⁿ), never by a ring change. Per facet it reports the measurement
//! triple (Mara, prism-012 Log (d)):
//!
//! 1. **lums**, tr(P_S) for the lit set S at each step boundary: how much
//!    light;
//! 2. **schedule-holonomy**, the horizontal twist: the ℤ/2 holonomy of the
//!    signed dependency cycles, from room's exact solver, certificate and
//!    all;
//! 3. **findings**, the vertical twist: what a caller-supplied ruler finds
//!    violated in the run's true set. prism carries them, never decides.
//!
//! plus the spectrum exactly (the charpoly) and the invariant factors,
//! which catch the shear the charpoly misses. Received darkness lives in
//! the ball's outer channel, never in a facet's triple.

#![cfg(all(feature = "lapack", feature = "bundle"))]

use std::collections::BTreeSet;

use prismqueer::brain::{Brain, Capacity, Memory};
use prismqueer::dependency::{DependencyGraph, Sign};
use prismqueer::discoball::{
    Ball, BallError, DarkSpot, Exit, Facet, FacetReport, Proof, Received, Rotation, Ruler,
};
use prismqueer::room::{coboundary, pair, Exactness, Map, Modulus, Room, Vertices, ZMod, Z};
use prismqueer::{IdentityPrism, Optic, Prism};

type Set = BTreeSet<u32>;

/// A Prism nobody wrote a `Stable` proof for. It passes its set through.
struct Unproved;

impl Prism for Unproved {
    type Input = Optic<(), Set>;
    type Focused = Optic<Set, Set>;
    type Projected = Optic<Set, Set>;
    type Refracted = Optic<Set, Set>;
    fn focus(&self, b: Self::Input) -> Self::Focused {
        let s = b.value().cloned().unwrap_or_default();
        b.next(s)
    }
    fn project(&self, b: Self::Focused) -> Self::Projected {
        let s = b.value().cloned().unwrap_or_default();
        b.next(s)
    }
    fn settle(&self, b: Self::Projected) -> Self::Refracted {
        let s = b.value().cloned().unwrap_or_default();
        b.next(s)
    }
}

/// A ruler that declares nothing: no findings, ever.
struct NoDeclarations;

impl Ruler for NoDeclarations {
    type Finding = ();
    fn findings(&self, _: &Facet) -> Vec<()> {
        Vec::new()
    }
}

fn room(n: u64) -> Room<Vertices> {
    Room::over(&(0..n).collect())
}

fn matrix(room: &Room<Vertices>, rows: &[&[i64]]) -> Map<Vertices, Vertices, Z> {
    let rows: Vec<Vec<Z>> = rows
        .iter()
        .map(|r| r.iter().map(|&x| Z::from(x)).collect())
        .collect();
    Map::from_rows(room, room, &rows).expect("square, on the room")
}

fn rules(edges: &[(usize, usize, Sign)]) -> DependencyGraph<usize> {
    let mut g = DependencyGraph::new();
    for &(premise, head, sign) in edges {
        g.add_edge(premise, head, sign);
    }
    g
}

fn set(xs: &[usize]) -> BTreeSet<usize> {
    xs.iter().copied().collect()
}

fn two() -> Modulus {
    Modulus::new(2).expect("2 ≥ 2")
}

/// The cyclic rotation i ↦ i + 1 mod n.
fn cycle(room: &Room<Vertices>) -> Rotation {
    let n = room.dim();
    let image: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
    Rotation::permutation(room, &image).expect("a permutation")
}

/// Check a horizontal twist's certificates in the room: each cycle z is
/// closed (δ*z = 0, so ⟨z, δg⟩ = 0 for every g) and pairs to 1 in ℤ/2 with
/// the facet's signed cochain. Returns the number of certificates.
fn checked_twist<F>(report: &FacetReport<F>) -> usize {
    let holonomy = report.triple().holonomy();
    let cycles = match holonomy.verdict() {
        Exactness::Genuine(cycles) => cycles,
        Exactness::Exact(g) => panic!("{}: expected a twist, got the choice {g:?}", report.name()),
    };
    let one = ZMod::new(1, &two());
    let closed = coboundary(holonomy.graph()).adjoint();
    for z in cycles {
        assert_eq!(z.pairing(), &one, "a twist is a half-turn");
        let paired = pair(&two(), z.chain(), holonomy.cochain()).expect("a cochain of the graph");
        assert_eq!(paired, one, "the certificate pairs to 1 with the signs");
        let boundary = closed.apply(z.chain()).expect("a chain of the graph");
        assert!(
            boundary.iter().all(|x| x == &Z::from(0)),
            "the certificate is a cycle"
        );
    }
    cycles.len()
}

// --- 1. A full turn ----------------------------------------------------------

/// A ball on four spots with two facets: a stable one (identity operator,
/// lit {0} then {0, 1}, a positive loop) and an unproved one (a shear block,
/// lit {2} then {2, 3}, an odd `not` loop).
fn ball_of_two() -> Ball {
    let h = room(4);
    let mut ball = Ball::new(&h);
    let mut lit = Facet::stable(
        &IdentityPrism::<Set>::new(),
        "lit",
        Map::identity(&h),
        rules(&[(0, 1, Sign::Positive), (1, 0, Sign::Positive)]),
    )
    .expect("no `not`");
    lit.step([0]).step([0, 1]);
    let shear = matrix(
        &h,
        &[&[1, 1, 0, 0], &[0, 1, 0, 0], &[0, 0, 2, 0], &[0, 0, 0, 3]],
    );
    let mut twisted = Facet::unproved(
        &Unproved,
        "twisted",
        shear,
        rules(&[(2, 3, Sign::Negative), (3, 2, Sign::Positive)]),
    );
    twisted.step([2]).step([2, 3]);
    ball.facet(lit).expect("on the room");
    ball.facet(twisted).expect("on the room");
    ball
}

/// Rotate through a full turn of `r`, `order` steps, checking at every step
/// that lums, the exact spectrum, the invariant factors, the twists and the
/// findings are unchanged, and the spots moved by the turn so far. After the
/// last step every spot is home.
fn full_turn(r: &Rotation, order: usize) {
    let mut ball = ball_of_two();
    let home = ball.measure(&NoDeclarations);
    assert_eq!(home.lums(), 4, "two facets, two spots each");
    let mut moved: Vec<usize> = (0..4).collect();
    for step in 1..=order {
        ball.rotate(r).expect("a rotation of the ball's room");
        moved = moved.iter().map(|&i| r.image()[i]).collect();
        let now = ball.measure(&NoDeclarations);
        assert_eq!(now.lums(), home.lums(), "step {step}: total lums");
        for (a, b) in home.facets().iter().zip(now.facets()) {
            let at = format!("step {step}, {}", a.name());
            assert_eq!(a.charpoly(), b.charpoly(), "{at}: the exact spectrum");
            assert_eq!(a.invariants(), b.invariants(), "{at}: the invariant factors");
            assert_eq!(a.triple().lums_trace(), b.triple().lums_trace(), "{at}: lums");
            assert_eq!(
                a.triple().horizontal(),
                b.triple().horizontal(),
                "{at}: the horizontal twist"
            );
            assert_eq!(a.triple().findings(), b.triple().findings(), "{at}: findings");
            assert_eq!(a.exit(), b.exit(), "{at}: exit");
            let expected: BTreeSet<usize> = a.spots().iter().map(|&i| moved[i]).collect();
            assert_eq!(b.spots(), &expected, "{at}: the spots moved by the turn");
        }
        if step < order {
            assert_ne!(now, home, "step {step}: the ball moved");
        }
    }
    let back = ball.measure(&NoDeclarations);
    assert_eq!(back, home, "a full turn returns every spot");
    assert!(
        moved.iter().enumerate().all(|(i, &j)| i == j),
        "the turn is whole"
    );
}

#[test]
fn a_full_turn_of_a_permutation_keeps_lums_and_spectrum_and_returns_every_spot() {
    let h = room(4);
    full_turn(&cycle(&h), 4);
}

/// A unitary of ℤ⁴ that isn't a permutation: the cycle with one sign
/// flipped. Its fourth power is −I, which conjugates to nothing, so the
/// spots are home after four steps, while the matrices in between carry
/// signs.
#[test]
fn a_full_turn_of_a_signed_permutation_keeps_lums_and_spectrum_and_returns_every_spot() {
    let h = room(4);
    let signed = matrix(
        &h,
        &[&[0, 0, 0, -1], &[1, 0, 0, 0], &[0, 1, 0, 0], &[0, 0, 1, 0]],
    );
    let r = Rotation::new(&signed).expect("a signed permutation");
    assert_eq!(r.image(), &[1, 2, 3, 0]);
    full_turn(&r, 4);
}

// --- 2. A horizontal twisted facet -------------------------------------------

/// A one-facet ball on three spots, all lit, with the given rules.
fn one_facet(rule_list: &[(usize, usize, Sign)]) -> Ball {
    let h = room(3);
    let mut ball = Ball::new(&h);
    let mut f = Facet::unproved(&Unproved, "rules", Map::identity(&h), rules(rule_list));
    f.step([0]).step([0, 1, 2]);
    ball.facet(f).expect("on the room");
    ball
}

#[test]
fn an_odd_not_loop_is_a_horizontal_twist_at_exit_one_and_not_darkness() {
    use Sign::{Negative, Positive};
    // p :- not q. q :- p. One `not` around the loop.
    let ball = one_facet(&[(1, 0, Negative), (0, 1, Positive)]);
    let reading = ball.measure(&NoDeclarations);
    let report = &reading.facets()[0];
    assert!(report.triple().horizontal(), "an odd `not` loop twists");
    assert_eq!(checked_twist(report), 1, "one loop, one certificate");
    assert!(report.triple().findings().is_empty(), "no declaration");
    assert_eq!(report.exit(), Exit::Uncertain, "twisted light, Partial");
    assert_eq!(report.exit().code(), 1);
    assert_eq!(report.triple().lums(), 3, "twisted light is still light");
    assert!(reading.dark().is_empty(), "a twist is not darkness");

    // Three `not`s around p, q, r: odd, a twist.
    let ball = one_facet(&[(0, 1, Negative), (1, 2, Negative), (2, 0, Negative)]);
    let reading = ball.measure(&NoDeclarations);
    assert_eq!(checked_twist(&reading.facets()[0]), 1);
    assert_eq!(reading.facets()[0].exit(), Exit::Uncertain);
}

#[test]
fn an_even_not_loop_is_a_choice_with_trivial_holonomy() {
    use Sign::{Negative, Positive};
    // p :- not q. q :- not p. Two `not`s: a choice, p and q opposite.
    let ball = one_facet(&[(1, 0, Negative), (0, 1, Negative)]);
    let reading = ball.measure(&NoDeclarations);
    let report = &reading.facets()[0];
    assert!(!report.triple().horizontal(), "an even loop is a choice");
    match report.triple().holonomy().verdict() {
        Exactness::Exact(choice) => {
            let two = two();
            assert_eq!(choice, &vec![ZMod::new(0, &two), ZMod::new(1, &two)], "p ≠ q");
        }
        Exactness::Genuine(cycles) => panic!("expected a choice, got {cycles:?}"),
    }
    assert_eq!(report.exit(), Exit::Clear);
    assert_eq!(report.exit().code(), 0);

    // Two `not`s and a plain dependency around p, q, r: still a choice.
    let ball = one_facet(&[(0, 1, Negative), (1, 2, Positive), (2, 0, Negative)]);
    assert_eq!(ball.measure(&NoDeclarations).facets()[0].exit(), Exit::Clear);
}

/// A stratified `not` closes no directed loop, so it never twists, even
/// when its undirected shadow has an odd cycle: q :- not p, r :- q, r :- p.
/// The holonomy is read on the dependency cycles only.
#[test]
fn a_stratified_not_with_an_odd_undirected_shadow_is_no_twist() {
    use Sign::{Negative, Positive};
    let ball = one_facet(&[(0, 1, Negative), (1, 2, Positive), (0, 2, Positive)]);
    let reading = ball.measure(&NoDeclarations);
    let report = &reading.facets()[0];
    assert!(!report.triple().horizontal(), "no loop, no twist");
    assert_eq!(report.triple().holonomy().graph().edges().count(), 0);
    assert_eq!(report.exit(), Exit::Clear);
}

#[test]
fn a_stable_facet_refuses_a_not() {
    let h = room(2);
    let refused = Facet::stable(
        &IdentityPrism::<Set>::new(),
        "claims stable",
        Map::identity(&h),
        rules(&[(0, 1, Sign::Negative)]),
    );
    assert_eq!(refused.err(), Some(BallError::NotInStable { premise: 0, head: 1 }));
}

// --- 3. A vertical twisted facet: mirror's pos-v05 ---------------------------

/// pos-v05 (mirror `tests/checker/fixtures/pos-v05`):
///
/// ```text
/// rule  owner  owner(?d, ?p) requires claim(?d, ?p)
/// fact  claim(doc, alex)
/// fact  claim(doc, reed)
/// functional owner(1) -> 2
/// ```
///
/// Positive, no loop, and still a paradox: the declaration is violated in
/// the true set. Its control has one claim and is clear.
const ATOMS: [(&str, &str, &str); 4] = [
    ("claim", "doc", "alex"),
    ("claim", "doc", "reed"),
    ("owner", "doc", "alex"),
    ("owner", "doc", "reed"),
];

/// `functional owner(1) -> 2`: no two `owner` atoms in the true set share a
/// first argument with different seconds. A finding names the two.
struct Functional;

impl Ruler for Functional {
    type Finding = (usize, usize);
    fn findings(&self, facet: &Facet) -> Vec<(usize, usize)> {
        let owners: Vec<usize> = facet
            .true_set()
            .into_iter()
            .filter(|&i| ATOMS[i].0 == "owner")
            .collect();
        let mut found = Vec::new();
        for (k, &a) in owners.iter().enumerate() {
            for &b in &owners[k + 1..] {
                if ATOMS[a].1 == ATOMS[b].1 && ATOMS[a].2 != ATOMS[b].2 {
                    found.push((a, b));
                }
            }
        }
        found
    }
}

/// Run the owner rule semi-naively in a `Brain` over the given claims, one
/// layer per commit, and record each committed layer as a facet step. The
/// facet's lums are checked against the brain's at every step boundary.
fn pos_v05(claims: &[usize]) -> Facet {
    let h = room(4);
    let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
    let claim = brain.relation("claim", 2);
    let owner = brain.relation("owner", 2);
    let mut facet = Facet::unproved(
        &Unproved,
        "pos-v05",
        Map::identity(&h),
        rules(&[(0, 2, Sign::Positive), (1, 3, Sign::Positive)]),
    );
    let mut lit = Vec::new();
    let mut brain_lums = Vec::new();
    for &i in claims {
        let t = [brain.intern(ATOMS[i].1.as_bytes()), brain.intern(ATOMS[i].2.as_bytes())];
        assert!(brain.deposit(claim, &t));
        lit.push(i);
    }
    brain.commit();
    facet.step(lit.clone());
    brain_lums.push(brain.lums());
    for &i in claims {
        let t = [brain.intern(ATOMS[i].1.as_bytes()), brain.intern(ATOMS[i].2.as_bytes())];
        assert!(brain.deposit(owner, &t), "owner from claim");
        lit.push(i + 2);
    }
    brain.commit();
    facet.step(lit);
    brain_lums.push(brain.lums());

    let mut ball = Ball::new(&h);
    ball.facet(facet.clone()).expect("on the room");
    let reading = ball.measure(&Functional);
    assert_eq!(reading.facets()[0].triple().lums_trace(), &brain_lums[..]);
    facet
}

#[test]
fn a_violated_declaration_is_a_vertical_twist_with_trivial_holonomy() {
    let h = room(4);
    let mut ball = Ball::new(&h);
    ball.facet(pos_v05(&[0, 1])).expect("on the room");
    let reading = ball.measure(&Functional);
    let report = &reading.facets()[0];
    let triple = report.triple();
    assert_eq!(triple.lums_trace(), &[2, 4], "two claims, then two owners");
    assert_eq!(triple.first_drop(), None, "monotone");
    assert!(!triple.horizontal(), "positive: trivial holonomy");
    assert_eq!(triple.findings(), &[(2, 3)], "owner(doc, alex) and owner(doc, reed)");
    assert!(triple.vertical());
    assert_eq!(report.exit(), Exit::Paradox);
    assert_eq!(report.exit().code(), 255);
    assert_eq!(report.proof(), Proof::Unproved, "a declaration: not stable");
    assert!(reading.dark().is_empty(), "a paradox is twisted light, not darkness");

    // The control: one claim, the declaration holds.
    let mut ball = Ball::new(&h);
    ball.facet(pos_v05(&[0])).expect("on the room");
    let reading = ball.measure(&Functional);
    let report = &reading.facets()[0];
    assert_eq!(report.triple().lums_trace(), &[1, 2]);
    assert!(report.triple().findings().is_empty());
    assert_eq!(report.exit(), Exit::Clear);
}

/// The findings are about atoms, not positions: the ruler reads the true set
/// in the facet's own chart, so turning the ball never moves a finding.
#[test]
fn findings_do_not_turn_with_the_ball() {
    let h = room(4);
    let mut ball = Ball::new(&h);
    ball.facet(pos_v05(&[0, 1])).expect("on the room");
    ball.rotate(&cycle(&h)).expect("a rotation");
    let reading = ball.measure(&Functional);
    assert_eq!(reading.facets()[0].triple().findings(), &[(2, 3)]);
    assert_eq!(reading.facets()[0].spots(), &set(&[3, 0, 1, 2]));
}

#[test]
fn lums_drop_is_reported_at_its_step() {
    let h = room(3);
    let mut f = Facet::unproved(&Unproved, "retracts", Map::identity(&h), rules(&[]));
    f.step([0, 1]).step([0, 1, 2]).step([2]);
    let mut ball = Ball::new(&h);
    ball.facet(f).expect("on the room");
    let reading = ball.measure(&NoDeclarations);
    assert_eq!(reading.facets()[0].triple().lums_trace(), &[2, 3, 1]);
    assert_eq!(reading.facets()[0].triple().first_drop(), Some(2));
}

// --- 4. A dark spot ----------------------------------------------------------

#[test]
fn received_darkness_is_in_the_outer_channel_and_changes_no_lums() {
    let lit = ball_of_two();
    let mut dark = ball_of_two();
    dark.receive(1, Received::Hole).expect("a spot of the ball");
    let (a, b) = (lit.measure(&NoDeclarations), dark.measure(&NoDeclarations));
    assert_eq!(b.facets(), a.facets(), "no facet's triple sees the darkness");
    assert_eq!(b.lums(), a.lums(), "darkness changes no lums");
    assert!(a.dark().is_empty());
    assert_eq!(b.dark(), &[DarkSpot::new(1, Received::Hole)]);

    // A dark spot is on the ball: it turns with it, and stays outside.
    dark.rotate(&cycle(&room(4))).expect("a rotation");
    dark.receive(0, Received::Budget).expect("a spot of the ball");
    let turned = dark.measure(&NoDeclarations);
    assert_eq!(
        turned.dark(),
        &[DarkSpot::new(2, Received::Hole), DarkSpot::new(0, Received::Budget)]
    );
    assert_eq!(turned.lums(), a.lums());

    assert_eq!(
        dark.receive(4, Received::Unreadable),
        Err(BallError::Outside { spot: 4, dim: 4 })
    );
}

// --- 5. The shear ------------------------------------------------------------

#[test]
fn the_invariant_factors_tell_a_shear_from_the_identity() {
    let h = room(2);
    let mut ball = Ball::new(&h);
    let mut identity = Facet::unproved(&Unproved, "identity", Map::identity(&h), rules(&[]));
    identity.step([0]);
    let mut shear = Facet::unproved(&Unproved, "shear", matrix(&h, &[&[1, 1], &[0, 1]]), rules(&[]));
    shear.step([0]);
    ball.facet(identity).expect("on the room");
    ball.facet(shear).expect("on the room");

    for turn in 0..2 {
        let reading = ball.measure(&NoDeclarations);
        let [id, sh] = reading.facets() else {
            panic!("two facets")
        };
        // (x − 1)² = x² − 2x + 1, leading first, for both.
        let expected: Vec<Z> = [1, -2, 1].into_iter().map(Z::from).collect();
        assert_eq!(id.charpoly().coefficients(), &expected[..], "turn {turn}");
        assert_eq!(id.charpoly(), sh.charpoly(), "turn {turn}: the spectrum can't tell");
        assert_ne!(id.invariants(), sh.invariants(), "turn {turn}: the factors can");
        assert_eq!(id.invariants().nontrivial().len(), 2, "x − 1, x − 1");
        assert_eq!(sh.invariants().nontrivial().len(), 1, "(x − 1)²");
        assert_eq!(id.invariants().product(), sh.invariants().product());
        ball.rotate(&cycle(&h)).expect("a rotation");
    }
}

// --- 6. Rotation is not a ring change ----------------------------------------

/// The compile-time half is the `compile_fail` doctest on
/// `prismqueer::discoball::Rotation`: a map that moved to ℤ/2 isn't a
/// rotation of a room over ℤ. The runtime half: over the fixed ring, only a
/// unitary is a rotation.
#[test]
fn only_a_unitary_over_the_fixed_ring_is_a_rotation() {
    let h = room(2);
    assert!(Rotation::new(&Map::identity(&h)).is_ok(), "the control");
    assert!(Rotation::new(&matrix(&h, &[&[0, -1], &[1, 0]])).is_ok());
    assert_eq!(
        Rotation::new(&matrix(&h, &[&[1, 1], &[0, 1]])).err(),
        Some(BallError::NotARotation),
        "a shear is invertible over ℤ, but not unitary: not a rotation"
    );
    assert_eq!(
        Rotation::new(&matrix(&h, &[&[2, 0], &[0, 1]])).err(),
        Some(BallError::NotARotation)
    );
    assert_eq!(
        Rotation::permutation(&h, &[0, 0]).err(),
        Some(BallError::NotARotation)
    );
    assert_eq!(
        Rotation::permutation(&h, &[0]).err(),
        Some(BallError::NotARotation)
    );

    // A rotation of another room doesn't turn this ball.
    let mut ball = Ball::new(&h);
    assert!(matches!(
        ball.rotate(&cycle(&room(3))),
        Err(BallError::Room(_))
    ));
    // Nor does a facet on another room join it.
    let other = Facet::unproved(&Unproved, "elsewhere", Map::identity(&room(3)), rules(&[]));
    assert!(matches!(ball.facet(other), Err(BallError::Room(_))));
    let mut out = Facet::unproved(&Unproved, "spills", Map::identity(&h), rules(&[]));
    out.step([2]);
    assert_eq!(ball.facet(out), Err(BallError::Outside { spot: 2, dim: 2 }));
    let loose = Facet::unproved(
        &Unproved,
        "rule off the ball",
        Map::identity(&h),
        rules(&[(0, 5, Sign::Positive)]),
    );
    assert_eq!(ball.facet(loose), Err(BallError::Outside { spot: 5, dim: 2 }));
}

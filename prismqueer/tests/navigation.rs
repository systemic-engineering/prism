//! The navigation floor (`docs/spec/navigation.md`): K3 over `Membership`,
//! K3-Datalog over hyperedges, an unexplored edge reads `maybe`. Mara's N1,
//! N2, N3, N4 and N5 (mirror `mara/witness-navigation-grade` 052e5a13,
//! `wnes_probe.out`).
//!
//! **Her data, not her code.** N3–N5 run over her probe's graphs, made again
//! from her seeds by CPython's Mersenne Twister ([`CPython`], below), written
//! from MT19937's definition and CPython's documented draws, in integers.
//! The fixed point is `prismqueer::navigation`'s; her counts are the oracle.

use std::collections::BTreeSet;

use prismqueer::navigation::{plus, times, Body, Edge, Label, Membership, Program, Rule, K3};

// ---------------------------------------------------------------------------
// CPython's `random.Random(seed)`, the draws her probe makes
// ---------------------------------------------------------------------------

/// MT19937, seeded as CPython seeds a small non-negative int
/// (`init_by_array([seed])`).
struct CPython {
    mt: [u32; 624],
    index: usize,
}

impl CPython {
    fn new(seed: u32) -> Self {
        let mut mt = [0u32; 624];
        mt[0] = 19_650_218;
        for i in 1..624 {
            mt[i] = 1_812_433_253u32
                .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        let key = [seed];
        let (mut i, mut j) = (1usize, 0usize);
        for _ in 0..624 {
            mt[i] = (mt[i] ^ (mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                mt[0] = mt[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            mt[i] = (mt[i] ^ (mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                mt[0] = mt[623];
                i = 1;
            }
        }
        mt[0] = 0x8000_0000;
        CPython { mt, index: 624 }
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            for k in 0..624 {
                let y = (self.mt[k] & 0x8000_0000) | (self.mt[(k + 1) % 624] & 0x7fff_ffff);
                let mag = if y & 1 == 1 { 0x9908_b0df } else { 0 };
                self.mt[k] = self.mt[(k + 397) % 624] ^ (y >> 1) ^ mag;
            }
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `random()` as the 53-bit integer x it divides by 2⁵³.
    fn random53(&mut self) -> u64 {
        let a = (self.next_u32() >> 5) as u64;
        let b = (self.next_u32() >> 6) as u64;
        (a << 26) | b
    }

    /// `random() < 0.3`. The double 0.3 is 5404319552844595 / 2⁵⁴, so this
    /// is 2x < 5404319552844595, exactly.
    fn below_three_tenths(&mut self) -> bool {
        2 * self.random53() < 5_404_319_552_844_595
    }

    /// `random() < 0.5`: x < 2⁵².
    fn below_half(&mut self) -> bool {
        self.random53() < 1 << 52
    }

    /// `_randbelow(n)`: k = n.bit_length() bits, rejected until below n.
    fn below(&mut self, n: u32) -> u32 {
        let k = 32 - n.leading_zeros();
        loop {
            let r = self.next_u32() >> (32 - k);
            if r < n {
                return r;
            }
        }
    }
}

#[test]
fn the_generator_is_cpythons() {
    // python3 -c "import random; r=random.Random(32); print([r.getrandbits(32) for _ in range(3)])"
    let mut r = CPython::new(32);
    assert_eq!(
        [r.next_u32(), r.next_u32(), r.next_u32()],
        [332_524_003, 3_979_276_737, 917_477_150]
    );
    // [r.choice((0,1,2)) for _ in range(5)], r.randrange(7) under Random(33)
    let mut r = CPython::new(33);
    let picks: Vec<u32> = (0..5).map(|_| r.below(3)).collect();
    assert_eq!(picks, [2, 0, 2, 0, 1]);
    assert_eq!(r.below(7), 3);
}

// ---------------------------------------------------------------------------
// Her graphs, read through the floor
// ---------------------------------------------------------------------------

const N: u8 = 7;
type Pair = (u8, u8);

/// Her `rand_graph(rng, 7, 0.3)`: each ordered pair u ≠ v, in order, an edge
/// with probability 0.3 and a uniform K3 label.
fn graph(rng: &mut CPython) -> Vec<(Pair, K3)> {
    let mut edges = Vec::new();
    for u in 0..N {
        for v in 0..N {
            if u != v && rng.below_three_tenths() {
                edges.push(((u, v), K3::ALL[rng.below(3) as usize]));
            }
        }
    }
    edges
}

fn named(value: K3, pair: Pair) -> Label<Pair> {
    let names = BTreeSet::from([pair]);
    match value {
        K3::True => Membership::In(names),
        K3::Maybe => Membership::Undecided(names),
        K3::False => Membership::Out(names),
    }
}

/// Reach from node 0 as K3-Datalog: `0 :-`, and `v :- u, edge(u, v)` per
/// edge. A node with no rule is `false`: a closed, fully listed graph.
fn reach(edges: impl IntoIterator<Item = (Pair, Edge<Pair>)>) -> Program<u8, Pair> {
    let mut program = Program::new();
    program.rule(Rule::fact(0));
    for ((u, v), edge) in edges {
        program.rule(Rule::new(v, [Body::Atom(u), Body::Edge(edge)]));
    }
    program
}

fn read(edges: &[(Pair, K3)]) -> Program<u8, Pair> {
    reach(edges.iter().map(|&(p, l)| (p, Edge::Read(named(l, p)))))
}

fn verdicts(program: &Program<u8, Pair>) -> [K3; N as usize] {
    let solution = program.solve();
    let mut out = [K3::False; N as usize];
    for (n, slot) in out.iter_mut().enumerate() {
        *slot = solution.value(&(n as u8)).unwrap_or(K3::False);
    }
    out
}

/// Decided in `partial`, and different in `full`.
fn flipped(partial: &[K3], full: &[K3]) -> usize {
    partial
        .iter()
        .zip(full)
        .filter(|(a, b)| a.is_decided() && a != b)
        .count()
}

// ---------------------------------------------------------------------------
// N1: the semiring
// ---------------------------------------------------------------------------

#[test]
fn n1_k3_is_a_commutative_idempotent_zero_closed_semiring() {
    let mut ok = 0;
    for a in K3::ALL {
        for b in K3::ALL {
            for c in K3::ALL {
                let laws = [
                    a.plus(b.plus(c)) == a.plus(b).plus(c),
                    a.times(b.times(c)) == a.times(b).times(c),
                    a.plus(b) == b.plus(a),
                    a.times(b) == b.times(a),
                    a.plus(K3::ZERO) == a,
                    a.times(K3::ONE) == a,
                    a.times(K3::ZERO) == K3::ZERO,
                    a.times(b.plus(c)) == a.times(b).plus(a.times(c)),
                    a.plus(a) == a,
                    K3::ONE.plus(a) == K3::ONE,
                    a.star() == K3::ONE.plus(a.times(a.star())),
                ];
                ok += laws.iter().all(|&l| l) as usize;
            }
        }
    }
    assert_eq!(ok, 27, "N1 semiring axioms + idempotent + 0-closed");
    assert_eq!((K3::ZERO, K3::ONE), (K3::False, K3::True));
}

#[test]
fn the_carrier_maps_membership_onto_k3() {
    let i: Membership<(), (), ()> = Membership::In(());
    let o: Membership<(), (), ()> = Membership::Out(());
    let u: Membership<(), (), ()> = Membership::Undecided(());
    assert_eq!(
        [K3::of(&i), K3::of(&o), K3::of(&u)],
        [K3::True, K3::False, K3::Maybe]
    );
}

// ---------------------------------------------------------------------------
// N2: monotone in both orders
// ---------------------------------------------------------------------------

#[test]
fn n2_min_and_max_are_monotone_in_both_orders() {
    let (mut pairs_t, mut and_t, mut or_t) = (0, 0, 0);
    let (mut pairs_k, mut and_k, mut or_k) = (0, 0, 0);
    for a in K3::ALL {
        for b in K3::ALL {
            for a2 in K3::ALL {
                for b2 in K3::ALL {
                    if a.leq_t(a2) && b.leq_t(b2) {
                        pairs_t += 1;
                        and_t += a.times(b).leq_t(a2.times(b2)) as usize;
                        or_t += a.plus(b).leq_t(a2.plus(b2)) as usize;
                    }
                    if a.leq_k(a2) && b.leq_k(b2) {
                        pairs_k += 1;
                        and_k += a.times(b).leq_k(a2.times(b2)) as usize;
                        or_k += a.plus(b).leq_k(a2.plus(b2)) as usize;
                    }
                }
            }
        }
    }
    assert_eq!((and_t, or_t, pairs_t), (36, 36, 36));
    assert_eq!((and_k, or_k, pairs_k), (25, 25, 25));
}

#[test]
fn the_two_orders() {
    assert!(K3::False.leq_t(K3::Maybe) && K3::Maybe.leq_t(K3::True));
    assert!(!K3::True.leq_t(K3::Maybe));
    assert!(K3::Maybe.leq_k(K3::False) && K3::Maybe.leq_k(K3::True));
    assert!(!K3::False.leq_k(K3::True) && !K3::True.leq_k(K3::False));
    assert!(!K3::False.leq_k(K3::Maybe));
    assert!(K3::True.is_decided() && K3::False.is_decided() && !K3::Maybe.is_decided());
}

// ---------------------------------------------------------------------------
// The lifted operations on the carrier
// ---------------------------------------------------------------------------

fn labels() -> Vec<Label<u8>> {
    let sets: [BTreeSet<u8>; 4] = [
        BTreeSet::new(),
        BTreeSet::from([1]),
        BTreeSet::from([2]),
        BTreeSet::from([1, 2]),
    ];
    let mut all = Vec::new();
    for s in sets {
        all.push(Membership::In(s.clone()));
        all.push(Membership::Undecided(s.clone()));
        all.push(Membership::Out(s));
    }
    all
}

#[test]
fn the_lifted_operations_are_aci_and_k3_is_their_image() {
    let all = labels();
    let mut triples = 0;
    for a in &all {
        assert_eq!(plus(a, a), *a);
        assert_eq!(times(a, a), *a);
        for b in &all {
            assert_eq!(plus(a, b), plus(b, a));
            assert_eq!(times(a, b), times(b, a));
            assert_eq!(K3::of(&plus(a, b)), K3::of(a).plus(K3::of(b)));
            assert_eq!(K3::of(&times(a, b)), K3::of(a).times(K3::of(b)));
            for c in &all {
                assert_eq!(plus(a, &plus(b, c)), plus(&plus(a, b), c));
                assert_eq!(times(a, &times(b, c)), times(&times(a, b), c));
                triples += 1;
            }
        }
    }
    assert_eq!(triples, 12 * 12 * 12);
}

#[test]
fn a_meet_of_two_ins_carries_both_certificates_a_join_of_two_outs_both_witnesses() {
    let a: Label<u8> = Membership::In(BTreeSet::from([1]));
    let b: Label<u8> = Membership::In(BTreeSet::from([2]));
    assert_eq!(times(&a, &b), Membership::In(BTreeSet::from([1, 2])));
    let x: Label<u8> = Membership::Out(BTreeSet::from([3]));
    let y: Label<u8> = Membership::Out(BTreeSet::from([4]));
    assert_eq!(plus(&x, &y), Membership::Out(BTreeSet::from([3, 4])));
    // The losing side's payload is dropped: a true beats a false's witness.
    assert_eq!(plus(&a, &x), a);
    assert_eq!(times(&a, &x), x);
}

// ---------------------------------------------------------------------------
// K3-Datalog over hyperedges
// ---------------------------------------------------------------------------

fn edge(value: K3, name: &'static str) -> Body<&'static str, &'static str> {
    let names = BTreeSet::from([name]);
    Body::Edge(Edge::Read(match value {
        K3::True => Membership::In(names),
        K3::Maybe => Membership::Undecided(names),
        K3::False => Membership::Out(names),
    }))
}

#[test]
fn a_rule_is_the_min_over_its_body_an_atom_the_max_over_its_rules() {
    let mut p: Program<&str, &str> = Program::new();
    p.rule(Rule::fact("a"));
    p.rule(Rule::new("b", [edge(K3::Maybe, "e1")]));
    // h :- a, b (a hyperedge with two tails, plus an edge): min = maybe
    p.rule(Rule::new(
        "h",
        [Body::Atom("a"), Body::Atom("b"), edge(K3::True, "e2")],
    ));
    // g :- a, e3=false | g :- b : max(false, maybe) = maybe
    p.rule(Rule::new("g", [Body::Atom("a"), edge(K3::False, "e3")]));
    p.rule(Rule::new("g", [Body::Atom("b")]));
    // k :- a, e4=false | k :- a, e5=false : false, with the cut {e4, e5}
    p.rule(Rule::new("k", [Body::Atom("a"), edge(K3::False, "e4")]));
    p.rule(Rule::new("k", [Body::Atom("a"), edge(K3::False, "e5")]));
    // t :- a, e6 | t :- a, e7, both true: both certificates
    p.rule(Rule::new("t", [Body::Atom("a"), edge(K3::True, "e6")]));
    p.rule(Rule::new("t", [Body::Atom("a"), edge(K3::True, "e7")]));
    assert_eq!(p.rules().len(), 9);
    assert_eq!(p.rules()[2].head(), &"h");
    assert_eq!(p.rules()[2].body().len(), 3);

    let s = p.solve();
    assert_eq!(s.value(&"a"), Some(K3::True));
    assert_eq!(s.get(&"a"), Some(&Membership::In(BTreeSet::new())));
    assert_eq!(
        s.get(&"h"),
        Some(&Membership::Undecided(BTreeSet::from(["e1"])))
    );
    assert_eq!(
        s.get(&"g"),
        Some(&Membership::Undecided(BTreeSet::from(["e1"])))
    );
    assert_eq!(
        s.get(&"k"),
        Some(&Membership::Out(BTreeSet::from(["e4", "e5"])))
    );
    assert_eq!(
        s.get(&"t"),
        Some(&Membership::In(BTreeSet::from(["e6", "e7"])))
    );
    assert_eq!(s.value(&"nowhere"), None);
    assert_eq!(s.iter().count(), 6);
}

#[test]
fn a_cycle_reaches_the_least_fixed_point() {
    // a :- b ; b :- a : nothing founds them, so both stay false.
    let mut p: Program<&str, &str> = Program::new();
    p.rule(Rule::new("a", [Body::Atom("b")]));
    p.rule(Rule::new("b", [Body::Atom("a")]));
    let s = p.solve();
    assert_eq!(
        (s.value(&"a"), s.value(&"b")),
        (Some(K3::False), Some(K3::False))
    );
    // and once one is founded, both rise.
    p.rule(Rule::new("a", [edge(K3::True, "seed")]));
    let s = p.solve();
    assert_eq!(s.get(&"b"), Some(&Membership::In(BTreeSet::from(["seed"]))));
    assert_eq!(Program::<u8, u8>::default().solve().iter().count(), 0);
}

// ---------------------------------------------------------------------------
// Absence is its own constructor
// ---------------------------------------------------------------------------

#[test]
fn an_unexplored_edge_reads_as_maybe_naming_itself() {
    let e: Edge<&str> = Edge::Unexplored("u->v");
    assert_eq!(e.reads(), Membership::Undecided(BTreeSet::from(["u->v"])));
    assert_eq!(K3::of(&e.reads()), K3::Maybe);
    // A lookup that finds nothing is unexplored, never Out.
    assert_eq!(Edge::from_lookup(None, "u->v"), Edge::Unexplored("u->v"));
    let found: Label<&str> = Membership::Out(BTreeSet::from(["u->v"]));
    assert_eq!(
        Edge::from_lookup(Some(found.clone()), "u->v"),
        Edge::Read(found)
    );

    let mut p: Program<&str, &str> = Program::new();
    p.rule(Rule::fact("u"));
    p.rule(Rule::new(
        "v",
        [Body::Atom("u"), Body::Edge(Edge::Unexplored("u->v"))],
    ));
    assert_eq!(
        p.solve().get(&"v"),
        Some(&Membership::Undecided(BTreeSet::from(["u->v"])))
    );
}

// ---------------------------------------------------------------------------
// N3: the fixed point is order-free
// ---------------------------------------------------------------------------

/// A Fisher–Yates shuffle driven by our own splitmix64: the order is a
/// strategy, and any fair one must do.
fn shuffle(state: &mut u64, order: &mut [usize]) {
    for i in (1..order.len()).rev() {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        order.swap(i, (z % (i as u64 + 1)) as usize);
    }
}

#[test]
fn n3_the_fixed_point_is_independent_of_rule_order() {
    let mut rng = CPython::new(31);
    let mut same = 0;
    for _ in 0..500 {
        let program = read(&graph(&mut rng));
        let base = program.solve();
        let all = (0..5).all(|_| {
            // her probe seeds each order from one random(); so do we.
            let mut state = rng.random53();
            program.solve_by(|order| shuffle(&mut state, order)) == base
        });
        same += all as usize;
    }
    assert_eq!(same, 500, "N3 closure independent of exploration order");
}

// ---------------------------------------------------------------------------
// N4: an unexplored edge reads maybe, never false
// ---------------------------------------------------------------------------

#[test]
fn n4_unexplored_as_maybe_flips_nothing_unexplored_as_false_flips_3191() {
    let mut rng = CPython::new(32);
    let (mut lower, mut flips_maybe, mut flips_false) = (0, 0, 0);
    for _ in 0..2000 {
        let edges = graph(&mut rng);
        let full = verdicts(&read(&edges));
        let seen: Vec<bool> = edges.iter().map(|_| rng.below_half()).collect();

        // The floor's reading: an edge not seen is unexplored.
        let partial = verdicts(&reach(edges.iter().zip(&seen).map(|(&(p, l), &s)| {
            (
                p,
                if s {
                    Edge::Read(named(l, p))
                } else {
                    Edge::Unexplored(p)
                },
            )
        })));
        // "Default deny" as a value: the unseen edges left out, i.e. false.
        let denied: Vec<(Pair, K3)> = edges
            .iter()
            .zip(&seen)
            .filter(|(_, &s)| s)
            .map(|(&e, _)| e)
            .collect();
        let denied = verdicts(&read(&denied));

        lower += partial.iter().zip(&full).all(|(a, b)| a.leq_k(*b)) as usize;
        flips_maybe += flipped(&partial, &full);
        flips_false += flipped(&denied, &full);
    }
    assert_eq!(lower, 2000, "a partial fixed point is a ≤k-lower bound");
    assert_eq!(
        flips_maybe, 0,
        "unexplored = maybe flips no decided verdict"
    );
    assert_eq!(flips_false, 3191, "unexplored = false flips Mara's 3,191");
}

// ---------------------------------------------------------------------------
// N5: settling never flips a decided verdict; growth can
// ---------------------------------------------------------------------------

#[test]
fn n5_settling_flips_nothing_one_added_edge_raises_471() {
    let mut rng = CPython::new(33);
    let (mut settle_flips, mut raised) = (0, 0);
    for _ in 0..2000 {
        let edges = graph(&mut rng);
        let before = verdicts(&read(&edges));

        let settled: Vec<(Pair, K3)> = edges
            .iter()
            .map(|&(p, l)| {
                let l = if l == K3::Maybe {
                    [K3::False, K3::True][rng.below(2) as usize]
                } else {
                    l
                };
                (p, l)
            })
            .collect();
        settle_flips += flipped(&before, &verdicts(&read(&settled)));

        let (u, v) = (rng.below(7) as u8, rng.below(7) as u8);
        let mut grown = edges.clone();
        if u != v && !edges.iter().any(|&(p, _)| p == (u, v)) {
            grown.push(((u, v), K3::True));
        }
        let after = verdicts(&read(&grown));
        raised += before
            .iter()
            .zip(&after)
            .filter(|(a, b)| **a == K3::False && **b != K3::False)
            .count();
    }
    assert_eq!(settle_flips, 0, "settling flips no decided verdict");
    assert_eq!(raised, 471, "one added edge raises Mara's 471");
}

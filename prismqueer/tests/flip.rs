//! The flip point (`docs/spec/flip.md`; prism-024 §2; Mara, mirror-029 II.5
//! and mirror `mara/bridge-grade` 3333dd80 §5, probe B4). Three cases:
//! sequenced and monotone (bisection, two thresholds in K3), unsequenced
//! (the minimal witness basis, an antichain, bounded), and with negation
//! (every flip, by scan). No feature: the default build.
//!
//! **The type rule.** `flip_point`, `bisect` and `basis` take a
//! [`Monotone`], made only from a `navigation::Program`; a closure gets the
//! scan only. The `compile_fail` doctest in `prismqueer::flip` holds it,
//! beside a control that compiles.
//!
//! **Her data, not her code.** B4 runs over her probe's instances, made again
//! from her seed (20261124) by CPython's Mersenne Twister
//! ([`crate::navigation::CPython`]), replaying every draw her probe makes
//! before B4 (B2's). The flip point, the scan and the basis are
//! `prismqueer::flip`'s; her counts are the oracle.

use std::collections::BTreeSet;

use prismqueer::flip::{
    basis, bisect, bound, earliest, flip_point, scan, Basis, Budget, Crossing, Monotone, Threshold,
    Witness,
};
use prismqueer::hold::{standing, Current, DecisionDraft, Output, Premise, Reading, Standing};
use prismqueer::navigation::{Body, Edge, Membership, Program, Rule, K3};
use prismqueer::oid::Oid;

use crate::navigation::CPython;

type Fact = Rule<u32, u32>;

const Q: u32 = 1000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A seeded splitmix64, for the three-valued instances (not Mara's data).
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }
}

fn facts_of(witness: &Witness, facts: &[Fact]) -> Vec<Fact> {
    witness.iter().map(|&p| facts[p - 1].clone()).collect()
}

/// Every member reaches the threshold, drops below it without any one of its
/// facts, and contains no other member.
fn minimal_antichain(m: &Monotone<u32, u32>, facts: &[Fact], t: Threshold, w: &[Witness]) -> bool {
    let minimal = w.iter().all(|s| {
        t.reached(m.verdict(&facts_of(s, facts)))
            && s.iter().all(|p| {
                let mut less = s.clone();
                less.remove(p);
                !t.reached(m.verdict(&facts_of(&less, facts)))
            })
    });
    let antichain = w
        .iter()
        .all(|u| w.iter().all(|v| u == v || !u.is_subset(v)));
    minimal && antichain
}

fn complete(b: Basis) -> Vec<Witness> {
    match b {
        Basis::Complete(w) => w,
        Basis::Partial { .. } => panic!("an unbounded budget must complete"),
    }
}

/// Bisection, scan and witness formula agree on both thresholds, the
/// bisection within ⌈log₂ n⌉ + 2 evaluations, and every crossing names the
/// checkpoint pair and its fact.
fn agree(m: &Monotone<u32, u32>, facts: &[Fact], with_basis: bool) -> bool {
    let n = facts.len();
    let fp = flip_point(m, facts);
    let sc = scan(facts, |p| m.verdict(p));
    let mut ok = sc.evaluations == n + 1 && sc.flips.iter().all(|f| f.from < f.to);
    for t in Threshold::BOTH {
        let b = fp.get(t);
        ok &= b.evaluations <= bound(n);
        ok &= b.crossing.position() == sc.first_reaching(t);
        ok &= b == &bisect(m, facts, t);
        if let Crossing::At { before, at, fact } = &b.crossing {
            ok &= *before + 1 == *at && std::ptr::eq(*fact, &facts[at - 1]);
        }
        if with_basis {
            let w = complete(basis(m, facts, t, Budget(usize::MAX)));
            ok &= b.crossing.position() == earliest(&w);
            ok &= minimal_antichain(m, facts, t, &w);
        }
    }
    ok
}

// ---------------------------------------------------------------------------
// Case 1: sequenced and monotone
// ---------------------------------------------------------------------------

/// `q :- a, unexplored(e)` and `q :- a, b`: maybe once `a`, true once `b`.
fn two_thresholds() -> Monotone<u32, u32> {
    let mut p: Program<u32, u32> = Program::new();
    p.rule(Rule::new(
        Q,
        [Body::Atom(1), Body::Edge(Edge::Unexplored(9))],
    ));
    p.rule(Rule::new(Q, [Body::Atom(1), Body::Atom(2)]));
    Monotone::new(p, Q)
}

#[test]
fn both_thresholds_are_found_each_with_its_pair_and_fact() {
    let m = two_thresholds();
    assert_eq!(m.target(), &Q);
    let facts = [Rule::fact(7), Rule::fact(1), Rule::fact(8), Rule::fact(2)];
    let fp = flip_point(&m, &facts);
    assert_eq!(
        fp.maybe.crossing,
        Crossing::At {
            before: 1,
            at: 2,
            fact: &facts[1]
        }
    );
    assert_eq!(
        fp.truth.crossing,
        Crossing::At {
            before: 3,
            at: 4,
            fact: &facts[3]
        }
    );
    assert_eq!(fp.get(Threshold::Maybe), &fp.maybe);
    assert_eq!(fp.get(Threshold::True), &fp.truth);
    // n = 4: v(4), v(0), then two halvings.
    assert_eq!((fp.maybe.evaluations, fp.truth.evaluations), (4, 4));
    assert_eq!(bound(4), 4);

    let sc = scan(&facts, |p| m.verdict(p));
    assert_eq!(sc.initial, K3::False);
    let flips: Vec<_> = sc
        .flips
        .iter()
        .map(|f| (f.before, f.at, f.from, f.to))
        .collect();
    assert_eq!(
        flips,
        [(1, 2, K3::False, K3::Maybe), (3, 4, K3::Maybe, K3::True)]
    );
    assert!(std::ptr::eq(sc.flips[1].fact, &facts[3]));
}

#[test]
fn a_threshold_can_read_never_or_initially() {
    let m = two_thresholds();
    // Never: v(n) is below; one evaluation.
    let facts = [Rule::fact(7), Rule::fact(8)];
    let fp = flip_point(&m, &facts);
    assert_eq!(fp.maybe.crossing, Crossing::Never);
    assert_eq!(fp.truth.crossing, Crossing::Never);
    assert_eq!(fp.maybe.evaluations, 1);
    assert_eq!(fp.maybe.crossing.position(), None);
    // Maybe never rises to true.
    let facts = [Rule::fact(7), Rule::fact(1)];
    let fp = flip_point(&m, &facts);
    assert_eq!(fp.maybe.crossing.position(), Some(2));
    assert_eq!(fp.truth.crossing, Crossing::Never);

    // Initially: the empty prefix is already at the threshold; nothing flipped.
    let mut p: Program<u32, u32> = Program::new();
    p.rule(Rule::new(Q, [Body::Edge(Edge::Unexplored(9))]));
    p.rule(Rule::new(Q, [Body::Atom(2)]));
    let m = Monotone::new(p, Q);
    let facts = [Rule::fact(7), Rule::fact(2)];
    let fp = flip_point(&m, &facts);
    assert_eq!(fp.maybe.crossing, Crossing::Initially);
    assert_eq!(fp.maybe.crossing.position(), Some(0));
    assert_eq!(fp.maybe.evaluations, 2);
    assert_eq!(fp.truth.crossing.position(), Some(2));
    let sc = scan(&facts, |p| m.verdict(p));
    assert_eq!(
        (sc.initial, sc.first_reaching(Threshold::Maybe)),
        (K3::Maybe, Some(0))
    );
    let w = complete(basis(&m, &facts, Threshold::Maybe, Budget(usize::MAX)));
    assert_eq!(w, [Witness::new()]);
    assert_eq!(earliest(&w), Some(0));
    assert_eq!(earliest(&[]), None);

    // No facts at all: one evaluation, and the bound holds at n = 0.
    let fp = flip_point(&m, &[]);
    assert_eq!(fp.maybe.crossing, Crossing::Initially);
    assert_eq!(fp.truth.crossing, Crossing::Never);
    assert!(fp.maybe.evaluations <= bound(0) && fp.truth.evaluations <= bound(0));
}

/// A random three-valued instance: `n` facts, some ground (`i :-`), some
/// derived (`i :- j`); up to three clauses for `q`, each a conjunction of
/// fact atoms, half of them resting on a maybe edge (unexplored or read
/// `Undecided`), so the two thresholds differ.
fn instance(rng: &mut Mix, n: usize) -> (Monotone<u32, u32>, Vec<Fact>) {
    let mut facts: Vec<Fact> = (0..n as u32)
        .map(|i| {
            if rng.below(4) == 0 {
                Rule::new(i, [Body::Atom(rng.below(n) as u32)])
            } else {
                Rule::fact(i)
            }
        })
        .collect();
    rng.shuffle(&mut facts);
    let mut p: Program<u32, u32> = Program::new();
    for c in 0..1 + rng.below(3) {
        let mut body: Vec<Body<u32, u32>> = (0..1 + rng.below(3))
            .map(|_| Body::Atom(rng.below(n) as u32))
            .collect();
        match rng.below(4) {
            0 => body.push(Body::Edge(Edge::Unexplored(c as u32))),
            1 => body.push(Body::Edge(Edge::Read(Membership::Undecided(
                BTreeSet::from([c as u32]),
            )))),
            _ => {}
        }
        p.rule(Rule::new(Q, body));
    }
    (Monotone::new(p, Q), facts)
}

#[test]
fn bisection_equals_scan_equals_the_witness_formula_on_random_monotone_instances() {
    let mut rng = Mix(20_261_124);
    let (mut ok, mut distinct, mut both) = (0, 0, 0);
    for _ in 0..300 {
        let n = 3 + rng.below(8);
        let (m, facts) = instance(&mut rng, n);
        ok += agree(&m, &facts, true) as usize;
        let fp = flip_point(&m, &facts);
        distinct += (fp.maybe.crossing.position() != fp.truth.crossing.position()) as usize;
        both += matches!(
            (&fp.maybe.crossing, &fp.truth.crossing),
            (Crossing::At { .. }, Crossing::At { .. })
        ) as usize;
    }
    assert_eq!(ok, 300);
    // Both thresholds are exercised, and they come apart.
    assert!(
        distinct > 50 && both > 50,
        "distinct {distinct}, both {both}"
    );
}

#[test]
fn bisection_stays_within_the_log_bound_on_long_logs() {
    let mut rng = Mix(31);
    for _ in 0..4 {
        let n = 512 + rng.below(100);
        let facts: Vec<Fact> = (0..n as u32).map(Rule::fact).collect();
        let (a, b, c) = (
            rng.below(n) as u32,
            rng.below(n) as u32,
            rng.below(n) as u32,
        );
        let mut p: Program<u32, u32> = Program::new();
        p.rule(Rule::new(Q, [Body::Atom(a), Body::Atom(b)]));
        p.rule(Rule::new(
            Q,
            [Body::Atom(c), Body::Edge(Edge::Unexplored(0))],
        ));
        let m = Monotone::new(p, Q);
        assert!(agree(&m, &facts, false));
        let fp = flip_point(&m, &facts);
        assert!(fp.maybe.evaluations <= 10 + 2 && fp.truth.evaluations <= 10 + 2);
        assert_eq!(fp.truth.crossing.position(), Some(a.max(b) as usize + 1));
        assert_eq!(
            fp.maybe.crossing.position(),
            Some(c.min(a.max(b)) as usize + 1)
        );
    }
}

// ---------------------------------------------------------------------------
// Case 2: unsequenced, the minimal witness basis
// ---------------------------------------------------------------------------

#[test]
fn the_basis_is_a_minimal_antichain() {
    // q :- a, b ; q :- c ; q :- a, b, c.  Facts in order [c, a, b, d].
    let mut p: Program<u32, u32> = Program::new();
    p.rule(Rule::new(Q, [Body::Atom(1), Body::Atom(2)]));
    p.rule(Rule::new(Q, [Body::Atom(3)]));
    p.rule(Rule::new(Q, [Body::Atom(1), Body::Atom(2), Body::Atom(3)]));
    let m = Monotone::new(p, Q);
    let facts = [Rule::fact(3), Rule::fact(1), Rule::fact(2), Rule::fact(4)];
    let w = complete(basis(&m, &facts, Threshold::True, Budget(usize::MAX)));
    assert_eq!(w, [Witness::from([1]), Witness::from([2, 3])]);
    assert!(minimal_antichain(&m, &facts, Threshold::True, &w));
    assert_eq!(earliest(&w), Some(1));

    // Overdetermined (Mara B5): two refuting spans, each a witness alone.
    let mut p: Program<u32, u32> = Program::new();
    p.rule(Rule::new(Q, [Body::Atom(1)]));
    p.rule(Rule::new(Q, [Body::Atom(2)]));
    let m = Monotone::new(p, Q);
    let facts = [Rule::fact(1), Rule::fact(2)];
    let w = complete(basis(&m, &facts, Threshold::True, Budget(usize::MAX)));
    assert_eq!(w, [Witness::from([1]), Witness::from([2])]);
}

#[test]
fn over_budget_is_typed_and_its_members_are_still_minimal() {
    let mut p: Program<u32, u32> = Program::new();
    p.rule(Rule::new(Q, [Body::Atom(1), Body::Atom(2)]));
    p.rule(Rule::new(Q, [Body::Atom(3)]));
    let m = Monotone::new(p, Q);
    let facts = [Rule::fact(3), Rule::fact(1), Rule::fact(2), Rule::fact(4)];
    // {}, {1}, {2}: then the budget runs out.
    let b = basis(&m, &facts, Threshold::True, Budget(3));
    assert_eq!(
        b,
        Basis::Partial {
            members: vec![Witness::from([1])],
            complete_below: 1,
            examined: 3,
        }
    );
    if let Basis::Partial { members, .. } = &b {
        assert!(minimal_antichain(&m, &facts, Threshold::True, members));
    }
    assert_eq!(
        basis(&m, &facts, Threshold::True, Budget(0)),
        Basis::Partial {
            members: vec![],
            complete_below: 0,
            examined: 0
        }
    );
    // 2⁴ subsets: exactly enough completes, one fewer is partial.
    let full = basis(&m, &facts, Threshold::True, Budget(16));
    assert_eq!(
        full,
        Basis::Complete(vec![Witness::from([1]), Witness::from([2, 3])])
    );
    assert!(matches!(
        basis(&m, &facts, Threshold::True, Budget(15)),
        Basis::Partial {
            complete_below: 4,
            examined: 15,
            ..
        }
    ));
    assert_eq!(full.members(), &[Witness::from([1]), Witness::from([2, 3])]);
}

// ---------------------------------------------------------------------------
// Case 3: with negation, every flip by scan
// ---------------------------------------------------------------------------

#[test]
fn the_scan_finds_every_flip_under_negation() {
    // q :- a ; r :- q, not b (a test-side rule; navigation has no `not`).
    let mut p: Program<u32, u32> = Program::new();
    p.rule(Rule::new(Q, [Body::Atom(1)]));
    let m = Monotone::new(p, Q);
    let not_b = |prefix: &[Fact]| {
        if prefix.contains(&Rule::fact(2)) {
            K3::False
        } else {
            m.verdict(prefix)
        }
    };
    let facts = [Rule::fact(1), Rule::fact(2), Rule::fact(5)];
    let sc = scan(&facts, not_b);
    let flips: Vec<_> = sc
        .flips
        .iter()
        .map(|f| (f.before, f.at, f.from, f.to))
        .collect();
    assert_eq!(
        flips,
        [(0, 1, K3::False, K3::True), (1, 2, K3::True, K3::False)]
    );
    // The flip back keeps the error that dissolved; bisection would see one
    // threshold (v(n) = false: "never") and lose both.
    assert_eq!(sc.first_reaching(Threshold::True), Some(1));
}

// ---------------------------------------------------------------------------
// B4: Mara's counts, from her seed
// ---------------------------------------------------------------------------

fn randint(r: &mut CPython, a: u32, b: u32) -> u32 {
    a + r.below(b - a + 1)
}

/// `random.sample(pop, k)` on a small population: CPython's pool path.
fn sample<T: Clone>(r: &mut CPython, pop: &[T], k: usize) -> Vec<T> {
    let n = pop.len();
    let mut pool = pop.to_vec();
    (0..k)
        .map(|i| {
            let j = r.below((n - i) as u32) as usize;
            let picked = pool[j].clone();
            pool[j] = pool[n - i - 1].clone();
            picked
        })
        .collect()
}

/// Every draw her probe makes before B4: B2's 3,000 trials (`rand_program`,
/// then a sample of up to 3 of 4 atoms, then a choice of 5). B1 and B3 draw
/// nothing.
fn replay_b2(r: &mut CPython) {
    let atoms: Vec<u32> = (0..7).collect();
    for _ in 0..3000 {
        for _ in 0..8 {
            let i = 1 + r.below(6);
            let lower = &atoms[..i as usize];
            let k = randint(r, 0, i.min(2));
            sample(r, lower, k as usize);
            let k = randint(r, 0, i.min(1));
            sample(r, lower, k as usize);
        }
        let k = randint(r, 0, 3);
        sample(r, &atoms[..4], k as usize);
        r.below(5);
    }
}

#[test]
fn b4_mara_400_400_400_and_144_multi_flips_under_negation() {
    let mut r = CPython::new(20_261_124);
    replay_b2(&mut r);
    let fs: Vec<u32> = (0..5).collect();
    let (mut mono_ok, mut anti_ok, mut multi, mut found) = (0, 0, 0, 0);
    for _ in 0..400 {
        let clauses: Vec<Vec<u32>> = (0..randint(&mut r, 1, 3))
            .map(|_| {
                let k = randint(&mut r, 1, 3) as usize;
                sample(&mut r, &fs, k)
            })
            .collect();
        let order = sample(&mut r, &fs, 5);
        let mut p: Program<u32, u32> = Program::new();
        for c in &clauses {
            p.rule(Rule::new(Q, c.iter().map(|&f| Body::Atom(f))));
        }
        let m = Monotone::new(p, Q);
        let facts: Vec<Fact> = order.iter().map(|&f| Rule::fact(f)).collect();

        // Two-valued: both thresholds are one, and the scan flips at most once.
        let fp = flip_point(&m, &facts);
        let w = complete(basis(&m, &facts, Threshold::True, Budget(usize::MAX)));
        mono_ok += (agree(&m, &facts, false)
            && scan(&facts, |p| m.verdict(p)).flips.len() <= 1
            && fp.truth.crossing.position() == earliest(&w)
            && fp.maybe.crossing.position() == earliest(&w)) as usize;
        anti_ok += minimal_antichain(&m, &facts, Threshold::True, &w) as usize;

        // q₂ :- q, not f₄.
        let negated = |prefix: &[Fact]| {
            if prefix.contains(&Rule::fact(4)) {
                K3::False
            } else {
                m.verdict(prefix)
            }
        };
        let sc = scan(&facts, negated);
        multi += (sc.flips.len() > 1) as usize;
        // Every flip, and only flips: replay v at each checkpoint.
        let v: Vec<K3> = (0..=facts.len()).map(|k| negated(&facts[..k])).collect();
        let expected: Vec<_> = (1..v.len())
            .filter(|&k| v[k] != v[k - 1])
            .map(|k| (k - 1, k, v[k - 1], v[k]))
            .collect();
        let got: Vec<_> = sc
            .flips
            .iter()
            .map(|f| (f.before, f.at, f.from, f.to))
            .collect();
        found += (got == expected && sc.initial == v[0]) as usize;
    }
    assert_eq!(mono_ok, 400, "B4 scan == bisection == witness-min");
    assert_eq!(anti_ok, 400, "B4 minimal witnesses an antichain");
    assert_eq!(multi, 144, "B4 negation: >1 flip");
    assert_eq!(found, 400, "the scan reports every flip");
}

// ---------------------------------------------------------------------------
// Tim's fixture as vectors
// ---------------------------------------------------------------------------

fn tim() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/spec/flip-vectors/tim.json"
    )))
    .expect("docs/spec/flip-vectors/tim.json parses")
}

fn word(v: K3) -> &'static str {
    match v {
        K3::False => "false",
        K3::Maybe => "maybe",
        K3::True => "true",
    }
}

fn s(v: &serde_json::Value) -> String {
    v.as_str().expect("a string").to_string()
}

fn derivations(t: &serde_json::Value) -> Vec<(String, Monotone<String, String>)> {
    t["derivations"]
        .as_array()
        .expect("derivations")
        .iter()
        .map(|d| {
            let name = s(&d["name"]);
            let mut p = Program::new();
            for rule in d["rules"].as_array().expect("rules") {
                let body = rule.as_array().expect("a body").iter().map(|item| {
                    match (item.get("fact"), item.get("unexplored")) {
                        (Some(f), None) => Body::Atom(s(f)),
                        (None, Some(e)) => Body::Edge(Edge::Unexplored(s(e))),
                        _ => panic!("a body item is a fact or an unexplored edge"),
                    }
                });
                p.rule(Rule::new(name.clone(), body));
            }
            (name.clone(), Monotone::new(p, name))
        })
        .collect()
}

#[test]
fn tims_vectors_give_the_expected_flip_points_and_scans() {
    let t = tim();
    let log: Vec<Rule<String, String>> = t["log"]
        .as_array()
        .expect("log")
        .iter()
        .enumerate()
        .map(|(i, e)| {
            assert_eq!(e["position"], i + 1);
            Rule::fact(s(&e["fact"]))
        })
        .collect();
    let ds = derivations(&t);
    assert_eq!(ds.len(), 2);
    for (name, m) in &ds {
        let fp = flip_point(m, &log);
        for (key, th) in [("maybe", Threshold::Maybe), ("true", Threshold::True)] {
            let e = &t["expected"]["flip_point"][name][key];
            let at = e["at"].as_u64().expect("at") as usize;
            assert_eq!(
                fp.get(th).crossing,
                Crossing::At {
                    before: e["before"].as_u64().expect("before") as usize,
                    at,
                    fact: &Rule::fact(s(&e["fact"])),
                },
                "{name} {key}"
            );
            assert!(fp.get(th).evaluations <= bound(log.len()));
        }
        let sc = scan(&log, |p| m.verdict(p));
        let got: Vec<_> = sc
            .flips
            .iter()
            .map(|f| (f.before, f.at, word(f.from), word(f.to)))
            .collect();
        let expected: Vec<_> = t["expected"]["scan"][name]
            .as_array()
            .expect("scan")
            .iter()
            .map(|f| {
                (
                    f["before"].as_u64().expect("before") as usize,
                    f["at"].as_u64().expect("at") as usize,
                    f["from"].as_str().expect("from"),
                    f["to"].as_str().expect("to"),
                )
            })
            .collect();
        assert_eq!(got, expected, "{name} scan");
    }
}

#[test]
fn tims_decision_d_stands_then_holds_through_hold() {
    let t = tim();
    let log: Vec<Rule<String, String>> = t["log"]
        .as_array()
        .expect("log")
        .iter()
        .map(|e| Rule::fact(s(&e["fact"])))
        .collect();
    let ds = derivations(&t);
    let outputs = |k: usize| -> Vec<Output> {
        ds.iter()
            .map(|(name, m)| Output::new(Oid::new(name), word(m.verdict(&log[..k]))))
            .collect()
    };
    let d = &t["expected"]["decision"];
    let sealed_at = d["sealed_at"].as_u64().expect("sealed_at") as usize;
    // D's premises in its declared order, each projected from the output at
    // the seal.
    let at_seal = outputs(sealed_at);
    let premises: Vec<Premise> = d["premises"]
        .as_array()
        .expect("premises")
        .iter()
        .map(|name| {
            let o = at_seal
                .iter()
                .find(|o| o.premise().as_str() == s(name))
                .expect("a premise is a derivation");
            Premise::of(o)
        })
        .collect();
    assert_eq!(premises.len(), ds.len());
    let decision = DecisionDraft::new(Oid::new(s(&d["rule"])), premises)
        .expect("distinct premises")
        .seal(Oid::new(s(&d["owner"])));

    for row in d["standing"].as_array().expect("standing") {
        let at = row["at"].as_u64().expect("at") as usize;
        let current = Current::new(outputs(at)).expect("one output per premise");
        let got = standing(&decision, &current);
        match (row["reads"].as_str().expect("reads"), got) {
            ("stands", got) => assert_eq!(got, Standing::Stands, "at {at}"),
            ("hold", Standing::Hold { moved, owner }) => {
                assert_eq!(owner.as_str(), s(&row["owner"]));
                let reasons: Vec<(String, String, String)> = moved
                    .iter()
                    .map(|mv| {
                        let now = match &mv.now {
                            Reading::At { status, .. } => status.clone(),
                            Reading::Unread => "unread".to_string(),
                        };
                        (mv.premise.as_str().to_string(), mv.sealed.clone(), now)
                    })
                    .collect();
                let expected: Vec<(String, String, String)> = row["moved"]
                    .as_array()
                    .expect("moved")
                    .iter()
                    .map(|mv| (s(&mv["premise"]), s(&mv["sealed"]), s(&mv["now"])))
                    .collect();
                assert_eq!(reasons, expected, "at {at}");
            }
            (reads, got) => panic!("at {at}: expected {reads}, got {got:?}"),
        }
    }
}

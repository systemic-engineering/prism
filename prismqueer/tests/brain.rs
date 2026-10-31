//! `brain`, the store side: its three invariants, and threshold independence.
//!
//! prism holds no evaluator. These tests drive the store with a tiny
//! semi-naive positive-Datalog evaluator that lives here, in the test file,
//! so `brain` stays placement-agnostic (where mirror's evaluator lives is
//! Alex's decision).
//!
//! Every fixture is public and non-personal: a small graph of letters, under
//! `Scope::public()`. Nothing personal enters `brain` before Mara's key
//! design lands (prism-008's privacy preconditions).
//!
//! The program (`edge` is the input; the rest is derived):
//!
//! ```text
//! path(x, y) :- edge(x, y).
//! path(x, z) :- path(x, y), edge(y, z).
//! both(x, y) :- path(x, y), path(y, x).
//! ```
//!
//! Each round's new facts settle as one crystal at the committed point, and
//! at the end every node's reach settles as one crystal: the Drop.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::Infallible;
use std::rc::Rc;

use prismqueer::brain::{
    Backend, Brain, Capacity, Crystal, Fact, Interner, Memory, Recall, Relation, Roots, Stats, Sym,
    Tick,
};
use prismqueer::oid::{Oid, Scope};

// ---------------------------------------------------------------------------
// The test-local evaluator: semi-naive, positive, over `brain`.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Term {
    Var(usize),
}

struct Atom {
    rel: Relation,
    terms: Vec<Term>,
}

struct Rule {
    head: Atom,
    body: Vec<Atom>,
    vars: usize,
}

fn atom(rel: Relation, vars: &[usize]) -> Atom {
    Atom {
        rel,
        terms: vars.iter().map(|&v| Term::Var(v)).collect(),
    }
}

/// One semi-naive round at the open tick `t`: every derivation that uses at
/// least one fact of tick `t − 1` and only facts of tick `< t`. Returns the
/// number of new facts deposited (at tick `t`).
fn round<B: Backend>(brain: &mut Brain<B>, rules: &[Rule]) -> usize {
    let t = brain.tick();
    let last = Tick(t.0 - 1);
    let mut heads: Vec<(Relation, Vec<Sym>)> = Vec::new();
    for rule in rules {
        for delta in 0..rule.body.len() {
            let mut bindings: Vec<Vec<Option<Sym>>> = vec![vec![None; rule.vars]];
            for (j, a) in rule.body.iter().enumerate() {
                let mut next = Vec::new();
                for b in &bindings {
                    let pattern: Vec<Option<Sym>> =
                        a.terms.iter().map(|&Term::Var(v)| b[v]).collect();
                    for (tuple, tick) in brain.scan(a.rel, &pattern) {
                        let fits = if j == delta { tick == last } else { tick < t };
                        if !fits {
                            continue;
                        }
                        let mut b2 = b.clone();
                        let unifies =
                            a.terms
                                .iter()
                                .zip(&tuple)
                                .all(|(&Term::Var(v), &s)| match b2[v] {
                                    Some(bound) => bound == s,
                                    None => {
                                        b2[v] = Some(s);
                                        true
                                    }
                                });
                        if unifies {
                            next.push(b2);
                        }
                    }
                }
                bindings = next;
            }
            for b in bindings {
                let tuple = rule
                    .head
                    .terms
                    .iter()
                    .map(|&Term::Var(v)| b[v].expect("range-restricted rule"))
                    .collect();
                heads.push((rule.head.rel, tuple));
            }
        }
    }
    heads
        .into_iter()
        .filter(|(rel, tuple)| brain.deposit(*rel, tuple))
        .count()
}

// ---------------------------------------------------------------------------
// The fixture: a public graph of letters, and its recompute-from-input.
// ---------------------------------------------------------------------------

/// Two cycles joined by a bridge, a tail into the first, and a self-loop.
const EDGES: &[(&str, &str)] = &[
    ("a", "b"),
    ("b", "c"),
    ("c", "a"),
    ("c", "d"),
    ("d", "e"),
    ("e", "f"),
    ("f", "d"),
    ("f", "g"),
    ("g", "g"),
    ("h", "a"),
];

const NODES: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h"];

/// Shortest walk of at least one edge, from `x` to every node it reaches.
fn distances(x: &str) -> BTreeMap<String, u64> {
    let mut dist = BTreeMap::new();
    let mut queue: VecDeque<(String, u64)> = VecDeque::from([(x.to_string(), 0)]);
    while let Some((n, d)) = queue.pop_front() {
        for (from, to) in EDGES {
            if *from == n && !dist.contains_key(*to) {
                dist.insert(to.to_string(), d + 1);
                queue.push_back((to.to_string(), d + 1));
            }
        }
    }
    dist
}

/// Every fact of the program with its deposit tick, from the input alone
/// (Lemma A: a fact depends only on the bytes it tested). A BFS, not the
/// evaluator: a genuinely different computation of the same facts.
fn from_input() -> BTreeMap<(&'static str, Vec<String>), u64> {
    let mut all = BTreeMap::new();
    for (x, y) in EDGES {
        all.insert(("edge", vec![x.to_string(), y.to_string()]), 0);
    }
    for x in NODES {
        for (z, d) in distances(x) {
            all.insert(("path", vec![x.to_string(), z]), d);
        }
    }
    for x in NODES {
        let dx = distances(x);
        for (y, dxy) in &dx {
            if let Some(dyx) = distances(y).get(*x) {
                all.insert(("both", vec![x.to_string(), y.clone()]), dxy.max(dyx) + 1);
            }
        }
    }
    all
}

/// What a recompute answers: tuples with their deposit ticks.
type Rows = Vec<(Vec<Sym>, Tick)>;

/// An eviction ruler, as a plain function.
type Warmth = fn(Fact<'_>) -> i64;

struct Program {
    edge: Relation,
    path: Relation,
    both: Relation,
    rules: Vec<Rule>,
}

fn program<B: Backend>(brain: &mut Brain<B>) -> Program {
    let edge = brain.relation("edge", 2);
    let path = brain.relation("path", 2);
    let both = brain.relation("both", 2);
    let rules = vec![
        Rule {
            head: atom(path, &[0, 1]),
            body: vec![atom(edge, &[0, 1])],
            vars: 2,
        },
        Rule {
            head: atom(path, &[0, 2]),
            body: vec![atom(path, &[0, 1]), atom(edge, &[1, 2])],
            vars: 3,
        },
        Rule {
            head: atom(both, &[0, 1]),
            body: vec![atom(path, &[0, 1]), atom(path, &[1, 0])],
            vars: 2,
        },
    ];
    Program {
        edge,
        path,
        both,
        rules,
    }
}

/// The caller-supplied recompute: answer a relation's pattern from the
/// input, with deposit ticks. It may over-answer; `brain` filters.
fn recompute(
    names: BTreeMap<Relation, &'static str>,
    calls: Rc<Cell<usize>>,
) -> impl FnMut(&Interner, Relation, &[Option<Sym>]) -> Rows {
    let all = from_input();
    move |interner, rel, _pattern| {
        calls.set(calls.get() + 1);
        let name = names[&rel];
        all.iter()
            .filter(|((r, _), _)| *r == name)
            .filter_map(|((_, args), &tick)| {
                let tuple: Option<Vec<Sym>> =
                    args.iter().map(|a| interner.get(a.as_bytes())).collect();
                tuple.map(|t| (t, Tick(tick)))
            })
            .collect()
    }
}

/// Everything a run shows: its facts with their ticks, its verdicts, and
/// its crystals, by name so that runs compare across interners.
#[derive(Debug, PartialEq)]
struct Light {
    facts: BTreeMap<(String, Vec<String>), Tick>,
    verdicts: Vec<Option<Tick>>,
    settled: Vec<(Oid, Vec<u8>)>,
    read_back: Vec<(Oid, Vec<u8>)>,
    ticks: Tick,
}

fn named<B: Backend>(brain: &Brain<B>, tuple: &[Sym]) -> Vec<String> {
    tuple
        .iter()
        .map(|&s| String::from_utf8(brain.interner().resolve(s).to_vec()).unwrap())
        .collect()
}

/// The canonical bytes of a set of facts: one line each, sorted.
fn canonical(header: String, lines: BTreeSet<String>) -> Vec<u8> {
    let mut out = header;
    for line in lines {
        out.push('\n');
        out.push_str(&line);
    }
    out.into_bytes()
}

fn run(capacity: Capacity, ruler: Option<Warmth>) -> (Light, Stats) {
    let trace = run_traced(capacity, ruler);
    (trace.light, trace.stats)
}

/// A run, with what it read beside the light at each committed tick.
struct Trace {
    light: Light,
    stats: Stats,
    /// `brain.lums()` after each commit: one value per committed tick.
    lums: Vec<u64>,
    /// New facts deposited in each committed tick (the input at tick 0).
    news: Vec<usize>,
    /// `brain.lums_of(r)` per relation, by name, at the end.
    lums_by_relation: BTreeMap<&'static str, u64>,
}

fn run_traced(capacity: Capacity, ruler: Option<Warmth>) -> Trace {
    run_traced_with(capacity, ruler, false)
}

/// A run; with `vacuum`, a vacuum pass over an empty sealed snapshot after
/// every settle, cooling every crystal it finds dangling (all of them).
fn run_traced_with(capacity: Capacity, ruler: Option<Warmth>, vacuum: bool) -> Trace {
    let mut lums = Vec::new();
    let mut news = Vec::new();
    let mut brain = Brain::new(capacity, Memory::default());
    let p = program(&mut brain);
    let names = BTreeMap::from([(p.edge, "edge"), (p.path, "path"), (p.both, "both")]);
    let calls = Rc::new(Cell::new(0));
    brain = brain.with_recompute(recompute(names.clone(), calls.clone()));
    if let Some(ruler) = ruler {
        brain = brain.with_ruler(ruler);
    }

    // The input, at tick 0.
    for (x, y) in EDGES {
        let t = [brain.intern(x.as_bytes()), brain.intern(y.as_bytes())];
        assert!(brain.deposit(p.edge, &t));
    }
    brain.commit();
    lums.push(brain.lums());
    news.push(EDGES.len());

    // Rounds to the fixpoint. Each round's facts settle at its committed point.
    let mut settled = Vec::new();
    loop {
        let new = round(&mut brain, &p.rules);
        let t = brain.commit();
        lums.push(brain.lums());
        news.push(new);
        if new == 0 {
            break;
        }
        let mut lines = BTreeSet::new();
        for (&rel, name) in &names {
            for (tuple, tick) in brain.scan(rel, &[None, None]) {
                if tick == t {
                    lines.insert(format!("{name} {}", named(&brain, &tuple).join(" ")));
                }
            }
        }
        assert_eq!(lines.len(), new);
        let bytes = canonical(format!("tick {}", t.0), lines);
        settled.push((brain.settle(bytes.clone()).unwrap(), bytes));
        if vacuum {
            let reach = brain.vacuum(&Roots::default().seal());
            brain.cool(reach.dangling());
        }
    }

    // Every node's reach settles too.
    for x in NODES {
        let s = brain.interner().get(x.as_bytes()).unwrap();
        let lines = brain
            .scan(p.path, &[Some(s), None])
            .into_iter()
            .map(|(tuple, tick)| format!("{} {}", named(&brain, &tuple)[1], tick.0))
            .collect();
        let bytes = canonical(format!("reach {x}"), lines);
        settled.push((brain.settle(bytes.clone()).unwrap(), bytes));
        if vacuum {
            let reach = brain.vacuum(&Roots::default().seal());
            brain.cool(reach.dangling());
        }
    }

    let mut facts = BTreeMap::new();
    for (&rel, name) in &names {
        for (tuple, tick) in brain.scan(rel, &[None, None]) {
            facts.insert((name.to_string(), named(&brain, &tuple)), tick);
        }
    }

    let sym = |brain: &Brain<Memory>, n: &str| brain.interner().get(n.as_bytes()).unwrap();
    let questions = [
        (p.path, "h", "g"),
        (p.path, "g", "a"),
        (p.both, "d", "f"),
        (p.both, "a", "d"),
        (p.both, "g", "g"),
        (p.edge, "f", "g"),
    ];
    let verdicts = questions
        .iter()
        .map(|&(rel, x, y)| {
            let t = [sym(&brain, x), sym(&brain, y)];
            brain.contains(rel, &t)
        })
        .collect();

    let read_back = settled
        .iter()
        .map(|(addr, _)| {
            let c = brain.crystal(addr).unwrap();
            (c.address().clone(), c.bytes().to_vec())
        })
        .collect();

    let ticks = brain.tick();
    let stats = brain.stats();
    assert_eq!(stats.recomputes, calls.get());
    // Reading lums and scanning everything back changed neither.
    assert_eq!(Some(&brain.lums()), lums.last());
    let lums_by_relation = names.iter().map(|(&r, &n)| (n, brain.lums_of(r))).collect();
    Trace {
        light: Light {
            facts,
            verdicts,
            settled,
            read_back,
            ticks,
        },
        stats,
        lums,
        news,
        lums_by_relation,
    }
}

// ---------------------------------------------------------------------------
// 1. Threshold independence.
// ---------------------------------------------------------------------------

#[test]
fn threshold_independence_tiny_and_unbounded_give_the_same_light() {
    let (wide, wide_stats) = run(Capacity::unbounded(), None);
    let (tiny, tiny_stats) = run(Capacity::bytes(0), None);
    eprintln!("unbounded: {wide_stats:?}");
    eprintln!("tiny:      {tiny_stats:?}");

    // The fixture is the program it claims to be.
    let expected: BTreeMap<(String, Vec<String>), Tick> = from_input()
        .into_iter()
        .map(|((r, args), t)| ((r.to_string(), args), Tick(t)))
        .collect();
    assert_eq!(wide.facts, expected);

    // The same facts, the same deposit ticks, the same crystals.
    assert_eq!(tiny.facts, wide.facts);
    assert_eq!(tiny.verdicts, wide.verdicts);
    assert_eq!(tiny.settled, wide.settled);
    assert_eq!(tiny.read_back, wide.read_back);
    assert_eq!(tiny, wide);

    // Eviction and recompute really happened at the small threshold, and
    // not at the unbounded one.
    assert!(tiny_stats.evicted_facts > 0, "{tiny_stats:?}");
    assert!(tiny_stats.restored_facts > 0, "{tiny_stats:?}");
    assert!(tiny_stats.recomputes > 0, "{tiny_stats:?}");
    assert!(tiny_stats.evicted_crystals > 0, "{tiny_stats:?}");
    assert!(tiny_stats.recalled_crystals > 0, "{tiny_stats:?}");
    assert_eq!(wide_stats.evicted_facts, 0);
    assert_eq!(wide_stats.evicted_crystals, 0);
    assert_eq!(wide_stats.recomputes, 0);
    assert_eq!(wide_stats.recalled_crystals, 0);
    assert!(tiny_stats.peak_hot_bytes < wide_stats.peak_hot_bytes);
}

// ---------------------------------------------------------------------------
// 2. Invariant 1: use can make a crystal warmer, never more true.
// ---------------------------------------------------------------------------

fn oldest_first(f: Fact<'_>) -> i64 {
    f.tick.0 as i64
}

fn newest_first(f: Fact<'_>) -> i64 {
    -(f.tick.0 as i64)
}

fn by_constants(f: Fact<'_>) -> i64 {
    f.tuple.iter().map(|s| i64::from(s.index())).sum()
}

fn scrambled(f: Fact<'_>) -> i64 {
    let h = f
        .tuple
        .iter()
        .fold(f.tick.0 * 97 + u64::from(f.relation.index()), |h, s| {
            h.wrapping_mul(2_654_435_761)
                .wrapping_add(u64::from(s.index()))
        });
    (h % 1009) as i64 - 504
}

fn all_equal(_: Fact<'_>) -> i64 {
    0
}

#[test]
fn warmer_never_truer_priority_and_threshold_never_change_the_light() {
    let (reference, _) = run(Capacity::unbounded(), None);
    let mut evictions = BTreeSet::new();
    for (name, ruler) in RULERS {
        for capacity in thresholds() {
            let (light, stats) = run(capacity, Some(ruler));
            evictions.insert(stats.evicted_facts);
            assert_eq!(light, reference, "{name} under {capacity:?}");
        }
    }
    // The runs really differed in what they held.
    assert!(evictions.len() > 3, "{evictions:?}");
}

// ---------------------------------------------------------------------------
// 2b. Lums: the count of light. Progress and monotonicity, never identity.
// ---------------------------------------------------------------------------

const RULERS: [(&str, Warmth); 5] = [
    ("oldest first", oldest_first),
    ("newest first", newest_first),
    ("by constants", by_constants),
    ("scrambled", scrambled),
    ("all equal", all_equal),
];

fn thresholds() -> [Capacity; 5] {
    [
        Capacity::bytes(0),
        Capacity::bytes(16),
        Capacity::bytes(100),
        Capacity::bytes(400),
        Capacity::unbounded(),
    ]
}

#[test]
fn lums_are_capacity_independent_evicted_facts_still_count() {
    let reference = run_traced(Capacity::unbounded(), None);
    eprintln!("reference lums: {:?}", reference.lums);
    let mut evicted_somewhere = 0;
    for (name, ruler) in RULERS {
        for capacity in thresholds() {
            let trace = run_traced(capacity, Some(ruler));
            eprintln!(
                "{name:>12} {capacity:?}: evicted {:>3} restored {:>3} lums {:?}",
                trace.stats.evicted_facts, trace.stats.restored_facts, trace.lums
            );
            if capacity == Capacity::unbounded() {
                assert_eq!(trace.stats.evicted_facts, 0);
            } else {
                // Every bounded run really forgot, and recomputed.
                assert!(trace.stats.evicted_facts > 0, "{name} under {capacity:?}");
                assert!(trace.stats.restored_facts > 0, "{name} under {capacity:?}");
                evicted_somewhere += 1;
            }
            assert_eq!(trace.lums, reference.lums, "{name} under {capacity:?}");
            assert_eq!(
                trace.lums_by_relation, reference.lums_by_relation,
                "{name} under {capacity:?}"
            );
        }
    }
    assert_eq!(evicted_somewhere, 20);
}

#[test]
fn lums_never_fall_across_commits_of_a_stable_program() {
    for (name, ruler) in RULERS {
        for capacity in thresholds() {
            let trace = run_traced(capacity, Some(ruler));
            for w in trace.lums.windows(2) {
                assert!(w[0] <= w[1], "{name} under {capacity:?}: {:?}", trace.lums);
            }
            // Each commit adds exactly the light its tick deposited.
            let mut sum = 0;
            for (&l, &n) in trace.lums.iter().zip(&trace.news) {
                sum += n as u64;
                assert_eq!(l, sum, "{name} under {capacity:?}");
            }
        }
    }
}

#[test]
fn equal_successive_lums_are_the_fixpoint() {
    let trace = run_traced(Capacity::bytes(0), None);
    assert!(trace.lums.len() > 2, "{:?}", trace.lums);
    for (i, w) in trace.lums.windows(2).enumerate() {
        // Equal counts iff the round derived nothing new.
        assert_eq!(w[0] == w[1], trace.news[i + 1] == 0, "{:?}", trace.lums);
    }
    // Only the last pair is equal: the run stops at the fixpoint.
    let n = trace.lums.len();
    assert_eq!(trace.lums[n - 1], trace.lums[n - 2]);
    assert!(trace.lums[..n - 1].windows(2).all(|w| w[0] < w[1]));
    // At the fixpoint, lums count every fact of the program, per relation
    // and in all.
    let all = from_input();
    assert_eq!(*trace.lums.last().unwrap(), all.len() as u64);
    for (rel, count) in &trace.lums_by_relation {
        let expected = all.keys().filter(|(r, _)| r == rel).count() as u64;
        assert_eq!(*count, expected, "{rel}");
    }
    assert_eq!(
        trace.lums_by_relation.values().sum::<u64>(),
        all.len() as u64
    );
}

#[test]
fn lums_never_double_count_a_restored_fact() {
    let mut brain = Brain::new(Capacity::bytes(0), Memory::default());
    let edge = brain.relation("edge", 2);
    let path = brain.relation("path", 2);
    let (a, b, c) = (brain.intern(b"a"), brain.intern(b"b"), brain.intern(b"c"));
    // Recompute knows exactly what this test deposits, with its ticks.
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    brain = brain.with_recompute(move |_: &Interner, rel: Relation, _: &[Option<Sym>]| {
        counted.set(counted.get() + 1);
        if rel == edge {
            vec![(vec![a, b], Tick(0)), (vec![b, c], Tick(0))]
        } else {
            vec![(vec![a, b], Tick(2))]
        }
    });
    assert_eq!(brain.lums(), 0);

    assert!(brain.deposit(edge, &[a, b]));
    assert!(brain.deposit(edge, &[b, c]));
    // The open tick's light is not read until it commits.
    assert_eq!(brain.lums(), 0);
    brain.commit();
    assert_eq!(brain.lums(), 2);
    assert_eq!(brain.lums_of(edge), 2);
    assert_eq!(brain.lums_of(path), 0);
    // Everything committed is evicted at capacity 0; lums stay.
    assert_eq!(brain.stats().evicted_facts, 2);
    assert_eq!(brain.stats().hot_bytes, 0);
    assert_eq!(brain.lums(), 2);

    // Recompute restores both, at their original tick. Lums stay.
    let back = brain.scan(edge, &[None, None]);
    assert_eq!(back, vec![(vec![a, b], Tick(0)), (vec![b, c], Tick(0))]);
    assert_eq!(brain.stats().restored_facts, 2);
    assert!(calls.get() > 0);
    assert_eq!(brain.lums(), 2);

    // Depositing a restored (or evicted) fact again is not new light.
    assert!(!brain.deposit(edge, &[a, b]));
    brain.commit();
    assert!(!brain.deposit(edge, &[b, c]), "evicted, recomputed, known");
    assert!(brain.deposit(path, &[a, b]));
    brain.commit();
    assert_eq!(brain.lums(), 3);
    assert_eq!(brain.lums_of(edge), 2);
    assert_eq!(brain.lums_of(path), 1);
    assert!(brain.stats().restored_facts > 2, "{:?}", brain.stats());
}

#[test]
fn lums_are_never_identity_the_same_crystals_settle_the_same_addresses() {
    // One brain derives the whole program; the other holds only the input.
    // Both settle the same crystals.
    let settle = |derive: bool| {
        let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
        let p = program(&mut brain);
        for (x, y) in EDGES {
            let t = [brain.intern(x.as_bytes()), brain.intern(y.as_bytes())];
            brain.deposit(p.edge, &t);
        }
        brain.commit();
        let mut history = vec![brain.lums()];
        if derive {
            // Derived light, never settled.
            while round(&mut brain, &p.rules) > 0 {
                brain.commit();
                history.push(brain.lums());
            }
        }
        let lines = brain
            .scan(p.edge, &[None, None])
            .into_iter()
            .map(|(tuple, _)| format!("edge {}", named(&brain, &tuple).join(" ")))
            .collect();
        let bytes = canonical("input".to_string(), lines);
        let address = brain.settle(bytes).unwrap();
        let crystal = brain.crystal(&address).unwrap();
        (address, crystal, history)
    };
    let (dim, dim_crystal, dim_lums) = settle(false);
    let (lit, lit_crystal, lit_lums) = settle(true);
    assert_ne!(dim_lums, lit_lums);
    assert!(
        dim_lums.last() < lit_lums.last(),
        "{dim_lums:?} {lit_lums:?}"
    );
    assert_eq!(dim, lit);
    assert_eq!(dim_crystal, lit_crystal);
    // The address is the bytes' alone.
    assert_eq!(&Oid::hash(&Scope::public(), lit_crystal.bytes()), &lit);
}

// ---------------------------------------------------------------------------
// 3. Invariant 2: forgetting costs nothing, because nothing is lost.
// ---------------------------------------------------------------------------

/// A backend that counts its reads: how we know a read came from cold.
#[derive(Default)]
struct Counted {
    inner: Memory,
    fetches: Cell<usize>,
}

impl Backend for Counted {
    type Error = Infallible;
    fn persist(&mut self, crystal: &Crystal) -> Result<(), Infallible> {
        self.inner.persist(crystal)
    }
    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        self.fetches.set(self.fetches.get() + 1);
        self.inner.fetch(address)
    }
}

#[test]
fn forgetting_costs_nothing_an_evicted_crystal_reads_back_identical() {
    let mut brain = Brain::new(Capacity::bytes(0), Counted::default());
    let bytes = b"edge a b\nedge b c".to_vec();
    let address = brain.settle(bytes.clone()).unwrap();
    assert_eq!(brain.stats().evicted_crystals, 1);
    assert_eq!(brain.stats().hot_bytes, 0);
    assert_eq!(brain.backend().inner.len(), 1);

    let back = brain.crystal(&address).unwrap();
    assert_eq!(brain.backend().fetches.get(), 1, "read from cold");
    assert_eq!(brain.stats().recalled_crystals, 1);
    assert_eq!(back.bytes(), &bytes[..]);
    assert_eq!(back.address(), &address);
    // The address recomputes from the bytes alone.
    assert_eq!(Oid::hash(&Scope::public(), back.bytes()), address);
    assert_eq!(Oid::hash(brain.scope(), back.bytes()), address);
}

#[test]
fn a_hot_crystal_reads_without_the_backend() {
    let mut brain = Brain::new(Capacity::unbounded(), Counted::default());
    let address = brain.settle(b"reach a".to_vec()).unwrap();
    let hot = brain.crystal(&address).unwrap();
    assert_eq!(brain.backend().fetches.get(), 0);
    assert_eq!(hot.bytes(), b"reach a");
    assert_eq!(brain.stats().evicted_crystals, 0);
    assert_eq!(brain.stats().hot_bytes, 7);
}

/// A backend that loses what it was given.
struct Forgetful;

impl Backend for Forgetful {
    type Error = Infallible;
    fn persist(&mut self, _: &Crystal) -> Result<(), Infallible> {
        Ok(())
    }
    fn fetch(&self, _: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        Ok(None)
    }
}

/// A backend that hands back other bytes than it was given.
struct Forger;

impl Backend for Forger {
    type Error = Infallible;
    fn persist(&mut self, _: &Crystal) -> Result<(), Infallible> {
        Ok(())
    }
    fn fetch(&self, _: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        Ok(Some(b"something else".to_vec()))
    }
}

/// A backend that refuses.
struct Refusing;

#[derive(Debug, PartialEq)]
struct Refused;

impl Backend for Refusing {
    type Error = Refused;
    fn persist(&mut self, _: &Crystal) -> Result<(), Refused> {
        Err(Refused)
    }
    fn fetch(&self, _: &Oid) -> Result<Option<Vec<u8>>, Refused> {
        Err(Refused)
    }
}

#[test]
fn a_backend_that_loses_or_forges_is_named_never_believed() {
    let mut lost = Brain::new(Capacity::bytes(0), Forgetful);
    let a = lost.settle(b"reach b".to_vec()).unwrap();
    assert_eq!(lost.crystal(&a), Err(Recall::Lost(a.clone())));

    let mut forged = Brain::new(Capacity::bytes(0), Forger);
    let b = forged.settle(b"reach c".to_vec()).unwrap();
    assert_eq!(forged.crystal(&b), Err(Recall::Forged(b.clone())));

    let mut refusing = Brain::new(Capacity::unbounded(), Refusing);
    assert_eq!(refusing.settle(b"reach d".to_vec()), Err(Refused));
    let never = Oid::hash(&Scope::public(), b"reach d");
    assert_eq!(refusing.crystal(&never), Err(Recall::Backend(Refused)));
    assert_eq!(refusing.stats().hot_bytes, 0, "not persisted, not held");
}

#[test]
fn without_a_recompute_no_fact_is_ever_forgotten() {
    let mut brain = Brain::new(Capacity::bytes(0), Memory::default());
    let edge = brain.relation("edge", 2);
    let (a, b) = (brain.intern(b"a"), brain.intern(b"b"));
    assert!(brain.deposit(edge, &[a, b]));
    assert!(!brain.deposit(edge, &[a, b]), "already known");
    assert_eq!(brain.commit(), Tick(0));
    assert_eq!(brain.stats().evicted_facts, 0);
    assert_eq!(brain.contains(edge, &[a, b]), Some(Tick(0)));
    assert_eq!(brain.contains(edge, &[b, a]), None);
    assert_eq!(brain.stats().hot_bytes, 16);
}

#[test]
fn an_uncommitted_fact_is_never_evicted() {
    let mut brain = Brain::new(Capacity::bytes(0), Memory::default()).with_recompute(
        |_: &Interner, _: Relation, _: &[Option<Sym>]| Vec::<(Vec<Sym>, Tick)>::new(),
    );
    let edge = brain.relation("edge", 2);
    let (a, b) = (brain.intern(b"a"), brain.intern(b"b"));
    assert!(brain.deposit(edge, &[a, b]));
    // A settle presses back, but the open tick isn't committed yet.
    brain.settle(b"tick 0".to_vec()).unwrap();
    assert_eq!(brain.stats().evicted_facts, 0);
    assert_eq!(brain.committed(), None);
    brain.commit();
    assert_eq!(brain.committed(), Some(Tick(0)));
    assert_eq!(brain.stats().evicted_facts, 1);
}

// ---------------------------------------------------------------------------
// 4. Invariant 3: only the Drop makes anything new. (The API-shape half is
//    the `compile_fail` doctests on `brain::Crystal`.)
// ---------------------------------------------------------------------------

#[test]
fn only_the_drop_makes_new_a_new_crystal_is_a_new_address() {
    let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
    let first = brain.settle(b"edge a b".to_vec()).unwrap();
    let second = brain.settle(b"edge a b\nedge b c".to_vec()).unwrap();
    assert_ne!(first, second);
    // Settling the same bytes again is not new: same address, nothing added.
    assert_eq!(brain.settle(b"edge a b".to_vec()).unwrap(), first);
    assert_eq!(brain.backend().len(), 2);
    assert_eq!(brain.crystal(&first).unwrap().bytes(), b"edge a b");

    // A crystal made outside a brain is the same crystal.
    let c = Crystal::settle(&Scope::public(), b"edge a b".to_vec());
    assert_eq!(c.address(), &first);
    assert_eq!(c, brain.crystal(&first).unwrap());
    let clone = c.clone();
    assert_eq!(clone.bytes(), c.bytes());
}

#[test]
fn interned_constants_and_relations_are_stable() {
    let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
    let a = brain.intern(b"a");
    assert_eq!(brain.intern(b"a"), a);
    assert_ne!(brain.intern(b"b"), a);
    assert_eq!(brain.interner().resolve(a), b"a");
    assert_eq!(brain.interner().get(b"zz"), None);
    let edge = brain.relation("edge", 2);
    assert_eq!(brain.relation("edge", 2), edge);
    assert_ne!(
        brain.relation("edge", 3),
        edge,
        "a relation is a name and an arity"
    );
    assert_eq!(brain.tick(), Tick(0));
}

// ---------------------------------------------------------------------------
// 6. The vacuum (prism-013 §3): reach from a sealed snapshot of roots, cool
//    what isn't reached, and never purge in the public scope.
// ---------------------------------------------------------------------------

/// An address in a scope this brain doesn't hold: dark here, not dangling.
fn elsewhere(bytes: &[u8]) -> Oid {
    Oid::hash(&Scope::keyed([7; 32]), bytes)
}

/// A small public crystal DAG, by name. References are declared beside the
/// bytes, never in them:
///
/// ```text
/// identity → a → {b, c}     b → d     c → {d, x̄}     d → ∅
/// e → f     f → ȳ     g → a                 (x̄, ȳ: dark, another scope)
/// ```
///
/// Held as a root, `identity` reaches a, b, c and d. e, f and g are settled
/// here and unreached: dangling. g points *into* the reached part, which
/// doesn't make it reached.
struct Dag {
    brain: Brain<Counted>,
    at: BTreeMap<&'static str, Oid>,
    bytes: BTreeMap<&'static str, Vec<u8>>,
}

impl Dag {
    fn new() -> Self {
        let mut dag = Dag {
            brain: Brain::new(Capacity::unbounded(), Counted::default()),
            at: BTreeMap::new(),
            bytes: BTreeMap::new(),
        };
        // Targets first, so every reference is an address already settled
        // (or dark).
        dag.settle("d", &[]);
        dag.settle("b", &["d"]);
        dag.settle_to("c", &["d"], &[elsewhere(b"x")]);
        dag.settle("a", &["b", "c"]);
        dag.settle("identity", &["a"]);
        dag.settle_to("f", &[], &[elsewhere(b"y")]);
        dag.settle("e", &["f"]);
        dag.settle("g", &["a"]);
        dag
    }

    fn settle(&mut self, name: &'static str, refs: &[&str]) {
        self.settle_to(name, refs, &[]);
    }

    fn settle_to(&mut self, name: &'static str, refs: &[&str], dark: &[Oid]) {
        let bytes = format!("node {name}").into_bytes();
        let references: Vec<Oid> = refs
            .iter()
            .map(|r| self.at[r].clone())
            .chain(dark.iter().cloned())
            .collect();
        let address = self.brain.settle_with(bytes.clone(), references).unwrap();
        self.at.insert(name, address);
        self.bytes.insert(name, bytes);
    }

    fn set(&self, names: &[&str]) -> BTreeSet<Oid> {
        names.iter().map(|n| self.at[n].clone()).collect()
    }

    fn bytes_of(&self, names: &[&str]) -> usize {
        names.iter().map(|n| self.bytes[n].len()).sum()
    }

    /// Every named crystal reads back by address with its settled bytes.
    fn reads_back(&self, names: &[&str]) {
        for n in names {
            let c = self.brain.crystal(&self.at[n]).unwrap();
            assert_eq!(c.bytes(), &self.bytes[n][..], "{n}");
            assert_eq!(c.address(), &self.at[n], "{n}");
        }
    }
}

const REACHED: &[&str] = &["identity", "a", "b", "c", "d"];
const UNREACHED: &[&str] = &["e", "f", "g"];

#[test]
fn the_vacuum_classifies_reachable_dangling_and_dark_exactly() {
    let mut dag = Dag::new();
    let mut roots = Roots::default();
    roots.hold(dag.at["identity"].clone());
    // A root held in another scope: this brain can't see into it.
    roots.hold(elsewhere(b"their identity"));
    let snapshot = roots.seal();

    let reach = dag.brain.vacuum(&snapshot);
    eprintln!("reachable: {:?}", reach.reachable());
    eprintln!("dangling:  {:?}", reach.dangling().addresses());
    eprintln!("dark:      {:?}", reach.dark());
    assert_eq!(reach.reachable(), &dag.set(REACHED));
    assert_eq!(reach.dangling().addresses(), &dag.set(UNREACHED));
    // Dark: referenced (or held as a root), but not in this scope. Never
    // dangling: what it reaches is unknown here.
    let dark = BTreeSet::from([
        elsewhere(b"x"),
        elsewhere(b"y"),
        elsewhere(b"their identity"),
    ]);
    assert_eq!(reach.dark(), &dark);
    assert!(reach.dark().is_disjoint(reach.dangling().addresses()));
    // The verdict says which snapshot it was decided on.
    assert_eq!(reach.dangling().as_of(), &snapshot);
}

#[test]
fn cooling_the_dangling_is_lossless_every_read_back_is_identical() {
    let mut dag = Dag::new();
    let mut roots = Roots::default();
    roots.hold(dag.at["identity"].clone());
    let reach = dag.brain.vacuum(&roots.seal());

    let hot = dag.brain.stats().hot_bytes;
    let lums = dag.brain.lums();
    assert_eq!(dag.brain.cool(reach.dangling()), UNREACHED.len());
    assert_eq!(dag.brain.stats().hot_bytes, hot - dag.bytes_of(UNREACHED));
    assert_eq!(dag.brain.stats().evicted_crystals, UNREACHED.len());
    assert_eq!(dag.brain.lums(), lums);
    // Cooling again finds nothing hot to cool.
    assert_eq!(dag.brain.cool(reach.dangling()), 0);
    // The backend still holds every crystal.
    assert_eq!(
        dag.brain.backend().inner.len(),
        REACHED.len() + UNREACHED.len()
    );

    // The reachable read back hot, without the backend.
    dag.reads_back(REACHED);
    assert_eq!(dag.brain.backend().fetches.get(), 0);
    // The cooled read back from cold, identical.
    dag.reads_back(UNREACHED);
    assert_eq!(dag.brain.backend().fetches.get(), UNREACHED.len());
}

#[test]
fn dangling_is_never_final_a_new_root_reaches_a_cooled_crystal_again() {
    let mut dag = Dag::new();
    let mut roots = Roots::default();
    roots.hold(dag.at["identity"].clone());
    let first = dag.brain.vacuum(&roots.seal());
    dag.brain.cool(first.dangling());
    assert!(first.dangling().addresses().contains(&dag.at["e"]));

    // A new identity reaches e (and through it f).
    roots.hold(dag.at["e"].clone());
    let second = dag.brain.vacuum(&roots.seal());
    eprintln!(
        "after e is held: dangling {:?}",
        second.dangling().addresses()
    );
    let mut reached = dag.set(REACHED);
    reached.extend(dag.set(&["e", "f"]));
    assert_eq!(second.reachable(), &reached);
    assert_eq!(second.dangling().addresses(), &dag.set(&["g"]));
    // Reachable again, and its read-back identical: cooling lost nothing.
    dag.reads_back(&["e", "f"]);
}

#[test]
fn a_vacuum_over_a_snapshot_never_sees_roots_held_after_sealing() {
    let mut dag = Dag::new();
    let mut roots = Roots::default();
    roots.hold(dag.at["identity"].clone());
    let snapshot = roots.seal();
    roots.hold(dag.at["e"].clone());
    roots.hold(dag.at["g"].clone());

    assert_eq!(snapshot.roots(), &dag.set(&["identity"]));
    let stale = dag.brain.vacuum(&snapshot);
    assert_eq!(stale.reachable(), &dag.set(REACHED));
    assert_eq!(stale.dangling().addresses(), &dag.set(UNREACHED));

    // A fresh seal sees them.
    let fresh = dag.brain.vacuum(&roots.seal());
    assert!(fresh.dangling().addresses().is_empty());
}

#[test]
fn the_vacuum_is_light_neutral_results_and_lums_never_change() {
    for capacity in thresholds() {
        let plain = run_traced_with(capacity, None, false);
        let vacuumed = run_traced_with(capacity, None, true);
        assert_eq!(vacuumed.light, plain.light, "{capacity:?}");
        assert_eq!(vacuumed.lums, plain.lums, "{capacity:?}");
        assert_eq!(vacuumed.news, plain.news, "{capacity:?}");
        assert_eq!(
            vacuumed.lums_by_relation, plain.lums_by_relation,
            "{capacity:?}"
        );
        if capacity == Capacity::unbounded() {
            // Only the vacuum cooled anything, and it cooled every crystal.
            assert_eq!(plain.stats.evicted_crystals, 0);
            assert_eq!(
                vacuumed.stats.evicted_crystals,
                plain.light.settled.len(),
                "{:?}",
                vacuumed.stats
            );
        }
    }
}

#[test]
fn references_never_change_an_address() {
    let bytes = b"node a".to_vec();
    let mut plain = Brain::new(Capacity::unbounded(), Memory::default());
    let mut referring = Brain::new(Capacity::unbounded(), Memory::default());
    let a = plain.settle(bytes.clone()).unwrap();
    let b = referring
        .settle_with(
            bytes.clone(),
            [elsewhere(b"x"), Oid::hash(&Scope::public(), b"node b")],
        )
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(a, Oid::hash(&Scope::public(), &bytes));
    assert_eq!(plain.crystal(&a).unwrap(), referring.crystal(&b).unwrap());

    // Declaring the same bytes again with other references: the same
    // address, nothing new persisted, and the references accumulate.
    let c = referring
        .settle_with(bytes.clone(), [elsewhere(b"y")])
        .unwrap();
    assert_eq!(c, a);
    assert_eq!(referring.backend().len(), 1);
    assert_eq!(referring.references(&a).map(BTreeSet::len), Some(3));
    assert_eq!(plain.references(&a), Some(&BTreeSet::new()));
    assert_eq!(plain.references(&elsewhere(b"x")), None);
}

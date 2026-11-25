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
    purge, recall, Backend, Brain, Capacity, Crystal, Epoch, Erasure, Fact, Held, Interner, Loss,
    Memory, Purge, Recall, Receipt, Relation, Roots, Slot, Stats, Strange, Sym, Tick,
};
use prismqueer::crossing::Grade;
use prismqueer::oid::{Oid, Scope, ScopeKey};

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
    fn purge(&mut self, order: &Purge<'_>) -> Result<(), Infallible> {
        self.inner.purge(order)
    }
    fn erased(&self, address: &Oid) -> Result<Option<Receipt>, Infallible> {
        self.inner.erased(address)
    }
    fn write(&mut self, slot: &Slot<'_>) -> Result<(), Infallible> {
        self.inner.write(slot)
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
    fn purge(&mut self, _: &Purge<'_>) -> Result<(), Infallible> {
        Ok(())
    }
    fn erased(&self, _: &Oid) -> Result<Option<Receipt>, Infallible> {
        Ok(None)
    }
    fn write(&mut self, _: &Slot<'_>) -> Result<(), Infallible> {
        Ok(())
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
    fn purge(&mut self, _: &Purge<'_>) -> Result<(), Infallible> {
        Ok(())
    }
    fn erased(&self, _: &Oid) -> Result<Option<Receipt>, Infallible> {
        Ok(None)
    }
    fn write(&mut self, _: &Slot<'_>) -> Result<(), Infallible> {
        Ok(())
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
    fn purge(&mut self, _: &Purge<'_>) -> Result<(), Refused> {
        Err(Refused)
    }
    fn erased(&self, _: &Oid) -> Result<Option<Receipt>, Refused> {
        Err(Refused)
    }
    fn write(&mut self, _: &Slot<'_>) -> Result<(), Refused> {
        Err(Refused)
    }
}

#[test]
fn a_backend_that_loses_or_forges_is_named_never_believed() {
    let mut lost = Brain::new(Capacity::bytes(0), Forgetful);
    let a = lost.settle(b"reach b".to_vec()).unwrap();
    assert_eq!(
        lost.crystal(&a),
        Err(Recall::Lost {
            address: a.clone(),
            reason: Loss::Missing
        })
    );

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
    let key = ScopeKey::take(&mut [7; 32]).expect("a throwaway held key");
    Oid::hash(&Scope::keyed(&key), bytes)
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
    let dag = Dag::new();
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
    let dag = Dag::new();
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

// ---------------------------------------------------------------------------
// 7. Keyed scopes and erasure (prism-026; docs/spec/brain.md, "Keyed scopes
//    and erasure"). Synthetic bytes and throwaway keys only (Mara's F8).
// ---------------------------------------------------------------------------

/// A throwaway held key: synthetic bytes, never a real secret.
fn throwaway(byte: u8) -> ScopeKey {
    ScopeKey::take(&mut [byte; 32]).expect("a throwaway held key")
}

/// `n` synthetic crystals settled in `key`'s scope, through `backend`.
fn settled<B: Backend<Error = Infallible>>(
    backend: &mut B,
    key: &ScopeKey,
    tag: &str,
    n: usize,
) -> Vec<Crystal> {
    let mut held = Held::open(backend, key);
    (0..n)
        .map(|i| {
            let bytes = format!("{tag} {i}").into_bytes();
            let address = held.settle_with(bytes.clone(), []).unwrap();
            let crystal = Crystal::settle(&Scope::keyed(key), bytes);
            assert_eq!(&address, crystal.address());
            crystal
        })
        .collect()
}

#[test]
fn an_erasure_names_its_scope_by_public_id_and_covers_its_index() {
    let key = throwaway(0x11);
    let id = key.id();
    let mut backend = Memory::default();
    let census = settled(&mut backend, &key, "node", 3);
    let slots = Held::open(&mut backend, &key).slots();
    let erasure = Erasure::mint(key, Epoch(3), &backend).expect("all in the scope");
    assert_eq!(erasure.scope(), id);
    assert_ne!(erasure.scope(), Scope::public().id());
    for c in &census {
        assert!(erasure.covers().contains(c.address()), "{c:?}");
    }
    for slot in [slots.manifest(), slots.light(), slots.root(), slots.chain()] {
        assert!(erasure.covers().contains(slot), "{slot:?}");
    }
}

/// A faulty adapter that lists one more address in a scope's manifest than
/// the scope settled there.
struct Planted {
    inner: Memory,
    manifest: Oid,
    extra: Oid,
}

impl Backend for Planted {
    type Error = Infallible;
    fn persist(&mut self, crystal: &Crystal) -> Result<(), Infallible> {
        self.inner.persist(crystal)
    }
    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        let held = self.inner.fetch(address)?;
        if address != &self.manifest {
            return Ok(held);
        }
        let mut list = held.unwrap_or_default();
        if !list.is_empty() {
            list.push(b'\n');
        }
        list.extend_from_slice(self.extra.as_str().as_bytes());
        Ok(Some(list))
    }
    fn purge(&mut self, order: &Purge<'_>) -> Result<(), Infallible> {
        self.inner.purge(order)
    }
    fn erased(&self, address: &Oid) -> Result<Option<Receipt>, Infallible> {
        self.inner.erased(address)
    }
    fn write(&mut self, slot: &Slot<'_>) -> Result<(), Infallible> {
        self.inner.write(slot)
    }
}

#[test]
fn an_erasure_never_covers_another_scopes_crystal_and_hands_the_key_back() {
    let key = throwaway(0x11);
    let other = throwaway(0x22);
    let mut inner = Memory::default();
    let _ = settled(&mut inner, &key, "node", 2);
    let foreign = settled(&mut inner, &other, "node", 1).remove(0);
    let public = Crystal::settle(&Scope::public(), b"node 1".to_vec());
    inner.persist(&public).unwrap();
    let manifest = Held::open(&mut inner, &key).slots().manifest().clone();
    let mut planted = Planted {
        inner,
        manifest,
        extra: foreign.address().clone(),
    };

    let stranger = Erasure::mint(key, Epoch(3), &planted).expect_err("another holder's crystal");
    assert_eq!(
        stranger.reason(),
        &Strange::Foreign(foreign.address().clone())
    );
    assert_eq!(stranger.address(), Some(foreign.address()));
    let key = stranger.into_key();
    assert_eq!(key.id(), throwaway(0x11).id(), "the same key, unconsumed");

    planted.extra = public.address().clone();
    let stranger = Erasure::mint(key, Epoch(3), &planted).expect_err("a public crystal");
    assert_eq!(stranger.address(), Some(public.address()));
    let shown = format!("{stranger:?}");
    assert!(shown.contains("ScopeKey(held)"), "{shown}");
    assert!(!shown.contains("17, 17"), "{shown}");

    // A listed address with no bytes is the scope's own list's: covered.
    let key = stranger.into_key();
    planted.extra = Oid::hash(&Scope::keyed(&key), b"node lost");
    let erasure = Erasure::mint(key, Epoch(3), &planted).expect("missing bytes are covered");
    assert!(erasure.covers().contains(&planted.extra));
}

#[test]
fn the_receipt_preimage_is_exactly_scope_id_then_epoch() {
    let key = throwaway(0x33);
    let mut backend = Memory::default();
    let _ = settled(&mut backend, &key, "node", 1);
    let erasure = Erasure::mint(key, Epoch(0x0102_0304_0506_0708), &backend).unwrap();
    let receipt = erasure.receipt();

    // Computed here from the key's bytes and the epoch alone, never through
    // the receipt: BLAKE3-derive-key(scope-id context, K) ‖ epoch (BE).
    let mut expected =
        blake3::derive_key("systemic.engineering://scope-id/v1", &[0x33; 32]).to_vec();
    expected.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(expected.len(), 40);
    assert_eq!(
        receipt.preimage(),
        expected,
        "scope id ‖ epoch, nothing else"
    );
    assert_eq!(receipt.address(), &Oid::hash(&Scope::public(), &expected));
    assert_eq!(receipt.scope(), erasure.scope());
    assert_eq!(receipt.epoch(), Epoch(0x0102_0304_0506_0708));

    // Another epoch, another receipt; the same scope and epoch, the same
    // address whatever the census (it never enters the address).
    let again = Erasure::mint(
        throwaway(0x33),
        Epoch(0x0102_0304_0506_0708),
        &Memory::default(),
    )
    .unwrap();
    assert_eq!(again.receipt().address(), receipt.address());
    let later = Erasure::mint(throwaway(0x33), Epoch(9), &Memory::default()).unwrap();
    assert_ne!(later.receipt().address(), receipt.address());
}

#[test]
fn the_receipt_is_sealed_by_the_scope_never_a_law_and_never_names_the_person() {
    let key = throwaway(0x44);
    let id = key.id();
    let erasure = Erasure::mint(key, Epoch(1), &Memory::default()).unwrap();
    let receipt = erasure.receipt();
    assert_eq!(receipt.grade(), Grade::Sealed);
    assert_ne!(receipt.grade(), Grade::Law);
    assert!(
        receipt.claim().proof().is_none(),
        "no proof: an attested act"
    );
    let seals: Vec<_> = receipt.claim().seals().iter().collect();
    assert_eq!(seals.len(), 1);
    assert_eq!(
        seals[0].by(),
        &id.oid(),
        "the signer is the scope, beside it"
    );
    assert_eq!(seals[0].over(), receipt.address());
    assert_eq!(receipt.claim().value(), receipt.address());
}

// --- The purge law (item 2) and Lost's reason (E1) -------------------------

/// A second backend, written independently of `Memory`: flat lists, linear
/// scans. The purge law holds of the port, so it must hold here too.
#[derive(Default)]
struct Ledger {
    held: Vec<(Oid, Vec<u8>)>,
    /// Every slot ever written, in first-write order, with its bytes now.
    slots: Vec<(Oid, Vec<u8>)>,
    tombstones: Vec<(Oid, Receipt)>,
}

impl Backend for Ledger {
    type Error = Infallible;
    fn persist(&mut self, crystal: &Crystal) -> Result<(), Infallible> {
        let address = crystal.address();
        let known = self.held.iter().any(|(a, _)| a == address);
        let erased = self.tombstones.iter().any(|(a, _)| a == address);
        if !known && !erased {
            self.held.push((address.clone(), crystal.bytes().to_vec()));
        }
        Ok(())
    }
    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        Ok(self
            .held
            .iter()
            .chain(&self.slots)
            .find(|(a, _)| a == address)
            .map(|(_, b)| b.clone()))
    }
    fn purge(&mut self, order: &Purge<'_>) -> Result<(), Infallible> {
        self.held.retain(|(a, _)| a != order.address());
        self.slots.retain(|(a, _)| a != order.address());
        self.tombstones
            .push((order.address().clone(), order.receipt().clone()));
        Ok(())
    }
    fn erased(&self, address: &Oid) -> Result<Option<Receipt>, Infallible> {
        Ok(self
            .tombstones
            .iter()
            .find(|(a, _)| a == address)
            .map(|(_, r)| r.clone()))
    }
    fn write(&mut self, slot: &Slot<'_>) -> Result<(), Infallible> {
        if self.tombstones.iter().any(|(a, _)| a == slot.address()) {
            return Ok(());
        }
        match self.slots.iter_mut().find(|(a, _)| a == slot.address()) {
            Some((_, bytes)) => *bytes = slot.bytes().to_vec(),
            None => self
                .slots
                .push((slot.address().clone(), slot.bytes().to_vec())),
        }
        Ok(())
    }
}

/// The purge law, over one backend: X's scope purged, Y's held scope and
/// the public scope untouched.
fn purge_law<B: Backend<Error = Infallible>>(mut backend: B) {
    let x = throwaway(0x51);
    let y = throwaway(0x52);
    let xs = settled(&mut backend, &x, "node", 3);
    let ys = settled(&mut backend, &y, "node", 3);
    let public: Vec<Crystal> = (0..3)
        .map(|i| Crystal::settle(&Scope::public(), format!("node {i}").into_bytes()))
        .collect();
    for c in &public {
        backend.persist(c).unwrap();
    }
    for c in &xs {
        assert_eq!(
            recall(&backend, &Scope::keyed(&x), c.address()).as_ref(),
            Ok(c)
        );
    }

    let erasure = Erasure::mint(x, Epoch(7), &backend).expect("X's own crystals");
    let receipt = erasure.receipt().clone();
    purge(&mut backend, &erasure).unwrap();

    for c in &xs {
        // Y reads its edge into X without X's key (Y's scope, or the public
        // one): Lost, erased, with the receipt.
        for reader in [Scope::keyed(&y), Scope::public()] {
            assert_eq!(
                recall(&backend, &reader, c.address()),
                Err(Recall::Lost {
                    address: c.address().clone(),
                    reason: Loss::Erased(receipt.clone())
                })
            );
        }
        assert_eq!(backend.fetch(c.address()), Ok(None), "the bytes are gone");
        assert_eq!(backend.erased(c.address()), Ok(Some(receipt.clone())));
        // A purge is final: persisting the same crystal again stores nothing.
        backend.persist(c).unwrap();
        assert_eq!(backend.fetch(c.address()), Ok(None), "not stored again");
    }
    for c in &ys {
        assert_eq!(
            recall(&backend, &Scope::keyed(&y), c.address()).as_ref(),
            Ok(c)
        );
        assert_eq!(backend.erased(c.address()), Ok(None));
    }
    for c in &public {
        assert_eq!(
            recall(&backend, &Scope::public(), c.address()).as_ref(),
            Ok(c)
        );
    }
}

#[test]
fn after_purge_recall_is_lost_erased_on_memory() {
    purge_law(Memory::default());
}

#[test]
fn after_purge_recall_is_lost_erased_on_the_ledger() {
    purge_law(Ledger::default());
}

#[test]
fn after_purge_recall_is_lost_erased_through_a_wrapping_backend() {
    purge_law(Counted::default());
}

#[test]
fn erased_and_missing_never_compare_equal_and_only_erased_carries_a_receipt() {
    let x = throwaway(0x61);
    let mut backend = Memory::default();
    let xs = settled(&mut backend, &x, "node", 1);
    let erasure = Erasure::mint(x, Epoch(1), &backend).unwrap();
    purge(&mut backend, &erasure).unwrap();

    let never = Oid::hash(&Scope::public(), b"node never");
    let Err(Recall::Lost { reason: erased, .. }) =
        recall(&backend, &Scope::public(), xs[0].address())
    else {
        panic!("an erased address is lost");
    };
    let Err(Recall::Lost {
        reason: missing, ..
    }) = recall(&backend, &Scope::public(), &never)
    else {
        panic!("a never-settled address is lost");
    };
    assert_eq!(erased, Loss::Erased(erasure.receipt().clone()));
    assert_eq!(missing, Loss::Missing);
    assert_ne!(erased, missing);
    assert_eq!(erased.receipt(), Some(erasure.receipt()));
    assert_eq!(missing.receipt(), None);
    // The same address lost both ways still reads two different answers.
    assert_ne!(
        Recall::<Infallible>::Lost {
            address: never.clone(),
            reason: erased.clone()
        },
        Recall::<Infallible>::Lost {
            address: never,
            reason: missing
        }
    );
}

/// A faulty backend that keeps the bytes it was told to purge: the read
/// fails closed, because a purge is final.
#[derive(Default)]
struct Hoarding {
    inner: Memory,
    tombstones: BTreeMap<Oid, Receipt>,
}

impl Backend for Hoarding {
    type Error = Infallible;
    fn persist(&mut self, crystal: &Crystal) -> Result<(), Infallible> {
        self.inner.persist(crystal)
    }
    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        self.inner.fetch(address)
    }
    fn purge(&mut self, order: &Purge<'_>) -> Result<(), Infallible> {
        self.tombstones
            .insert(order.address().clone(), order.receipt().clone());
        Ok(())
    }
    fn erased(&self, address: &Oid) -> Result<Option<Receipt>, Infallible> {
        Ok(self.tombstones.get(address).cloned())
    }
    fn write(&mut self, slot: &Slot<'_>) -> Result<(), Infallible> {
        self.inner.write(slot)
    }
}

#[test]
fn a_backend_that_keeps_purged_bytes_still_reads_erased() {
    let x = throwaway(0x71);
    let mut backend = Hoarding::default();
    let xs = settled(&mut backend, &x, "node", 1);
    let erasure = Erasure::mint(x, Epoch(2), &backend).unwrap();
    purge(&mut backend, &erasure).unwrap();
    assert!(
        backend.fetch(xs[0].address()).unwrap().is_some(),
        "it hoarded"
    );
    assert_eq!(
        recall(&backend, &Scope::public(), xs[0].address()),
        Err(Recall::Lost {
            address: xs[0].address().clone(),
            reason: Loss::Erased(erasure.receipt().clone())
        })
    );
}

#[test]
fn a_refusing_backend_refuses_the_purge_and_the_tombstone_read() {
    let x = throwaway(0x72);
    let mut backend = Memory::default();
    let xs = settled(&mut backend, &x, "node", 1);
    let stranger = Erasure::mint(throwaway(0x72), Epoch(2), &Refusing).expect_err("refused");
    assert_eq!(stranger.reason(), &Strange::Backend(Refused));
    assert_eq!(stranger.address(), None);
    assert_eq!(stranger.into_key().id(), x.id(), "the key, unconsumed");
    assert_eq!(
        Held::open(&mut Refusing, &x).settle_with(b"node 0".to_vec(), []),
        Err(Recall::Backend(Refused))
    );
    let erasure = Erasure::mint(x, Epoch(2), &backend).unwrap();
    assert_eq!(purge(&mut Refusing, &erasure), Err(Refused));
    assert_eq!(
        recall(&Refusing, &Scope::public(), xs[0].address()),
        Err(Recall::Backend(Refused))
    );
}

// --- Item 3: the per-scope reference index, purged with the scope -----------

/// The index law, over one backend: X's index is the projection of the
/// lists X declared, persisted in X's slots; Y's is Y's alone; after X's
/// purge, X's index reads erased and Y's is untouched. `keeps_bytes` is
/// false for a faithful backend, whose slots must be gone too.
fn index_law<B: Backend<Error = Infallible>>(mut backend: B, keeps_bytes: bool) {
    let x = throwaway(0x81);
    let y = throwaway(0x82);
    let outside = elsewhere(b"outside");
    let (a, b, c) = {
        let mut held = Held::open(&mut backend, &x);
        assert!(held.index().unwrap().is_empty(), "never settled: empty");
        let a = held
            .settle_with(b"node a".to_vec(), [outside.clone()])
            .unwrap();
        let b = held.settle_with(b"node b".to_vec(), [a.clone()]).unwrap();
        let c = held.settle_with(b"node c".to_vec(), []).unwrap();
        // The same bytes again, with another reference: a union.
        let again = held.settle_with(b"node b".to_vec(), [c.clone()]).unwrap();
        assert_eq!(again, b);
        (a, b, c)
    };
    let declared = BTreeMap::from([
        (a.clone(), BTreeSet::from([outside.clone()])),
        (b.clone(), BTreeSet::from([a.clone(), c.clone()])),
        (c.clone(), BTreeSet::new()),
    ]);
    let ya = Held::open(&mut backend, &y)
        .settle_with(b"node a".to_vec(), [a.clone()])
        .unwrap();
    assert_ne!(ya, a, "the same bytes, another scope, another address");

    // Rebuilt from the persisted lists by a fresh borrow: nothing was kept.
    let index = Held::open(&mut backend, &x).index().unwrap();
    assert_eq!(index.entries(), &declared);
    assert_eq!(index.scope(), x.id());
    assert_eq!(index.len(), 3);
    assert_eq!(index.references(&b), declared.get(&b));
    assert_eq!(index.references(&ya), None, "Y's crystal isn't X's");
    assert_eq!(
        Held::open(&mut backend, &x).index().unwrap(),
        index,
        "a projection: the same every time"
    );
    let theirs = Held::open(&mut backend, &y).index().unwrap();
    assert_eq!(
        theirs.entries(),
        &BTreeMap::from([(ya.clone(), BTreeSet::from([a.clone()]))])
    );
    assert_eq!(
        Held::open(&mut backend, &x).crystal(&a).unwrap().bytes(),
        b"node a"
    );

    let slots = Held::open(&mut backend, &x).slots();
    let lists: Vec<Oid> = {
        let held = Held::open(&mut backend, &x);
        [&a, &b, &c].map(|k| held.references_slot(k)).to_vec()
    };
    let erasure = Erasure::mint(x, Epoch(5), &backend).expect("X's own index");
    let receipt = erasure.receipt().clone();
    for list in &lists {
        assert!(erasure.covers().contains(list), "{list:?}");
    }
    purge(&mut backend, &erasure).unwrap();

    // A copy of X's synthetic key, only to look: the index reads erased.
    let x = throwaway(0x81);
    let erased = Recall::Lost {
        address: slots.manifest().clone(),
        reason: Loss::Erased(receipt.clone()),
    };
    assert_eq!(Held::open(&mut backend, &x).index(), Err(erased.clone()));
    for slot in lists.iter().chain([slots.manifest()]) {
        assert_eq!(backend.erased(slot), Ok(Some(receipt.clone())));
        if !keeps_bytes {
            assert_eq!(backend.fetch(slot), Ok(None), "the list is gone");
        }
    }
    // A purge is final: nothing settles in the scope again.
    assert_eq!(
        Held::open(&mut backend, &x).settle_with(b"node d".to_vec(), []),
        Err(erased)
    );
    // Y's index is untouched.
    assert_eq!(Held::open(&mut backend, &y).index().unwrap(), theirs);
}

#[test]
fn the_index_is_rebuilt_from_persisted_lists_and_purged_on_memory() {
    index_law(Memory::default(), false);
}

#[test]
fn the_index_is_rebuilt_from_persisted_lists_and_purged_on_the_ledger() {
    index_law(Ledger::default(), false);
}

#[test]
fn the_index_is_rebuilt_from_persisted_lists_and_purged_through_a_wrapper() {
    index_law(Counted::default(), false);
}

#[test]
fn a_hoarded_index_still_reads_erased_through_the_tombstone() {
    index_law(Hoarding::default(), true);
}

#[test]
fn no_slot_is_written_outside_its_scope() {
    let mut ledger = Ledger::default();
    let x = throwaway(0x83);
    let y = throwaway(0x84);
    let mut theirs = BTreeSet::new();
    for key in [&x, &y] {
        let mut held = Held::open(&mut ledger, key);
        let a = held.settle_with(b"node a".to_vec(), []).unwrap();
        let b = held.settle_with(b"node b".to_vec(), [a.clone()]).unwrap();
        let slots = held.slots();
        theirs
            .extend([slots.manifest(), slots.light(), slots.root(), slots.chain()].map(Oid::clone));
        theirs.extend([held.references_slot(&a), held.references_slot(&b)]);
    }
    let written: BTreeSet<Oid> = ledger.slots.iter().map(|(a, _)| a.clone()).collect();
    assert!(!written.is_empty());
    assert!(
        written.is_subset(&theirs),
        "a slot outside both scopes: {:?}",
        written.difference(&theirs).collect::<Vec<_>>()
    );
    assert!(!written.contains(&Oid::slot(&Scope::public(), b"manifest")));
}

#[test]
fn the_mint_reads_the_census_at_the_mint_never_a_stale_one() {
    let mut backend = Memory::default();
    let x = throwaway(0x85);
    let early = {
        let mut held = Held::open(&mut backend, &x);
        held.settle_with(b"node 0".to_vec(), []).unwrap();
        held.settle_with(b"node 1".to_vec(), []).unwrap();
        held.index().unwrap()
    };
    let late = Held::open(&mut backend, &x)
        .settle_with(b"node 2".to_vec(), [])
        .unwrap();
    assert_eq!(early.references(&late), None, "settled after");
    let erasure = Erasure::mint(x, Epoch(1), &backend).unwrap();
    for c in early.entries().keys().chain([&late]) {
        assert!(erasure.covers().contains(c), "{c:?}");
    }
    purge(&mut backend, &erasure).unwrap();
    assert_eq!(
        recall(&backend, &Scope::public(), &late),
        Err(Recall::Lost {
            address: late.clone(),
            reason: Loss::Erased(erasure.receipt().clone())
        })
    );
}

#[test]
fn the_public_scopes_index_never_shrinks() {
    let mut brain = Brain::new(Capacity::bytes(0), Memory::default());
    let a = brain
        .settle_with(b"node a".to_vec(), [elsewhere(b"x")])
        .unwrap();
    let b = brain.settle_with(b"node b".to_vec(), [a.clone()]).unwrap();
    assert!(brain.stats().evicted_crystals >= 2, "every crystal evicted");
    let reach = brain.vacuum(&Roots::default().seal());
    let _ = brain.cool(reach.dangling());
    brain.settle_with(b"node a".to_vec(), []).unwrap();
    // A keyed scope purged elsewhere touches nothing here.
    let mut other = Memory::default();
    let x = throwaway(0x87);
    let _ = settled(&mut other, &x, "node", 2);
    let erasure = Erasure::mint(x, Epoch(1), &other).unwrap();
    purge(&mut other, &erasure).unwrap();
    // Outside Capacity, never evicted, never shrinking.
    assert_eq!(
        brain.references(&a),
        Some(&BTreeSet::from([elsewhere(b"x")]))
    );
    assert_eq!(brain.references(&b), Some(&BTreeSet::from([a.clone()])));
    assert_eq!(
        brain
            .vacuum(&Roots::default().seal())
            .dangling()
            .addresses()
            .len(),
        2
    );
}

#[test]
fn a_malformed_slot_is_never_believed() {
    let x = throwaway(0x86);
    let mut forger = Forger;
    let manifest = Held::open(&mut forger, &x).slots().manifest().clone();
    assert_eq!(
        Held::open(&mut forger, &x).index(),
        Err(Recall::Forged(manifest.clone()))
    );
    let stranger = Erasure::mint(x, Epoch(1), &forger).expect_err("a malformed manifest");
    assert_eq!(stranger.reason(), &Strange::Malformed(manifest.clone()));
    assert_eq!(stranger.address(), Some(&manifest));
    assert_eq!(stranger.into_key().id(), throwaway(0x86).id());
}

// --- Item 4: lums and deposit ticks per scope, erased with it ---------------

/// The light law, over one backend: X's deposits and lums persist in X's
/// light, Y's apart; after X's purge none of X's reads, and Y's is
/// untouched.
fn light_law<B: Backend<Error = Infallible>>(mut backend: B, keeps_bytes: bool) {
    let x = throwaway(0x91);
    let y = throwaway(0x92);
    {
        let mut held = Held::open(&mut backend, &x);
        assert_eq!(held.lums(Tick(9)), Ok(0), "no light yet");
        assert_eq!(held.tick_of(b"edge a b"), Ok(None));
        assert_eq!(held.deposit(b"edge a b", Tick(0)), Ok(true));
        assert_eq!(held.deposit(b"edge b c", Tick(0)), Ok(true));
        assert_eq!(held.deposit(b"path a c", Tick(1)), Ok(true));
        // Known already: false, and its tick stays.
        assert_eq!(held.deposit(b"edge a b", Tick(2)), Ok(false));
    }
    assert_eq!(
        Held::open(&mut backend, &y).deposit(b"edge a b", Tick(5)),
        Ok(true)
    );

    // Read back by a fresh borrow: persisted, never kept.
    let read = |backend: &mut B, key: &ScopeKey| {
        let held = Held::open(backend, key);
        (
            held.tick_of(b"edge a b"),
            held.tick_of(b"path a c"),
            [0, 1, 4, 5, 7].map(|t| held.lums(Tick(t))),
        )
    };
    let ours = read(&mut backend, &x);
    assert_eq!(
        ours,
        (
            Ok(Some(Tick(0))),
            Ok(Some(Tick(1))),
            [Ok(2), Ok(3), Ok(3), Ok(3), Ok(3)]
        )
    );
    let theirs = read(&mut backend, &y);
    assert_eq!(
        theirs,
        (
            Ok(Some(Tick(5))),
            Ok(None),
            [Ok(0), Ok(0), Ok(0), Ok(1), Ok(1)]
        )
    );

    let light = Held::open(&mut backend, &x).slots().light().clone();
    let erasure = Erasure::mint(x, Epoch(4), &backend).expect("X's own light");
    assert!(erasure.covers().contains(&light));
    purge(&mut backend, &erasure).unwrap();

    // A copy of X's synthetic key, only to look: nothing of X's light reads.
    let x = throwaway(0x91);
    let erased = Recall::Lost {
        address: light.clone(),
        reason: Loss::Erased(erasure.receipt().clone()),
    };
    assert_eq!(
        read(&mut backend, &x),
        (
            Err(erased.clone()),
            Err(erased.clone()),
            [0, 1, 4, 5, 7].map(|_| Err(erased.clone()))
        )
    );
    assert_eq!(
        Held::open(&mut backend, &x).deposit(b"edge a b", Tick(8)),
        Err(erased)
    );
    assert_eq!(backend.erased(&light), Ok(Some(erasure.receipt().clone())));
    if !keeps_bytes {
        assert_eq!(backend.fetch(&light), Ok(None), "the light is gone");
    }
    assert_eq!(read(&mut backend, &y), theirs, "Y's light untouched");
}

#[test]
fn lums_and_ticks_are_per_scope_and_erased_with_it_on_memory() {
    light_law(Memory::default(), false);
}

#[test]
fn lums_and_ticks_are_per_scope_and_erased_with_it_on_the_ledger() {
    light_law(Ledger::default(), false);
}

#[test]
fn lums_and_ticks_are_per_scope_and_erased_with_it_through_a_wrapper() {
    light_law(Counted::default(), false);
}

#[test]
fn hoarded_lums_and_ticks_still_read_erased_through_the_tombstone() {
    light_law(Hoarding::default(), true);
}

#[test]
fn the_backend_holds_a_facts_name_never_its_bytes() {
    let mut ledger = Ledger::default();
    let x = throwaway(0x93);
    let mut held = Held::open(&mut ledger, &x);
    assert_eq!(held.deposit(b"edge alpha beta", Tick(3)), Ok(true));
    let light = held.slots().light().clone();
    let (_, bytes) = ledger
        .slots
        .iter()
        .find(|(a, _)| a == &light)
        .expect("the light is written");
    let text = String::from_utf8_lossy(bytes);
    assert!(!text.contains("alpha"), "{text}");
    assert!(text.ends_with(" 3"), "a name and its tick: {text}");
}

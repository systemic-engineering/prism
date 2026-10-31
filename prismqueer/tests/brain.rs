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
    Backend, Brain, Capacity, Crystal, Fact, Interner, Memory, Recall, Relation, Stats, Sym, Tick,
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

    // Rounds to the fixpoint. Each round's facts settle at its committed point.
    let mut settled = Vec::new();
    loop {
        let new = round(&mut brain, &p.rules);
        let t = brain.commit();
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
    (
        Light {
            facts,
            verdicts,
            settled,
            read_back,
            ticks,
        },
        stats,
    )
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
    let rulers: [(&str, Warmth); 5] = [
        ("oldest first", oldest_first),
        ("newest first", newest_first),
        ("by constants", by_constants),
        ("scrambled", scrambled),
        ("all equal", all_equal),
    ];
    let thresholds = [
        Capacity::bytes(0),
        Capacity::bytes(16),
        Capacity::bytes(100),
        Capacity::bytes(400),
        Capacity::unbounded(),
    ];
    let mut evictions = BTreeSet::new();
    for (name, ruler) in rulers {
        for capacity in thresholds {
            let (light, stats) = run(capacity, Some(ruler));
            evictions.insert(stats.evicted_facts);
            assert_eq!(light, reference, "{name} under {capacity:?}");
        }
    }
    // The runs really differed in what they held.
    assert!(evictions.len() > 3, "{evictions:?}");
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

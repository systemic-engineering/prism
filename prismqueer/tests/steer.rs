//! O3, the first ouroboros closure: prismqueer tests prismqueer, steered by
//! prismqueer.
//!
//! Property under test: the Instrument's kernel has dimension b0 + b1
//! (tonics plus commas). b0 and b1 come from union-find, an instrument
//! outside the loop, so the closure cannot certify itself.
//!
//! Steering: prismqueer's own spectrum chooses the next input. The measure is
//! the quietest non-kernel mode of the Instrument, how close the graph is to a
//! new tonic or comma appearing: the hardest cases for the kernel count. It is
//! compared exactly, as an algebraic number from the room's exact Laplacian,
//! so the path is the same on every platform; LAPACK's float is only a
//! reading beside it.
//!
//! Stop: the search stops moving when the chord of distinct graphs stops
//! growing. A clean search is Partial, never Success.

#![cfg(all(feature = "lapack", feature = "bundle"))]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use num_bigint::BigInt;

use prismqueer::flux::pillar::{steer, Arbitrary, Budget, Patience, Sample, Stop};
use prismqueer::oid::Addressable;
use prismqueer::poly::{Algebraic, Budget as Trials, ZPoly};
use prismqueer::room;
use prismqueer::spectral::instrument::Instrument;
use prismqueer::spectral::{graph_from_edges, Graph};
use terni::Imperfect;

/// A small graph with every vertex on some edge.
struct Field {
    n: u64,
    edges: Vec<(u64, u64)>,
}

impl Field {
    fn graph(&self) -> Graph {
        graph_from_edges(&self.edges)
    }
}

fn covered(sample: &mut Sample, n: u64) -> Field {
    let mut edges = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if sample.draw_bool() {
                edges.push((i, j));
            }
        }
    }
    for v in 0..n {
        if !edges.iter().any(|&(a, b)| a == v || b == v) {
            let w = (v + 1) % n;
            edges.push((v.min(w), v.max(w)));
        }
    }
    Field { n, edges }
}

struct Seven(Field);
impl Arbitrary for Seven {
    fn arbitrary(sample: &mut Sample) -> Self {
        Seven(covered(sample, 7))
    }
}

struct Tiny(Field);
impl Arbitrary for Tiny {
    fn arbitrary(sample: &mut Sample) -> Self {
        let n = sample.draw_integer(2, 3) as u64;
        Tiny(covered(sample, n))
    }
}

/// Union-find, outside the loop: (components, independent cycles).
fn betti(f: &Field) -> (usize, usize) {
    let mut parent: Vec<usize> = (0..f.n as usize).collect();
    fn root(p: &mut Vec<usize>, i: usize) -> usize {
        if p[i] == i {
            i
        } else {
            let r = root(p, p[i]);
            p[i] = r;
            r
        }
    }
    let mut edges = BTreeSet::new();
    let mut b0 = f.n as usize;
    for &(u, v) in &f.edges {
        if !edges.insert((u, v)) {
            continue;
        }
        let (ru, rv) = (root(&mut parent, u as usize), root(&mut parent, v as usize));
        if ru != rv {
            parent[ru] = rv;
            b0 -= 1;
        }
    }
    (b0, edges.len() + b0 - f.n as usize)
}

fn kernel_is_tonics_plus_commas(f: &Field) -> bool {
    let (b0, b1) = betti(f);
    Instrument::of(&f.graph())
        .map(|d| d.kernel_dimension() == Imperfect::Success(b0 + b1))
        .unwrap_or(false)
}

/// A last-bit wobble of a float reading, the way another platform's LAPACK
/// may round it: none, or 1 to 8 ulps up or down per graph, chosen by the
/// graph's address and a salt. dsyev's error bound is a small multiple of
/// n·ε·‖D‖, so a few ulps is what another BLAS kernel may differ by.
#[derive(Clone, Copy, Debug)]
enum Wobble {
    Still,
    Ulp(u8),
}

fn wobbled(reading: f64, f: &Field, wobble: Wobble) -> f64 {
    match wobble {
        Wobble::Still => reading,
        Wobble::Ulp(salt) => {
            // FNV-1a over the salt and the address: a fair coin per graph.
            let key = std::iter::once(salt)
                .chain(f.graph().oid().as_str().bytes())
                .fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
                    (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
                });
            let step = if key >> 32 & 1 == 0 {
                f64::next_up
            } else {
                f64::next_down
            };
            (0..=(key >> 40) % 8).fold(reading, |r, _| step(r))
        }
    }
}

/// The steering score: the quietest non-kernel mode, exactly.
///
/// `exact` is the smallest positive eigenvalue λ of L₀ ([`room::quietest`]);
/// D's quietest non-kernel |mode| is √λ, so they order graphs alike. `None`
/// is a graph with no non-kernel mode, quieter than none. `reading` is
/// LAPACK's float for √λ, kept beside it and never compared.
#[derive(Clone, Debug)]
struct Quietest {
    exact: Option<Algebraic>,
    reading: f64,
}

impl Ord for Quietest {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.exact, &other.exact) {
            (Some(a), Some(b)) => compared(a, b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}

impl PartialOrd for Quietest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Quietest {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Quietest {}

/// Exact work is pure, so it is remembered across the suite's searches:
/// each graph factored once, each pair of tags compared once.
fn memo<K: Ord + Clone, V: Clone>(
    cache: &'static OnceLock<Mutex<BTreeMap<K, V>>>,
    key: K,
    work: impl FnOnce() -> V,
) -> V {
    let cache = cache.get_or_init(Mutex::default);
    if let Some(v) = cache.lock().expect("unpoisoned").get(&key) {
        return v.clone();
    }
    let v = work();
    cache.lock().expect("unpoisoned").insert(key, v.clone());
    v
}

fn compared(a: &Algebraic, b: &Algebraic) -> Ordering {
    static PAIRS: OnceLock<Mutex<BTreeMap<String, Ordering>>> = OnceLock::new();
    memo(&PAIRS, format!("{a:?} {b:?}"), || {
        a.compare(b).expect("a certified tag isolates its root")
    })
}

fn exact(g: &Graph) -> Option<Algebraic> {
    static MODES: OnceLock<Mutex<BTreeMap<String, Option<Algebraic>>>> = OnceLock::new();
    memo(
        &MODES,
        g.oid().as_str().to_owned(),
        || match room::quietest(g.multigraph(), Trials(10_000)) {
            room::Quietest::Mode(a) => Some(a),
            room::Quietest::Silent => None,
            room::Quietest::Unresolved(rest) => panic!("factoring ran out on {rest:?}"),
        },
    )
}

/// The quietest non-kernel mode: prismqueer measuring where it is hardest.
fn quietest(f: &Field, wobble: Wobble) -> Quietest {
    let g = f.graph();
    let exact = exact(&g);
    let d = Instrument::of(&g).expect("measured");
    let mut modes: Vec<f64> = d.eigenvalues().iter().map(|x| x.abs()).collect();
    modes.sort_by(f64::total_cmp);
    let reading = modes
        .get(d.kernel_dimension().ok().expect("an exact count"))
        .copied()
        .map_or(f64::INFINITY, |m| wobbled(m, f, wobble));
    Quietest { exact, reading }
}

/// A ℤ[x] polynomial, leading coefficient first.
fn z(coefficients: &[i64]) -> ZPoly {
    ZPoly::from_descending(coefficients.iter().map(|&c| BigInt::from(c)))
}

fn seeds(range: std::ops::Range<u64>) -> Vec<Vec<u8>> {
    range.map(|s| s.to_le_bytes().to_vec()).collect()
}

#[test]
fn steering_by_the_spectrum_beats_blind_sampling() {
    let budget = 200;
    let blind = seeds(0..budget)
        .into_iter()
        .map(|s| {
            quietest(
                &Seven::arbitrary(&mut Sample::from_bytes(s)).0,
                Wobble::Still,
            )
        })
        .min()
        .expect("a budget of runs");

    let steered = steer::<Seven, _>(
        seeds(0..8),
        Budget(budget as usize),
        Patience(budget as usize),
        |s| quietest(&s.0, Wobble::Still),
        |s| kernel_is_tonics_plus_commas(&s.0),
        |s| s.0.graph().oid(),
    );
    match steered {
        Imperfect::Partial(run, coverage) => {
            let best = run.best_score.expect("scored");
            eprintln!(
                "blind best {:.4}; steered best {:.4} after {:?} runs, {:?}",
                blind.reading,
                best.reading,
                coverage.len(),
                run.stop
            );
            assert!(
                best < blind,
                "steered {best:?} did not beat blind {blind:?}"
            );
            // Pin, for this deterministic configuration: the steered search
            // reaches Fiedler's extremum, the path on 7 vertices. Its L₀ has
            // the eigenvalues 2 − 2cos(kπ/7), and the quietest, k = 1, is the
            // least root of y³ − 5y² + 6y − 1: the minimal polynomial of
            // 2cos(π/7), x³ − x² − 2x + 1, at x = 2 − y. Its other factor,
            // y³ − 7y² + 14y − 7 (the even k), comes first in canonical order
            // and is the wrong one: its least root is 2 − 2cos(2π/7).
            assert_eq!(
                best.exact,
                Some(Algebraic::new(z(&[1, -5, 6, -1]), 0)),
                "did not reach the path"
            );
        }
        Imperfect::Failure(cx, _) => panic!("kernel != b0 + b1 on {:?}", cx.buffer),
        Imperfect::Success(_) => panic!("a search must never be Success"),
    }
}

/// Cross-platform by construction: a last-bit difference in LAPACK's
/// eigenvalues, a few ulps up or down on any graph, must not move the
/// steered search. Same path, same best, same exact score, same stop. On
/// x86_64-linux OpenBLAS the float-steered search took another path than on
/// aarch64-darwin, and this wobble reproduces it on darwin.
#[test]
fn the_steered_search_does_not_read_the_last_bit() {
    let run = |wobble| match steer::<Seven, _>(
        seeds(0..8),
        Budget(200),
        Patience(200),
        |s| quietest(&s.0, wobble),
        |s| kernel_is_tonics_plus_commas(&s.0),
        |s| s.0.graph().oid(),
    ) {
        Imperfect::Partial(run, coverage) => format!(
            "{:?} {:?} {:?} {coverage:?}",
            run.best,
            run.best_score.map(|q| q.exact),
            run.stop
        ),
        other => panic!("expected a clean Partial, got {other:?}"),
    };
    let still = run(Wobble::Still);
    for salt in 0..16 {
        assert!(
            run(Wobble::Ulp(salt)) == still,
            "a few ulps of wobble (salt {salt}) moved the steered search"
        );
    }
}

#[test]
fn the_search_stops_moving_when_nothing_new_appears() {
    // Every covered graph on 2 or 3 labeled vertices, counted by brute force.
    let mut all = BTreeSet::new();
    for mask in 0u8..8 {
        let e: Vec<(u64, u64)> = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, e)| e)
            .collect();
        if (0..3).all(|v| e.iter().any(|&(a, b)| a == v || b == v)) {
            all.insert(graph_from_edges(&e).oid());
        }
    }
    all.insert(graph_from_edges(&[(0, 1)]).oid());

    // Every run's address, in order: the search's own trace.
    let mut runs = Vec::new();
    match steer::<Tiny, _>(
        seeds(0..4),
        Budget(10_000),
        Patience(64),
        |t| quietest(&t.0, Wobble::Still),
        |t| kernel_is_tonics_plus_commas(&t.0),
        |t| {
            let a = t.0.graph().oid();
            runs.push(a.clone());
            a
        },
    ) {
        Imperfect::Partial(run, coverage) => {
            assert_eq!(
                run.stop,
                Stop::Saturated,
                "it must stop by saturation, not budget"
            );
            let seen = coverage.foata().expect("known")[0].len();
            assert!(
                seen <= all.len(),
                "saw {seen} distinct graphs, only {} exist",
                all.len()
            );
            assert_eq!(coverage.len(), Some(runs.len()));
            // It stopped because nothing new appeared, not after a fixed
            // count: exactly Patience runs follow the last new graph.
            let mut fresh = BTreeSet::new();
            let last_new = runs
                .iter()
                .enumerate()
                .filter(|(_, a)| fresh.insert((*a).clone()))
                .map(|(i, _)| i)
                .last()
                .expect("at least one run");
            assert_eq!(fresh.len(), seen, "the chord is the distinct addresses");
            assert!(last_new > 0, "more than one graph appeared");
            assert_eq!(
                runs.len(),
                last_new + 1 + 64,
                "the search ran {} times; the last new graph was run {last_new}",
                runs.len()
            );
        }
        other => panic!("expected a saturated Partial, got {other:?}"),
    }
}

#[test]
fn among_tied_scores_the_least_address_is_kept() {
    // A constant measure ties every run: the best is the seed with the least
    // address, wherever it came in the order. Not the first seed here, so
    // keeping the first seen would be caught.
    let hash =
        |x: &u64| prismqueer::oid::Oid::hash(&prismqueer::oid::Scope::public(), &x.to_le_bytes());
    let least = (0..5u64).min_by_key(hash).expect("five seeds");
    assert_ne!(least, 0, "the least address must not be the first seed");
    match steer::<u64, _>(
        seeds(0..5),
        Budget(5),
        Patience(10),
        |_| 1u8,
        |_| true,
        hash,
    ) {
        Imperfect::Partial(run, _) => {
            assert_eq!(run.best, Some(least.to_le_bytes().to_vec()));
            assert_eq!(run.best_score, Some(1));
            assert_eq!(run.stop, Stop::Budget);
        }
        other => panic!("expected a clean Partial, got {other:?}"),
    }
}

#[test]
fn among_tied_scores_and_addresses_the_first_seen_is_kept() {
    // One address for every run: nothing breaks the tie, the first stays.
    let one = prismqueer::oid::Oid::hash(&prismqueer::oid::Scope::public(), b"one");
    match steer::<u64, _>(
        seeds(0..5),
        Budget(5),
        Patience(10),
        |_| 1u8,
        |_| true,
        |_| one.clone(),
    ) {
        Imperfect::Partial(run, _) => {
            assert_eq!(run.best, Some(0u64.to_le_bytes().to_vec()));
        }
        other => panic!("expected a clean Partial, got {other:?}"),
    }
}

#[test]
fn a_search_that_keeps_finding_new_values_runs_out_of_budget() {
    // Every run is addressed afresh, so something new always appears. It
    // must never be called Saturated, however small the Patience.
    let mut fresh = 0u64;
    match steer::<u64, _>(
        seeds(0..4),
        Budget(40),
        Patience(2),
        |_| 0u8,
        |_| true,
        |_| {
            fresh += 1;
            prismqueer::oid::Oid::hash(&prismqueer::oid::Scope::public(), &fresh.to_le_bytes())
        },
    ) {
        Imperfect::Partial(run, coverage) => {
            assert_eq!(run.stop, Stop::Budget, "new values kept appearing");
            assert_eq!(coverage.len(), Some(40), "every run of the budget ran");
            assert_eq!(coverage.foata().expect("known")[0].len(), 40);
        }
        other => panic!("expected a budget-stopped Partial, got {other:?}"),
    }
}

#[test]
fn a_failure_is_kept_and_replays() {
    // A deliberately false claim: no graph has a comma.
    match steer::<Seven, _>(
        seeds(0..8),
        Budget(200),
        Patience(200),
        |s| quietest(&s.0, Wobble::Still),
        |s| betti(&s.0).1 == 0,
        |s| s.0.graph().oid(),
    ) {
        Imperfect::Failure(cx, _) => {
            let replayed = Seven::arbitrary(&mut Sample::from_bytes(cx.buffer.clone()));
            assert!(
                betti(&replayed.0).1 > 0,
                "the kept buffer must replay to a graph with a comma"
            );
        }
        other => panic!("expected Failure, got {other:?}"),
    }
}

#[test]
fn steering_is_deterministic() {
    let run = || {
        steer::<Seven, _>(
            seeds(0..8),
            Budget(64),
            Patience(64),
            |s| quietest(&s.0, Wobble::Still),
            |s| kernel_is_tonics_plus_commas(&s.0),
            |s| s.0.graph().oid(),
        )
    };
    assert_eq!(format!("{:?}", run()), format!("{:?}", run()));
}

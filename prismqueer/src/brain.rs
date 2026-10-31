//! `brain`: the store. A hot tier for facts, the Drop for crystals, and
//! eviction under a machine-specific [`Capacity`] that never changes what
//! is true.
//!
//! Named by Alex (prism-008, item 6). This is the store side only, and it
//! is placement-agnostic: **prism holds no evaluator.** An evaluator (today
//! mirror's, in `rust/engine`) deposits facts, scans them, commits ticks and
//! settles crystals; where that evaluator lives is Alex's decision. The
//! tests drive `brain` with a tiny semi-naive positive-Datalog evaluator that
//! lives in `tests/brain.rs`.
//!
//! # The parts
//!
//! - **The hot tier.** Facts per [`Relation`], as compact tuples of
//!   interned constants ([`Sym`]), each with its deposit [`Tick`] stored
//!   beside it: one copy of the tuple, no separate tick map.
//! - **The Drop.** An item settles into a [`Crystal`]: its canonical bytes
//!   and its address `Oid::hash(scope, bytes)`. Crystals are immutable. The
//!   only way to make something new is a new crystal under a new address.
//! - **Eviction under [`Capacity`].** At a committed point, `brain` evicts
//!   cached crystals first (the [`Backend`] holds them), then committed
//!   facts, coldest first by a declared [`Ruler`]. An evicted fact comes
//!   back on demand through the caller's [`Recompute`]; an evicted crystal
//!   reads back by address through the [`Backend`].
//! - **The backend.** prism defines [`Backend`]; mirror implements it, and
//!   persisting crystals is its job. prism never imports mirror. [`Memory`]
//!   is the in-memory backend for tests.
//!
//! # The invariants (prism-008, confirmed by Alex)
//!
//! 1. **Use can make a crystal warmer, never more true.** The ruler and the
//!    threshold decide only what is held hot. Every answer, every deposit
//!    tick and every crystal is the same under any of them.
//! 2. **Forgetting costs nothing, because nothing is lost.** A crystal is
//!    persisted before it can be evicted, and a fact is evicted only when a
//!    [`Recompute`] can bring it back with its tick. Without one, no fact is
//!    ever evicted. A read checks that the bytes rehash to their address.
//! 3. **Only the Drop makes anything new.** No public API changes a
//!    crystal's bytes under its address: [`Crystal::settle`] is the only
//!    constructor, and it computes the address from the bytes.
//!
//! # Two budgets, two types
//!
//! An evaluation budget (`flux::pillar::Budget`) may change a verdict, so it
//! must be the same on every machine. An eviction threshold must never
//! change a verdict, which is what lets it differ by machine. [`Capacity`]
//! is the threshold, and it never converts to or from `Budget`.
//!
//! # The Drop's rulers (prism-013 §4)
//!
//! Capacity is the **must** ruler: back pressure only. Eviction happens at
//! committed points ([`Brain::commit`] and [`Brain::settle`]) and never
//! touches a fact of the open tick. A restored fact keeps the tick it was
//! deposited at, so drop timing never changes content.
//!
//! # No clock
//!
//! Nothing here reads time. A [`Tick`] is the evaluator's round, keys are
//! content addresses, and eviction order is the ruler's warmth with a
//! structural tie-break (relation, then tuple). Everything is deterministic.
//!
//! # Privacy precondition
//!
//! **Nothing personal enters `brain`** until Mara's key design lands
//! (prism-008's privacy preconditions): every address keyed by its scope's
//! `K_id`, private facts keyed by their scoped address with their spectral
//! position only inside the scope, and each scope's declared mode respected.
//! Until then a [`Brain`] works only in the public scope: [`Brain::new`]
//! takes no scope, and there is no constructor that does.
//!
//! # Lums: progress and monotonicity, never identity
//!
//! [`Brain::lums`] is the count of light: the number of distinct facts ever
//! deposited at or before the committed tick (prism-012, Ask 2; tr(P_S) for
//! the lit set S). It is a counter kept **beside** the hot tier, not the hot
//! tier's size: an evicted fact still counts, because eviction is lossless
//! forgetting, and a fact a [`Recompute`] restores is not counted again,
//! because it keeps its original deposit tick. So lums are the same under
//! every [`Capacity`] and [`Ruler`] (invariant 1), readable in O(1) at each
//! step boundary without copying anything.
//!
//! For a positive program lums never fall, and two equal successive counts
//! mean the fixpoint. Lums are **progress and monotonicity, never
//! identity**: they are read beside an address, never part of one, and
//! never in a crystal's bytes. Two brains that settle the same bytes settle
//! the same address, however much light either has seen.
//!
//! Not here yet: the vacuum (unreachability), keyed private scopes, and
//! windowed input.
//!
//! # Crystals are immutable
//!
//! The control compiles: a crystal's bytes and address can be read.
//!
//! ```
//! use prismqueer::brain::Crystal;
//! use prismqueer::oid::{Oid, Scope};
//! let c = Crystal::settle(&Scope::public(), b"edge a b".to_vec());
//! let bytes: &[u8] = c.bytes();
//! assert_eq!(c.address(), &Oid::hash(&Scope::public(), bytes));
//! ```
//!
//! Each of the following changes one line of it, and doesn't compile
//! (rustdoc on stable doesn't check the error code, which is why the control
//! is here). There is no mutable access to the bytes:
//!
//! ```compile_fail,E0599
//! use prismqueer::brain::Crystal;
//! use prismqueer::oid::{Oid, Scope};
//! let c = Crystal::settle(&Scope::public(), b"edge a b".to_vec());
//! let bytes: &mut [u8] = c.bytes_mut();
//! assert_eq!(c.address(), &Oid::hash(&Scope::public(), bytes));
//! ```
//!
//! and the bytes it lends are shared, never `&mut`:
//!
//! ```compile_fail,E0308
//! use prismqueer::brain::Crystal;
//! use prismqueer::oid::{Oid, Scope};
//! let mut c = Crystal::settle(&Scope::public(), b"edge a b".to_vec());
//! let bytes: &mut [u8] = c.bytes();
//! assert_eq!(c.address(), &Oid::hash(&Scope::public(), bytes));
//! ```
//!
//! Its fields are private, so they can't be written:
//!
//! ```compile_fail,E0616
//! use prismqueer::brain::Crystal;
//! use prismqueer::oid::{Oid, Scope};
//! let mut c = Crystal::settle(&Scope::public(), b"edge a b".to_vec());
//! c.bytes = b"edge a c".to_vec().into();
//! assert_eq!(c.address(), &Oid::hash(&Scope::public(), c.bytes()));
//! ```
//!
//! nor can a crystal be built with an address that isn't its bytes':
//!
//! ```compile_fail,E0451
//! use prismqueer::brain::Crystal;
//! use prismqueer::oid::{Oid, Scope};
//! let c = Crystal { address: Oid::hash(&Scope::public(), b"edge a b"), bytes: b"edge a c".to_vec().into() };
//! assert_eq!(c.address(), &Oid::hash(&Scope::public(), c.bytes()));
//! ```

use std::cell::Cell;
use std::collections::{BTreeMap, VecDeque};
use std::convert::Infallible;
use std::ops::Bound;
use std::sync::Arc;

use crate::oid::{Oid, Scope};

// ---------------------------------------------------------------------------
// Constants, relations, ticks
// ---------------------------------------------------------------------------

/// An interned constant: a small index into an [`Interner`].
///
/// Indices are handed out in order of first interning, so they are as
/// deterministic as the evaluation that interns them.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sym(u32);

impl Sym {
    /// The constant's index in its interner.
    pub fn index(self) -> u32 {
        self.0
    }
}

/// Bytes to [`Sym`]s and back. Constants are never evicted.
#[derive(Clone, Debug, Default)]
pub struct Interner {
    by_bytes: BTreeMap<Box<[u8]>, Sym>,
    bytes: Vec<Box<[u8]>>,
}

impl Interner {
    /// The constant for `bytes`, interning it if it is new.
    pub fn intern(&mut self, bytes: &[u8]) -> Sym {
        if let Some(&s) = self.by_bytes.get(bytes) {
            return s;
        }
        let s = Sym(u32::try_from(self.bytes.len()).expect("fewer than 2³² constants"));
        self.bytes.push(bytes.into());
        self.by_bytes.insert(bytes.into(), s);
        s
    }

    /// The constant for `bytes`, if it was ever interned.
    pub fn get(&self, bytes: &[u8]) -> Option<Sym> {
        self.by_bytes.get(bytes).copied()
    }

    /// The bytes of a constant this interner handed out.
    pub fn resolve(&self, sym: Sym) -> &[u8] {
        &self.bytes[sym.0 as usize]
    }
}

/// A relation: a name and an arity. Two arities under one name are two
/// relations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Relation(u32);

impl Relation {
    /// The relation's index in its brain.
    pub fn index(self) -> u32 {
        self.0
    }
}

/// An evaluation round. Not a clock: the evaluator's own count of committed
/// points, the same on every machine.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(pub u64);

/// A hot fact, as the [`Ruler`] sees it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Fact<'a> {
    /// Its relation.
    pub relation: Relation,
    /// Its tuple of constants.
    pub tuple: &'a [Sym],
    /// The tick it was deposited at.
    pub tick: Tick,
}

// ---------------------------------------------------------------------------
// Capacity, the eviction threshold
// ---------------------------------------------------------------------------

/// The eviction threshold: how many bytes `brain` may hold hot on this
/// machine. A fact costs its tuple (4 bytes a constant) plus its tick (8);
/// a cached crystal costs its bytes.
///
/// Capacity must never change a verdict, so it is its own type and never
/// converts to or from an evaluation budget, which may.
#[cfg_attr(
    feature = "bundle",
    doc = r#"
The control compiles: each budget goes where it belongs.

```
use prismqueer::brain::{Brain, Capacity, Memory};
use prismqueer::flux::pillar::Budget;
fn search(budget: Budget) -> usize { budget.0 }
let _ = search(Budget(64));
let _ = Brain::new(Capacity::bytes(64), Memory::default());
```

A capacity is not an evaluation budget:

```compile_fail,E0308
use prismqueer::brain::{Brain, Capacity, Memory};
use prismqueer::flux::pillar::Budget;
fn search(budget: Budget) -> usize { budget.0 }
let _ = search(Capacity::bytes(64));
let _ = Brain::new(Capacity::bytes(64), Memory::default());
```

an evaluation budget is not a capacity:

```compile_fail,E0308
use prismqueer::brain::{Brain, Capacity, Memory};
use prismqueer::flux::pillar::Budget;
fn search(budget: Budget) -> usize { budget.0 }
let _ = search(Budget(64));
let _ = Brain::new(Budget(64), Memory::default());
```

and neither converts into the other:

```compile_fail,E0277
use prismqueer::brain::{Brain, Capacity, Memory};
use prismqueer::flux::pillar::Budget;
fn search(budget: Budget) -> usize { budget.0 }
let _ = search(Capacity::bytes(64).into());
let _ = Brain::new(Capacity::bytes(64), Memory::default());
```

```compile_fail,E0277
use prismqueer::brain::{Brain, Capacity, Memory};
use prismqueer::flux::pillar::Budget;
fn search(budget: Budget) -> usize { budget.0 }
let _ = search(Budget(64));
let _ = Brain::new(Budget(64).into(), Memory::default());
```
"#
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Capacity(Option<usize>);

impl Capacity {
    /// Hold at most `n` bytes hot.
    pub fn bytes(n: usize) -> Self {
        Capacity(Some(n))
    }

    /// Hold everything hot; never evict.
    pub fn unbounded() -> Self {
        Capacity(None)
    }

    /// Whether `used` bytes fit.
    pub fn admits(self, used: usize) -> bool {
        self.0.is_none_or(|n| used <= n)
    }
}

// ---------------------------------------------------------------------------
// Rulers and recompute
// ---------------------------------------------------------------------------

/// The eviction priority: a declared, signed ruler. The coldest fact (the
/// lowest warmth) goes first; ties break by relation, then tuple.
///
/// A ruler decides only what stays hot, never what is true.
pub trait Ruler {
    /// How warm a fact is.
    fn warmth(&self, fact: Fact<'_>) -> i64;
}

impl<F: Fn(Fact<'_>) -> i64> Ruler for F {
    fn warmth(&self, fact: Fact<'_>) -> i64 {
        self(fact)
    }
}

/// The default ruler: the oldest deposit is the coldest.
#[derive(Copy, Clone, Debug, Default)]
pub struct Recency;

impl Ruler for Recency {
    fn warmth(&self, fact: Fact<'_>) -> i64 {
        i64::try_from(fact.tick.0).unwrap_or(i64::MAX)
    }
}

/// Recompute evicted facts on demand, from the input (Lemma A: a fact
/// depends only on the bytes it tested).
///
/// Asked for a relation and a pattern (`None` is a free position), it
/// returns every fact of the relation that matches, each with the tick it
/// is deposited at. It may return more: `brain` keeps only the matching
/// facts of committed ticks. Constants it can't find in the interner were
/// never deposited, so it can leave those facts out.
pub trait Recompute {
    /// The facts of `relation` matching `pattern`, with their deposit ticks.
    fn recompute(
        &mut self,
        interner: &Interner,
        relation: Relation,
        pattern: &[Option<Sym>],
    ) -> Vec<(Vec<Sym>, Tick)>;
}

impl<F> Recompute for F
where
    F: FnMut(&Interner, Relation, &[Option<Sym>]) -> Vec<(Vec<Sym>, Tick)>,
{
    fn recompute(
        &mut self,
        interner: &Interner,
        relation: Relation,
        pattern: &[Option<Sym>],
    ) -> Vec<(Vec<Sym>, Tick)> {
        self(interner, relation, pattern)
    }
}

// ---------------------------------------------------------------------------
// Crystals and the backend
// ---------------------------------------------------------------------------

/// A settled item: canonical bytes under their address. Immutable.
///
/// [`Crystal::settle`] is the only constructor, and it computes the address
/// from the bytes, so no crystal's address can disagree with its bytes, and
/// nothing lends its bytes mutably.
#[derive(Clone, PartialEq, Eq)]
pub struct Crystal {
    address: Oid,
    bytes: Arc<[u8]>,
}

impl Crystal {
    /// Settle `bytes` in `scope`: the address is `Oid::hash(scope, bytes)`.
    pub fn settle(scope: &Scope, bytes: Vec<u8>) -> Self {
        Crystal {
            address: Oid::hash(scope, &bytes),
            bytes: bytes.into(),
        }
    }

    /// The crystal's address.
    pub fn address(&self) -> &Oid {
        &self.address
    }

    /// The crystal's canonical bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl std::fmt::Debug for Crystal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Crystal({:?}, {} B)", self.address, self.bytes.len())
    }
}

/// Where crystals persist. prism defines it; mirror implements it.
///
/// A backend hands back bytes, never addresses: `brain` recomputes the
/// address of whatever it reads and refuses bytes that don't match.
pub trait Backend {
    /// Why the backend couldn't persist or fetch.
    type Error;

    /// Persist a crystal. Persisting the same crystal twice is harmless.
    fn persist(&mut self, crystal: &Crystal) -> Result<(), Self::Error>;

    /// The bytes persisted under `address`, if any.
    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Self::Error>;
}

/// The in-memory backend, for tests.
#[derive(Clone, Debug, Default)]
pub struct Memory {
    crystals: BTreeMap<Oid, Arc<[u8]>>,
}

impl Memory {
    /// How many crystals it holds.
    pub fn len(&self) -> usize {
        self.crystals.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.crystals.is_empty()
    }
}

impl Backend for Memory {
    type Error = Infallible;

    fn persist(&mut self, crystal: &Crystal) -> Result<(), Infallible> {
        self.crystals
            .insert(crystal.address.clone(), crystal.bytes.clone());
        Ok(())
    }

    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        Ok(self.crystals.get(address).map(|b| b.to_vec()))
    }
}

/// Why a crystal couldn't be read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recall<E> {
    /// The backend failed.
    Backend(E),
    /// Neither hot nor in the backend: the address was never settled here,
    /// or the backend lost it.
    Lost(Oid),
    /// The backend's bytes don't hash to the address. Not believed.
    Forged(Oid),
}

// ---------------------------------------------------------------------------
// The brain
// ---------------------------------------------------------------------------

/// What a brain did with its capacity. Diagnostics only: two runs that differ
/// here never differ in what is true.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Facts evicted from the hot tier.
    pub evicted_facts: usize,
    /// Evicted facts brought back by a recompute.
    pub restored_facts: usize,
    /// Calls to the recompute.
    pub recomputes: usize,
    /// Crystals evicted from the hot cache.
    pub evicted_crystals: usize,
    /// Crystals read back from the backend.
    pub recalled_crystals: usize,
    /// Bytes held hot now.
    pub hot_bytes: usize,
    /// The most bytes held hot at once.
    pub peak_hot_bytes: usize,
}

/// One relation's hot facts: the tuple, and its tick beside it.
struct Table {
    arity: usize,
    rows: BTreeMap<Box<[Sym]>, Tick>,
    /// Facts evicted and not yet restored. While zero, a miss is a miss.
    evicted: usize,
    /// Lums: distinct facts deposited at committed ticks.
    lit: u64,
    /// Distinct facts deposited at the open tick, not yet lit.
    pending: u64,
}

impl Table {
    fn row_bytes(&self) -> usize {
        self.arity * std::mem::size_of::<Sym>() + std::mem::size_of::<Tick>()
    }
}

fn matches(tuple: &[Sym], pattern: &[Option<Sym>]) -> bool {
    tuple
        .iter()
        .zip(pattern)
        .all(|(s, p)| p.is_none_or(|p| p == *s))
}

/// The store: a hot tier of facts, the Drop, and eviction under a
/// [`Capacity`]. Public scope only (see the module docs).
pub struct Brain<B> {
    scope: Scope,
    capacity: Capacity,
    backend: B,
    ruler: Box<dyn Ruler>,
    recompute: Option<Box<dyn Recompute>>,
    interner: Interner,
    relations: BTreeMap<(String, usize), Relation>,
    tables: Vec<Table>,
    open: Tick,
    committed: Option<Tick>,
    crystals: BTreeMap<Oid, Crystal>,
    settled_order: VecDeque<Oid>,
    stats: Stats,
    recalled: Cell<usize>,
    lit: u64,
    pending: u64,
}

impl<B: Backend> Brain<B> {
    /// A brain in the public scope, holding at most `capacity` hot, with
    /// crystals persisted to `backend`. The default ruler is [`Recency`];
    /// without a [`Recompute`] no fact is ever evicted.
    pub fn new(capacity: Capacity, backend: B) -> Self {
        Brain {
            scope: Scope::public(),
            capacity,
            backend,
            ruler: Box::new(Recency),
            recompute: None,
            interner: Interner::default(),
            relations: BTreeMap::new(),
            tables: Vec::new(),
            open: Tick(0),
            committed: None,
            crystals: BTreeMap::new(),
            settled_order: VecDeque::new(),
            stats: Stats::default(),
            recalled: Cell::new(0),
            lit: 0,
            pending: 0,
        }
    }

    /// Evict by this ruler instead.
    pub fn with_ruler(mut self, ruler: impl Ruler + 'static) -> Self {
        self.ruler = Box::new(ruler);
        self
    }

    /// Recompute evicted facts with this, which also lets facts be evicted.
    pub fn with_recompute(mut self, recompute: impl Recompute + 'static) -> Self {
        self.recompute = Some(Box::new(recompute));
        self
    }

    /// The scope every address is taken in: public, for now.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// The backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// The interner.
    pub fn interner(&self) -> &Interner {
        &self.interner
    }

    /// Intern a constant.
    pub fn intern(&mut self, bytes: &[u8]) -> Sym {
        self.interner.intern(bytes)
    }

    /// The relation `name` of `arity`, declared on first use.
    pub fn relation(&mut self, name: &str, arity: usize) -> Relation {
        let next = Relation(u32::try_from(self.tables.len()).expect("fewer than 2³² relations"));
        let tables = &mut self.tables;
        *self
            .relations
            .entry((name.to_string(), arity))
            .or_insert_with(|| {
                tables.push(Table {
                    arity,
                    rows: BTreeMap::new(),
                    evicted: 0,
                    lit: 0,
                    pending: 0,
                });
                next
            })
    }

    /// The open tick: where deposits go now.
    pub fn tick(&self) -> Tick {
        self.open
    }

    /// The last committed tick, if any.
    pub fn committed(&self) -> Option<Tick> {
        self.committed
    }

    /// What the brain did with its capacity so far.
    pub fn stats(&self) -> Stats {
        Stats {
            recalled_crystals: self.recalled.get(),
            ..self.stats
        }
    }

    /// Lums: the number of distinct facts ever deposited at or before the
    /// committed tick. Evicted facts still count; restored facts are not
    /// counted again; the open tick's deposits count once it commits. O(1).
    ///
    /// **Progress and monotonicity, never identity**: read it beside an
    /// address, never as part of one (see the module docs).
    pub fn lums(&self) -> u64 {
        self.lit
    }

    /// [`Brain::lums`] for one relation. The relations' lums sum to the
    /// brain's. O(1).
    pub fn lums_of(&self, relation: Relation) -> u64 {
        self.tables[relation.0 as usize].lit
    }

    /// Deposit a fact at the open tick. False if it was already known
    /// (hot, or evicted and recomputed), in which case its tick stays.
    pub fn deposit(&mut self, relation: Relation, tuple: &[Sym]) -> bool {
        if self.contains(relation, tuple).is_some() {
            return false;
        }
        let table = &mut self.tables[relation.0 as usize];
        let cost = table.row_bytes();
        table.rows.insert(tuple.into(), self.open);
        table.pending += 1;
        self.pending += 1;
        self.hold(cost);
        true
    }

    /// The tick a fact was deposited at, if it is known.
    pub fn contains(&mut self, relation: Relation, tuple: &[Sym]) -> Option<Tick> {
        let table = &self.tables[relation.0 as usize];
        assert_eq!(tuple.len(), table.arity, "a tuple of the relation's arity");
        if let Some(&tick) = table.rows.get(tuple) {
            return Some(tick);
        }
        let pattern: Vec<Option<Sym>> = tuple.iter().copied().map(Some).collect();
        self.restore(relation, &pattern);
        self.tables[relation.0 as usize].rows.get(tuple).copied()
    }

    /// Every known fact of `relation` matching `pattern` (`None` is a free
    /// position), with its deposit tick, in tuple order. Evicted facts are
    /// recomputed first.
    pub fn scan(&mut self, relation: Relation, pattern: &[Option<Sym>]) -> Vec<(Vec<Sym>, Tick)> {
        let arity = self.tables[relation.0 as usize].arity;
        assert_eq!(pattern.len(), arity, "a pattern of the relation's arity");
        self.restore(relation, pattern);
        let prefix: Vec<Sym> = pattern.iter().map_while(|p| *p).collect();
        self.tables[relation.0 as usize]
            .rows
            .range::<[Sym], _>((Bound::Included(&prefix[..]), Bound::Unbounded))
            .take_while(|(t, _)| t.starts_with(&prefix))
            .filter(|(t, _)| matches(t, pattern))
            .map(|(t, &tick)| (t.to_vec(), tick))
            .collect()
    }

    /// Close the open tick: it becomes committed, back pressure applies,
    /// and the next tick opens. Returns the tick just committed.
    pub fn commit(&mut self) -> Tick {
        let closed = self.open;
        self.committed = Some(closed);
        self.open = Tick(closed.0 + 1);
        // The open tick's light is lit. A table's pending is non-zero only
        // if something was deposited this tick.
        if self.pending > 0 {
            for table in &mut self.tables {
                table.lit += std::mem::take(&mut table.pending);
            }
            self.lit += std::mem::take(&mut self.pending);
        }
        self.press();
        closed
    }

    /// The Drop: settle `bytes` as a crystal, persist it, and return its
    /// address. The same bytes settle to the same address, and add nothing.
    pub fn settle(&mut self, bytes: Vec<u8>) -> Result<Oid, B::Error> {
        let crystal = Crystal::settle(&self.scope, bytes);
        let address = crystal.address.clone();
        if !self.crystals.contains_key(&address) {
            self.backend.persist(&crystal)?;
            let cost = crystal.bytes.len();
            self.crystals.insert(address.clone(), crystal);
            self.settled_order.push_back(address.clone());
            self.hold(cost);
            self.press();
        }
        Ok(address)
    }

    /// Read a crystal by address: hot if it is held, otherwise from the
    /// backend, whose bytes must hash to the address.
    pub fn crystal(&self, address: &Oid) -> Result<Crystal, Recall<B::Error>> {
        if let Some(c) = self.crystals.get(address) {
            return Ok(c.clone());
        }
        let bytes = self
            .backend
            .fetch(address)
            .map_err(Recall::Backend)?
            .ok_or_else(|| Recall::Lost(address.clone()))?;
        let crystal = Crystal::settle(&self.scope, bytes);
        if &crystal.address != address {
            return Err(Recall::Forged(address.clone()));
        }
        self.recalled.set(self.recalled.get() + 1);
        Ok(crystal)
    }

    fn hold(&mut self, bytes: usize) {
        self.stats.hot_bytes += bytes;
        self.stats.peak_hot_bytes = self.stats.peak_hot_bytes.max(self.stats.hot_bytes);
    }

    /// Bring back evicted facts of `relation` that match `pattern`, with the
    /// ticks they were deposited at. Only committed ticks count.
    fn restore(&mut self, relation: Relation, pattern: &[Option<Sym>]) {
        let i = relation.0 as usize;
        if self.tables[i].evicted == 0 {
            return;
        }
        let (Some(recompute), Some(committed)) = (self.recompute.as_mut(), self.committed) else {
            return;
        };
        let rows = recompute.recompute(&self.interner, relation, pattern);
        self.stats.recomputes += 1;
        let table = &mut self.tables[i];
        let cost = table.row_bytes();
        let mut restored = 0;
        for (tuple, tick) in rows {
            let fits = tuple.len() == table.arity && matches(&tuple, pattern) && tick <= committed;
            if fits && !table.rows.contains_key(&tuple[..]) {
                table.rows.insert(tuple.into(), tick);
                restored += 1;
            }
        }
        table.evicted = table.evicted.saturating_sub(restored);
        self.stats.restored_facts += restored;
        self.hold(restored * cost);
    }

    /// Back pressure: while over capacity, evict cached crystals (oldest
    /// settled first; the backend holds them), then committed facts,
    /// coldest first. Never a fact of the open tick, and never a fact that
    /// can't be recomputed.
    fn press(&mut self) {
        while !self.capacity.admits(self.stats.hot_bytes) {
            let Some(address) = self.settled_order.pop_front() else {
                break;
            };
            let crystal = self.crystals.remove(&address).expect("held in order");
            self.stats.hot_bytes -= crystal.bytes.len();
            self.stats.evicted_crystals += 1;
        }
        if self.capacity.admits(self.stats.hot_bytes) || self.recompute.is_none() {
            return;
        }
        let Some(committed) = self.committed else {
            return;
        };
        let mut cold: Vec<(i64, Relation, Box<[Sym]>)> = Vec::new();
        for (i, table) in self.tables.iter().enumerate() {
            let relation = Relation(i as u32);
            for (tuple, &tick) in &table.rows {
                if tick <= committed {
                    let fact = Fact {
                        relation,
                        tuple,
                        tick,
                    };
                    cold.push((self.ruler.warmth(fact), relation, tuple.clone()));
                }
            }
        }
        cold.sort();
        for (_, relation, tuple) in cold {
            if self.capacity.admits(self.stats.hot_bytes) {
                break;
            }
            let table = &mut self.tables[relation.0 as usize];
            table.rows.remove(&tuple);
            table.evicted += 1;
            self.stats.hot_bytes -= table.row_bytes();
            self.stats.evicted_facts += 1;
        }
    }
}

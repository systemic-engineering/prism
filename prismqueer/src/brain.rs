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
//! # The vacuum: the fourth law (prism-013 §3)
//!
//! Alex: a crystal that no identity reaches is dangling; no observer
//! observes it, so it can be vacuumed. With Seam's corrections:
//!
//! - **Cooling is lossless forgetting** (invariant 2). [`Brain::cool`]
//!   evicts from memory; the backend still holds the bytes, and they read
//!   back by address.
//! - **Erasure** (deleting a scope's `K_id`) is the holder's act, and
//!   unreachability never drives it.
//! - **Purge** (deleting bytes) is irreversible: allowed only with an
//!   [`Erasure`], which only consuming a held scope's key mints. The public
//!   scope has no key to consume, so it cools and never purges.
//! - **Dangling is relative to the scopes this brain holds.** An address it
//!   can't see into is dark, never dangling; reach doesn't pass through it.
//! - **Unreachability is a negation**, decided only on a [`SealedRoots`]
//!   snapshot, and never final: [`Dangling`] is "not reached as of".
//! - **Reference counting holds within one key only.**
//!
//! References are declared at the Drop ([`Brain::settle_with`]) and kept
//! beside the crystal, never in its bytes, so they never change an address.
//! Roots are identities (addresses held in a relation), sealed with
//! [`Roots::seal`]. Reach reuses [`crate::dependency`]. A vacuum pass
//! changes no answer, no crystal and no lum (invariant 1).
//!
//! Keyed scopes and their erasure are below ([`Erasure`]; prism-026,
//! `docs/spec/brain.md`). Not here yet: a keyed [`Brain`], and windowed
//! input.
//!
//! # One port
//!
//! [`Backend`] is prism's one content-store port (prism-017 ask 7,
//! prism-019): it is the port with a proof (`docs/spec/brain.md`).
//! `store::Store` is retired. A read by address outside a [`Brain`] is
//! [`recall`], which checks integrity the same way [`Brain::crystal`] does.
//! The control compiles: the port resolves, and bounds an adapter.
//!
//! ```
//! use prismqueer::brain::Backend as Port;
//! fn adapter<P: Port>(_: &P) {}
//! ```
//!
//! The retired trait doesn't resolve:
//!
//! ```compile_fail,E0432
//! use prismqueer::store::Store as Port;
//! fn adapter<P: Port>(_: &P) {}
//! ```
//!
//! nor does its re-export:
//!
//! ```compile_fail,E0432
//! use prismqueer::Store as Port;
//! fn adapter<P: Port>(_: &P) {}
//! ```
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
//!
//! # The vacuum runs on a sealed snapshot of roots
//!
//! Unreachability is a negation, so it is decided only on a settled
//! snapshot. The control compiles: the vacuum takes roots that were sealed.
//!
//! ```
//! use std::collections::BTreeSet;
//! use prismqueer::brain::{Brain, Capacity, Memory, Roots, SealedRoots};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let root = brain.settle(b"identity".to_vec()).unwrap();
//! let mut roots = Roots::default();
//! roots.hold(root.clone());
//! let reach = brain.vacuum(&roots.seal());
//! assert!(reach.reachable().contains(&root));
//! ```
//!
//! The live root set, which can still grow, is refused:
//!
//! ```compile_fail,E0308
//! use std::collections::BTreeSet;
//! use prismqueer::brain::{Brain, Capacity, Memory, Roots, SealedRoots};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let root = brain.settle(b"identity".to_vec()).unwrap();
//! let mut roots = Roots::default();
//! roots.hold(root.clone());
//! let reach = brain.vacuum(&roots);
//! assert!(reach.reachable().contains(&root));
//! ```
//!
//! so is a bare set of addresses:
//!
//! ```compile_fail,E0308
//! use std::collections::BTreeSet;
//! use prismqueer::brain::{Brain, Capacity, Memory, Roots, SealedRoots};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let root = brain.settle(b"identity".to_vec()).unwrap();
//! let mut roots = Roots::default();
//! roots.hold(root.clone());
//! let reach = brain.vacuum(&BTreeSet::from([root.clone()]));
//! assert!(reach.reachable().contains(&root));
//! ```
//!
//! and a sealed snapshot can't be built except by sealing:
//!
//! ```compile_fail,E0451
//! use std::collections::BTreeSet;
//! use prismqueer::brain::{Brain, Capacity, Memory, Roots, SealedRoots};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let root = brain.settle(b"identity".to_vec()).unwrap();
//! let mut roots = Roots::default();
//! roots.hold(root.clone());
//! let reach = brain.vacuum(&SealedRoots { roots: BTreeSet::from([root.clone()]) });
//! assert!(reach.reachable().contains(&root));
//! ```
//!
//! # The public scope has no Erasure
//!
//! An [`Erasure`] is minted only by consuming a held scope's [`ScopeKey`],
//! and there is no `ScopeKey` for the public scope (`ScopeKey::take` refuses
//! `K_public`'s bytes). The control compiles: a held key, a crystal of its
//! scope, a mint, and cooling, which needs no evidence.
//!
//! ```
//! use prismqueer::brain::{Brain, Capacity, Crystal, Epoch, Erasure, Memory, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let _ = brain.settle(b"node e".to_vec()).unwrap();
//! let reach = brain.vacuum(&Roots::default().seal());
//! assert_eq!(brain.cool(reach.dangling()), 1);
//! let key = ScopeKey::take(&mut [7; 32]).unwrap();
//! let census = [Crystal::settle(&Scope::keyed(&key), b"node e".to_vec())];
//! let erasure = Erasure::mint(key, Epoch(1), &census).unwrap();
//! assert_eq!(erasure.covers().len(), 1);
//! ```
//!
//! Each of the following changes its last lines, and doesn't compile. The
//! public scope is not a key:
//!
//! ```compile_fail,E0308
//! use prismqueer::brain::{Brain, Capacity, Crystal, Epoch, Erasure, Memory, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let _ = brain.settle(b"node e".to_vec()).unwrap();
//! let reach = brain.vacuum(&Roots::default().seal());
//! assert_eq!(brain.cool(reach.dangling()), 1);
//! let erasure = Erasure::mint(Scope::public(), Epoch(1), &[]).unwrap();
//! ```
//!
//! and has none:
//!
//! ```compile_fail,E0599
//! use prismqueer::brain::{Brain, Capacity, Crystal, Epoch, Erasure, Memory, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let _ = brain.settle(b"node e".to_vec()).unwrap();
//! let reach = brain.vacuum(&Roots::default().seal());
//! assert_eq!(brain.cool(reach.dangling()), 1);
//! let erasure = Erasure::mint(ScopeKey::public(), Epoch(1), &[]).unwrap();
//! ```
//!
//! an Erasure can't be built by hand, say to cover a public crystal:
//!
//! ```compile_fail,E0451
//! use std::collections::BTreeSet;
//! use prismqueer::brain::{Brain, Capacity, Crystal, Epoch, Erasure, Memory, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let _ = brain.settle(b"node e".to_vec()).unwrap();
//! let reach = brain.vacuum(&Roots::default().seal());
//! assert_eq!(brain.cool(reach.dangling()), 1);
//! let key = ScopeKey::take(&mut [7; 32]).unwrap();
//! let real = Erasure::mint(key, Epoch(1), &[]).unwrap();
//! let public = brain.settle(b"node e".to_vec()).unwrap();
//! let erasure = Erasure { receipt: real.receipt().clone(), covers: BTreeSet::from([public]) };
//! ```
//!
//! a dangling verdict is not a key (it is never final):
//!
//! ```compile_fail,E0308
//! use prismqueer::brain::{Brain, Capacity, Crystal, Epoch, Erasure, Memory, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let _ = brain.settle(b"node e".to_vec()).unwrap();
//! let reach = brain.vacuum(&Roots::default().seal());
//! assert_eq!(brain.cool(reach.dangling()), 1);
//! let erasure = Erasure::mint(reach.dangling().clone(), Epoch(1), &[]).unwrap();
//! ```
//!
//! and minting consumes the key: it can't be used after.
//!
//! ```compile_fail,E0382
//! use prismqueer::brain::{Brain, Capacity, Crystal, Epoch, Erasure, Memory, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let _ = brain.settle(b"node e".to_vec()).unwrap();
//! let reach = brain.vacuum(&Roots::default().seal());
//! assert_eq!(brain.cool(reach.dangling()), 1);
//! let key = ScopeKey::take(&mut [7; 32]).unwrap();
//! let erasure = Erasure::mint(key, Epoch(1), &[]).unwrap();
//! let again = Scope::keyed(&key);
//! ```
//!
//! # A purge reads `Lost`, erased; dangling and dark are not `Lost`
//!
//! The control compiles: an erasure purges what it covers, and the read
//! says why the bytes are gone.
//!
//! ```
//! use prismqueer::brain::{purge, recall, Backend, Brain, Capacity, Crystal, Epoch, Erasure, Loss, Memory, Recall, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let reach = brain.vacuum(&Roots::default().seal());
//! let key = ScopeKey::take(&mut [7; 32]).unwrap();
//! let crystal = Crystal::settle(&Scope::keyed(&key), b"node e".to_vec());
//! let mut backend = Memory::default();
//! backend.persist(&crystal).unwrap();
//! let erasure = Erasure::mint(key, Epoch(1), &[crystal.clone()]).unwrap();
//! purge(&mut backend, &erasure).unwrap();
//! let loss: Loss = Loss::Erased(erasure.receipt().clone());
//! assert_eq!(
//!     recall(&backend, &Scope::public(), crystal.address()),
//!     Err(Recall::Lost { address: crystal.address().clone(), reason: loss })
//! );
//! ```
//!
//! A dangling verdict is not erasure evidence:
//!
//! ```compile_fail,E0308
//! use prismqueer::brain::{purge, recall, Backend, Brain, Capacity, Crystal, Epoch, Erasure, Loss, Memory, Recall, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let reach = brain.vacuum(&Roots::default().seal());
//! let key = ScopeKey::take(&mut [7; 32]).unwrap();
//! let crystal = Crystal::settle(&Scope::keyed(&key), b"node e".to_vec());
//! let mut backend = Memory::default();
//! backend.persist(&crystal).unwrap();
//! purge(&mut backend, reach.dangling()).unwrap();
//! ```
//!
//! a dangling verdict is not a loss:
//!
//! ```compile_fail,E0308
//! use prismqueer::brain::{purge, recall, Backend, Brain, Capacity, Crystal, Epoch, Erasure, Loss, Memory, Recall, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let reach = brain.vacuum(&Roots::default().seal());
//! let loss: Loss = reach.dangling().clone();
//! ```
//!
//! nor is a dark set:
//!
//! ```compile_fail,E0308
//! use prismqueer::brain::{purge, recall, Backend, Brain, Capacity, Crystal, Epoch, Erasure, Loss, Memory, Recall, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let reach = brain.vacuum(&Roots::default().seal());
//! let loss: Loss = reach.dark().clone();
//! ```
//!
//! and a purge order can't be written by hand, to reach past the census:
//!
//! ```compile_fail,E0451
//! use prismqueer::brain::{purge, recall, Backend, Brain, Capacity, Crystal, Epoch, Erasure, Loss, Memory, Purge, Recall, Roots};
//! use prismqueer::oid::{Scope, ScopeKey};
//! let mut brain = Brain::new(Capacity::unbounded(), Memory::default());
//! let public = brain.settle(b"node e".to_vec()).unwrap();
//! let key = ScopeKey::take(&mut [7; 32]).unwrap();
//! let erasure = Erasure::mint(key, Epoch(1), &[]).unwrap();
//! let mut backend = Memory::default();
//! backend.purge(&Purge { address: &public, receipt: erasure.receipt() }).unwrap();
//! ```

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::Infallible;
use std::ops::Bound;
use std::sync::Arc;

use crate::dependency::{DependencyGraph, Sign, Through};
use prism_laws::crossing::{attest, Claim, Grade};
use crate::oid::{Oid, Scope, ScopeId, ScopeKey};

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

// The settled item itself is a law (docs/spec/laws.md): `crossing` replays
// over it. It lives in prism-laws and is re-exported here at its old path.
pub use prism_laws::crystal::Crystal;

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

    /// Carry out a purge order: delete the bytes under [`Purge::address`],
    /// everywhere this backend keeps them (every replica, outside git), and
    /// keep [`Purge::receipt`] as the address's tombstone, so it reads
    /// [`Loss::Erased`] from then on. Irreversible. Only [`purge`] issues an
    /// order, and only for an address its [`Erasure`] covers.
    ///
    /// Required, never defaulted: a default would either delete nothing or
    /// forget the receipt. A persist of a tombstoned address stores nothing.
    fn purge(&mut self, order: &Purge<'_>) -> Result<(), Self::Error>;

    /// The receipt of the erasure that purged `address`, if one did.
    fn erased(&self, address: &Oid) -> Result<Option<Receipt>, Self::Error>;
}

/// One purge order: an address an [`Erasure`] covers, and its receipt. No
/// public constructor: only [`purge`] builds one, so a backend is never asked
/// to delete an address outside an erasure's census.
#[derive(Debug)]
pub struct Purge<'e> {
    address: &'e Oid,
    receipt: &'e Receipt,
}

impl Purge<'_> {
    /// The address to delete.
    pub fn address(&self) -> &Oid {
        self.address
    }

    /// The receipt to keep as its tombstone.
    pub fn receipt(&self) -> &Receipt {
        self.receipt
    }
}

/// Purge every address `erasure` covers from `backend`: each one's bytes
/// deleted, its receipt kept as the tombstone. After it, [`recall`] reads
/// each as `Lost { reason: Loss::Erased(receipt) }`, on every backend.
///
/// A free function, like [`recall`], so no adapter can widen an order: the
/// erasure's census is the whole of what it deletes.
pub fn purge<B: Backend + ?Sized>(backend: &mut B, erasure: &Erasure) -> Result<(), B::Error> {
    for address in &erasure.covers {
        backend.purge(&Purge {
            address,
            receipt: &erasure.receipt,
        })?;
    }
    Ok(())
}

/// Evidence that a held scope's key was erased: the holder's attested act
/// (prism-026 item 1).
///
/// **Minted only by consuming the scope's [`ScopeKey`]** ([`Erasure::mint`]
/// takes it by value), so in this process the key is gone once the mint
/// returns. It can't prove that no copy exists elsewhere, and nothing can:
/// an Erasure is a sealed judgment, never a law. The public scope has no
/// `ScopeKey`, so it has no Erasure: it cools and never purges. A dangling
/// verdict is never this evidence; it is never final.
///
/// It covers exactly the crystals the holder showed it while the key was
/// live (the census), each checked to be in the key's scope, so an Erasure
/// can never reach another scope's crystals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Erasure {
    receipt: Receipt,
    covers: BTreeSet<Oid>,
}

/// When an erasure was attested, in the holder's own count: an evaluator's
/// round or a store's committed point. Not a clock; prism reads no time.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

/// An erasure receipt (prism-026 item 7): the scope's public id and an
/// epoch, **never the person**.
///
/// - **Its address** is `Oid::hash(Scope::public(), scope_id ‖ epoch)`: the
///   32 bytes of the [`ScopeId`], then the epoch as a big-endian `u64`
///   ([`Receipt::preimage`], 40 bytes). Nothing derived from the holder's
///   identity beyond the scope id goes in. Public, so anyone holding the
///   receipt recomputes it.
/// - **Its grade is [`Grade::Sealed`]**, never Law: the receipt is a
///   [`Claim`] with no proof and one seal. An erasure is the holder's
///   attested act; consuming the key in this process can't prove that no
///   copy exists elsewhere.
/// - **The signer sits beside the address**, never in it (`ruler.md`), and
///   its label is the scope id: the holder appears through its boundary,
///   never its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    scope: ScopeId,
    epoch: Epoch,
    claim: Claim,
}

impl Receipt {
    fn mint(scope: ScopeId, epoch: Epoch) -> Self {
        let address = Oid::hash(&Scope::public(), &Self::preimage_of(scope, epoch));
        let seal = attest(&address, scope.oid());
        let claim = Claim::new(address, None, [seal]).expect("one seal: never anonymous");
        Receipt {
            scope,
            epoch,
            claim,
        }
    }

    fn preimage_of(scope: ScopeId, epoch: Epoch) -> Vec<u8> {
        let mut preimage = Vec::with_capacity(40);
        preimage.extend_from_slice(scope.as_bytes());
        preimage.extend_from_slice(&epoch.0.to_be_bytes());
        preimage
    }

    /// The bytes its address is the hash of: scope id ‖ epoch (big-endian).
    pub fn preimage(&self) -> Vec<u8> {
        Self::preimage_of(self.scope, self.epoch)
    }

    /// Its address, in the public scope.
    pub fn address(&self) -> &Oid {
        self.claim.value()
    }

    /// The erased scope's public id.
    pub fn scope(&self) -> ScopeId {
        self.scope
    }

    /// When the erasure was attested.
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// The claim: the address, no proof, the holder's seal beside it.
    pub fn claim(&self) -> &Claim {
        &self.claim
    }

    /// Always [`Grade::Sealed`]: the holder's attested act, never a law.
    pub fn grade(&self) -> Grade {
        self.claim.grade()
    }
}

impl Erasure {
    /// Erase a held scope: consume its key, covering the `census`, the
    /// scope's crystals the holder shows it. A crystal that isn't in the
    /// key's scope is refused as a [`Stranger`], and the key comes back
    /// unconsumed.
    pub fn mint(key: ScopeKey, epoch: Epoch, census: &[Crystal]) -> Result<Erasure, Stranger> {
        let held = Scope::keyed(&key);
        let stranger = census
            .iter()
            .find(|c| &Oid::hash(&held, c.bytes()) != c.address())
            .map(|c| c.address().clone());
        if let Some(address) = stranger {
            return Err(Stranger { key, address });
        }
        let covers = census.iter().map(|c| c.address().clone()).collect();
        let scope = key.id();
        // The key is consumed here: dropped, so its bytes are zeroed.
        drop(key);
        Ok(Erasure {
            receipt: Receipt::mint(scope, epoch),
            covers,
        })
    }

    /// The erased scope's public id.
    pub fn scope(&self) -> ScopeId {
        self.receipt.scope
    }

    /// The receipt: scope id and epoch, never the person.
    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }

    /// The addresses it covers.
    pub fn covers(&self) -> &BTreeSet<Oid> {
        &self.covers
    }
}

/// A crystal offered to [`Erasure::mint`] that isn't in the key's scope. The
/// key comes back unconsumed.
#[derive(Debug)]
pub struct Stranger {
    key: ScopeKey,
    address: Oid,
}

impl Stranger {
    /// The crystal's address.
    pub fn address(&self) -> &Oid {
        &self.address
    }

    /// The key, back to its holder.
    pub fn into_key(self) -> ScopeKey {
        self.key
    }
}

/// The in-memory backend, for tests.
#[derive(Clone, Debug, Default)]
pub struct Memory {
    crystals: BTreeMap<Oid, Arc<[u8]>>,
    erased: BTreeMap<Oid, Receipt>,
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
            .insert(crystal.address().clone(), crystal.shared_bytes());
        Ok(())
    }

    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        Ok(self.crystals.get(address).map(|b| b.to_vec()))
    }

    fn purge(&mut self, order: &Purge<'_>) -> Result<(), Infallible> {
        self.crystals.remove(order.address());
        self.erased
            .insert(order.address().clone(), order.receipt().clone());
        Ok(())
    }

    fn erased(&self, address: &Oid) -> Result<Option<Receipt>, Infallible> {
        Ok(self.erased.get(address).cloned())
    }
}

// ---------------------------------------------------------------------------
// The vacuum: roots, reach, and the verdicts
// ---------------------------------------------------------------------------

/// The live root set: identities, each an address held in a relation
/// (`docs/spec/identity.md`, "Address and identity"). It can still grow, so
/// the vacuum never reads it; [`Roots::seal`] takes the snapshot it reads.
///
/// prism doesn't see the relation an address is held in (that is mirror's);
/// whoever holds a root here vouches that it is held.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roots {
    roots: BTreeSet<Oid>,
}

impl Roots {
    /// Hold an identity as a root.
    pub fn hold(&mut self, identity: Oid) {
        self.roots.insert(identity);
    }

    /// The snapshot of the roots held now. Roots held later are not in it.
    pub fn seal(&self) -> SealedRoots {
        SealedRoots {
            roots: self.roots.clone(),
        }
    }
}

/// A settled snapshot of roots: the only root set a vacuum takes. The only
/// way to get one is [`Roots::seal`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedRoots {
    roots: BTreeSet<Oid>,
}

impl SealedRoots {
    /// The roots as sealed.
    pub fn roots(&self) -> &BTreeSet<Oid> {
        &self.roots
    }
}

/// The crystals of this scope that a snapshot's roots didn't reach.
///
/// **Never final.** Unreachability is a negation: it says "not reached as
/// of [`Dangling::as_of`]", like an `Imperfect::Partial` that carries what
/// it doesn't know yet. A root held later can reach any of them again. So
/// the only thing it licenses is [`Brain::cool`], which is lossless, and it
/// never converts to an [`Erasure`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dangling {
    addresses: BTreeSet<Oid>,
    as_of: SealedRoots,
}

impl Dangling {
    /// The addresses not reached.
    pub fn addresses(&self) -> &BTreeSet<Oid> {
        &self.addresses
    }

    /// The snapshot the verdict was decided on, and holds only for.
    pub fn as_of(&self) -> &SealedRoots {
        &self.as_of
    }
}

/// What a vacuum found, from a sealed snapshot of roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
    reachable: BTreeSet<Oid>,
    dangling: Dangling,
    dark: BTreeSet<Oid>,
}

impl Reach {
    /// Crystals of this scope that the roots reach.
    pub fn reachable(&self) -> &BTreeSet<Oid> {
        &self.reachable
    }

    /// Crystals of this scope that the roots didn't reach, as of the
    /// snapshot. Never final.
    pub fn dangling(&self) -> &Dangling {
        &self.dangling
    }

    /// Addresses named (by a reference or a root) that this brain doesn't
    /// hold: another scope's, or not settled here. Unknown, never dangling,
    /// and reach doesn't pass through them: what they point to is unseen.
    pub fn dark(&self) -> &BTreeSet<Oid> {
        &self.dark
    }
}

/// Read a crystal back by address through any [`Backend`], checked: fetch,
/// rehash under `scope`, and refuse bytes that aren't the address's.
///
/// This is the one read through the port (A2 in `docs/spec/brain.md`:
/// integrity checked on every read; durability is the adapter's
/// obligation). [`Brain::crystal`] reads through it on a cache miss. It is a
/// free function, not a trait method, so no adapter can override it and
/// skip the rehash.
///
/// - an erased address: [`Recall::Lost`] with [`Loss::Erased`] and the
///   receipt, asked first, so it fails closed even if a faulty backend kept
///   the bytes (a purge is final);
/// - bytes that hash to `address`: the crystal;
/// - no bytes: [`Recall::Lost`] with [`Loss::Missing`], never an empty
///   value;
/// - other bytes: [`Recall::Forged`], never believed;
/// - the backend failed: [`Recall::Backend`].
pub fn recall<B: Backend + ?Sized>(
    backend: &B,
    scope: &Scope<'_>,
    address: &Oid,
) -> Result<Crystal, Recall<B::Error>> {
    let bytes = backend
        .fetch(address)
        .map_err(Recall::Backend)?
        .ok_or_else(|| Recall::Lost {
            address: address.clone(),
            reason: Loss::Missing,
        })?;
    let crystal = Crystal::settle(scope, bytes);
    if crystal.address() != address {
        return Err(Recall::Forged(address.clone()));
    }
    Ok(crystal)
}

/// Why a crystal couldn't be read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recall<E> {
    /// The backend failed.
    Backend(E),
    /// Neither hot nor in the backend, and why: erased (final, with its
    /// receipt) or missing (never settled here, or the backend lost it).
    Lost {
        /// The address read.
        address: Oid,
        /// Why there are no bytes.
        reason: Loss,
    },
    /// The backend's bytes don't hash to the address. Not believed.
    Forged(Oid),
}

/// Why an address has no bytes (mirror-036 E1). A purge and a lost disk
/// never read the same.
///
/// Kept apart from the vacuum's verdicts: [`Dangling`] is "not reached as
/// of" and never final, a dark address is one this brain can't see into,
/// and `Lost` is a read's answer. None converts into another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loss {
    /// Purged under an [`Erasure`]: lawful and final. The receipt names the
    /// scope's public id and the epoch, never the person.
    Erased(Receipt),
    /// No bytes and no tombstone: never settled here, or a durability fault
    /// (A2: keeping the bytes is the adapter's obligation).
    Missing,
}

impl Loss {
    /// The erasure receipt, if the bytes were erased. `Missing` has none.
    pub fn receipt(&self) -> Option<&Receipt> {
        match self {
            Loss::Erased(receipt) => Some(receipt),
            Loss::Missing => None,
        }
    }
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
    scope: Scope<'static>,
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
    /// Every crystal settled here, with the addresses it points to. Beside
    /// the crystals, never in their bytes, and never evicted.
    references: BTreeMap<Oid, BTreeSet<Oid>>,
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
            references: BTreeMap::new(),
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
    pub fn scope(&self) -> &Scope<'static> {
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
        self.settle_with(bytes, [])
    }

    /// [`Brain::settle`], declaring the addresses the crystal points to.
    ///
    /// The references are kept beside the crystal, never in its bytes, so
    /// they never change its address. Settling the same bytes again adds
    /// any new references to the old ones: a union, which can only reach
    /// more, so it errs toward keeping crystals warm.
    pub fn settle_with(
        &mut self,
        bytes: Vec<u8>,
        references: impl IntoIterator<Item = Oid>,
    ) -> Result<Oid, B::Error> {
        let crystal = Crystal::settle(&self.scope, bytes);
        let address = crystal.address().clone();
        if !self.crystals.contains_key(&address) {
            self.backend.persist(&crystal)?;
            let cost = crystal.bytes().len();
            self.crystals.insert(address.clone(), crystal);
            self.settled_order.push_back(address.clone());
            self.hold(cost);
            self.press();
        }
        // Only once it is persisted is it in this scope's census.
        self.references
            .entry(address.clone())
            .or_default()
            .extend(references);
        Ok(address)
    }

    /// Read a crystal by address: hot if it is held, otherwise from the
    /// backend, whose bytes must hash to the address.
    pub fn crystal(&self, address: &Oid) -> Result<Crystal, Recall<B::Error>> {
        if let Some(c) = self.crystals.get(address) {
            return Ok(c.clone());
        }
        let crystal = recall(&self.backend, &self.scope, address)?;
        self.recalled.set(self.recalled.get() + 1);
        Ok(crystal)
    }

    /// The addresses a crystal settled here points to, or `None` if it was
    /// never settled here.
    pub fn references(&self, address: &Oid) -> Option<&BTreeSet<Oid>> {
        self.references.get(address)
    }

    /// The vacuum: reach over the reference graph from a sealed snapshot of
    /// roots, and classify every address it knows of as reachable, dangling
    /// or dark. Changes nothing; [`Brain::cool`] acts on the verdict.
    ///
    /// "Dangling" is relative to the scopes this brain holds: the public
    /// scope, for now. Reference counting would hold within this one key
    /// only, so none is kept across scopes.
    pub fn vacuum(&self, roots: &SealedRoots) -> Reach {
        let mut graph = DependencyGraph::new();
        for (from, tos) in &self.references {
            graph.add_node(from.clone());
            for to in tos {
                graph.add_edge(from.clone(), to.clone(), Sign::Positive);
            }
        }
        for root in &roots.roots {
            graph.add_node(root.clone());
        }
        // Dark addresses have no edges here, so reach stops at them.
        let reached = graph.reach(&roots.roots, Through::Positive);
        let held = |a: &&Oid| self.references.contains_key(*a);
        let reachable: BTreeSet<Oid> = reached.iter().filter(held).cloned().collect();
        let dangling = self
            .references
            .keys()
            .filter(|a| !reachable.contains(*a))
            .cloned()
            .collect();
        let dark = self
            .references
            .values()
            .flatten()
            .chain(&roots.roots)
            .filter(|a| !held(a))
            .cloned()
            .collect();
        Reach {
            reachable,
            dangling: Dangling {
                addresses: dangling,
                as_of: roots.clone(),
            },
            dark,
        }
    }

    /// Cool the dangling: evict them from the hot cache. Lossless, because
    /// every crystal was persisted when it settled, so each still reads back
    /// by address. Returns how many were hot.
    ///
    /// A stale verdict is harmless: a crystal cooled that is reached again
    /// costs only a backend read. Temperature never changes truth.
    pub fn cool(&mut self, dangling: &Dangling) -> usize {
        let mut cooled = 0;
        for address in &dangling.addresses {
            if let Some(crystal) = self.crystals.remove(address) {
                self.stats.hot_bytes -= crystal.bytes().len();
                self.stats.evicted_crystals += 1;
                cooled += 1;
            }
        }
        if cooled > 0 {
            let crystals = &self.crystals;
            self.settled_order.retain(|a| crystals.contains_key(a));
        }
        cooled
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
            self.stats.hot_bytes -= crystal.bytes().len();
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

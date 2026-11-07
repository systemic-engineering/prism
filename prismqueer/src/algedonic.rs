//! `prismqueer::algedonic`: when is a tension real enough to bump into?
//! Mara's A1–A3 (mirror `mara/roomba-monad` 2739e94c, §4.1, §5.1–5.3, §9).
//! The names are provisional.
//!
//! The live pillar `algedonic_of_magnitude` mixed two questions into one
//! verdict with an unsigned `confidence: 0.5`. It is split:
//!
//! - **T1, touchable: [`cross`]**`(enclosure, region)`. A reading is an
//!   [`Enclosure`] `[lo, hi]` of the true value, and a ruler's [`Region`] has
//!   exact rational edges. `In` when `[lo, hi] ⊆ R`, `Out` when they are
//!   disjoint, `Undecided` otherwise. No number rides on it. Zero is a
//!   reading: an exact zero is `In` or `Out` like any other value.
//!   - **Theorem (Mara §5.1, PROVED):** two correct enclosures of one value
//!     never give `In` and `Out`. They can differ as decided versus
//!     `Undecided`, when one is tighter.
//! - **T2, real: [`persist`]**`(evidence, ruler)`. Wald's sequential test over
//!   **distinct contexts**, deciding `Accept`, `Reject` or `Continue`.
//!   - **`Undecided` never counts as evidence** (§5.1 d), so float noise
//!     can't accumulate into reality.
//!   - Evidence is keyed by `(ruler, nbhd, instrument)` at one site
//!     ([`Evidence`]). One visit counts once per (ruler, decision): walking
//!     the same context again is a [`Recorded::Repeat`], never a second
//!     observation (no contraction, §5.2). Two instruments are two contexts.
//! - **T3, heard: the ruler's [`Clock`]**, the agreed delay as data. It is
//!   read against ticks the caller supplies. Nothing here reads a wall clock.
//!
//! The verdict shape is terni's [`Membership`] (`fractal` feature), reused,
//! not mirrored: `In` and `Out` are decided, `Undecided` is a fact about the
//! instrument. Each arm carries the enclosure that decided it.
//!
//! # The signal is a pair (A1)
//!
//! A [`Signal`] is `(pain, pleasure)`, each an [`Enclosure`], with the
//! instrument's address. It is ordered componentwise, a product order, and
//! there is no net: `(0, 0)`, `(0.4, 0.4)` and `(0.9, 0.9)` all net to zero,
//! and stay three signals. Pleasure is never `1 − pain`.
//!
//! # The ruler is a signed record (A2)
//!
//! A [`Ruler`] is `{ region, question, (α, β) with p₀/p₁, clock, signer }`.
//! It has no default and no public constructor: a [`Draft`] holds the
//! judgment, and only [`Draft::seal`] turns it into a ruler, minting the
//! [`Signer`]. The signer sits **beside** the ruler's address
//! ([`Ruler::oid`] hashes the draft, never the signer), so one judgment
//! signed by two people has one address and two signers.
//!
//! prism checks that a signer exists, never who it is: the signer's identity
//! is an address the caller vouches for, as a [`Roots`](crate::brain::Roots)
//! holder vouches for its roots. Verifying a signature is outside prism.
//!
//! A ruler can't be had by default:
//!
//! ```compile_fail,E0599
//! use prismqueer::algedonic::Ruler;
//! let _ = Ruler::default();
//! ```
//!
//! nor built around a signer that wasn't sealed:
//!
//! ```compile_fail,E0451
//! use prismqueer::algedonic::Signer;
//! use prismqueer::oid::Oid;
//! let _ = Signer { by: Oid::new("engineer"), over: Oid::new("ruler") };
//! ```
//!
//! and an unsealed draft doesn't persist:
//!
//! ```compile_fail,E0308
//! use num_bigint::BigInt;
//! use num_rational::BigRational;
//! use prismqueer::algedonic::{persist, Clock, Draft, Evidence, Question, Region, Wald};
//! use prismqueer::oid::Oid;
//! let q = |n: i64, d: i64| BigRational::new(BigInt::from(n), BigInt::from(d));
//! let wald = Wald::new(q(1, 20), q(1, 20), q(1, 6), q(1, 2)).unwrap();
//! let draft = Draft::new(Region::above(q(1, 1)), Question::Max, wald, Clock::agreed(10, 1, 100));
//! let _ = persist(&Evidence::at(Oid::new("leg")), &draft);
//! ```
//!
//! The control: sealing is the only way to a ruler, and it persists.
//!
//! ```
//! use num_bigint::BigInt;
//! use num_rational::BigRational;
//! use prismqueer::algedonic::{persist, Clock, Draft, Evidence, Persistence, Question, Region, Wald};
//! use prismqueer::oid::Oid;
//! let q = |n: i64, d: i64| BigRational::new(BigInt::from(n), BigInt::from(d));
//! let wald = Wald::new(q(1, 20), q(1, 20), q(1, 6), q(1, 2)).unwrap();
//! let draft = Draft::new(Region::above(q(1, 1)), Question::Max, wald, Clock::agreed(10, 1, 100));
//! let ruler = draft.seal(Oid::new("engineer"));
//! assert_eq!(ruler.signer().over(), &ruler.oid());
//! assert_eq!(persist(&Evidence::at(Oid::new("leg")), &ruler), Persistence::Continue);
//! ```
//!
//! # Hashing a decision (§5.1 a)
//!
//! [`decision_oid`] hashes `(site, nbhd, instrument_oid, ruler_oid,
//! decision)` in the public scope, under the shape prefix `"decision\0"`,
//! every field length-prefixed. Never an enclosure's bits: two machines'
//! enclosures a last bit apart hash alike whenever they decide alike.
//!
//! # How this relates to `discoball::Ruler`
//!
//! One discipline, not one type (Mara, prism-016 Log). Both rulers are the
//! caller's and neither has a default. They judge different things:
//!
//! - `discoball::Ruler` (behind `lapack`) is a trait, the vertical
//!   twist's judge: which declarations a facet's true set violates. Its
//!   findings are the caller's values, carried, never thresholded.
//! - [`Ruler`] here is a record, a threshold's judge: a region, a question,
//!   error rates and a clock, signed. It certifies a crossing (T1), decides
//!   persistence (T2) and carries the delay (T3).
//!
//! A discoball ruler is not signed yet. Requiring a signer of it is the
//! open half of the shared discipline.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

use crate::oid::{Oid, Scope};

pub use terni::fractal::Membership;

// --- Enclosures and regions -------------------------------------------------

/// A reading: the closed interval `[lo, hi]` that the instrument's own error
/// analysis guarantees holds the true value. Exact rational ends; a float
/// end is the dyadic rational it is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Enclosure {
    lo: BigRational,
    hi: BigRational,
}

/// Not an enclosure: `lo > hi`, or a float end that isn't finite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotEnclosure;

impl Enclosure {
    /// `[lo, hi]`. Refused when `lo > hi`.
    pub fn new(lo: BigRational, hi: BigRational) -> Result<Self, NotEnclosure> {
        if lo > hi {
            return Err(NotEnclosure);
        }
        Ok(Enclosure { lo, hi })
    }

    /// An exact value: `[v, v]`.
    pub fn exact(v: BigRational) -> Self {
        Enclosure {
            lo: v.clone(),
            hi: v,
        }
    }

    /// `[lo, hi]` from floats, each the dyadic rational it is. The outward
    /// rounding is the instrument's: this takes the ends as given.
    pub fn of_floats(lo: f64, hi: f64) -> Result<Self, NotEnclosure> {
        match (BigRational::from_float(lo), BigRational::from_float(hi)) {
            (Some(lo), Some(hi)) => Enclosure::new(lo, hi),
            _ => Err(NotEnclosure),
        }
    }

    /// The closure `[lo, hi]` of an exact root's isolating interval
    /// `(lo, hi]`: certified, since the root lies in it.
    pub fn of_isolating(interval: &crate::poly::Interval) -> Self {
        Enclosure {
            lo: interval.lo().clone(),
            hi: interval.hi().clone(),
        }
    }

    /// The lower end.
    pub fn lo(&self) -> &BigRational {
        &self.lo
    }

    /// The upper end.
    pub fn hi(&self) -> &BigRational {
        &self.hi
    }
}

/// The interval order: `[a, b] ≤ [c, d]` iff `a ≤ c` and `b ≤ d`. Partial:
/// `[0, 3]` and `[1, 2]` are incomparable.
impl PartialOrd for Enclosure {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        both(self.lo.cmp(&other.lo), self.hi.cmp(&other.hi))
    }
}

/// Two orderings as one product order: equal when both are, otherwise the
/// direction they share, or none.
fn both(a: Ordering, b: Ordering) -> Option<Ordering> {
    match (a, b) {
        (x, Ordering::Equal) | (Ordering::Equal, x) => Some(x),
        (x, y) if x == y => Some(x),
        _ => None,
    }
}

/// One end of a region.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    /// The end, excluded.
    Open(BigRational),
    /// The end, included.
    Closed(BigRational),
    /// No end.
    Unbounded,
}

/// A ruler's region: an interval of the reals with exact rational edges. For
/// a scalar threshold ε, `(ε, ∞)` ([`Region::above`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Region {
    lo: Edge,
    hi: Edge,
}

/// The region holds no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptyRegion;

impl Region {
    /// The region between `lo` and `hi`. Refused when empty.
    pub fn new(lo: Edge, hi: Edge) -> Result<Self, EmptyRegion> {
        let empty = match (&lo, &hi) {
            (Edge::Closed(a), Edge::Closed(b)) => a > b,
            (Edge::Open(a) | Edge::Closed(a), Edge::Open(b) | Edge::Closed(b)) => a >= b,
            _ => false,
        };
        if empty {
            return Err(EmptyRegion);
        }
        Ok(Region { lo, hi })
    }

    /// `(ε, ∞)`: strictly above a threshold.
    pub fn above(eps: BigRational) -> Self {
        Region {
            lo: Edge::Open(eps),
            hi: Edge::Unbounded,
        }
    }

    /// Whether `v` is in the region.
    pub fn contains(&self, v: &BigRational) -> bool {
        above_lo(&self.lo, v) && below_hi(&self.hi, v)
    }
}

/// `v` clears the lower edge.
fn above_lo(edge: &Edge, v: &BigRational) -> bool {
    match edge {
        Edge::Open(a) => v > a,
        Edge::Closed(a) => v >= a,
        Edge::Unbounded => true,
    }
}

/// `v` clears the upper edge.
fn below_hi(edge: &Edge, v: &BigRational) -> bool {
    match edge {
        Edge::Open(b) => v < b,
        Edge::Closed(b) => v <= b,
        Edge::Unbounded => true,
    }
}

/// A certified crossing: each arm carries the enclosure that decided it.
pub type Crossing = Membership<Enclosure, Enclosure, Enclosure>;

/// **T1.** `In` if `[lo, hi] ⊆ R`, `Out` if they are disjoint, `Undecided`
/// otherwise. Both are decided exactly on rationals; regions are convex, so
/// the ends decide.
pub fn cross(enclosure: &Enclosure, region: &Region) -> Crossing {
    let (lo, hi) = (&enclosure.lo, &enclosure.hi);
    if region.contains(lo) && region.contains(hi) {
        Membership::In(enclosure.clone())
    } else if !above_lo(&region.lo, hi) || !below_hi(&region.hi, lo) {
        Membership::Out(enclosure.clone())
    } else {
        Membership::Undecided(enclosure.clone())
    }
}

/// A crossing without its enclosure: what is hashed and counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Decision {
    /// Certified inside the region.
    In,
    /// Certified outside it.
    Out,
    /// The enclosure straddles an edge.
    Undecided,
}

impl Decision {
    /// The decision a crossing made.
    pub fn of(crossing: &Crossing) -> Self {
        match crossing {
            Membership::In(_) => Decision::In,
            Membership::Out(_) => Decision::Out,
            Membership::Undecided(_) => Decision::Undecided,
        }
    }

    fn tag(self) -> u8 {
        match self {
            Decision::In => 1,
            Decision::Out => 2,
            Decision::Undecided => 3,
        }
    }
}

// --- A1: the signal ---------------------------------------------------------

/// An algedonic signal: two channels, never one. Pain and pleasure each an
/// enclosure, with the instrument that read them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signal {
    pain: Enclosure,
    pleasure: Enclosure,
    instrument: Oid,
}

impl Signal {
    /// The pair, read by `instrument`.
    pub fn new(pain: Enclosure, pleasure: Enclosure, instrument: Oid) -> Self {
        Signal {
            pain,
            pleasure,
            instrument,
        }
    }

    /// The pain channel.
    pub fn pain(&self) -> &Enclosure {
        &self.pain
    }

    /// The pleasure channel.
    pub fn pleasure(&self) -> &Enclosure {
        &self.pleasure
    }

    /// The instrument that read both.
    pub fn instrument(&self) -> &Oid {
        &self.instrument
    }
}

/// Componentwise: a product order over (pain, pleasure), each in the
/// interval order. Two instruments' signals are not compared.
impl PartialOrd for Signal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.instrument != other.instrument {
            return None;
        }
        both(
            self.pain.partial_cmp(&other.pain)?,
            self.pleasure.partial_cmp(&other.pleasure)?,
        )
    }
}

// --- A2: the ruler ----------------------------------------------------------

/// Which aggregate a ruler reads a neighbourhood by (§4.2): the semiring of
/// its question. Different questions are different functions of the same
/// neighbourhood.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Question {
    /// "Is any edge here broken?" Boolean (∨, ∧) over certified crossings.
    Any,
    /// "How bad is the worst edge?" (max, ·): one loud edge is never diluted.
    Max,
    /// "How likely is the whole path?" (+, ×): the product along it.
    Product,
    /// "How much in total?" (+).
    Sum,
}

impl Question {
    fn tag(self) -> u8 {
        match self {
            Question::Any => 1,
            Question::Max => 2,
            Question::Product => 3,
            Question::Sum => 4,
        }
    }
}

/// Wald's sequential test, as the ruler signs it: the error rates `(α, β)`
/// (a false alarm, a miss) and the two hypotheses it decides between, the
/// chance `p₀` that a context certifies `In` when the tension isn't real and
/// `p₁` when it is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Wald {
    alpha: BigRational,
    beta: BigRational,
    p0: BigRational,
    p1: BigRational,
}

/// The rates don't make a test: each must lie strictly between 0 and 1,
/// `α + β < 1`, and `p₀ < p₁`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotATest;

impl Wald {
    /// The test. Refused unless `0 < α, β, p₀, p₁ < 1`, `α + β < 1` and
    /// `p₀ < p₁`.
    pub fn new(
        alpha: BigRational,
        beta: BigRational,
        p0: BigRational,
        p1: BigRational,
    ) -> Result<Self, NotATest> {
        let unit = |x: &BigRational| x.is_positive() && x < &BigRational::one();
        let valid = [&alpha, &beta, &p0, &p1].into_iter().all(unit)
            && &alpha + &beta < BigRational::one()
            && p0 < p1;
        if !valid {
            return Err(NotATest);
        }
        Ok(Wald {
            alpha,
            beta,
            p0,
            p1,
        })
    }

    /// α, the false-alarm rate.
    pub fn alpha(&self) -> &BigRational {
        &self.alpha
    }

    /// β, the miss rate.
    pub fn beta(&self) -> &BigRational {
        &self.beta
    }

    /// p₀: a context certifies `In` though the tension isn't real.
    pub fn p0(&self) -> &BigRational {
        &self.p0
    }

    /// p₁: a context certifies `In` and the tension is real.
    pub fn p1(&self) -> &BigRational {
        &self.p1
    }
}

/// T3's clock, as data (Beer B11): the reaction time and importance agreed
/// with the person concerned, and the delay they set. In ticks the caller
/// counts; prism reads no wall clock.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Clock {
    reaction: u64,
    importance: u64,
    delay: u64,
}

impl Clock {
    /// The clock as agreed. prism doesn't compute the delay from the other
    /// two: that function is the signer's.
    pub fn agreed(reaction: u64, importance: u64, delay: u64) -> Self {
        Clock {
            reaction,
            importance,
            delay,
        }
    }

    /// The reaction time.
    pub fn reaction(&self) -> u64 {
        self.reaction
    }

    /// The importance.
    pub fn importance(&self) -> u64 {
        self.importance
    }

    /// The delay before the signal breaks with the autonomy.
    pub fn delay(&self) -> u64 {
        self.delay
    }

    /// Whether the delay has run out at tick `now`, the clock having started
    /// at `started`. A tick before the start is not expiry.
    pub fn expired(&self, started: u64, now: u64) -> bool {
        now.checked_sub(started).is_some_and(|d| d >= self.delay)
    }
}

/// An unsigned judgment: everything a ruler is but its signer. It decides
/// nothing until sealed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Draft {
    region: Region,
    question: Question,
    wald: Wald,
    clock: Clock,
}

impl Draft {
    /// The judgment.
    pub fn new(region: Region, question: Question, wald: Wald, clock: Clock) -> Self {
        Draft {
            region,
            question,
            wald,
            clock,
        }
    }

    /// Its address: `"ruler\0" ‖ region ‖ question ‖ α ‖ β ‖ p₀ ‖ p₁ ‖ clock`
    /// in the public scope, every rational sign-tagged and length-prefixed.
    pub fn oid(&self) -> Oid {
        let mut bytes = b"ruler\0".to_vec();
        for edge in [&self.region.lo, &self.region.hi] {
            match edge {
                Edge::Open(a) => {
                    bytes.push(1);
                    rational(&mut bytes, a);
                }
                Edge::Closed(a) => {
                    bytes.push(2);
                    rational(&mut bytes, a);
                }
                Edge::Unbounded => bytes.push(3),
            }
        }
        bytes.push(self.question.tag());
        let w = &self.wald;
        for r in [&w.alpha, &w.beta, &w.p0, &w.p1] {
            rational(&mut bytes, r);
        }
        let c = &self.clock;
        for t in [c.reaction, c.importance, c.delay] {
            bytes.extend_from_slice(&t.to_le_bytes());
        }
        Oid::hash(&Scope::public(), &bytes)
    }

    /// Seal the judgment as `by`'s: the only way to a [`Ruler`].
    pub fn seal(self, by: Oid) -> Ruler {
        let over = self.oid();
        Ruler {
            draft: self,
            signer: Signer { by, over },
        }
    }
}

/// Who signed a ruler, and the ruler address they signed. Minted only by
/// [`Draft::seal`]. It sits beside the address, never in it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signer {
    by: Oid,
    over: Oid,
}

impl Signer {
    /// The signer's identity, as the caller vouched for it.
    pub fn by(&self) -> &Oid {
        &self.by
    }

    /// The ruler address signed.
    pub fn over(&self) -> &Oid {
        &self.over
    }
}

/// A signed ruler: a [`Draft`] sealed by its [`Signer`]. No default, no
/// public constructor.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ruler {
    draft: Draft,
    signer: Signer,
}

impl Ruler {
    /// Its address, the draft's: the signer is beside it.
    pub fn oid(&self) -> Oid {
        self.signer.over.clone()
    }

    /// The signer.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The region.
    pub fn region(&self) -> &Region {
        &self.draft.region
    }

    /// The question.
    pub fn question(&self) -> Question {
        self.draft.question
    }

    /// The sequential test.
    pub fn wald(&self) -> &Wald {
        &self.draft.wald
    }

    /// The clock.
    pub fn clock(&self) -> &Clock {
        &self.draft.clock
    }

    /// Read a neighbourhood under the ruler's question, then cross its region.
    ///
    /// - `Any`: `In` if some reading crosses `In`, `Out` if all cross `Out`
    ///   (none, too: nothing is broken), otherwise `Undecided`. Its arms
    ///   carry the hull of the readings.
    /// - `Max`, `Sum`, `Product`: the interval aggregate, crossed. The empty
    ///   neighbourhood reads the semiring's unit: 0, 0 and 1.
    pub fn read(&self, neighbourhood: &[Enclosure]) -> Crossing {
        let region = self.region();
        match self.question() {
            Question::Any => {
                let hull = hull(neighbourhood);
                let crossings: Vec<Decision> = neighbourhood
                    .iter()
                    .map(|e| Decision::of(&cross(e, region)))
                    .collect();
                if crossings.contains(&Decision::In) {
                    Membership::In(hull)
                } else if crossings.iter().all(|d| *d == Decision::Out) {
                    Membership::Out(hull)
                } else {
                    Membership::Undecided(hull)
                }
            }
            Question::Max => cross(&fold(neighbourhood, zero(), max), region),
            Question::Sum => cross(&fold(neighbourhood, zero(), sum), region),
            Question::Product => cross(
                &fold(neighbourhood, Enclosure::exact(BigRational::one()), product),
                region,
            ),
        }
    }
}

fn zero() -> Enclosure {
    Enclosure::exact(BigRational::zero())
}

fn fold(
    readings: &[Enclosure],
    unit: Enclosure,
    op: fn(&Enclosure, &Enclosure) -> Enclosure,
) -> Enclosure {
    let mut it = readings.iter();
    match it.next() {
        None => unit,
        Some(first) => it.fold(first.clone(), |acc, e| op(&acc, e)),
    }
}

fn max(a: &Enclosure, b: &Enclosure) -> Enclosure {
    Enclosure {
        lo: a.lo.clone().max(b.lo.clone()),
        hi: a.hi.clone().max(b.hi.clone()),
    }
}

fn sum(a: &Enclosure, b: &Enclosure) -> Enclosure {
    Enclosure {
        lo: &a.lo + &b.lo,
        hi: &a.hi + &b.hi,
    }
}

fn product(a: &Enclosure, b: &Enclosure) -> Enclosure {
    let ends = [&a.lo * &b.lo, &a.lo * &b.hi, &a.hi * &b.lo, &a.hi * &b.hi];
    let lo = ends.iter().min().expect("four").clone();
    let hi = ends.iter().max().expect("four").clone();
    Enclosure { lo, hi }
}

/// The smallest enclosure holding every reading; `[0, 0]` for none.
fn hull(readings: &[Enclosure]) -> Enclosure {
    let lo = readings.iter().map(|e| &e.lo).min();
    let hi = readings.iter().map(|e| &e.hi).max();
    match (lo, hi) {
        (Some(lo), Some(hi)) => Enclosure {
            lo: lo.clone(),
            hi: hi.clone(),
        },
        _ => zero(),
    }
}

// --- T2: evidence and persistence --------------------------------------------

/// What recording a decision did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// A context not seen before under this ruler: it counts.
    New,
    /// The same context again: logged by the caller, never counted twice.
    Repeat,
}

/// One context read two ways under one ruler. A deterministic instrument
/// over the same bytes reads the same, so one of the two is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contradiction {
    /// The neighbourhood.
    pub nbhd: Oid,
    /// The instrument.
    pub instrument: Oid,
    /// The decision recorded first.
    pub kept: Decision,
    /// The decision refused.
    pub refused: Decision,
}

/// The decisions at one site, one per `(ruler, nbhd, instrument)`: a
/// grow-only map, so a context counts once however often it is walked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    site: Oid,
    decisions: BTreeMap<(Oid, Oid, Oid), Decision>,
}

impl Evidence {
    /// No evidence yet at `site`.
    pub fn at(site: Oid) -> Self {
        Evidence {
            site,
            decisions: BTreeMap::new(),
        }
    }

    /// The site.
    pub fn site(&self) -> &Oid {
        &self.site
    }

    /// Record `decision`, read in `nbhd` by `instrument` under `ruler`. A
    /// context already recorded is a [`Recorded::Repeat`], or a
    /// [`Contradiction`] if it decided otherwise.
    pub fn record(
        &mut self,
        nbhd: Oid,
        instrument: Oid,
        ruler: &Ruler,
        decision: Decision,
    ) -> Result<Recorded, Contradiction> {
        let key = (ruler.oid(), nbhd, instrument);
        match self.decisions.get(&key) {
            None => {
                self.decisions.insert(key, decision);
                Ok(Recorded::New)
            }
            Some(&kept) if kept == decision => Ok(Recorded::Repeat),
            Some(&kept) => Err(Contradiction {
                nbhd: key.1,
                instrument: key.2,
                kept,
                refused: decision,
            }),
        }
    }

    /// How many distinct contexts were read under `ruler`, decided or not.
    pub fn contexts(&self, ruler: &Ruler) -> usize {
        self.under(ruler).count()
    }

    /// The decisions under `ruler`.
    fn under<'a>(&'a self, ruler: &Ruler) -> impl Iterator<Item = Decision> + 'a {
        let oid = ruler.oid();
        self.decisions
            .iter()
            .filter(move |((r, _, _), _)| *r == oid)
            .map(|(_, d)| *d)
    }

    /// The address of every recorded decision ([`decision_oid`]).
    pub fn oids(&self) -> Vec<Oid> {
        self.decisions
            .iter()
            .map(|((r, n, i), d)| decision_oid(&self.site, n, i, r, *d))
            .collect()
    }
}

/// Wald's three decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Persistence {
    /// The tension is real: stay, and offer.
    Accept,
    /// It isn't: drop the pending offer, keep the visits.
    Reject,
    /// Not yet: walk on and collect contexts.
    Continue,
}

/// **T2.** Wald's sequential test over the distinct contexts decided under
/// `ruler`. With `k` contexts certified `In` and `m` certified `Out`, the
/// likelihood ratio is `Λ = (p₁/p₀)^k · ((1−p₁)/(1−p₀))^m`, exact. `Accept`
/// when `Λ ≥ (1−β)/α`, `Reject` when `Λ ≤ β/(1−α)`, otherwise `Continue`.
/// `Undecided` contexts are not observations: they don't enter Λ.
///
/// The boundaries are Wald's approximations, which Mara marks READING
/// (secondary sources only).
pub fn persist(evidence: &Evidence, ruler: &Ruler) -> Persistence {
    let w = ruler.wald();
    let one = BigRational::one();
    let (mut k, mut m) = (0i32, 0i32);
    for d in evidence.under(ruler) {
        match d {
            Decision::In => k += 1,
            Decision::Out => m += 1,
            Decision::Undecided => {}
        }
    }
    let ratio = (&w.p1 / &w.p0).pow(k) * ((&one - &w.p1) / (&one - &w.p0)).pow(m);
    if ratio >= (&one - &w.beta) / &w.alpha {
        Persistence::Accept
    } else if ratio <= &w.beta / (&one - &w.alpha) {
        Persistence::Reject
    } else {
        Persistence::Continue
    }
}

// --- Hashing ------------------------------------------------------------------

/// The address of a decision (§5.1 a): `"decision\0" ‖ site ‖ nbhd ‖
/// instrument ‖ ruler ‖ tag` in the public scope, each address
/// length-prefixed (`u64le`). The enclosure is not in it.
pub fn decision_oid(
    site: &Oid,
    nbhd: &Oid,
    instrument: &Oid,
    ruler: &Oid,
    decision: Decision,
) -> Oid {
    let mut bytes = b"decision\0".to_vec();
    for a in [site, nbhd, instrument, ruler] {
        delimited(&mut bytes, a.as_str().as_bytes());
    }
    bytes.push(decision.tag());
    Oid::hash(&Scope::public(), &bytes)
}

fn delimited(bytes: &mut Vec<u8>, field: &[u8]) {
    bytes.extend_from_slice(&(field.len() as u64).to_le_bytes());
    bytes.extend_from_slice(field);
}

/// `sign ‖ |numerator| ‖ denominator`, the sign a byte, each magnitude
/// length-prefixed little-endian. Reduced, so one rational has one encoding.
fn rational(bytes: &mut Vec<u8>, r: &BigRational) {
    let (sign, numer) = r.numer().to_bytes_le();
    bytes.push(match sign {
        Sign::Minus => 2,
        Sign::NoSign => 0,
        Sign::Plus => 1,
    });
    delimited(bytes, &numer);
    delimited(bytes, &BigInt::to_bytes_le(r.denom()).1);
}

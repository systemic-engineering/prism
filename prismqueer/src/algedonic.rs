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
//! - **T2, real: [`persist`]**`(evidence, ruler, calibration)`. Wald's
//!   sequential test over **distinct contexts**, deciding `Accept`, `Reject`
//!   or `Continue`, with the ruler's (α, β) and the instrument's
//!   [`Calibration`] (p₀, p₁).
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
//! A [`Ruler`] is `{ region, question, (α, β), clock, signer }`.
//! It has no default and no public constructor: a [`Draft`] holds the
//! judgment, and only [`Draft::seal`] turns it into a ruler, minting the
//! [`Signer`]. The signer sits **beside** the ruler's address
//! ([`Ruler::oid`] hashes the draft, never the signer), so one judgment
//! signed by two people has one address and two signers.
//!
//! Its address is the canonical one, a law for the whole stack
//! (`docs/spec/ruler.md`, prism-Reed's lean, Alex's call):
//! `"ruler\0" ‖ region ‖ question ‖ α ‖ β ‖ clock` in the public scope.
//!
//! # The calibration is a measurement, beside the ruler
//!
//! p₀ and p₁ (the chance a context certifies `In` when the tension isn't
//! real, and when it is) are the instrument's operating characteristic: they
//! are measured, not judged. They live in a [`Calibration`], a crystal with
//! its own address, never in the ruler's. Recalibrating an instrument never
//! moves a ruler, and every [`Verdict`] names the calibration it read.
//!
//! **Every value carries its domain of validity** (Alex, 2026-11-16). A
//! calibration holds for the [`Region`] it was measured in, and that region
//! is in its address. [`persist`] refuses a calibration for another region
//! than the ruler's ([`Refused::Region`]); it never reads it silently.
//!
//! # One verdict per instrument, unless independence is signed
//!
//! A calibration speaks for its own instrument's decisions only. Two
//! instruments' evidence combines only under an [`Independence`]: a signed
//! judgment that their errors are independent in a region, sealed like a
//! ruler, with no default. Under it, [`persist_jointly`] multiplies the
//! instruments' likelihood ratios, and its verdict names every calibration
//! and the relation.
//!
//! There is no independence by default:
//!
//! ```compile_fail,E0599
//! use prismqueer::algedonic::Independence;
//! let _ = Independence::default();
//! ```
//!
//! The control: sealing a judgment is the only way to one, and it combines.
//!
//! ```
//! use num_bigint::BigInt;
//! use num_rational::BigRational;
//! use prismqueer::algedonic::*;
//! use prismqueer::oid::Oid;
//! let q = |n: i64, d: i64| BigRational::new(BigInt::from(n), BigInt::from(d));
//! let above = || Region::above(q(1, 1));
//! let ruler = Draft::new(above(), Question::Max, Wald::new(q(1, 20), q(1, 20)).unwrap(), Clock::agreed(10, 1, 100))
//!     .seal(Oid::new("engineer"));
//! let knee = Calibration::new(Oid::new("knee"), above(), q(1, 6), q(1, 2), "labelled").unwrap();
//! let ear = Calibration::new(Oid::new("ear"), above(), q(1, 4), q(3, 4), "labelled").unwrap();
//! let both = IndependenceDraft::new([Oid::new("knee"), Oid::new("ear")], above())
//!     .unwrap()
//!     .seal(Oid::new("engineer"));
//! let verdict = persist_jointly(&Evidence::at(Oid::new("leg")), &ruler, &[knee, ear], &both).unwrap();
//! assert_eq!(verdict.persistence(), Persistence::Continue);
//! assert_eq!(verdict.independence(), Some(&both.oid()));
//! ```
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
//! use prismqueer::algedonic::{persist, Calibration, Clock, Draft, Evidence, Question, Region, Wald};
//! use prismqueer::oid::Oid;
//! let q = |n: i64, d: i64| BigRational::new(BigInt::from(n), BigInt::from(d));
//! let wald = Wald::new(q(1, 20), q(1, 20)).unwrap();
//! let knee = Calibration::new(Oid::new("knee"), Region::above(q(1, 1)), q(1, 6), q(1, 2), "labelled").unwrap();
//! let draft = Draft::new(Region::above(q(1, 1)), Question::Max, wald, Clock::agreed(10, 1, 100));
//! let _ = persist(&Evidence::at(Oid::new("leg")), &draft, &knee);
//! ```
//!
//! The control: sealing is the only way to a ruler, and it persists.
//!
//! ```
//! use num_bigint::BigInt;
//! use num_rational::BigRational;
//! use prismqueer::algedonic::{persist, Calibration, Clock, Draft, Evidence, Persistence, Question, Region, Wald};
//! use prismqueer::oid::Oid;
//! let q = |n: i64, d: i64| BigRational::new(BigInt::from(n), BigInt::from(d));
//! let wald = Wald::new(q(1, 20), q(1, 20)).unwrap();
//! let knee = Calibration::new(Oid::new("knee"), Region::above(q(1, 1)), q(1, 6), q(1, 2), "labelled").unwrap();
//! let draft = Draft::new(Region::above(q(1, 1)), Question::Max, wald, Clock::agreed(10, 1, 100));
//! let ruler = draft.seal(Oid::new("engineer"));
//! assert_eq!(ruler.signer().over(), &ruler.oid());
//! let verdict = persist(&Evidence::at(Oid::new("leg")), &ruler, &knee).unwrap();
//! assert_eq!(verdict.persistence(), Persistence::Continue);
//! assert_eq!((verdict.ruler(), verdict.calibrations()), (&ruler.oid(), &[knee.oid()][..]));
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
/// (a false alarm, a miss), the signer's costs. The hypotheses' p₀/p₁ are
/// the instrument's, in its [`Calibration`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Wald {
    alpha: BigRational,
    beta: BigRational,
}

/// The rates don't make a test: each must lie strictly between 0 and 1, and
/// `α + β < 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotATest;

/// Strictly between 0 and 1.
fn unit(x: &BigRational) -> bool {
    x.is_positive() && x < &BigRational::one()
}

impl Wald {
    /// The test. Refused unless `0 < α, β < 1` and `α + β < 1`.
    pub fn new(alpha: BigRational, beta: BigRational) -> Result<Self, NotATest> {
        if !(unit(&alpha) && unit(&beta) && &alpha + &beta < BigRational::one()) {
            return Err(NotATest);
        }
        Ok(Wald { alpha, beta })
    }

    /// α, the false-alarm rate.
    pub fn alpha(&self) -> &BigRational {
        &self.alpha
    }

    /// β, the miss rate.
    pub fn beta(&self) -> &BigRational {
        &self.beta
    }
}

/// An instrument's operating characteristic (its d′) in a region: `p₀`,
/// the chance a context certifies `In` when the tension isn't real, and
/// `p₁` when it is, sealed with the method that measured them. A
/// measurement, so a crystal of its own: its address is never folded into a
/// ruler's (`docs/spec/ruler.md`). It holds in the region it was measured
/// for, and nowhere else.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Calibration {
    instrument: Oid,
    region: Region,
    p0: BigRational,
    p1: BigRational,
    method: String,
}

/// The rates don't calibrate: `0 < p₀ < p₁ < 1` fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotACalibration;

impl Calibration {
    /// `instrument`'s calibration for `region`, measured by `method`.
    /// Refused unless `0 < p₀ < p₁ < 1`.
    pub fn new(
        instrument: Oid,
        region: Region,
        p0: BigRational,
        p1: BigRational,
        method: impl Into<String>,
    ) -> Result<Self, NotACalibration> {
        if !(unit(&p0) && unit(&p1) && p0 < p1) {
            return Err(NotACalibration);
        }
        Ok(Calibration {
            instrument,
            region,
            p0,
            p1,
            method: method.into(),
        })
    }

    /// The instrument calibrated.
    pub fn instrument(&self) -> &Oid {
        &self.instrument
    }

    /// The region it was calibrated for: where it holds.
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// p₀: a context certifies `In` though the tension isn't real.
    pub fn p0(&self) -> &BigRational {
        &self.p0
    }

    /// p₁: a context certifies `In` and the tension is real.
    pub fn p1(&self) -> &BigRational {
        &self.p1
    }

    /// How p₀ and p₁ were measured.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// The bytes addressed: `"calibration\0" ‖ bytes(instrument) ‖ region ‖
    /// p₀ ‖ p₁ ‖ bytes(method)` (`docs/spec/ruler.md`). Its context, the
    /// instrument and the region, is part of its address.
    pub fn preimage(&self) -> Vec<u8> {
        let mut bytes = b"calibration\0".to_vec();
        delimited(&mut bytes, self.instrument.as_str().as_bytes());
        region(&mut bytes, &self.region);
        rational(&mut bytes, &self.p0);
        rational(&mut bytes, &self.p1);
        delimited(&mut bytes, self.method.as_bytes());
        bytes
    }

    /// Its own address, in the public scope.
    pub fn oid(&self) -> Oid {
        Oid::hash(&Scope::public(), &self.preimage())
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

    /// The canonical ruler preimage (`docs/spec/ruler.md`): `"ruler\0" ‖
    /// region ‖ question ‖ α ‖ β ‖ clock`, every field self-delimiting. The
    /// signer and the calibration are never in it.
    pub fn preimage(&self) -> Vec<u8> {
        let mut bytes = b"ruler\0".to_vec();
        region(&mut bytes, &self.region);
        bytes.push(self.question.tag());
        let w = &self.wald;
        for r in [&w.alpha, &w.beta] {
            rational(&mut bytes, r);
        }
        let c = &self.clock;
        for t in [c.reaction, c.importance, c.delay] {
            bytes.extend_from_slice(&t.to_le_bytes());
        }
        bytes
    }

    /// Its address: the preimage, in the public scope.
    pub fn oid(&self) -> Oid {
        Oid::hash(&Scope::public(), &self.preimage())
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

/// Who signed a judgment (a ruler or an independence relation), and the
/// address they signed. Minted only by [`Draft::seal`] and
/// [`IndependenceDraft::seal`]. It sits beside the address, never in it.
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

    /// The address signed.
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

// --- Independence: the only way to combine instruments ---------------------

/// An unsigned judgment that some instruments' errors are independent in a
/// region: two or more distinct instruments, kept sorted. It combines
/// nothing until sealed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndependenceDraft {
    instruments: Vec<Oid>,
    region: Region,
}

/// Not an independence judgment: fewer than two instruments, or one
/// instrument named twice (refused, never collapsed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotIndependent;

impl IndependenceDraft {
    /// `instruments` judged independent in `region`. Refused unless they are
    /// two or more and distinct.
    pub fn new(
        instruments: impl IntoIterator<Item = Oid>,
        region: Region,
    ) -> Result<Self, NotIndependent> {
        let mut instruments: Vec<Oid> = instruments.into_iter().collect();
        let given = instruments.len();
        instruments.sort();
        instruments.dedup();
        if instruments.len() < 2 || instruments.len() != given {
            return Err(NotIndependent);
        }
        Ok(IndependenceDraft {
            instruments,
            region,
        })
    }

    /// The instruments, sorted.
    pub fn instruments(&self) -> &[Oid] {
        &self.instruments
    }

    /// The region the judgment holds in.
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// `"independence\0" ‖ u64le(n) ‖ bytes(instrument)… ‖ region`, the
    /// instruments sorted by their UTF-8 bytes (`docs/spec/ruler.md`). The
    /// signer is never in it.
    pub fn preimage(&self) -> Vec<u8> {
        let mut bytes = b"independence\0".to_vec();
        bytes.extend_from_slice(&(self.instruments.len() as u64).to_le_bytes());
        for i in &self.instruments {
            delimited(&mut bytes, i.as_str().as_bytes());
        }
        region(&mut bytes, &self.region);
        bytes
    }

    /// Its address: the preimage, in the public scope.
    pub fn oid(&self) -> Oid {
        Oid::hash(&Scope::public(), &self.preimage())
    }

    /// Seal the judgment as `by`'s: the only way to an [`Independence`].
    pub fn seal(self, by: Oid) -> Independence {
        let over = self.oid();
        Independence {
            draft: self,
            signer: Signer { by, over },
        }
    }
}

/// A signed independence relation: an [`IndependenceDraft`] sealed by its
/// [`Signer`]. No default, no public constructor. The only way two
/// instruments' evidence combines ([`persist_jointly`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Independence {
    draft: IndependenceDraft,
    signer: Signer,
}

impl Independence {
    /// Its address, the draft's: the signer is beside it.
    pub fn oid(&self) -> Oid {
        self.signer.over.clone()
    }

    /// The signer.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The instruments, sorted.
    pub fn instruments(&self) -> &[Oid] {
        &self.draft.instruments
    }

    /// The region the judgment holds in.
    pub fn region(&self) -> &Region {
        &self.draft.region
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

    /// The decisions `instrument` made under `ruler`.
    fn by<'a>(&'a self, ruler: &Ruler, instrument: &'a Oid) -> impl Iterator<Item = Decision> + 'a {
        let oid = ruler.oid();
        self.decisions
            .iter()
            .filter(move |((r, _, i), _)| *r == oid && i == instrument)
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

/// A persistence verdict, with every address it depended on side by side:
/// the ruler's, each calibration's, and the independence relation's when
/// more than one instrument was read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    persistence: Persistence,
    ruler: Oid,
    calibrations: Vec<Oid>,
    independence: Option<Oid>,
}

impl Verdict {
    /// Wald's decision.
    pub fn persistence(&self) -> Persistence {
        self.persistence
    }

    /// The ruler judged under.
    pub fn ruler(&self) -> &Oid {
        &self.ruler
    }

    /// The calibrations read, in instrument order: one for [`persist`].
    pub fn calibrations(&self) -> &[Oid] {
        &self.calibrations
    }

    /// The independence relation the instruments combined under; `None`
    /// for one instrument, which needs none.
    pub fn independence(&self) -> Option<&Oid> {
        self.independence.as_ref()
    }
}

/// Why persistence refused to read: never a silent use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// A calibration, or an independence relation, holds in another region
    /// than the ruler judges.
    Region {
        /// The calibration's or the relation's address.
        of: Oid,
        /// The region it holds in.
        holds: Box<Region>,
        /// The ruler's address.
        ruler: Oid,
        /// The ruler's region.
        ruled: Box<Region>,
    },
    /// The calibrations' instruments aren't exactly the relation's, each
    /// once.
    Instruments {
        /// The relation's address.
        independence: Oid,
        /// The instruments it declares, sorted.
        declared: Vec<Oid>,
        /// The calibrations' instruments, sorted, repeats kept.
        calibrated: Vec<Oid>,
    },
}

/// `of` holds in `holds`; refused unless that is `ruler`'s region.
fn holds(of: Oid, holds: &Region, ruler: &Ruler) -> Result<(), Refused> {
    if holds == ruler.region() {
        return Ok(());
    }
    Err(Refused::Region {
        of,
        holds: Box::new(holds.clone()),
        ruler: ruler.oid(),
        ruled: Box::new(ruler.region().clone()),
    })
}

/// `calibration`'s likelihood ratio over its instrument's decisions under
/// `ruler`: `(p₁/p₀)^k · ((1−p₁)/(1−p₀))^m`, exact.
fn likelihood(evidence: &Evidence, ruler: &Ruler, c: &Calibration) -> BigRational {
    let one = BigRational::one();
    let (mut k, mut m) = (0i32, 0i32);
    for d in evidence.by(ruler, &c.instrument) {
        match d {
            Decision::In => k += 1,
            Decision::Out => m += 1,
            Decision::Undecided => {}
        }
    }
    (&c.p1 / &c.p0).pow(k) * ((&one - &c.p1) / (&one - &c.p0)).pow(m)
}

/// Wald's decision for `ratio` against `wald`'s boundaries.
fn decide(ratio: &BigRational, w: &Wald) -> Persistence {
    let one = BigRational::one();
    if ratio >= &((&one - &w.beta) / &w.alpha) {
        Persistence::Accept
    } else if ratio <= &(&w.beta / (&one - &w.alpha)) {
        Persistence::Reject
    } else {
        Persistence::Continue
    }
}

/// **T2.** Wald's sequential test over the distinct contexts that
/// `calibration`'s instrument decided under `ruler`. With `k` contexts
/// certified `In` and `m` certified `Out`, the likelihood ratio is
/// `Λ = (p₁/p₀)^k · ((1−p₁)/(1−p₀))^m`, exact. `Accept` when
/// `Λ ≥ (1−β)/α`, `Reject` when `Λ ≤ β/(1−α)`, otherwise `Continue`.
/// `Undecided` contexts are not observations: they don't enter Λ. Another
/// instrument's decisions don't either: this calibration doesn't speak for
/// them ([`persist_jointly`] combines instruments).
///
/// A calibration for another region than the ruler's is
/// [`Refused::Region`]: a measurement holds where it was taken.
///
/// The boundaries are Wald's approximations, which Mara marks READING
/// (secondary sources only).
pub fn persist(
    evidence: &Evidence,
    ruler: &Ruler,
    calibration: &Calibration,
) -> Result<Verdict, Refused> {
    holds(calibration.oid(), &calibration.region, ruler)?;
    Ok(Verdict {
        persistence: decide(&likelihood(evidence, ruler, calibration), ruler.wald()),
        ruler: ruler.oid(),
        calibrations: vec![calibration.oid()],
        independence: None,
    })
}

/// **T2 across instruments.** Wald's test over several instruments'
/// decisions under `ruler`, combined by the signed `independence`: the
/// likelihood ratio is the product of each calibration's own
/// ([`persist`]'s Λ per instrument), against the ruler's boundaries.
///
/// Refused ([`Refused`]) when a calibration's region or the relation's
/// isn't the ruler's, or when the calibrations' instruments aren't exactly
/// the relation's, each once. The verdict names every calibration, in
/// instrument order, and the relation.
pub fn persist_jointly(
    evidence: &Evidence,
    ruler: &Ruler,
    calibrations: &[Calibration],
    independence: &Independence,
) -> Result<Verdict, Refused> {
    holds(independence.oid(), independence.region(), ruler)?;
    let mut sorted: Vec<&Calibration> = calibrations.iter().collect();
    sorted.sort_by(|a, b| a.instrument.cmp(&b.instrument));
    let calibrated: Vec<Oid> = sorted.iter().map(|c| c.instrument.clone()).collect();
    if calibrated != independence.instruments() {
        return Err(Refused::Instruments {
            independence: independence.oid(),
            declared: independence.instruments().to_vec(),
            calibrated,
        });
    }
    let mut ratio = BigRational::one();
    for c in &sorted {
        holds(c.oid(), &c.region, ruler)?;
        ratio *= likelihood(evidence, ruler, c);
    }
    Ok(Verdict {
        persistence: decide(&ratio, ruler.wald()),
        ruler: ruler.oid(),
        calibrations: sorted.iter().map(|c| c.oid()).collect(),
        independence: Some(independence.oid()),
    })
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

/// `edge(lo) ‖ edge(hi)`, each edge a tag (01 open, 02 closed, 03
/// unbounded) and, when bounded, its rational (`docs/spec/ruler.md`).
fn region(bytes: &mut Vec<u8>, region: &Region) {
    for edge in [&region.lo, &region.hi] {
        match edge {
            Edge::Open(a) => {
                bytes.push(1);
                rational(bytes, a);
            }
            Edge::Closed(a) => {
                bytes.push(2);
                rational(bytes, a);
            }
            Edge::Unbounded => bytes.push(3),
        }
    }
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

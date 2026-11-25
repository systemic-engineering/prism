//! `prism_laws::flip`: when did the verdict change (`docs/spec/flip.md`;
//! prism-024 §2; Mara, mirror-029 II.5 and mirror `mara/bridge-grade`
//! 3333dd80 §5). The names are provisional; Alex names them.
//!
//! A sequence of facts is the sequencer's order (chorus checkpoints in
//! production, a slice here); v(k) is the verdict on the first k facts.
//!
//! - **Sequenced, monotone** ([`flip_point`], [`bisect`]): two thresholds in
//!   K3 (≥ maybe, = true), each found by bisection in at most
//!   [`bound`]`(n)` = ⌈log₂ n⌉ + 2 evaluations, as the checkpoint pair
//!   (k−1, k) and the fact at k.
//! - **Unsequenced** ([`basis`]): the minimal witness basis, an antichain,
//!   under a declared [`Budget`]; over budget is a typed [`Basis::Partial`].
//!   Mara's identity: the threshold's k = [`earliest`] of the basis.
//! - **Any verdict, negation included** ([`scan`]): every flip
//!   (k−1, k, from, to), in n + 1 evaluations.
//!
//! **The type rule.** Bisection is sound only for a monotone verdict, so it
//! takes a [`Monotone`], which only a [`navigation::Program`](Program) can
//! make. A closure gets the scan only; handing one to `flip_point` doesn't
//! compile:
//!
//! ```compile_fail,E0308
//! use prism_laws::flip::flip_point;
//! use prism_laws::navigation::{Rule, K3};
//! let facts: Vec<Rule<&str, &str>> = vec![Rule::fact("a"), Rule::fact("b")];
//! let negated = |prefix: &[Rule<&str, &str>]| {
//!     if prefix.len() > 1 { K3::False } else { K3::True }
//! };
//! flip_point(&negated, &facts);
//! ```
//!
//! The control, which compiles: a program, made monotone by construction,
//! is bisected; the closure is scanned.
//!
//! ```
//! use prism_laws::flip::{flip_point, scan, Crossing, Monotone};
//! use prism_laws::navigation::{Body, Program, Rule, K3};
//! let mut program: Program<&str, &str> = Program::new();
//! program.rule(Rule::new("q", [Body::Atom("b")]));
//! let m = Monotone::new(program, "q");
//! let facts = vec![Rule::fact("a"), Rule::fact("b")];
//! assert_eq!(flip_point(&m, &facts).truth.crossing.position(), Some(2));
//! let negated = |prefix: &[Rule<&str, &str>]| {
//!     if prefix.len() > 1 { K3::False } else { K3::True }
//! };
//! assert_eq!(scan(&facts, negated).flips.len(), 1);
//! ```

use std::collections::BTreeSet;

use crate::navigation::{Program, Rule, K3};

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

/// One of K3's two thresholds over prefixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Threshold {
    /// v ≥ maybe.
    Maybe,
    /// v = true.
    True,
}

impl Threshold {
    /// Both, lower first.
    pub const BOTH: [Threshold; 2] = [Threshold::Maybe, Threshold::True];

    /// Whether `v` is at or above this threshold.
    pub fn reached(self, v: K3) -> bool {
        match self {
            Threshold::Maybe => v >= K3::Maybe,
            Threshold::True => v == K3::True,
        }
    }
}

// ---------------------------------------------------------------------------
// The monotone verdict
// ---------------------------------------------------------------------------

/// A verdict that is monotone over prefixes **by construction**: a positive
/// K3 program and a target atom, read over facts that are positive rules
/// added to it. Only [`Monotone::new`] makes one, from a [`Program`], so
/// bisection and the witness basis can't be handed anything else.
#[derive(Clone, Debug, PartialEq)]
pub struct Monotone<A, C> {
    program: Program<A, C>,
    target: A,
}

impl<A: Ord + Clone, C: Ord + Clone> Monotone<A, C> {
    /// The verdict on `target` in `program` plus the facts given.
    pub fn new(program: Program<A, C>, target: A) -> Self {
        Monotone { program, target }
    }

    /// The target atom.
    pub fn target(&self) -> &A {
        &self.target
    }

    /// The verdict with `facts` added: the target's value at the least
    /// fixed point, `false` if no rule names it. The facts' order doesn't
    /// matter (the fixed point is order-free).
    pub fn verdict<'r>(&self, facts: impl IntoIterator<Item = &'r Rule<A, C>>) -> K3
    where
        A: 'r,
        C: 'r,
    {
        let mut program = self.program.clone();
        for fact in facts {
            program.rule(fact.clone());
        }
        program.solve().value(&self.target).unwrap_or(K3::False)
    }
}

// ---------------------------------------------------------------------------
// Case 1: bisection
// ---------------------------------------------------------------------------

/// Where a threshold is first reached over the prefixes.
#[derive(Clone, Debug, PartialEq)]
pub enum Crossing<'f, F> {
    /// Not reached even on every fact.
    Never,
    /// Already reached on the empty prefix: nothing flipped.
    Initially,
    /// Reached first at checkpoint `at` (1-based), not at `before` = at − 1;
    /// `fact` is the fact at position `at`.
    At {
        /// Checkpoint k − 1.
        before: usize,
        /// Checkpoint k.
        at: usize,
        /// The fact at k.
        fact: &'f F,
    },
}

impl<F> Crossing<'_, F> {
    /// The first k reaching the threshold: `None` for never, `0` for
    /// initially.
    pub fn position(&self) -> Option<usize> {
        match self {
            Crossing::Never => None,
            Crossing::Initially => Some(0),
            Crossing::At { at, .. } => Some(*at),
        }
    }
}

/// A threshold's crossing, and the evaluations of v it took.
#[derive(Clone, Debug, PartialEq)]
pub struct Bisected<'f, F> {
    /// Where the threshold is first reached.
    pub crossing: Crossing<'f, F>,
    /// Evaluations of v: at most [`bound`]`(n)`.
    pub evaluations: usize,
}

/// Both thresholds' crossings.
#[derive(Clone, Debug, PartialEq)]
pub struct FlipPoint<'f, F> {
    /// v ≥ maybe.
    pub maybe: Bisected<'f, F>,
    /// v = true.
    pub truth: Bisected<'f, F>,
}

impl<'f, F> FlipPoint<'f, F> {
    /// The crossing for `threshold`.
    pub fn get(&self, threshold: Threshold) -> &Bisected<'f, F> {
        match threshold {
            Threshold::Maybe => &self.maybe,
            Threshold::True => &self.truth,
        }
    }
}

/// ⌈log₂ n⌉ + 2: the most evaluations [`bisect`] makes over n facts (v(n),
/// v(0), then one per halving).
pub fn bound(n: usize) -> usize {
    let log = if n <= 1 {
        0
    } else {
        (usize::BITS - (n - 1).leading_zeros()) as usize
    };
    log + 2
}

/// One threshold's crossing, by bisection over the prefixes.
pub fn bisect<'f, A: Ord + Clone, C: Ord + Clone>(
    verdict: &Monotone<A, C>,
    facts: &'f [Rule<A, C>],
    threshold: Threshold,
) -> Bisected<'f, Rule<A, C>> {
    let n = facts.len();
    let mut evaluations = 0;
    let mut reached = |k: usize| {
        evaluations += 1;
        threshold.reached(verdict.verdict(&facts[..k]))
    };
    let crossing = if !reached(n) {
        Crossing::Never
    } else if n == 0 || reached(0) {
        Crossing::Initially
    } else {
        // v(lo) below, v(hi) at: the crossing is in (lo, hi].
        let (mut lo, mut hi) = (0, n);
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if reached(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        Crossing::At {
            before: hi - 1,
            at: hi,
            fact: &facts[hi - 1],
        }
    };
    Bisected {
        crossing,
        evaluations,
    }
}

/// Both thresholds' crossings, each by [`bisect`]: the checkpoint pair
/// (k−1, k) and the fact at k.
pub fn flip_point<'f, A: Ord + Clone, C: Ord + Clone>(
    verdict: &Monotone<A, C>,
    facts: &'f [Rule<A, C>],
) -> FlipPoint<'f, Rule<A, C>> {
    FlipPoint {
        maybe: bisect(verdict, facts, Threshold::Maybe),
        truth: bisect(verdict, facts, Threshold::True),
    }
}

// ---------------------------------------------------------------------------
// Case 3: the scan, for any verdict
// ---------------------------------------------------------------------------

/// A flip between checkpoints `before` = k − 1 and `at` = k.
#[derive(Clone, Debug, PartialEq)]
pub struct Flip<'f, F> {
    /// Checkpoint k − 1.
    pub before: usize,
    /// Checkpoint k.
    pub at: usize,
    /// The fact at k.
    pub fact: &'f F,
    /// v(k − 1).
    pub from: K3,
    /// v(k).
    pub to: K3,
}

/// Every flip over the prefixes.
#[derive(Clone, Debug, PartialEq)]
pub struct Scan<'f, F> {
    /// v(0).
    pub initial: K3,
    /// Every k with v(k − 1) ≠ v(k), in order.
    pub flips: Vec<Flip<'f, F>>,
    /// Evaluations of v: n + 1.
    pub evaluations: usize,
}

impl<F> Scan<'_, F> {
    /// The first k at which v reaches `threshold`: `0` if v(0) does.
    pub fn first_reaching(&self, threshold: Threshold) -> Option<usize> {
        if threshold.reached(self.initial) {
            return Some(0);
        }
        self.flips
            .iter()
            .find(|f| threshold.reached(f.to))
            .map(|f| f.at)
    }
}

/// Every flip of any verdict over prefixes, negation included: v(0) … v(n),
/// each once.
pub fn scan<F>(facts: &[F], mut verdict: impl FnMut(&[F]) -> K3) -> Scan<'_, F> {
    let initial = verdict(&facts[..0]);
    let mut prev = initial;
    let mut flips = Vec::new();
    for k in 1..=facts.len() {
        let v = verdict(&facts[..k]);
        if v != prev {
            flips.push(Flip {
                before: k - 1,
                at: k,
                fact: &facts[k - 1],
                from: prev,
                to: v,
            });
        }
        prev = v;
    }
    Scan {
        initial,
        flips,
        evaluations: facts.len() + 1,
    }
}

// ---------------------------------------------------------------------------
// Case 2: the minimal witness basis
// ---------------------------------------------------------------------------

/// A set of fact positions (1-based).
pub type Witness = BTreeSet<usize>;

/// How many subsets [`basis`] may examine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget(pub usize);

/// The minimal witness basis, or as much of it as the budget reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Basis {
    /// Every ⊆-minimal witness, smallest first: an antichain.
    Complete(Vec<Witness>),
    /// The budget ran out. Every member is minimal, and every witness of
    /// fewer than `complete_below` facts is a member.
    Partial {
        /// The minimal witnesses found.
        members: Vec<Witness>,
        /// Complete for witnesses smaller than this.
        complete_below: usize,
        /// Subsets examined (the whole budget).
        examined: usize,
    },
}

impl Basis {
    /// The members found, complete or not.
    pub fn members(&self) -> &[Witness] {
        match self {
            Basis::Complete(members) | Basis::Partial { members, .. } => members,
        }
    }
}

/// The ⊆-minimal subsets of `facts` on which `verdict` reaches `threshold`,
/// examining subsets by size, smallest first, within `budget`. A subset
/// containing a member is examined (it counts) but not evaluated.
pub fn basis<A: Ord + Clone, C: Ord + Clone>(
    verdict: &Monotone<A, C>,
    facts: &[Rule<A, C>],
    threshold: Threshold,
    budget: Budget,
) -> Basis {
    let n = facts.len();
    let mut members: Vec<Witness> = Vec::new();
    let mut examined = 0;
    for size in 0..=n {
        // The first combination of `size` positions, then lexicographic.
        let mut pick: Vec<usize> = (1..=size).collect();
        loop {
            if examined == budget.0 {
                return Basis::Partial {
                    members,
                    complete_below: size,
                    examined,
                };
            }
            examined += 1;
            let subset: Witness = pick.iter().copied().collect();
            if !members.iter().any(|m| m.is_subset(&subset))
                && threshold.reached(verdict.verdict(subset.iter().map(|&p| &facts[p - 1])))
            {
                members.push(subset);
            }
            if !next_combination(&mut pick, n) {
                break;
            }
        }
    }
    Basis::Complete(members)
}

/// Advance `pick` (strictly increasing positions in 1..=n) to the next
/// combination of its size; false when it was the last.
fn next_combination(pick: &mut [usize], n: usize) -> bool {
    let k = pick.len();
    let Some(i) = (0..k).rev().find(|&i| pick[i] < n - (k - 1 - i)) else {
        return false;
    };
    pick[i] += 1;
    let base = pick[i];
    for (offset, slot) in pick[i + 1..].iter_mut().enumerate() {
        *slot = base + 1 + offset;
    }
    true
}

/// Mara's formula: min over the witnesses of the witness's last position
/// (0 for the empty witness). Over a complete basis it is the threshold's
/// first k.
pub fn earliest(members: &[Witness]) -> Option<usize> {
    members
        .iter()
        .map(|w| w.iter().next_back().copied().unwrap_or(0))
        .min()
}

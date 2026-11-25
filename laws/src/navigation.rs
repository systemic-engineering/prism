//! `prism_laws::navigation`: the K3 floor a navigator reads over
//! (`docs/spec/navigation.md`; prism-025 Addendum item 1; Mara, mirror
//! `mara/witness-navigation-grade` 052e5a13 §2.1–§2.3). The names are
//! provisional; Alex names them. Not "D2": prism's D2 is Hypothesis.
//!
//! - **The algebra is [`K3`]**: {false < maybe < true} with ⊕ = max,
//!   ⊗ = min. A commutative, idempotent, 0-closed semiring (Mara N1).
//! - **The carrier is [`Membership`]**, with set-valued payloads: `In` =
//!   true, `Undecided` = maybe, `Out` = false. "`Membership` is the carrier,
//!   not the algebra": [`plus`] and [`times`] lift K3 onto it, keeping the
//!   union of the payloads that carry the result's value.
//! - **For atoms it is K3-Datalog over hyperedges** ([`Program`]): a rule's
//!   value is the ⊗ over its body, an atom's the ⊕ over its rules, at the
//!   least fixed point.
//! - **An unexplored edge reads as `maybe`, never `false`**
//!   ([`Edge::Unexplored`]). Absence has its own constructor, and it reads
//!   as `Undecided`.
//!
//! ```
//! use prism_laws::navigation::{Body, Edge, K3, Program, Rule};
//! let mut program: Program<&str, &str> = Program::new();
//! program.rule(Rule::fact("a"));
//! program.rule(Rule::new("b", [Body::Atom("a"), Body::Edge(Edge::Unexplored("a->b"))]));
//! let solution = program.solve();
//! assert_eq!(solution.value(&"a"), Some(K3::True));
//! assert_eq!(solution.value(&"b"), Some(K3::Maybe));
//! ```

use std::collections::{BTreeMap, BTreeSet};

pub use crate::algedonic::Membership;

// ---------------------------------------------------------------------------
// The algebra
// ---------------------------------------------------------------------------

/// K3: three values in the truth order, false < maybe < true.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum K3 {
    /// Decided false. The semiring's 0.
    False,
    /// Not decided.
    Maybe,
    /// Decided true. The semiring's 1.
    True,
}

impl K3 {
    /// All three, in the truth order.
    pub const ALL: [K3; 3] = [K3::False, K3::Maybe, K3::True];

    /// 0, the ⊕ identity: false.
    pub const ZERO: K3 = K3::False;

    /// 1, the ⊗ identity: true.
    pub const ONE: K3 = K3::True;

    /// ⊕: max in the truth order.
    pub fn plus(self, other: K3) -> K3 {
        self.max(other)
    }

    /// ⊗: min in the truth order.
    pub fn times(self, other: K3) -> K3 {
        self.min(other)
    }

    /// The Kleene star. K3 is 0-closed (1 ⊕ a = 1), so a* = 1 for every a.
    pub fn star(self) -> K3 {
        K3::ONE
    }

    /// The truth order: `self ≤t other`.
    pub fn leq_t(self, other: K3) -> bool {
        self <= other
    }

    /// The knowledge order: `self ≤k other`. Maybe is below both decided
    /// values; false and true are incomparable.
    pub fn leq_k(self, other: K3) -> bool {
        self == other || self == K3::Maybe
    }

    /// Decided: false or true.
    pub fn is_decided(self) -> bool {
        self != K3::Maybe
    }

    /// The carrier map: `In` ↦ true, `Undecided` ↦ maybe, `Out` ↦ false.
    pub fn of<I, O, U>(membership: &Membership<I, O, U>) -> K3 {
        match membership {
            Membership::In(_) => K3::True,
            Membership::Undecided(_) => K3::Maybe,
            Membership::Out(_) => K3::False,
        }
    }
}

// ---------------------------------------------------------------------------
// The carrier
// ---------------------------------------------------------------------------

/// A label on the carrier: a [`Membership`] whose arms each carry a set of
/// names: certificates for `In`, witnesses (a cut) for `Out`, the frontier a
/// `maybe` rests on for `Undecided`.
pub type Label<C> = Membership<BTreeSet<C>, BTreeSet<C>, BTreeSet<C>>;

fn payload<C>(label: &Label<C>) -> &BTreeSet<C> {
    match label {
        Membership::In(s) | Membership::Out(s) | Membership::Undecided(s) => s,
    }
}

fn label<C>(value: K3, names: BTreeSet<C>) -> Label<C> {
    match value {
        K3::True => Membership::In(names),
        K3::Maybe => Membership::Undecided(names),
        K3::False => Membership::Out(names),
    }
}

/// The value `to` and the union of the payloads of the labels in `of` that
/// carry it.
fn gather<'a, C: Ord + Clone + 'a>(to: K3, of: impl IntoIterator<Item = &'a Label<C>>) -> Label<C> {
    let names = of
        .into_iter()
        .filter(|l| K3::of(*l) == to)
        .flat_map(|l| payload(l).iter().cloned())
        .collect();
    label(to, names)
}

/// Lifted ⊕: the max value, with the union of the payloads that carry it.
pub fn plus<C: Ord + Clone>(a: &Label<C>, b: &Label<C>) -> Label<C> {
    gather(K3::of(a).plus(K3::of(b)), [a, b])
}

/// Lifted ⊗: the min value, with the union of the payloads that carry it.
pub fn times<C: Ord + Clone>(a: &Label<C>, b: &Label<C>) -> Label<C> {
    gather(K3::of(a).times(K3::of(b)), [a, b])
}

// ---------------------------------------------------------------------------
// Edges: absence is its own constructor
// ---------------------------------------------------------------------------

/// An edge label as a navigator holds it: read, or not yet explored.
#[derive(Clone, Debug, PartialEq)]
pub enum Edge<C> {
    /// Read, with its label.
    Read(Label<C>),
    /// Not explored. **Reads as `maybe`, never `false`**, naming itself.
    Unexplored(C),
}

impl<C: Ord + Clone> Edge<C> {
    /// A lookup's result: found is read, nothing found is
    /// [`Unexplored`](Edge::Unexplored) under `name`. Never `Out`.
    pub fn from_lookup(found: Option<Label<C>>, name: C) -> Self {
        match found {
            Some(l) => Edge::Read(l),
            None => Edge::Unexplored(name),
        }
    }

    /// The label this edge reads as: its own, or `Undecided` naming itself.
    pub fn reads(&self) -> Label<C> {
        match self {
            Edge::Read(l) => l.clone(),
            Edge::Unexplored(name) => Membership::Undecided(BTreeSet::from([name.clone()])),
        }
    }
}

// ---------------------------------------------------------------------------
// K3-Datalog over hyperedges
// ---------------------------------------------------------------------------

/// One item of a rule's body: an atom, or an edge label.
#[derive(Clone, Debug, PartialEq)]
pub enum Body<A, C> {
    /// An atom's current value.
    Atom(A),
    /// An edge's label.
    Edge(Edge<C>),
}

/// A ground rule `head :- body₁, …, bodyₖ`: one hyperedge.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule<A, C> {
    head: A,
    body: Vec<Body<A, C>>,
}

impl<A, C> Rule<A, C> {
    /// `head :- body`.
    pub fn new(head: A, body: impl IntoIterator<Item = Body<A, C>>) -> Self {
        Rule {
            head,
            body: body.into_iter().collect(),
        }
    }

    /// `head :-` with an empty body: true, with no certificate.
    pub fn fact(head: A) -> Self {
        Rule {
            head,
            body: Vec::new(),
        }
    }

    /// The head.
    pub fn head(&self) -> &A {
        &self.head
    }

    /// The body.
    pub fn body(&self) -> &[Body<A, C>] {
        &self.body
    }
}

/// A positive K3-Datalog program: ground rules over atoms `A`, edges named
/// by `C`.
#[derive(Clone, Debug, PartialEq)]
pub struct Program<A, C> {
    rules: Vec<Rule<A, C>>,
}

impl<A, C> Default for Program<A, C> {
    fn default() -> Self {
        Program { rules: Vec::new() }
    }
}

impl<A: Ord + Clone, C: Ord + Clone> Program<A, C> {
    /// No rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a rule.
    pub fn rule(&mut self, rule: Rule<A, C>) {
        self.rules.push(rule);
    }

    /// The rules, in the order added.
    pub fn rules(&self) -> &[Rule<A, C>] {
        &self.rules
    }

    /// The least fixed point, rules visited in the order added.
    pub fn solve(&self) -> Solution<A, C> {
        self.solve_by(|_| {})
    }

    /// The least fixed point, with `schedule` choosing the order of each
    /// pass: it gets the rule indices and may permute them (a strategy;
    /// fate chooses order, never truth). Every pass visits every rule, so
    /// every schedule is fair and reaches the same fixed point (Mara N3).
    pub fn solve_by(&self, mut schedule: impl FnMut(&mut [usize])) -> Solution<A, C> {
        // Each body item as it is read: an atom, or the label its edge reads as.
        let bodies: Vec<Vec<Item<A, C>>> = self
            .rules
            .iter()
            .map(|r| {
                r.body
                    .iter()
                    .map(|b| match b {
                        Body::Atom(a) => Item::Atom(a.clone()),
                        Body::Edge(e) => Item::Label(e.reads()),
                    })
                    .collect()
            })
            .collect();

        // Phase 1: the values, from all-false up, in the truth order.
        let mut value: BTreeMap<A, K3> = BTreeMap::new();
        for (r, body) in self.rules.iter().zip(&bodies) {
            value.insert(r.head.clone(), K3::ZERO);
            for it in body {
                if let Item::Atom(a) = it {
                    value.insert(a.clone(), K3::ZERO);
                }
            }
        }
        let read = |value: &BTreeMap<A, K3>, it: &Item<A, C>| match it {
            Item::Atom(a) => value[a],
            Item::Label(l) => K3::of(l),
        };
        let rule_value = |value: &BTreeMap<A, K3>, i: usize| {
            bodies[i]
                .iter()
                .fold(K3::ONE, |acc, it| acc.times(read(value, it)))
        };
        let mut order: Vec<usize> = (0..self.rules.len()).collect();
        loop {
            schedule(&mut order);
            let mut changed = false;
            for &i in &order {
                let head = &self.rules[i].head;
                let next = value[head].plus(rule_value(&value, i));
                if next != value[head] {
                    value.insert(head.clone(), next);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Phase 2: the payloads, the least sets under union given the values.
        let mut names: BTreeMap<A, BTreeSet<C>> = BTreeMap::new();
        loop {
            let mut changed = false;
            for (i, r) in self.rules.iter().enumerate() {
                let v = rule_value(&value, i);
                if v != value[&r.head] {
                    continue;
                }
                let mut gathered: BTreeSet<C> = BTreeSet::new();
                for it in bodies[i].iter().filter(|it| read(&value, it) == v) {
                    match it {
                        Item::Atom(a) => {
                            gathered.extend(names.get(a).into_iter().flatten().cloned())
                        }
                        Item::Label(l) => gathered.extend(payload(l).iter().cloned()),
                    }
                }
                let there = names.entry(r.head.clone()).or_default();
                let before = there.len();
                there.extend(gathered);
                changed |= there.len() != before;
            }
            if !changed {
                break;
            }
        }

        Solution {
            labels: value
                .into_iter()
                .map(|(a, v)| {
                    let n = names.remove(&a).unwrap_or_default();
                    (a, label(v, n))
                })
                .collect(),
        }
    }
}

/// A body item as the fixed point reads it.
enum Item<A, C> {
    Atom(A),
    Label(Label<C>),
}

/// The least fixed point: every atom the program names, with its label.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution<A, C> {
    labels: BTreeMap<A, Label<C>>,
}

impl<A: Ord, C> Solution<A, C> {
    /// `atom`'s label, if the program names it.
    pub fn get(&self, atom: &A) -> Option<&Label<C>> {
        self.labels.get(atom)
    }

    /// `atom`'s K3 value, if the program names it.
    pub fn value(&self, atom: &A) -> Option<K3> {
        self.labels.get(atom).map(K3::of)
    }

    /// Every atom and its label, ordered by atom.
    pub fn iter(&self) -> impl Iterator<Item = (&A, &Label<C>)> {
        self.labels.iter()
    }
}

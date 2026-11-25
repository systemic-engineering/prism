//! `prism_laws::hold`: a decision stands while what it read stands
//! (`docs/spec/hold.md`; prism-024 §1; Mara, mirror-029 I.1 and mirror
//! `mara/bridge-grade` 3333dd80 §1). The names are provisional; Alex names
//! them.
//!
//! *A decision sealed at premise projections `(p₁ … pₖ)` is HOLD exactly
//! when some `pᵢ`'s projection address now differs from the one sealed.
//! Only its owner's seal over the current tuple clears it.*
//!
//! - **The standing half is PROVED** (Mara's Lemma H, a lemma of incremental
//!   soundness, IS §5): equal projection addresses mean the decision's
//!   inputs are unchanged, so its seal still covers what it read. That is
//!   early cutoff.
//! - **The HOLD half is policy** (the authority rule): a decision is a
//!   person's act, so a miss asks its owner to re-seal, never recomputes.
//!   The "iff" over-HOLDs: an address can move while what was acted on
//!   didn't. The [`Projection`] decides by how much.
//!
//! # The projection is one named choice
//!
//! [`StatusAlone`] is the projection every decision uses unless it names
//! another: an output's status, with λ, ρ, the cites and the head beside it.
//! **Mara's lean, flagged for Alex.** Including λ is another [`Projection`]
//! (or an edit of [`StatusAlone`]'s `read`); the law doesn't change. Every
//! projected address names its projection, so two projections never share
//! an address.
//!
//! # Facts never enter a decision
//!
//! A [`DecisionDraft`] takes [`Premise`]s only, and a premise is made only
//! by projecting a derivation's [`Output`]. A fact set is not one:
//!
//! ```compile_fail,E0271
//! use prism_laws::hold::{DecisionDraft, Facts, StatusAlone};
//! use prism_laws::oid::Oid;
//! let mut facts = Facts::new();
//! facts.add(Oid::new("span_ok_0"));
//! let _ = DecisionDraft::<StatusAlone>::new(Oid::new("ship"), [facts]);
//! ```
//!
//! nor its address:
//!
//! ```compile_fail,E0271
//! use prism_laws::hold::{DecisionDraft, Facts, StatusAlone};
//! use prism_laws::oid::Oid;
//! let head = Facts::new().address();
//! let _ = DecisionDraft::<StatusAlone>::new(Oid::new("ship"), [head]);
//! ```
//!
//! and a premise can't be written without projecting an output:
//!
//! ```compile_fail,E0451
//! use prism_laws::hold::Premise;
//! use prism_laws::oid::Oid;
//! let _: Premise = Premise { premise: Oid::new("h1"), status: "true".into(), address: Oid::new("x"), projection: std::marker::PhantomData };
//! ```
//!
//! The control: a derivation's output, projected, seals.
//!
//! ```
//! use prism_laws::hold::{standing, Current, Decision, DecisionDraft, Output, Premise, Standing};
//! use prism_laws::oid::Oid;
//! let out = Output::new(Oid::new("h1"), "unrefuted");
//! let d: Decision = DecisionDraft::new(Oid::new("ship"), [Premise::of(&out)])
//!     .unwrap()
//!     .seal(Oid::new("owner"));
//! assert_eq!(standing(&d, &Current::new([out]).unwrap()), Standing::Stands);
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::marker::PhantomData;

use crate::algedonic::Signer;
use crate::oid::{Oid, Scope};

/// `u64le(len(bytes)) ‖ bytes`: one self-delimiting field, as in
/// `docs/spec/ruler.md`. For a [`Projection`]'s `read`.
pub fn field(preimage: &mut Vec<u8>, bytes: &[u8]) {
    preimage.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    preimage.extend_from_slice(bytes);
}

// ---------------------------------------------------------------------------
// Facts
// ---------------------------------------------------------------------------

/// A fact set: the addresses of admitted facts. It only grows, and its
/// address moves with every fact. It is what a derivation reads, never what
/// a decision reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts(BTreeSet<Oid>);

impl Facts {
    /// The empty set.
    pub fn new() -> Self {
        Facts(BTreeSet::new())
    }

    /// Add a fact; `false` if it was already there.
    pub fn add(&mut self, fact: Oid) -> bool {
        self.0.insert(fact)
    }

    /// Is `fact` in the set?
    pub fn contains(&self, fact: &Oid) -> bool {
        self.0.contains(fact)
    }

    /// The facts, ordered by address.
    pub fn iter(&self) -> impl Iterator<Item = &Oid> {
        self.0.iter()
    }

    /// How many facts.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// No facts?
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The set's address, order-free: `"facts\0" ‖ u64le(n) ‖ field(fact)…`,
    /// sorted, in the public scope. The facts head.
    pub fn address(&self) -> Oid {
        let mut preimage = b"facts\0".to_vec();
        preimage.extend_from_slice(&(self.0.len() as u64).to_le_bytes());
        for fact in &self.0 {
            field(&mut preimage, fact.as_str().as_bytes());
        }
        Oid::hash(&Scope::public(), &preimage)
    }
}

// ---------------------------------------------------------------------------
// Outputs and their projection
// ---------------------------------------------------------------------------

/// A declared derivation's output: which premise it is, its status, and what
/// sits beside the status (the stage λ, ρ; the cites; the facts head).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    premise: Oid,
    status: String,
    lambda: u64,
    rho: u64,
    /// Sorted and deduplicated: a set, with a slice view.
    cites: Vec<Oid>,
    head: Option<Oid>,
}

impl Output {
    /// Premise `premise` with `status`, at stage (0, 0), citing nothing,
    /// with no head.
    pub fn new(premise: Oid, status: impl Into<String>) -> Self {
        Output {
            premise,
            status: status.into(),
            lambda: 0,
            rho: 0,
            cites: Vec::new(),
            head: None,
        }
    }

    /// At stage (λ, ρ).
    pub fn at(mut self, lambda: u64, rho: u64) -> Self {
        self.lambda = lambda;
        self.rho = rho;
        self
    }

    /// Citing `cites`, a set.
    pub fn citing(mut self, cites: impl IntoIterator<Item = Oid>) -> Self {
        let set: BTreeSet<Oid> = cites.into_iter().collect();
        self.cites = set.into_iter().collect();
        self
    }

    /// Derived over the fact set at `head`.
    pub fn headed(mut self, head: Oid) -> Self {
        self.head = Some(head);
        self
    }

    /// Which premise.
    pub fn premise(&self) -> &Oid {
        &self.premise
    }

    /// The status: the verdict word a decision acts on.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The stage (λ, ρ).
    pub fn stage(&self) -> (u64, u64) {
        (self.lambda, self.rho)
    }

    /// The cites, ordered, each once.
    pub fn cites(&self) -> &[Oid] {
        &self.cites
    }

    /// The facts head, if the output carries one.
    pub fn head(&self) -> Option<&Oid> {
        self.head.as_ref()
    }

    /// The full output address: premise, status, stage, cites and head,
    /// `"output\0" ‖ …` in the public scope.
    pub fn address(&self) -> Oid {
        let mut preimage = b"output\0".to_vec();
        field(&mut preimage, self.premise.as_str().as_bytes());
        field(&mut preimage, self.status.as_bytes());
        preimage.extend_from_slice(&self.lambda.to_le_bytes());
        preimage.extend_from_slice(&self.rho.to_le_bytes());
        preimage.extend_from_slice(&(self.cites.len() as u64).to_le_bytes());
        for cite in &self.cites {
            field(&mut preimage, cite.as_str().as_bytes());
        }
        match &self.head {
            None => preimage.push(0),
            Some(head) => {
                preimage.push(1);
                field(&mut preimage, head.as_str().as_bytes());
            }
        }
        Oid::hash(&Scope::public(), &preimage)
    }
}

/// What a decision reads of a premise's output. Its `read` writes the
/// projected part, self-delimiting; its `NAME` goes into every projected
/// address, so two projections never share one.
pub trait Projection {
    /// The projection's name.
    const NAME: &'static str;
    /// Write what a decision reads of `output` into `preimage`.
    fn read(output: &Output, preimage: &mut Vec<u8>);
}

/// **The premise projection: the status alone.** λ, ρ, the cites and the
/// head sit beside it, so a stage move is reported, never a HOLD. Mara's
/// lean (mirror-029 I.1, "yours to decide, Alex"), **flagged for Alex**:
/// this is the one place the choice lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StatusAlone;

impl Projection for StatusAlone {
    const NAME: &'static str = "status";
    fn read(output: &Output, preimage: &mut Vec<u8>) {
        field(preimage, output.status.as_bytes());
    }
}

/// A premise as a decision read it: its name, the status it read, and its
/// projection's address. Made only by [`Premise::of`], projecting an
/// [`Output`].
pub struct Premise<P = StatusAlone> {
    premise: Oid,
    status: String,
    address: Oid,
    projection: PhantomData<fn() -> P>,
}

impl<P: Projection> Premise<P> {
    /// Project `output`: `"premise\0" ‖ field(P::NAME) ‖ field(premise) ‖
    /// P::read(output)`, in the public scope.
    pub fn of(output: &Output) -> Self {
        let mut preimage = b"premise\0".to_vec();
        field(&mut preimage, P::NAME.as_bytes());
        field(&mut preimage, output.premise.as_str().as_bytes());
        P::read(output, &mut preimage);
        Premise {
            premise: output.premise.clone(),
            status: output.status.clone(),
            address: Oid::hash(&Scope::public(), &preimage),
            projection: PhantomData,
        }
    }
}

impl<P> Premise<P> {
    /// Which premise.
    pub fn premise(&self) -> &Oid {
        &self.premise
    }

    /// The status it read: for a sealed premise, the status it was sealed
    /// at, readable as long as the decision is.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The projection's address: what the decision sealed.
    pub fn address(&self) -> &Oid {
        &self.address
    }
}

impl<P> Clone for Premise<P> {
    fn clone(&self) -> Self {
        Premise {
            premise: self.premise.clone(),
            status: self.status.clone(),
            address: self.address.clone(),
            projection: PhantomData,
        }
    }
}

impl<P> PartialEq for Premise<P> {
    fn eq(&self, other: &Self) -> bool {
        self.premise == other.premise
            && self.status == other.status
            && self.address == other.address
    }
}

impl<P> Eq for Premise<P> {}

impl<P> fmt::Debug for Premise<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Premise")
            .field("premise", &self.premise)
            .field("status", &self.status)
            .field("address", &self.address)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// The current outputs
// ---------------------------------------------------------------------------

/// Two different outputs for one premise: two addresses for one key
/// (shape (a), prism-024 §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The premise with two outputs.
    pub premise: Oid,
}

/// The derivations' current outputs, one per premise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Current(BTreeMap<Oid, Output>);

impl Current {
    /// The outputs, keyed by premise. The same output twice is one; two
    /// different outputs for one premise are a [`Conflict`].
    pub fn new(outputs: impl IntoIterator<Item = Output>) -> Result<Self, Conflict> {
        let mut map = BTreeMap::new();
        for output in outputs {
            if let Some(there) = map.get(&output.premise) {
                if *there != output {
                    return Err(Conflict {
                        premise: output.premise,
                    });
                }
                continue;
            }
            map.insert(output.premise.clone(), output);
        }
        Ok(Current(map))
    }

    /// `premise`'s current output, if derived.
    pub fn get(&self, premise: &Oid) -> Option<&Output> {
        self.0.get(premise)
    }
}

// ---------------------------------------------------------------------------
// Decisions
// ---------------------------------------------------------------------------

/// A premise named twice in one tuple.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Twice {
    /// The premise named twice.
    pub premise: Oid,
}

/// A decision before its seal: the rule it decides, and the tuple of premise
/// projections it read, in declared order.
pub struct DecisionDraft<P = StatusAlone> {
    rule: Oid,
    premises: Vec<Premise<P>>,
}

impl<P: Projection> DecisionDraft<P> {
    /// `rule` over `premises`. Premises only: a fact set is not one.
    /// Refused when a premise is named twice.
    pub fn new(rule: Oid, premises: impl IntoIterator<Item = Premise<P>>) -> Result<Self, Twice> {
        let premises: Vec<Premise<P>> = premises.into_iter().collect();
        let mut seen = BTreeSet::new();
        for p in &premises {
            if !seen.insert(&p.premise) {
                return Err(Twice {
                    premise: p.premise.clone(),
                });
            }
        }
        Ok(DecisionDraft { rule, premises })
    }

    /// The decision's address: `"decision\0" ‖ field(P::NAME) ‖ field(rule) ‖
    /// u64le(k) ‖ (field(premise) ‖ field(projection address))…`, in the
    /// public scope. Never the signer.
    pub fn address(&self) -> Oid {
        let mut preimage = b"decision\0".to_vec();
        field(&mut preimage, P::NAME.as_bytes());
        field(&mut preimage, self.rule.as_str().as_bytes());
        preimage.extend_from_slice(&(self.premises.len() as u64).to_le_bytes());
        for p in &self.premises {
            field(&mut preimage, p.premise.as_str().as_bytes());
            field(&mut preimage, p.address.as_str().as_bytes());
        }
        Oid::hash(&Scope::public(), &preimage)
    }

    /// Seal it as `by`'s. Anyone may seal a draft; only the owner's seal
    /// clears a HOLD ([`reseal`]).
    pub fn seal(self, by: Oid) -> Decision<P> {
        let over = self.address();
        Decision {
            draft: self,
            signer: Signer::mint(by, over),
        }
    }
}

/// A sealed decision: its draft, and its owner's signer beside its address.
pub struct Decision<P = StatusAlone> {
    draft: DecisionDraft<P>,
    signer: Signer,
}

impl<P: Projection> Decision<P> {
    /// The decision's address (the draft's); the signer is beside it.
    pub fn address(&self) -> Oid {
        self.draft.address()
    }
}

impl<P> Decision<P> {
    /// The owner: who sealed it, and who alone can re-seal it.
    pub fn owner(&self) -> &Oid {
        self.signer.by()
    }

    /// The seal.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The rule it decides.
    pub fn rule(&self) -> &Oid {
        &self.draft.rule
    }

    /// The tuple it sealed, in declared order.
    pub fn premises(&self) -> &[Premise<P>] {
        &self.draft.premises
    }
}

impl<P> Clone for Decision<P> {
    fn clone(&self) -> Self {
        Decision {
            draft: DecisionDraft {
                rule: self.draft.rule.clone(),
                premises: self.draft.premises.clone(),
            },
            signer: self.signer.clone(),
        }
    }
}

impl<P> PartialEq for Decision<P> {
    fn eq(&self, other: &Self) -> bool {
        self.draft.rule == other.draft.rule
            && self.draft.premises == other.draft.premises
            && self.signer == other.signer
    }
}

impl<P> Eq for Decision<P> {}

impl<P> fmt::Debug for Decision<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Decision")
            .field("rule", &self.draft.rule)
            .field("premises", &self.draft.premises)
            .field("signer", &self.signer)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// The law
// ---------------------------------------------------------------------------

/// Whether a decision still covers what it read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Every premise's projection is the one sealed.
    Stands,
    /// Some moved: why, per premise (in declared order), and the owner who
    /// can re-seal. A reason, never a new class: a move to `maybe` is a HOLD
    /// like any other, a new reading at a new key, never a downgrade.
    Hold {
        /// The premises whose projection moved, or that have no current
        /// output, each with its reason.
        moved: Vec<Moved>,
        /// The decision's owner.
        owner: Oid,
    },
}

/// Why a premise HOLDs a decision: which premise, the status it was sealed
/// at, and how it reads now. The old seal is untouched and stays readable at
/// its key; the new reading has its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    /// Which premise.
    pub premise: Oid,
    /// The status it was sealed at.
    pub sealed: String,
    /// How it reads now.
    pub now: Reading,
}

/// How a moved premise reads now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reading {
    /// Derived: its status now, and the key (projection address) of this
    /// new reading.
    At {
        /// The status now.
        status: String,
        /// The new reading's key.
        key: Oid,
    },
    /// No current output: unread, its own constructor (the absence rule,
    /// `docs/spec/navigation.md`), never a status.
    Unread,
}

/// The declared premises whose current projection isn't `sealed`'s, each with
/// its reason.
fn moved<P: Projection>(sealed: &[Premise<P>], current: &Current) -> Vec<Moved> {
    sealed
        .iter()
        .filter_map(|p| {
            let now = match current.get(&p.premise) {
                None => Reading::Unread,
                Some(output) => {
                    let read = Premise::<P>::of(output);
                    if read.address == p.address {
                        return None;
                    }
                    Reading::At {
                        status: read.status,
                        key: read.address,
                    }
                }
            };
            Some(Moved {
                premise: p.premise.clone(),
                sealed: p.status.clone(),
                now,
            })
        })
        .collect()
}

/// The HOLD law: project each declared premise's current output and compare
/// it with the sealed projection. A missing output counts as moved, and reads
/// [`Reading::Unread`].
pub fn standing<P: Projection>(decision: &Decision<P>, current: &Current) -> Standing {
    let moved = moved(decision.premises(), current);
    if moved.is_empty() {
        Standing::Stands
    } else {
        Standing::Hold {
            moved,
            owner: decision.owner().clone(),
        }
    }
}

/// Why a seal doesn't clear a HOLD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Sealed by someone other than the decision's owner.
    NotOwner {
        /// The owner.
        owner: Oid,
        /// Who sealed.
        by: Oid,
    },
    /// Another rule, or other premise names: another decision.
    OtherDecision,
    /// Over the tuple already sealed, which is no longer current.
    OldTuple,
    /// Over a tuple that isn't the current one: the premises that don't
    /// match.
    NotCurrent {
        /// The premises whose sealed projection isn't the current one.
        premises: Vec<Oid>,
    },
}

/// Clear `held` with the seal `new`: only the owner's, over the same rule
/// and premise names, over the current tuple. A standing decision re-sealed
/// over its own (current) tuple is accepted as itself.
pub fn reseal<P: Projection>(
    held: &Decision<P>,
    new: Decision<P>,
    current: &Current,
) -> Result<Decision<P>, Refused> {
    if new.owner() != held.owner() {
        return Err(Refused::NotOwner {
            owner: held.owner().clone(),
            by: new.owner().clone(),
        });
    }
    let names =
        |d: &Decision<P>| -> Vec<Oid> { d.premises().iter().map(|p| p.premise.clone()).collect() };
    if new.rule() != held.rule() || names(&new) != names(held) {
        return Err(Refused::OtherDecision);
    }
    let off = moved(new.premises(), current);
    if off.is_empty() {
        return Ok(new);
    }
    if new.premises() == held.premises() {
        return Err(Refused::OldTuple);
    }
    Err(Refused::NotCurrent {
        premises: off.into_iter().map(|m| m.premise).collect(),
    })
}

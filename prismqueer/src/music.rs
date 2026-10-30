//! `prismqueer::music` — the engine's music (prism-007, prism-008).
//!
//! Spec: Mara, `2026-10-29-mara-song-of-the-past-void-commas-and-the-beat.md`
//! (mirror). Babbage built the engine; Ada wrote the Notes. This is the
//! engine side: pure maths, no observer.
//!
//! - A [`Note`] is one derivation: head, rule, premises. No beat inside it.
//! - Notes compose by union ([`Piece::compose`]), idempotent and commutative.
//! - Beats (Kleene ticks) and chords are computed from the composed piece,
//!   never stored, because geodesic certificates don't compose (§10): a
//!   shorter route arriving from another observer moves a beat.
//! - The [`Cadence`] is the fixpoint.
//! - [`ring_true`] reads a tuning (a ratio-valued field): consonance, or
//!   located dissonances with their exact commas (§5).
//!
//! prism is the instrument, not the tune: it never picks the tuning. That's a
//! ruler, and rulers are mirror's.

use std::collections::{BTreeMap, BTreeSet};

use crate::field::{check_flat, Edge, Field, Flat, Flatness, Graph, Ratio};
use crate::spectral::kleinos::VertexId;

/// A fact or a derived atom, by name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Atom(String);

impl Atom {
    /// The atom called `name`.
    pub fn new(name: &str) -> Self {
        Atom(name.to_string())
    }
}

/// The rule a derivation used, by name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rule(String);

impl Rule {
    /// The rule called `name`.
    pub fn new(name: &str) -> Self {
        Rule(name.to_string())
    }
}

/// One derivation: `head` by `rule` from `premises`. A fact has no premises.
/// A Note carries no beat; beats belong to the piece it's heard in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    head: Atom,
    rule: Rule,
    premises: Vec<Atom>,
}

impl Note {
    /// A derivation.
    pub fn new(head: Atom, rule: Rule, premises: impl IntoIterator<Item = Atom>) -> Self {
        Note {
            head,
            rule,
            premises: premises.into_iter().collect(),
        }
    }
}

/// A set of Notes: what composes.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Piece(BTreeSet<Note>);

impl<const N: usize> From<[Note; N]> for Piece {
    fn from(notes: [Note; N]) -> Self {
        Piece(notes.into_iter().collect())
    }
}

/// The fixpoint of a piece: the number of beats until nothing new is heard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cadence {
    beats: usize,
}

impl Cadence {
    /// How many beats the piece takes to settle (0 for an empty piece).
    pub fn beats(&self) -> usize {
        self.beats
    }
}

impl Piece {
    /// Compose two pieces: the union of their Notes.
    pub fn compose(&self, other: &Self) -> Self {
        Piece(self.0.union(&other.0).cloned().collect())
    }

    /// How many Notes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the piece has no Notes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Each derivable atom's beat: the least Kleene tick. A fact is beat 0;
    /// otherwise one more than its latest premise, minimised over all its
    /// derivations. An atom nothing derives has no beat.
    pub fn beats(&self) -> BTreeMap<Atom, usize> {
        let mut beat: BTreeMap<Atom, usize> = BTreeMap::new();
        loop {
            let mut moved = false;
            for n in &self.0 {
                if let Some(t) = self.tick_of(n, &beat) {
                    if beat.get(&n.head).is_none_or(|&old| t < old) {
                        beat.insert(n.head.clone(), t);
                        moved = true;
                    }
                }
            }
            if !moved {
                return beat;
            }
        }
    }

    fn tick_of(&self, n: &Note, beat: &BTreeMap<Atom, usize>) -> Option<usize> {
        let mut latest: Option<usize> = None;
        for p in &n.premises {
            let t = *beat.get(p)?;
            latest = Some(latest.map_or(t, |l| l.max(t)));
        }
        Some(latest.map_or(0, |l| l + 1))
    }

    /// The geodesic projection: per atom, the Note that reaches it on its
    /// beat (the least such Note when several do). Computed after
    /// composition, never composed.
    pub fn geodesic(&self) -> BTreeMap<Atom, Note> {
        let beat = self.beats();
        let mut out: BTreeMap<Atom, Note> = BTreeMap::new();
        for n in &self.0 {
            if self.tick_of(n, &beat) == beat.get(&n.head).copied() {
                out.entry(n.head.clone()).or_insert_with(|| n.clone());
            }
        }
        out
    }

    /// All atoms on one beat.
    pub fn chord(&self, at: usize) -> BTreeSet<Atom> {
        self.beats()
            .into_iter()
            .filter(|&(_, t)| t == at)
            .map(|(a, _)| a)
            .collect()
    }

    /// The fixpoint: beats until nothing new is heard.
    pub fn cadence(&self) -> Cadence {
        Cadence {
            beats: self.beats().values().max().map_or(0, |m| m + 1),
        }
    }
}

/// A located dissonance: the readings around one loop, the edges it runs
/// along, and its exact comma. Read from a [`Tuning`], it also names the
/// Notes on those edges.
#[derive(Clone, Debug, PartialEq)]
pub struct Dissonance {
    notes: Vec<VertexId>,
    edges: Vec<Edge>,
    comma: Ratio,
    by: Vec<Note>,
}

impl Dissonance {
    /// The loop, starting and ending at its smallest reading.
    pub fn notes(&self) -> &[VertexId] {
        &self.notes
    }
    /// The edges around the loop, in path order. On a two-Note loop these
    /// are the two parallel edges: out along one, back along the other.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    /// How far it falls short of closing, exactly.
    pub fn comma(&self) -> &Ratio {
        &self.comma
    }
    /// The Notes on [`edges`](Self::edges), in path order. Empty when read
    /// from a bare field ([`ring_true`]), which has no Notes to name.
    pub fn by(&self) -> &[Note] {
        &self.by
    }
}

/// A tuning made of Notes: each Note relates its one premise to its head by
/// an exact interval, and is its own edge. Two Notes on the same pair of
/// atoms are two parallel edges, never one: if their intervals differ, the
/// 2-loop between them holds the comma. Atoms are numbered in order
/// ([`Tuning::atoms`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    atoms: Vec<Atom>,
    notes: BTreeMap<Edge, Note>,
    field: Field<Graph, Ratio>,
}

/// Why a tuning couldn't be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TuningError {
    /// This Note doesn't have exactly one premise, so it relates no pair.
    NotAnInterval(Note),
}

impl Tuning {
    /// A tuning from Notes and their intervals, premise to head. Refused if
    /// a Note is a fact or has several premises.
    pub fn new(intervals: impl IntoIterator<Item = (Note, Ratio)>) -> Result<Self, TuningError> {
        let intervals: Vec<(Note, Ratio)> = intervals.into_iter().collect();
        if let Some((n, _)) = intervals.iter().find(|(n, _)| n.premises.len() != 1) {
            return Err(TuningError::NotAnInterval(n.clone()));
        }
        let atoms: Vec<Atom> = intervals
            .iter()
            .flat_map(|(n, _)| [n.premises[0].clone(), n.head.clone()])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let id = |a: &Atom| atoms.binary_search(a).expect("collected") as VertexId;
        // Edge identities as Field::multigraph numbers them: per pair, in order.
        let mut seen: BTreeMap<(VertexId, VertexId), usize> = BTreeMap::new();
        let mut notes = BTreeMap::new();
        let mut given = Vec::with_capacity(intervals.len());
        for (n, r) in intervals {
            let (from, to) = (id(&n.premises[0]), id(&n.head));
            let edge = Edge::between(from, to, 0);
            let nth = seen.entry(edge.ends()).or_insert(0);
            notes.insert(Edge::between(from, to, *nth), n);
            *nth += 1;
            given.push(((from, to), r));
        }
        Ok(Tuning {
            atoms,
            notes,
            field: Field::multigraph(given),
        })
    }

    /// The atoms; atom `i` is reading (vertex) `i`.
    pub fn atoms(&self) -> &[Atom] {
        &self.atoms
    }

    /// The tuning as a ratio-valued field on its multigraph.
    pub fn field(&self) -> &Field<Graph, Ratio> {
        &self.field
    }

    /// "Does it ring true?", with each dissonance naming its Notes.
    pub fn ring_true(&self) -> Ringing {
        match ring_true(&self.field) {
            Ringing::Dissonant(found) => Ringing::Dissonant(
                found
                    .into_iter()
                    .map(|d| Dissonance {
                        by: d.edges.iter().map(|e| self.notes[e].clone()).collect(),
                        ..d
                    })
                    .collect(),
            ),
            consonant => consonant,
        }
    }
}

/// Whether a tuning rings true.
#[derive(Clone, Debug, PartialEq)]
pub enum Ringing {
    /// Consonance: every reading agrees with one tuning (a potential exists).
    True(Flat<Ratio>),
    /// The loops that don't close, located, with their commas.
    Dissonant(Vec<Dissonance>),
}

/// "Does it ring true?": consonance, or where the dissonance is and how far.
/// A bare field has no Notes, so its dissonances name none
/// ([`Tuning::ring_true`] does).
pub fn ring_true(tuning: &Field<Graph, Ratio>) -> Ringing {
    match check_flat(tuning) {
        Flatness::Flat(flat) => Ringing::True(flat),
        Flatness::Curved(curved) => Ringing::Dissonant(
            curved
                .loops()
                .iter()
                .map(|l| Dissonance {
                    notes: l.vertices().to_vec(),
                    edges: l.edges().to_vec(),
                    comma: l.holonomy().clone(),
                    by: Vec::new(),
                })
                .collect(),
        ),
    }
}

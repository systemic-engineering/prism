//! Progressions: histories in Foata normal form.

use std::collections::BTreeSet;
use std::fmt;
use std::marker::PhantomData;

use terni::Loss;

use super::Independence;
use crate::oid::{Addressable, Oid};

/// A history up to reordering of independent steps: an element of the trace
/// monoid M(E, I), with one adjoined absorbing element for a lost history.
///
/// A known history is stored in Foata normal form: a sequence of chords, each
/// a set of pairwise independent events, where every event sits one chord
/// after the deepest event it depends on. The form is unique (Cartier–Foata),
/// so equality is trace equivalence.
///
/// The relation is a type parameter: one pipeline, one relation.
pub struct Progression<E: Ord, I> {
    body: Body<E>,
    rel: PhantomData<I>,
}

enum Body<E> {
    Known(Vec<BTreeSet<E>>),
    Lost,
}

impl<E: Ord, I: Independence<E>> Progression<E, I> {
    /// The progression of a word: push each event in order.
    pub fn from_word(word: impl IntoIterator<Item = E>) -> Self {
        let mut progression = Self::default();
        for e in word {
            progression.push(e);
        }
        progression
    }

    /// Append one event. It lands one chord after the deepest event it depends
    /// on. Equal events always depend on each other, so none is ever dropped.
    /// Appending to a lost history leaves it lost.
    pub fn push(&mut self, e: E) {
        let Body::Known(steps) = &mut self.body else {
            return;
        };
        let rel = I::default();
        let at = steps
            .iter()
            .rposition(|step| step.iter().any(|f| *f == e || !rel.independent(f, &e)))
            .map_or(0, |deepest| deepest + 1);
        if at == steps.len() {
            steps.push(BTreeSet::new());
        }
        steps[at].insert(e);
    }

    /// The Foata normal form, if the history is known.
    pub fn foata(&self) -> Option<&[BTreeSet<E>]> {
        match &self.body {
            Body::Known(steps) => Some(steps),
            Body::Lost => None,
        }
    }

    /// Parallel time: the number of chords, if known.
    pub fn height(&self) -> Option<usize> {
        self.foata().map(<[_]>::len)
    }

    /// Sequential time: the number of events, if known.
    pub fn len(&self) -> Option<usize> {
        self.foata()
            .map(|steps| steps.iter().map(BTreeSet::len).sum())
    }

    /// Whether this is the known, empty history.
    pub fn is_empty(&self) -> bool {
        self.len() == Some(0)
    }

    /// Whether the history was lost.
    pub fn is_total(&self) -> bool {
        matches!(self.body, Body::Lost)
    }
}

impl<E: Ord, I> Default for Progression<E, I> {
    fn default() -> Self {
        Self {
            body: Body::Known(Vec::new()),
            rel: PhantomData,
        }
    }
}

impl<E: Ord + Clone, I> Clone for Progression<E, I> {
    fn clone(&self) -> Self {
        let body = match &self.body {
            Body::Known(steps) => Body::Known(steps.clone()),
            Body::Lost => Body::Lost,
        };
        Self {
            body,
            rel: PhantomData,
        }
    }
}

impl<E: Ord, I> PartialEq for Progression<E, I> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.body, &other.body) {
            (Body::Known(a), Body::Known(b)) => a == b,
            (Body::Lost, Body::Lost) => true,
            _ => false,
        }
    }
}

impl<E: Ord, I> Eq for Progression<E, I> {}

impl<E: Ord + fmt::Debug, I> fmt::Debug for Progression<E, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.body {
            Body::Known(steps) => f.debug_tuple("Progression").field(steps).finish(),
            Body::Lost => f.write_str("Progression(lost)"),
        }
    }
}

/// Concatenation in M(E, I), with the lost history absorbing.
impl<E: Ord + Clone, I: Independence<E>> Loss for Progression<E, I> {
    fn zero() -> Self {
        Self::default()
    }

    fn total() -> Self {
        Self {
            body: Body::Lost,
            rel: PhantomData,
        }
    }

    fn is_zero(&self) -> bool {
        self.is_empty()
    }

    fn combine(mut self, other: Self) -> Self {
        match other.body {
            Body::Lost => Self::total(),
            Body::Known(steps) => {
                for e in steps.into_iter().flatten() {
                    self.push(e);
                }
                self
            }
        }
    }
}

/// The address hashes the relation and every chord boundary: `{a, b}` and
/// `{a}{b}` never share an address, nor does one word under two relations,
/// nor a lost history and a known one.
impl<E: Ord + Addressable, I: Independence<E>> Addressable for Progression<E, I> {
    fn oid(&self) -> Oid {
        let mut bytes = I::default().oid().as_str().as_bytes().to_vec();
        match &self.body {
            Body::Lost => bytes.extend_from_slice(b"|lost"),
            Body::Known(steps) => {
                for step in steps {
                    let mut events: Vec<Oid> = step.iter().map(Addressable::oid).collect();
                    events.sort();
                    bytes.push(b'[');
                    for e in events {
                        bytes.extend_from_slice(e.as_str().as_bytes());
                        bytes.push(b',');
                    }
                    bytes.push(b']');
                }
            }
        }
        Oid::hash(&bytes)
    }
}

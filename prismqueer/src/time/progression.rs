//! Traces in Foata normal form.

use std::collections::BTreeSet;
use std::marker::PhantomData;

use super::Independence;
use crate::oid::{Addressable, Oid};

/// A history up to reordering of independent steps.
///
/// Stored in Foata normal form: a sequence of steps, each a set of pairwise
/// independent events, where every event sits one step after the deepest
/// event it depends on. The form is unique (Cartier–Foata), so equality is
/// trace equivalence.
///
/// The relation is a type parameter: one pipeline, one relation.
pub struct Progression<E: Ord, I> {
    steps: Vec<BTreeSet<E>>,
    rel: PhantomData<I>,
}

impl<E: Ord, I: Independence<E>> Progression<E, I> {
    /// The trace of a word: push each event in order.
    pub fn from_word(word: impl IntoIterator<Item = E>) -> Self {
        let mut trace = Self::default();
        for e in word {
            trace.push(e);
        }
        trace
    }

    /// Append one event. It lands one step after the deepest event it depends
    /// on. Equal events always depend on each other, so none is ever dropped.
    pub fn push(&mut self, e: E) {
        let rel = I::default();
        let at = self
            .steps
            .iter()
            .rposition(|step| step.iter().any(|f| *f == e || !rel.independent(f, &e)))
            .map_or(0, |deepest| deepest + 1);
        if at == self.steps.len() {
            self.steps.push(BTreeSet::new());
        }
        self.steps[at].insert(e);
    }

    /// The Foata normal form.
    pub fn foata(&self) -> &[BTreeSet<E>] {
        &self.steps
    }

    /// Parallel time: the number of Foata steps.
    pub fn height(&self) -> usize {
        self.steps.len()
    }

    /// Monophony time: the number of events.
    pub fn len(&self) -> usize {
        self.steps.iter().map(BTreeSet::len).sum()
    }

    /// Whether the trace has no events.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

impl<E: Ord, I> Default for Progression<E, I> {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            rel: PhantomData,
        }
    }
}

impl<E: Ord + Clone, I> Clone for Progression<E, I> {
    fn clone(&self) -> Self {
        Self {
            steps: self.steps.clone(),
            rel: PhantomData,
        }
    }
}

impl<E: Ord, I> PartialEq for Progression<E, I> {
    fn eq(&self, other: &Self) -> bool {
        self.steps == other.steps
    }
}

impl<E: Ord, I> Eq for Progression<E, I> {}

/// The address hashes the relation and every step boundary: `{a, b}` and
/// `{a}{b}` never share an address, and neither does one word under two
/// relations.
impl<E: Ord + Addressable, I: Independence<E>> Addressable for Progression<E, I> {
    fn oid(&self) -> Oid {
        let mut bytes = I::default().oid().as_str().as_bytes().to_vec();
        for step in &self.steps {
            let mut events: Vec<Oid> = step.iter().map(Addressable::oid).collect();
            events.sort();
            bytes.push(b'[');
            for e in events {
                bytes.extend_from_slice(e.as_str().as_bytes());
                bytes.push(b',');
            }
            bytes.push(b']');
        }
        Oid::hash(&bytes)
    }
}

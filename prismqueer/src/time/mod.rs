//! `prismqueer::time` — histories up to reordering of independent steps.
//!
//! A [`Progression`] is an element of the trace monoid M(E, I), stored in Foata
//! normal form. Linear time is one rung of it: under [`Monophony`] nothing
//! commutes and a trace is a word. Under [`Polyphony`] independent steps share
//! a Foata step, and every observer's linearization has the same address.
//!
//! Spec: `docs/math/nonlinear-inference.md` §2 and §6.1.

mod independence;
mod progression;

pub use independence::{Independence, Monophony, Polyphony, Touches};
pub use progression::Progression;

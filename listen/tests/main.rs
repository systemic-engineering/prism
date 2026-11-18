//! listen's tests, one binary.
//!
//! Vectors: mirror's atom vectors and `mirror.facts`, copied as data into
//! docs/spec/listen-vectors/mirror (see its README for the commit and the
//! paths); everything else is written here from mirror's spec text, each case
//! citing its section. No test reads mirror's engine source, or derives from
//! it.

mod cut;
mod facts;
mod refusals;
mod typed;
mod vectors;

//! prism's floor: the exact laws a verdict rests on (docs/spec/laws.md).
//!
//! A sibling of `echo` and `chorus`, moved out of prismqueer (the lab),
//! which re-exports every module here at its old path. What this crate
//! doesn't depend on, no law can call: no float entry (the lab owns its
//! float carrier), no proc-macro, no serde, no mirror, no native numerics
//! (`checks.laws-independent`). A float appears only as the argument of a
//! judge, read once as the exact rational it is (`tests/floor.rs`).

#![forbid(unsafe_code)]

// The doc examples name the crate's paths as a user would.
extern crate self as prism_laws;

// The identity law: Oid, Scope, BLAKE3 keyed by scope.
pub mod oid;

// A settled item: canonical bytes under their address (was brain::Crystal).
pub mod crystal;

// Groups, lattices, graphs, faces, flatness.
pub mod field;

// Rooms and maps over ℤ, ℤ/n, ℚ/ℤ, ℚ: Smith, charpoly, solve, holes.
pub mod room;

// GL(n) over ℤ and ℚ.
pub mod gl;

// Exact polynomials over ℤ and ℚ: gcds, square-free parts, real roots
// isolated by Sturm sequences, factoring; the judges locate and identify.
pub mod poly;

// The kernel check: a signed selection against the consistent readings of
// exact relations (prism-019 ask 4).
pub mod agreement;

// Frames, hypotheses and their composites.
pub mod hypothesis;

// Two parties' frames glued along declared spans, a pushout (prism-021 D4).
pub mod kleinos;

// A frame's ruler for length, certified by an exact LDLᵀ, and a Form of any
// signature (prism-021 D5).
pub mod metrix;

// A precision matrix estimated exactly over ℚ from integer counts under a
// declared (chordal) support; a Metrix only once certified and sealed
// (prism-023 §1, D6). `measure_under` is the primitive.
pub mod measured;

// The HOLD law: a decision stands while every premise projection is the one
// sealed (prism-024 §1).
pub mod hold;

// The navigation floor: K3 over Membership, K3-Datalog over hyperedges; an
// unexplored edge reads maybe (prism-025 Addendum item 1).
pub mod navigation;

// The flip point: when a verdict over a sequence of facts changed
// (prism-024 §2).
pub mod flip;

// Replayed edges and claims.
pub mod crossing;

// The seals: Ruler, Signer, drafts, Calibration, Independence, persist, cross.
pub mod algedonic;

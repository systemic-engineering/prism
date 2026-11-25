//! `room`: a law of prism's floor, re-exported at its old path, with the
//! lab's measured half on top. The exact half lives in `prism_laws::room`
//! (docs/spec/laws.md); the floor has no float entry at all.
//!
//! The measured half (`lapack`, [`reading`](self)): the lab's float carrier
//! `F64`, the one explicit step onto it ([`Measure`] for an entry,
//! [`Measuring`] for a map or a mirror), and the readings LAPACK gives of a
//! measured mirror ([`Resonant`]: its `Modes`, or its spectrum alone).
//!
//! The control: an exact map measured, then composed with another measured
//! one.
//!
//! ```
//! # #[cfg(feature = "lapack")] {
//! use prismqueer::field::Graph;
//! use prismqueer::room::{coboundary, Measuring};
//! let delta = coboundary(&Graph::new([0, 1, 2], [(0, 1), (1, 2), (0, 2)]));
//! let _ = delta.measure().then(&delta.adjoint().measure()).unwrap();
//! # }
//! ```
//!
//! An exact map then a measured one: the move to floats is never implicit.
//!
//! ```compile_fail,E0308
//! use prismqueer::field::Graph;
//! use prismqueer::room::{coboundary, Measuring};
//! let delta = coboundary(&Graph::new([0, 1, 2], [(0, 1), (1, 2), (0, 2)]));
//! let _ = delta.then(&delta.adjoint().measure());
//! ```
//!
//! An exact solve over the float carrier: `F64` isn't `Exactly`, since a
//! nonzero float pairing may be rounding. Floats are read through a band.
//!
//! ```compile_fail,E0277
//! use prismqueer::field::Graph;
//! use prismqueer::room::{solve, F64};
//! let g = Graph::new([], [(0, 1), (0, 1)]);
//! let _ = solve(&(), &g, &[F64(6.0), F64(-6.0)]);
//! ```

pub use prism_laws::room::*;

// The measured half (`lapack`): the float carrier, the one explicit step to
// it, and the eigen and singular-value readings through LAPACK
// (docs/spec/laws.md, "The float half under R2").
#[cfg(feature = "lapack")]
mod reading;
#[cfg(feature = "lapack")]
pub(crate) use reading::Norms;
#[cfg(feature = "lapack")]
pub use reading::{Measure, Measuring, Modes, Resonant, F64};

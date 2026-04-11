//! Prism — focus | project | refract.
//!
//! A Beam is a semifunctor. A Prism is the monoid lifted into it.
//! Three operations. Three dimensions of space.

pub mod beam;
pub mod trace;

pub use beam::{Beam, Operation, PureBeam};
pub use trace::{Op, Step, StepOutput, Trace, Traced};

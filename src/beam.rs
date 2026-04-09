//! Beam — the pipeline value carrier.
//!
//! `Beam` is a trait. Two implementations:
//!
//! - `PureBeam<In, Out, E>` — prod. Input + output. No trace overhead.
//! - `TraceBeam<In, Out, E>` — debug. Same fields + full `Trace`. (forthcoming)
//!
//! Three luminosity states:
//!
//! - `Radiant` — full-strength value, zero loss.
//! - `Dimmed`  — value present, information was lost getting here.
//! - `Dark`    — the step failed.
//!
//! `Luminosity<V, E>` is the extracted state machine. PureBeam is a flat
//! struct: `{ input: In, luminosity: Luminosity<Out, E> }`.
//!
//! Two transition methods:
//! - `next(value)` — lossless, same error type. The common case.
//! - `advance(luminosity)` — full control: change error type, carry loss, or fail.
//!
//! The type chain is enforced by the compiler:
//!
//!   `type Next<T>: Beam<In = Self::Out, Out = T, Error = Self::Error>`

use std::convert::Infallible;

use crate::loss::ShannonLoss;
use crate::trace::Op;

// ---------------------------------------------------------------------------
// Operation trait
// ---------------------------------------------------------------------------

/// A self-contained pipeline operation. Wraps a prism (and closure for
/// split/zoom). The beam is NOT stored here — it arrives via `apply`.
///
/// `op()` identifies which pipeline stage this is. Used by `TraceBeam`
/// to record the step without any caller cooperation.
pub trait Operation<B: Beam> {
    type Output: Beam;
    fn op(&self) -> Op;
    fn apply(self, beam: B) -> Self::Output;
}

// ---------------------------------------------------------------------------
// Luminosity — the extracted state machine
// ---------------------------------------------------------------------------

/// The luminosity of a beam: how much signal is present, and how was it lost.
///
/// `combine` propagates accumulated loss from a prior step into the next.
/// Callers must not combine from a Dark luminosity — that is a programming error.
#[derive(Clone, Debug)]
pub enum Luminosity<V, E> {
    Radiant(V),
    Dimmed(V, ShannonLoss),
    Dark(E),
}

impl<V, E> Luminosity<V, E> {
    /// Owned result: Ok if Radiant/Dimmed, Err if Dark.
    pub fn result(self) -> Result<V, E> {
        match self {
            Self::Radiant(v) | Self::Dimmed(v, _) => Ok(v),
            Self::Dark(e) => Err(e),
        }
    }

    /// Borrowed result.
    pub fn result_ref(&self) -> Result<&V, &E> {
        match self {
            Self::Radiant(v) | Self::Dimmed(v, _) => Ok(v),
            Self::Dark(e) => Err(e),
        }
    }

    /// Extract the value if light, or None if Dark.
    pub fn value(self) -> Option<V> {
        self.result().ok()
    }

    /// Loss at this step.
    pub fn loss(&self) -> ShannonLoss {
        match self {
            Self::Radiant(..)       => ShannonLoss::zero(),
            Self::Dimmed(_, loss)   => loss.clone(),
            Self::Dark(..)          => ShannonLoss::total(),
        }
    }

    /// Propagate accumulated loss from `self` through `next`.
    ///
    /// Rules:
    /// - Radiant self: pass `next` through unchanged.
    /// - Dimmed self:  carry loss into `next` (Dark `next` passes through).
    /// - Dark self:    programming error — panics.
    pub fn combine<V2, E2>(self, next: Luminosity<V2, E2>) -> Luminosity<V2, E2> {
        match self {
            Self::Dark(_) => panic!("combine called on Dark luminosity — check is_light() first"),
            Self::Radiant(_) => next,
            Self::Dimmed(_, loss) => match next {
                Luminosity::Radiant(v)       => Luminosity::Dimmed(v, loss),
                Luminosity::Dimmed(v, loss2) => Luminosity::Dimmed(v, loss + loss2),
                Luminosity::Dark(e)          => Luminosity::Dark(e),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Beam trait
// ---------------------------------------------------------------------------

/// The pipeline value carrier. Flows through prisms, carrying its current
/// value and the record of how it got here.
///
/// # Type chain
///
/// `Next<T>` and `NextWithError<T, E>` enforce that each step's `Out`
/// becomes the next step's `In`. The compiler rejects invalid pipelines.
///
/// # Transition methods
///
/// - `next(value)` — lossless, same error type. The common case in prism methods.
/// - `advance(luminosity)` — full control: new error type, carry loss, or fail.
///
/// **Contract:** Both panic if called on a Dark beam. Call `is_light()` first.
pub trait Beam: Sized {
    type In;
    type Out;
    type Error;

    /// Advance to a new output type, preserving error type.
    /// The old `Out` becomes the new `In`. Type chain maintained.
    type Next<T>: Beam<In = Self::Out, Out = T, Error = Self::Error>;

    /// Advance to a new output and error type.
    /// The old `Out` becomes the new `In`. Type chain maintained.
    type NextWithError<T, E>: Beam<In = Self::Out, Out = T, Error = E>;

    /// The input that entered this step.
    fn input(&self) -> &Self::In;

    /// Information loss at this step.
    /// Zero for Radiant, positive for Dimmed, infinite for Dark.
    fn loss(&self) -> ShannonLoss;

    /// The output of this step, or the error if Dark.
    fn result(&self) -> Result<&Self::Out, &Self::Error>;

    /// Whether this beam is carrying a value (Radiant or Dimmed).
    fn is_light(&self) -> bool {
        self.result().is_ok()
    }

    /// Whether this beam failed (Dark).
    fn is_dark(&self) -> bool {
        self.result().is_err()
    }

    /// Lossless transition to a new output, preserving error type.
    /// The current `Out` becomes the new `In`. Loss state preserved.
    /// Panics if called on a Dark beam.
    fn next<T>(self, value: T) -> Self::Next<T>;

    /// Full transition: new output and error type via `Luminosity`.
    /// The current `Out` becomes the new `In`. Loss accumulates per `combine`.
    /// Panics if called on a Dark beam.
    fn advance<T, E>(self, luminosity: Luminosity<T, E>) -> Self::NextWithError<T, E>;

    /// Apply an operation to this beam. The pipeline DSL entry point.
    ///
    /// `beam.apply(Focus(&prism)).apply(Project(&prism)).apply(Refract(&prism))`
    fn apply<O: Operation<Self>>(self, op: O) -> O::Output {
        op.apply(self)
    }
}

// ---------------------------------------------------------------------------
// PureBeam
// ---------------------------------------------------------------------------

/// Production beam. Flat struct: input + luminosity. No trace overhead.
pub struct PureBeam<In, Out, E = Infallible> {
    input: In,
    luminosity: Luminosity<Out, E>,
}

impl<In, Out, E> PureBeam<In, Out, E> {
    /// Construct a lossless Radiant beam.
    pub fn radiant(input: In, output: Out) -> Self {
        Self { input, luminosity: Luminosity::Radiant(output) }
    }

    /// Construct a Dimmed beam (value with information loss).
    pub fn dimmed(input: In, output: Out, loss: ShannonLoss) -> Self {
        Self { input, luminosity: Luminosity::Dimmed(output, loss) }
    }

    /// Construct a Dark beam (step failed).
    pub fn dark(input: In, error: E) -> Self {
        Self { input, luminosity: Luminosity::Dark(error) }
    }
}

impl<In, Out, E> Beam for PureBeam<In, Out, E> {
    type In    = In;
    type Out   = Out;
    type Error = E;
    type Next<T>              = PureBeam<Out, T, E>;
    type NextWithError<T, NE> = PureBeam<Out, T, NE>;

    fn input(&self) -> &In {
        &self.input
    }

    fn loss(&self) -> ShannonLoss {
        self.luminosity.loss()
    }

    fn result(&self) -> Result<&Out, &E> {
        self.luminosity.result_ref()
    }

    fn next<T>(self, value: T) -> PureBeam<Out, T, E> {
        match self.luminosity {
            Luminosity::Dark(_) =>
                panic!("next called on Dark beam — check is_light() first"),
            Luminosity::Radiant(old_out) => PureBeam {
                input: old_out,
                luminosity: Luminosity::Radiant(value),
            },
            Luminosity::Dimmed(old_out, loss) => PureBeam {
                input: old_out,
                luminosity: Luminosity::Dimmed(value, loss),
            },
        }
    }

    fn advance<T, NE>(self, next: Luminosity<T, NE>) -> PureBeam<Out, T, NE> {
        match self.luminosity {
            Luminosity::Dark(_) =>
                panic!("advance called on Dark beam — check is_light() first"),
            Luminosity::Radiant(old_out) => PureBeam {
                input: old_out,
                luminosity: next,
            },
            Luminosity::Dimmed(old_out, loss) => PureBeam {
                input: old_out,
                luminosity: match next {
                    Luminosity::Radiant(v)       => Luminosity::Dimmed(v, loss),
                    Luminosity::Dimmed(v, loss2) => Luminosity::Dimmed(v, loss + loss2),
                    Luminosity::Dark(e)          => Luminosity::Dark(e),
                },
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    type Pure<I, O> = PureBeam<I, O, String>;

    #[test]
    fn radiant_is_light() {
        let b: Pure<(), u32> = PureBeam::radiant((), 42);
        assert!(b.is_light());
        assert!(!b.is_dark());
        assert_eq!(b.result(), Ok(&42));
        assert_eq!(b.input(), &());
        assert!(b.loss().is_zero());
    }

    #[test]
    fn dimmed_is_light_with_loss() {
        let b: Pure<(), u32> = PureBeam::dimmed((), 42, ShannonLoss::new(1.5));
        assert!(b.is_light());
        assert_eq!(b.result(), Ok(&42));
        assert_eq!(b.loss().as_f64(), 1.5);
    }

    #[test]
    fn dark_is_err() {
        let b: Pure<(), u32> = PureBeam::dark((), "oops".to_string());
        assert!(b.is_dark());
        assert!(!b.is_light());
        assert_eq!(b.result(), Err(&"oops".to_string()));
        assert!(b.loss().as_f64().is_infinite());
    }

    #[test]
    fn next_preserves_radiant() {
        let b: PureBeam<(), u32, String> = PureBeam::radiant((), 10);
        let n = b.next("hello");
        assert!(n.is_light());
        assert_eq!(n.result(), Ok(&"hello"));
        assert_eq!(n.input(), &10u32);
        assert!(n.loss().is_zero());
    }

    #[test]
    fn next_preserves_dimmed_loss() {
        let b: Pure<(), u32> = PureBeam::dimmed((), 10, ShannonLoss::new(2.0));
        let n = b.next(20u32);
        assert_eq!(n.loss().as_f64(), 2.0);
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    fn advance_radiant_with_radiant_stays_radiant() {
        let b: Pure<(), u32> = PureBeam::radiant((), 5);
        let n = b.advance(Luminosity::<&str, String>::Radiant("hi"));
        assert!(n.is_light());
        assert!(n.loss().is_zero());
    }

    #[test]
    fn advance_radiant_with_dimmed_becomes_dimmed() {
        let b: Pure<(), u32> = PureBeam::radiant((), 5);
        let n = b.advance(Luminosity::<&str, String>::Dimmed("hi", ShannonLoss::new(1.0)));
        assert!(n.is_light());
        assert_eq!(n.loss().as_f64(), 1.0);
    }

    #[test]
    fn advance_dimmed_with_radiant_carries_loss() {
        let b: Pure<(), u32> = PureBeam::dimmed((), 5, ShannonLoss::new(1.0));
        let n = b.advance(Luminosity::<u32, String>::Radiant(10));
        assert_eq!(n.loss().as_f64(), 1.0);
    }

    #[test]
    fn advance_dimmed_with_dimmed_accumulates_loss() {
        let b: Pure<(), u32> = PureBeam::dimmed((), 5, ShannonLoss::new(1.0));
        let n = b.advance(Luminosity::<u32, String>::Dimmed(10, ShannonLoss::new(0.5)));
        assert_eq!(n.loss().as_f64(), 1.5);
    }

    #[test]
    fn advance_with_dark_fails() {
        let b: Pure<(), u32> = PureBeam::radiant((), 5);
        let n: PureBeam<u32, u32, i32> = b.advance(Luminosity::Dark(-1i32));
        assert!(n.is_dark());
        assert_eq!(n.result(), Err(&-1i32));
    }

    #[test]
    fn type_chain_three_steps() {
        let b0: PureBeam<(), u32> = PureBeam::radiant((), 42u32);
        let b1: PureBeam<u32, String> = b0.next("hello".to_string());
        let b2: PureBeam<String, Vec<char>> = b1.next(vec!['a', 'b']);
        assert_eq!(b2.input(), &"hello".to_string());
        assert_eq!(b2.result(), Ok(&vec!['a', 'b']));
    }

    #[test]
    #[should_panic(expected = "next called on Dark beam")]
    fn next_on_dark_panics() {
        let b: Pure<(), u32> = PureBeam::dark((), "err".to_string());
        let _ = b.next(0u32);
    }

    #[test]
    #[should_panic(expected = "advance called on Dark beam")]
    fn advance_on_dark_panics() {
        let b: Pure<(), u32> = PureBeam::dark((), "err".to_string());
        let _ = b.advance(Luminosity::<u32, String>::Radiant(0));
    }

    // --- Luminosity::combine ---

    #[test]
    fn combine_radiant_passes_through() {
        let a = Luminosity::<u32, String>::Radiant(1);
        let b = Luminosity::<&str, String>::Dimmed("x", ShannonLoss::new(2.0));
        let c = a.combine(b);
        assert_eq!(c.loss().as_f64(), 2.0);
    }

    #[test]
    fn combine_dimmed_accumulates() {
        let a = Luminosity::<u32, String>::Dimmed(1, ShannonLoss::new(1.0));
        let b = Luminosity::<u32, String>::Radiant(2);
        let c = a.combine(b);
        assert_eq!(c.loss().as_f64(), 1.0);
        assert_eq!(c.result_ref(), Ok(&2));
    }

    #[test]
    #[should_panic(expected = "combine called on Dark luminosity")]
    fn combine_dark_panics() {
        let a = Luminosity::<u32, &str>::Dark("boom");
        let b = Luminosity::<u32, &str>::Radiant(1);
        let _ = a.combine(b);
    }
}

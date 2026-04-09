//! Beam — the pipeline value carrier.
//!
//! `Beam` is a trait. Two implementations:
//!
//! - `PureBeam<In, Out, E>` — prod. Input + output. No trace overhead.
//! - `TraceBeam<In, Out, E>` — debug. Same fields + full `Trace`. (forthcoming)
//!
//! Three states in both:
//!
//! - `Radiant` — full-strength value, zero loss.
//! - `Dimmed`  — value present, information was lost getting here.
//! - `Dark`    — the step failed. Input stored. Output phantom.
//!
//! The type chain is enforced by the compiler:
//!
//!   `type Next<T>: Beam<In = Self::Out, Out = T, Error = Self::Error>`
//!
//! Each prism step's output type becomes the next step's input type.
//! You cannot wire prisms together incorrectly at compile time.

use std::convert::Infallible;
use std::marker::PhantomData;

use crate::loss::ShannonLoss;

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
/// # Factory methods
///
/// Prisms construct the next beam using `advance`, `advance_lossy`, `fail`,
/// and their `_err` variants. These consume `self` and return `Self::Next<T>`
/// or `Self::NextWithError<T, E>`.
///
/// **Contract:** `advance*` and `fail_err` must only be called on non-Dark beams.
/// They panic in debug mode if called on Dark. Prisms should call `is_ok()`
/// before advancing.
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
    /// For Dark beams: the input at the time of failure.
    fn input(&self) -> &Self::In;

    /// Information loss at this step.
    /// Zero for Radiant, positive for Dimmed, infinite for Dark.
    fn loss(&self) -> ShannonLoss;

    /// The output of this step, or the error if Dark.
    fn result(&self) -> Result<&Self::Out, &Self::Error>;

    /// Whether this beam is carrying a value (Radiant or Dimmed).
    fn is_ok(&self) -> bool {
        self.result().is_ok()
    }

    /// Whether this beam failed (Dark).
    fn is_dark(&self) -> bool {
        self.result().is_err()
    }

    // --- Factory: same error type ---

    /// Advance to a new output value.
    /// The current `Out` becomes the new `In`. Loss state preserved.
    fn advance<T>(self, output: T) -> Self::Next<T>;

    /// Advance with additional information loss.
    /// Radiant → Dimmed. Dimmed loss accumulates.
    fn advance_lossy<T>(self, output: T, loss: ShannonLoss) -> Self::Next<T>;

    /// Fail: transition to Dark with an error.
    /// Stores the current `In` in the Dark beam.
    fn fail(self, error: Self::Error) -> Self;

    // --- Factory: new error type ---

    /// Advance to a new output and error type.
    /// The current `Out` becomes the new `In`. Error type changes.
    fn advance_err<T, E>(self, output: T) -> Self::NextWithError<T, E>;

    /// Advance with loss and a new error type.
    fn advance_lossy_err<T, E>(self, output: T, loss: ShannonLoss) -> Self::NextWithError<T, E>;

    /// Fail with a new error type.
    /// The current `Out` becomes the Dark beam's `In`
    /// (the value we were trying to transform when failure occurred).
    fn fail_err<T, E>(self, error: E) -> Self::NextWithError<T, E>;
}

// ---------------------------------------------------------------------------
// PureBeam
// ---------------------------------------------------------------------------

/// Production beam. Input + output, no trace overhead.
///
/// The `Out` type parameter is phantom in the `Dark` variant —
/// there is no output value when a step fails. The type is still
/// tracked for the type chain.
pub enum PureBeam<In, Out, E = Infallible> {
    Radiant { input: In, output: Out },
    Dimmed  { input: In, output: Out, loss: ShannonLoss },
    Dark    { input: In, error: E, _out: PhantomData<Out> },
}

impl<In, Out, E> PureBeam<In, Out, E> {
    /// Construct a lossless Radiant beam.
    pub fn radiant(input: In, output: Out) -> Self {
        Self::Radiant { input, output }
    }

    /// Construct a Dimmed beam (value with information loss).
    pub fn dimmed(input: In, output: Out, loss: ShannonLoss) -> Self {
        Self::Dimmed { input, output, loss }
    }

    /// Construct a Dark beam (step failed).
    pub fn dark(input: In, error: E) -> Self {
        Self::Dark { input, error, _out: PhantomData }
    }
}

impl<In, Out, E> Beam for PureBeam<In, Out, E> {
    type In = In;
    type Out = Out;
    type Error = E;
    type Next<T> = PureBeam<Out, T, E>;
    type NextWithError<T, NE> = PureBeam<Out, T, NE>;

    fn input(&self) -> &In {
        match self {
            Self::Radiant { input, .. }
            | Self::Dimmed { input, .. }
            | Self::Dark { input, .. } => input,
        }
    }

    fn loss(&self) -> ShannonLoss {
        match self {
            Self::Radiant { .. } => ShannonLoss::zero(),
            Self::Dimmed { loss, .. } => loss.clone(),
            Self::Dark { .. } => ShannonLoss::new(f64::INFINITY),
        }
    }

    fn result(&self) -> Result<&Out, &E> {
        match self {
            Self::Radiant { output, .. } | Self::Dimmed { output, .. } => Ok(output),
            Self::Dark { error, .. } => Err(error),
        }
    }

    fn advance<T>(self, output: T) -> PureBeam<Out, T, E> {
        match self {
            Self::Radiant { output: old, .. } =>
                PureBeam::Radiant { input: old, output },
            Self::Dimmed { output: old, loss, .. } =>
                PureBeam::Dimmed { input: old, output, loss },
            Self::Dark { .. } =>
                panic!("advance called on Dark beam — check is_ok() first"),
        }
    }

    fn advance_lossy<T>(self, output: T, extra: ShannonLoss) -> PureBeam<Out, T, E> {
        match self {
            Self::Radiant { output: old, .. } =>
                PureBeam::Dimmed { input: old, output, loss: extra },
            Self::Dimmed { output: old, loss, .. } =>
                PureBeam::Dimmed { input: old, output, loss: loss + extra },
            Self::Dark { .. } =>
                panic!("advance_lossy called on Dark beam — check is_ok() first"),
        }
    }

    fn fail(self, error: E) -> Self {
        match self {
            Self::Radiant { input, .. } | Self::Dimmed { input, .. } =>
                Self::Dark { input, error, _out: PhantomData },
            Self::Dark { .. } =>
                panic!("fail called on already-Dark beam"),
        }
    }

    fn advance_err<T, NE>(self, output: T) -> PureBeam<Out, T, NE> {
        match self {
            Self::Radiant { output: old, .. } =>
                PureBeam::Radiant { input: old, output },
            Self::Dimmed { output: old, loss, .. } =>
                PureBeam::Dimmed { input: old, output, loss },
            Self::Dark { .. } =>
                panic!("advance_err called on Dark beam — check is_ok() first"),
        }
    }

    fn advance_lossy_err<T, NE>(self, output: T, extra: ShannonLoss) -> PureBeam<Out, T, NE> {
        match self {
            Self::Radiant { output: old, .. } =>
                PureBeam::Dimmed { input: old, output, loss: extra },
            Self::Dimmed { output: old, loss, .. } =>
                PureBeam::Dimmed { input: old, output, loss: loss + extra },
            Self::Dark { .. } =>
                panic!("advance_lossy_err called on Dark beam — check is_ok() first"),
        }
    }

    fn fail_err<T, NE>(self, error: NE) -> PureBeam<Out, T, NE> {
        match self {
            Self::Radiant { output, .. } | Self::Dimmed { output, .. } =>
                PureBeam::Dark { input: output, error, _out: PhantomData },
            Self::Dark { .. } =>
                panic!("fail_err called on already-Dark beam"),
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
    fn radiant_is_ok() {
        let b: Pure<(), u32> = PureBeam::radiant((), 42);
        assert!(b.is_ok());
        assert!(!b.is_dark());
        assert_eq!(b.result(), Ok(&42));
        assert_eq!(b.input(), &());
        assert!(b.loss().is_zero());
    }

    #[test]
    fn dimmed_is_ok_with_loss() {
        let b: Pure<(), u32> = PureBeam::dimmed((), 42, ShannonLoss::new(1.5));
        assert!(b.is_ok());
        assert_eq!(b.result(), Ok(&42));
        assert_eq!(b.loss().as_f64(), 1.5);
    }

    #[test]
    fn dark_is_err() {
        let b: Pure<(), u32> = PureBeam::dark((), "oops".to_string());
        assert!(b.is_dark());
        assert!(!b.is_ok());
        assert_eq!(b.result(), Err(&"oops".to_string()));
        assert!(b.loss().as_f64().is_infinite());
    }

    #[test]
    fn advance_radiant_preserves_state() {
        let b: PureBeam<(), u32, String> = PureBeam::radiant((), 10);
        let next = b.advance("hello");
        assert!(next.is_ok());
        assert_eq!(next.result(), Ok(&"hello"));
        assert_eq!(next.input(), &10u32);
        assert!(next.loss().is_zero());
    }

    #[test]
    fn advance_dimmed_preserves_loss() {
        let b: Pure<(), u32> = PureBeam::dimmed((), 10, ShannonLoss::new(2.0));
        let next = b.advance(20u32);
        assert_eq!(next.loss().as_f64(), 2.0);
        assert_eq!(next.input(), &10u32);
    }

    #[test]
    fn advance_lossy_radiant_becomes_dimmed() {
        let b: PureBeam<(), u32, String> = PureBeam::radiant((), 5);
        let next = b.advance_lossy("x", ShannonLoss::new(0.5));
        assert!(next.is_ok());
        assert_eq!(next.loss().as_f64(), 0.5);
    }

    #[test]
    fn advance_lossy_dimmed_accumulates() {
        let b: Pure<(), u32> = PureBeam::dimmed((), 5, ShannonLoss::new(1.0));
        let next = b.advance_lossy(10u32, ShannonLoss::new(0.5));
        assert_eq!(next.loss().as_f64(), 1.5);
    }

    #[test]
    fn fail_transitions_to_dark() {
        let b: Pure<(), u32> = PureBeam::radiant((), 99);
        let dark = b.fail("bang".to_string());
        assert!(dark.is_dark());
        assert_eq!(dark.result(), Err(&"bang".to_string()));
        assert_eq!(dark.input(), &());
    }

    #[test]
    fn fail_err_carries_output_as_new_input() {
        let b: PureBeam<(), u32, String> = PureBeam::radiant((), 42);
        let dark: PureBeam<u32, &str, i32> = b.fail_err::<&str, i32>(-1);
        assert!(dark.is_dark());
        // The Dark beam's input is the old output (42)
        assert_eq!(dark.input(), &42u32);
        assert_eq!(dark.result(), Err(&-1i32));
    }

    #[test]
    fn advance_err_changes_error_type() {
        let b: PureBeam<(), u32, String> = PureBeam::radiant((), 7);
        let next: PureBeam<u32, &str, i32> = b.advance_err::<&str, i32>("hello");
        assert!(next.is_ok());
        assert_eq!(next.result(), Ok(&"hello"));
        assert_eq!(next.input(), &7u32);
    }

    #[test]
    fn type_chain_three_steps() {
        // () → u32 → String → Vec<char>
        let b0: PureBeam<(), u32> = PureBeam::radiant((), 42u32);
        let b1: PureBeam<u32, String> = b0.advance("hello".to_string());
        let b2: PureBeam<String, Vec<char>> = b1.advance(vec!['a', 'b']);
        assert_eq!(b2.input(), &"hello".to_string());
        assert_eq!(b2.result(), Ok(&vec!['a', 'b']));
    }

    #[test]
    #[should_panic(expected = "advance called on Dark beam")]
    fn advance_on_dark_panics() {
        let b: Pure<(), u32> = PureBeam::dark((), "err".to_string());
        let _ = b.advance(0u32);
    }

    #[test]
    #[should_panic(expected = "fail called on already-Dark beam")]
    fn double_fail_panics() {
        let b: Pure<(), u32> = PureBeam::dark((), "first".to_string());
        let _ = b.fail("second".to_string());
    }
}

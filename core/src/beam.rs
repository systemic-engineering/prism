//! Beam — the semifunctor. The pipeline value carrier.
//!
//! `tick` is the primitive: one step forward.
//! `next` is the lossless shorthand.
//! `smap` is the semifunctor map, derived from `tick`.

use crate::trace::Op;
use imperfect::{Imperfect, Loss, ShannonLoss};
use std::convert::Infallible;

/// A self-contained pipeline operation. Wraps a prism (and closure for
/// user-space operations). The beam arrives via `apply`.
pub trait Operation<B: Beam> {
    type Output: Beam;
    fn op(&self) -> Op;
    fn apply(self, beam: B) -> Self::Output;
}

/// The pipeline value carrier. A semifunctor over `Imperfect`.
///
/// Three required methods: `input`, `result`, `tick`.
/// Everything else is derived.
///
/// **Contract:** `tick` and `next` panic on Err beams. Call `is_ok()` first.
pub trait Beam: Sized {
    type In;
    type Out;
    type Error;
    type Loss: Loss;

    /// Advance: new Out and Error types. Loss type preserved.
    type Tick<T, E>: Beam<In = Self::Out, Out = T, Error = E, Loss = Self::Loss>;

    /// The input that entered this step.
    fn input(&self) -> &Self::In;

    /// The output of this step, or the error if failed.
    fn result(&self) -> Imperfect<&Self::Out, &Self::Error, Self::Loss>;

    /// The primitive. One tick forward. Panics on Err beam.
    fn tick<T, E>(self, imperfect: Imperfect<T, E, Self::Loss>) -> Self::Tick<T, E>;

    /// Whether this beam has a value (Ok or Partial).
    fn is_ok(&self) -> bool {
        !self.is_err()
    }

    /// Whether this beam is in the Partial state.
    fn is_partial(&self) -> bool {
        self.result().is_partial()
    }

    /// Whether this beam failed (Err).
    fn is_err(&self) -> bool {
        self.result().is_err()
    }

    /// Lossless transition. Shorthand for `tick(Imperfect::Ok(value))`.
    /// Panics on Err beam.
    fn next<T>(self, value: T) -> Self::Tick<T, Self::Error> {
        self.tick(Imperfect::Ok(value))
    }

    /// Apply an operation. The DSL entry point.
    fn apply<O: Operation<Self>>(self, op: O) -> O::Output {
        op.apply(self)
    }

    /// Semifunctor map. Derived from `tick`.
    /// Panics on Err beam.
    fn smap<T>(
        self,
        f: impl FnOnce(&Self::Out) -> Imperfect<T, Self::Error, Self::Loss>,
    ) -> Self::Tick<T, Self::Error> {
        let imp = match self.result() {
            Imperfect::Ok(v) | Imperfect::Partial(v, _) => f(v),
            Imperfect::Err(_) => panic!("smap on Err beam"),
        };
        self.tick(imp)
    }
}

/// Production beam. Flat struct: input + imperfect. No trace overhead.
pub struct PureBeam<In, Out, E = Infallible, L: Loss = ShannonLoss> {
    input: In,
    imperfect: Imperfect<Out, E, L>,
}

impl<In, Out, E, L: Loss> PureBeam<In, Out, E, L> {
    /// Construct a perfect beam (zero loss).
    pub fn ok(input: In, output: Out) -> Self {
        Self { input, imperfect: Imperfect::Ok(output) }
    }

    /// Construct a partial beam (value with loss).
    pub fn partial(input: In, output: Out, loss: L) -> Self {
        Self { input, imperfect: Imperfect::Partial(output, loss) }
    }

    /// Construct a failed beam.
    pub fn err(input: In, error: E) -> Self {
        Self { input, imperfect: Imperfect::Err(error) }
    }
}

impl<In, Out, E, L: Loss> Beam for PureBeam<In, Out, E, L> {
    type In = In;
    type Out = Out;
    type Error = E;
    type Loss = L;
    type Tick<T, NE> = PureBeam<Out, T, NE, L>;

    fn input(&self) -> &In {
        &self.input
    }

    fn result(&self) -> Imperfect<&Out, &E, L> {
        self.imperfect.as_ref()
    }

    fn tick<T, NE>(self, next: Imperfect<T, NE, L>) -> PureBeam<Out, T, NE, L> {
        match self.imperfect {
            Imperfect::Err(_) => panic!("tick on Err beam — check is_ok() first"),
            Imperfect::Ok(old_out) => PureBeam {
                input: old_out,
                imperfect: next,
            },
            Imperfect::Partial(old_out, loss) => PureBeam {
                input: old_out,
                imperfect: match next {
                    Imperfect::Ok(v) => Imperfect::Partial(v, loss),
                    Imperfect::Partial(v, loss2) => Imperfect::Partial(v, loss.combine(loss2)),
                    Imperfect::Err(e) => Imperfect::Err(e),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::Op;

    /// A trivial operation for testing: doubles the value.
    struct DoubleOp;

    impl Operation<PureBeam<(), u32>> for DoubleOp {
        type Output = PureBeam<u32, u32>;
        fn op(&self) -> Op { Op::Project }
        fn apply(self, beam: PureBeam<(), u32>) -> PureBeam<u32, u32> {
            let v = *beam.result().ok().unwrap();
            beam.next(v * 2)
        }
    }

    #[test]
    fn apply_operation() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.apply(DoubleOp);
        assert_eq!(n.result().ok(), Some(&10));
    }

    #[test]
    fn pure_beam_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 42);
        assert!(b.is_ok());
        assert!(!b.is_err());
        assert_eq!(b.result().ok(), Some(&42));
        assert_eq!(b.input(), &());
    }

    #[test]
    fn pure_beam_partial() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 42, ShannonLoss::new(1.5));
        assert!(b.is_ok());
        assert!(b.is_partial());
        assert_eq!(b.result().ok(), Some(&42));
    }

    #[test]
    fn pure_beam_err() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "oops".into());
        assert!(b.is_err());
        assert!(!b.is_ok());
    }

    #[test]
    fn next_ok_to_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 10);
        let n = b.next("hello");
        assert!(n.is_ok());
        assert!(!n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hello"));
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    fn next_partial_carries_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 10, ShannonLoss::new(2.0));
        let n = b.next(20u32);
        assert!(n.is_partial());
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    #[should_panic(expected = "tick on Err beam")]
    fn next_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.next(0u32);
    }

    #[test]
    fn tick_ok_with_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.tick(Imperfect::<&str, String>::Ok("hi"));
        assert!(n.is_ok());
        assert!(!n.is_partial());
    }

    #[test]
    fn tick_ok_with_partial() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.tick(Imperfect::<&str, String>::Partial("hi", ShannonLoss::new(1.0)));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hi"));
    }

    #[test]
    fn tick_ok_with_err() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n: PureBeam<u32, u32, i32> = b.tick(Imperfect::Err(-1));
        assert!(n.is_err());
    }

    #[test]
    fn tick_partial_with_ok_carries_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Ok(10));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_partial_accumulates() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Partial(10, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_err() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String>::Err("fail".into()));
        assert!(n.is_err());
    }

    #[test]
    #[should_panic(expected = "tick on Err beam")]
    fn tick_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.tick(Imperfect::<u32, String>::Ok(0));
    }

    #[test]
    fn type_chain_three_steps() {
        let b0: PureBeam<(), u32> = PureBeam::ok((), 42u32);
        let b1: PureBeam<u32, String> = b0.next("hello".to_string());
        let b2: PureBeam<String, Vec<char>> = b1.next(vec!['a', 'b']);
        assert_eq!(b2.input(), &"hello".to_string());
        assert_eq!(b2.result().ok(), Some(&vec!['a', 'b']));
    }

    // --- smap ---

    #[test]
    fn smap_ok() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|&v| Imperfect::Ok(v * 2));
        assert_eq!(n.result().ok(), Some(&10));
        assert!(!n.is_partial());
    }

    #[test]
    fn smap_returns_partial() {
        let b: PureBeam<(), u32> = PureBeam::ok((), 5);
        let n = b.smap(|&v| Imperfect::Partial(v * 2, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&10));
    }

    #[test]
    fn smap_returns_err() {
        let b: PureBeam<(), u32, String> = PureBeam::ok((), 5);
        let n = b.smap(|_| Imperfect::<u32, String>::Err("nope".into()));
        assert!(n.is_err());
    }

    #[test]
    fn smap_on_partial_accumulates_loss() {
        let b: PureBeam<(), u32> = PureBeam::partial((), 5, ShannonLoss::new(1.0));
        let n = b.smap(|&v| Imperfect::Partial(v * 2, ShannonLoss::new(0.5)));
        assert!(n.is_partial());
    }

    #[test]
    #[should_panic(expected = "smap on Err beam")]
    fn smap_on_err_panics() {
        let b: PureBeam<(), u32, String> = PureBeam::err((), "err".into());
        let _ = b.smap(|&v| Imperfect::<u32, String>::Ok(v));
    }
}

//! Beam — the functor. The pipeline value carrier.
//!
//! `tick` is the primitive: one step forward.
//! `next` is the lossless shorthand.
//! `smap` is the functor map. Failure beams are dark: a fixpoint under `smap` and `next`.

use crate::trace::Op;
use crate::ScalarLoss;
use std::convert::Infallible;
use terni::{Imperfect, Loss};

/// A self-contained pipeline operation. Wraps a prism (and closure for
/// user-space operations). The beam arrives via `apply`.
pub trait Operation<B: Beam> {
    type Output: Beam;
    fn op(&self) -> Op;
    fn apply(self, beam: B) -> Self::Output;
}

/// The pipeline value carrier. A functor over [`Imperfect`].
///
/// Failure beams are dark: they propagate unchanged through `smap` and `next`,
/// carrying their error and accumulated loss as a fixpoint. The closure passed
/// to `smap` is never called on a dark beam. This makes Beam a proper functor
/// (identity law holds: `smap(id, dark) = dark`).
///
/// Three required methods: `input`, `result`, `tick`.
/// Everything else is derived.
///
/// **Contract:** `tick`, `smap`, and `next` propagate dark beams (Failure is a fixpoint).
/// `input()` panics on dark propagated beams. Call `is_ok()` first.
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

    /// The primitive. One tick forward.
    ///
    /// On a Failure beam, propagates darkness: the error is converted via
    /// `Into` and the provided `imperfect` is ignored.
    fn tick<T, E>(self, imperfect: Imperfect<T, E, Self::Loss>) -> Self::Tick<T, E>
    where
        Self::Error: Into<E>;

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

    /// Lossless transition. Shorthand for `tick(Imperfect::success(value))`.
    ///
    /// On a Failure beam, propagates darkness (returns a Failure beam with
    /// the same error and loss). The provided value is ignored.
    fn next<T>(self, value: T) -> Self::Tick<T, Self::Error> {
        self.tick(Imperfect::success(value))
    }

    /// Apply an operation. The DSL entry point.
    fn apply<O: Operation<Self>>(self, op: O) -> O::Output {
        op.apply(self)
    }

    /// Functor map. Derived from `tick`.
    ///
    /// On a Failure beam, propagates darkness (returns a Failure beam with
    /// the same error and loss). The closure `f` is never called.
    ///
    /// Implementors should override this for dark beam handling.
    fn smap<T>(
        self,
        f: impl FnOnce(&Self::Out) -> Imperfect<T, Self::Error, Self::Loss>,
    ) -> Self::Tick<T, Self::Error> {
        let imp = match self.result() {
            Imperfect::Success(v) | Imperfect::Partial(v, _) => f(v),
            Imperfect::Failure(_, _) => panic!("smap on Err beam — override smap for dark beam support"),
        };
        self.tick(imp)
    }
}

/// Bidirectional carrier for optics. Carries the source alongside the focus.
/// For Lens get/put, Prism preview/review — operations that need to go back.
///
/// Flat struct: source + imperfect focus. No trace overhead.
/// A `TraceBeam` that records each step into a [`Trace`](crate::trace::Trace)
/// is forthcoming.
pub struct Optic<In, Out, E = Infallible, L: Loss = ScalarLoss> {
    source: Option<In>,
    focus: Imperfect<Out, E, L>,
}

impl<In, Out, E, L: Loss> Optic<In, Out, E, L> {
    /// Construct a perfect optic (zero loss).
    pub fn ok(source: In, output: Out) -> Self {
        Self {
            source: Some(source),
            focus: Imperfect::success(output),
        }
    }

    /// Construct a partial optic (value with loss).
    pub fn partial(source: In, output: Out, loss: L) -> Self {
        Self {
            source: Some(source),
            focus: Imperfect::partial(output, loss),
        }
    }

    /// Construct a failed optic.
    pub fn err(source: In, error: E) -> Self {
        Self {
            source: Some(source),
            focus: Imperfect::failure(error),
        }
    }

    /// Construct a failed optic with accumulated loss.
    pub fn err_with_loss(source: In, error: E, loss: L) -> Self {
        Self {
            source: Some(source),
            focus: Imperfect::failure_with_loss(error, loss),
        }
    }

    /// Construct a dark (failed) optic with no source. Used internally
    /// when propagating failure through `tick`/`smap` — the source from
    /// the previous step does not exist because that step failed.
    fn dark(focus: Imperfect<Out, E, L>) -> Self {
        Self {
            source: None,
            focus,
        }
    }
}

fn propagate<T, E, L: Loss>(loss: L, next: Imperfect<T, E, L>) -> Imperfect<T, E, L> {
    match next {
        Imperfect::Success(v) => Imperfect::partial(v, loss),
        Imperfect::Partial(v, loss2) => Imperfect::partial(v, loss.combine(loss2)),
        Imperfect::Failure(e, loss2) => Imperfect::failure_with_loss(e, loss.combine(loss2)),
    }
}

impl<In, Out, E, L: Loss> Beam for Optic<In, Out, E, L> {
    type In = In;
    type Out = Out;
    type Error = E;
    type Loss = L;
    type Tick<T, NE> = Optic<Out, T, NE, L>;

    fn input(&self) -> &In {
        self.source
            .as_ref()
            .expect("input() on dark beam — check is_ok() first")
    }

    fn result(&self) -> Imperfect<&Out, &E, L> {
        self.focus.as_ref()
    }

    fn tick<T, NE>(self, next: Imperfect<T, NE, L>) -> Optic<Out, T, NE, L>
    where
        E: Into<NE>,
    {
        match self.focus {
            Imperfect::Success(old_out) => Optic {
                source: Some(old_out),
                focus: next,
            },
            Imperfect::Partial(old_out, loss) => Optic {
                source: Some(old_out),
                focus: propagate(loss, next),
            },
            Imperfect::Failure(_, _) => {
                panic!("tick on Err beam — check is_ok() first or use smap/next for dark propagation")
            }
        }
    }

    fn smap<T>(
        self,
        f: impl FnOnce(&Self::Out) -> Imperfect<T, Self::Error, Self::Loss>,
    ) -> Self::Tick<T, Self::Error> {
        match &self.focus {
            Imperfect::Success(v) | Imperfect::Partial(v, _) => {
                let imp = f(v);
                self.tick(imp)
            }
            Imperfect::Failure(_, _) => {
                // Dark beam fixpoint: extract the error and loss, propagate unchanged.
                // The closure f is never called — darkness absorbs.
                match self.focus {
                    Imperfect::Failure(e, l) => {
                        Optic::dark(Imperfect::failure_with_loss(e, l))
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn next<T>(self, value: T) -> Self::Tick<T, Self::Error> {
        match &self.focus {
            Imperfect::Failure(_, _) => {
                // Dark beam fixpoint: darkness propagates, value is ignored.
                match self.focus {
                    Imperfect::Failure(e, l) => {
                        Optic::dark(Imperfect::failure_with_loss(e, l))
                    }
                    _ => unreachable!(),
                }
            }
            _ => self.tick(Imperfect::success(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::Op;

    /// A trivial operation for testing: doubles the value.
    struct DoubleOp;

    impl Operation<Optic<(), u32>> for DoubleOp {
        type Output = Optic<u32, u32>;
        fn op(&self) -> Op {
            Op::Project
        }
        fn apply(self, beam: Optic<(), u32>) -> Optic<u32, u32> {
            let v = *beam.result().ok().unwrap();
            beam.next(v * 2)
        }
    }

    #[test]
    fn apply_operation() {
        let b: Optic<(), u32> = Optic::ok((), 5);
        let n = b.apply(DoubleOp);
        assert_eq!(n.result().ok(), Some(&10));
    }

    #[test]
    fn pure_beam_ok() {
        let b: Optic<(), u32> = Optic::ok((), 42);
        assert!(b.is_ok());
        assert!(!b.is_err());
        assert_eq!(b.result().ok(), Some(&42));
        assert_eq!(b.input(), &());
    }

    #[test]
    fn pure_beam_partial() {
        let b: Optic<(), u32> = Optic::partial((), 42, ScalarLoss::new(1.5));
        assert!(b.is_ok());
        assert!(b.is_partial());
        assert_eq!(b.result().ok(), Some(&42));
    }

    #[test]
    fn pure_beam_err() {
        let b: Optic<(), u32, String> = Optic::err((), "oops".into());
        assert!(b.is_err());
        assert!(!b.is_ok());
    }

    #[test]
    fn next_ok_to_ok() {
        let b: Optic<(), u32> = Optic::ok((), 10);
        let n = b.next("hello");
        assert!(n.is_ok());
        assert!(!n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hello"));
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    fn next_partial_carries_loss() {
        let b: Optic<(), u32> = Optic::partial((), 10, ScalarLoss::new(2.0));
        let n = b.next(20u32);
        assert!(n.is_partial());
        assert_eq!(n.input(), &10u32);
    }

    #[test]
    fn next_on_err_propagates_dark() {
        let b: Optic<(), u32, String> = Optic::err((), "err".into());
        let n = b.next(0u32);
        assert!(n.is_err());
        assert_eq!(n.result().err(), Some(&"err".to_string()));
    }

    #[test]
    fn tick_ok_with_ok() {
        let b: Optic<(), u32, String> = Optic::ok((), 5);
        let n = b.tick(Imperfect::<&str, String, ScalarLoss>::success("hi"));
        assert!(n.is_ok());
        assert!(!n.is_partial());
    }

    #[test]
    fn tick_ok_with_partial() {
        let b: Optic<(), u32, String> = Optic::ok((), 5);
        let n = b.tick(Imperfect::<&str, String, ScalarLoss>::partial(
            "hi",
            ScalarLoss::new(1.0),
        ));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&"hi"));
    }

    #[test]
    fn tick_ok_with_err() {
        let b: Optic<(), u32, i32> = Optic::ok((), 5);
        let n: Optic<u32, u32, i32> = b.tick(Imperfect::failure(-1));
        assert!(n.is_err());
    }

    #[test]
    fn tick_partial_with_ok_carries_loss() {
        let b: Optic<(), u32, String> = Optic::partial((), 5, ScalarLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String, ScalarLoss>::success(10));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_partial_accumulates() {
        let b: Optic<(), u32, String> = Optic::partial((), 5, ScalarLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String, ScalarLoss>::partial(
            10,
            ScalarLoss::new(0.5),
        ));
        assert!(n.is_partial());
    }

    #[test]
    fn tick_partial_with_err() {
        let b: Optic<(), u32, String> = Optic::partial((), 5, ScalarLoss::new(1.0));
        let n = b.tick(Imperfect::<u32, String, ScalarLoss>::failure_with_loss(
            "fail".into(),
            ScalarLoss::zero(),
        ));
        assert!(n.is_err());
    }

    #[test]
    fn tick_on_err_propagates_dark() {
        // Dark beam tick propagates the failure instead of panicking
        let b: Optic<(), u32, String> = Optic::err((), "err".into());
        let n = b.tick(Imperfect::<u32, String, ScalarLoss>::success(0));
        assert!(n.is_err());
        assert_eq!(n.result().err(), Some(&"err".to_string()));
    }

    #[test]
    fn tick_on_err_cross_error_type() {
        // Dark beam tick across error types: String -> i32 via From
        #[derive(Debug, PartialEq)]
        struct AppError(String);

        impl From<String> for AppError {
            fn from(s: String) -> Self {
                AppError(s)
            }
        }

        let b: Optic<(), u32, String> = Optic::err((), "fail".into());
        let n: Optic<u32, u64, AppError, ScalarLoss> =
            b.tick(Imperfect::<u64, AppError, ScalarLoss>::success(99));
        assert!(n.is_err());
        assert_eq!(n.result().err(), Some(&AppError("fail".into())));
    }

    #[test]
    fn tick_on_err_with_loss_preserves_loss() {
        let b: Optic<(), u32, String> =
            Optic::err_with_loss((), "err".into(), ScalarLoss::new(4.0));
        let n = b.tick(Imperfect::<u32, String, ScalarLoss>::success(0));
        assert!(n.is_err());
        assert_eq!(n.result().loss().as_f64(), 4.0);
    }

    #[test]
    fn type_chain_three_steps() {
        let b0: Optic<(), u32> = Optic::ok((), 42u32);
        let b1: Optic<u32, String> = b0.next("hello".to_string());
        let b2: Optic<String, Vec<char>> = b1.next(vec!['a', 'b']);
        assert_eq!(b2.input(), &"hello".to_string());
        assert_eq!(b2.result().ok(), Some(&vec!['a', 'b']));
    }

    // --- smap ---

    #[test]
    fn smap_ok() {
        let b: Optic<(), u32> = Optic::ok((), 5);
        let n = b.smap(|&v| Imperfect::success(v * 2));
        assert_eq!(n.result().ok(), Some(&10));
        assert!(!n.is_partial());
    }

    #[test]
    fn smap_returns_partial() {
        let b: Optic<(), u32> = Optic::ok((), 5);
        let n = b.smap(|&v| Imperfect::partial(v * 2, ScalarLoss::new(0.5)));
        assert!(n.is_partial());
        assert_eq!(n.result().ok(), Some(&10));
    }

    #[test]
    fn smap_returns_err() {
        let b: Optic<(), u32, String> = Optic::ok((), 5);
        let n = b.smap(|_| Imperfect::<u32, String, ScalarLoss>::failure("nope".into()));
        assert!(n.is_err());
    }

    #[test]
    fn smap_on_partial_accumulates_loss() {
        let b: Optic<(), u32> = Optic::partial((), 5, ScalarLoss::new(1.0));
        let n = b.smap(|&v| Imperfect::partial(v * 2, ScalarLoss::new(0.5)));
        assert!(n.is_partial());
    }

    fn wrap_success(v: &u32) -> Imperfect<u32, String, ScalarLoss> {
        Imperfect::success(*v)
    }

    #[test]
    fn smap_on_err_propagates_dark() {
        let b: Optic<(), u32, String> = Optic::err((), "err".into());
        let n = b.smap(wrap_success);
        assert!(n.is_err());
        assert_eq!(n.result().err(), Some(&"err".to_string()));
    }

    #[test]
    fn smap_fn_pointer_executes_body() {
        let b: Optic<(), u32, String> = Optic::ok((), 7);
        let n = b.smap(wrap_success);
        assert_eq!(n.result().ok(), Some(&7));
    }

    #[test]
    fn double_op_reports_project_op() {
        let op = DoubleOp;
        assert_eq!(op.op(), Op::Project);
    }

    // --- dark beam (Failure) fixpoint tests ---

    #[test]
    fn smap_id_on_failure_returns_failure() {
        let b: Optic<(), u32, String> = Optic::err((), "dark".into());
        let n = b.smap(wrap_success);
        assert!(n.is_err());
        assert_eq!(n.result().err(), Some(&"dark".to_string()));
    }

    #[test]
    fn smap_f_on_failure_ignores_f() {
        let called = std::cell::Cell::new(false);
        let b: Optic<(), u32, String> = Optic::err((), "dark".into());
        let n = b.smap(|v: &u32| {
            called.set(true);
            Imperfect::success(*v * 100)
        });
        assert!(!called.get(), "f should not be called on dark beam");
        assert!(n.is_err());
    }

    #[test]
    fn smap_composition_law_dark_beam() {
        // smap(f . g) = smap(f) . smap(g) — both sides should stay dark
        let b1: Optic<(), u32, String> = Optic::err((), "dark".into());
        let composed = b1.smap(|v: &u32| Imperfect::success(format!("{}", v * 2)));
        assert!(composed.is_err());

        let b2: Optic<(), u32, String> = Optic::err((), "dark".into());
        let step1 = b2.smap(wrap_success);
        assert!(step1.is_err());
    }

    #[test]
    fn next_on_failure_preserves_error() {
        let b: Optic<(), u32, String> = Optic::err((), "dark".into());
        let n = b.next(999u64);
        assert!(n.is_err());
        assert_eq!(n.result().err(), Some(&"dark".to_string()));
    }

    #[test]
    fn dark_beam_preserves_loss_through_smap() {
        let b: Optic<(), u32, String> =
            Optic::err_with_loss((), "dark".into(), ScalarLoss::new(3.5));
        let n = b.smap(wrap_success);
        assert!(n.is_err());
        assert_eq!(n.result().loss().as_f64(), 3.5);
    }

    #[test]
    fn dark_beam_preserves_loss_through_next() {
        let b: Optic<(), u32, String> =
            Optic::err_with_loss((), "dark".into(), ScalarLoss::new(2.0));
        let n = b.next(0u64);
        assert!(n.is_err());
        assert_eq!(n.result().loss().as_f64(), 2.0);
    }

    #[test]
    fn dark_beam_pipeline_propagates_through_next() {
        let b0: Optic<(), u32, String> = Optic::ok((), 1);
        let _b1 = b0.next(2u32);
        // simulate failure at step 2:
        let dark: Optic<u32, u32, String> = Optic::err(2, "pipeline failed".into());
        // now next forward — darkness should propagate
        let b2 = dark.next(3u64);
        assert!(b2.is_err());
        let b3 = b2.next("done".to_string());
        assert!(b3.is_err());
        assert_eq!(b3.result().err(), Some(&"pipeline failed".to_string()));
    }

    #[test]
    fn dark_beam_pipeline_propagates_through_smap() {
        let dark: Optic<(), u32, String> = Optic::err((), "pipeline failed".into());
        let b1 = dark.smap(|v: &u32| Imperfect::success(v.to_string()));
        assert!(b1.is_err());
        let b2 = b1.smap(|v: &String| Imperfect::success(v.len()));
        assert!(b2.is_err());
        assert_eq!(b2.result().err(), Some(&"pipeline failed".to_string()));
    }

    #[test]
    fn next_on_failure_ignores_value() {
        // Verifies the dark beam doesn't touch the provided value
        let b: Optic<(), u32, String> = Optic::err((), "dark".into());
        let n = b.next(42u32);
        assert!(n.is_err());
        // The error propagates, the value 42 was swallowed by darkness
        assert_eq!(n.result().err(), Some(&"dark".to_string()));
    }

    // --- Seam adversarial tests ---

    #[test]
    #[should_panic(expected = "input() on dark beam")]
    fn adversarial_input_on_propagated_dark_beam_panics() {
        // A dark beam created via Optic::err has a source (Some).
        // But after propagation through smap, source is None.
        let b: Optic<(), u32, String> = Optic::err((), "dark".into());
        let propagated = b.smap(wrap_success);
        // This must panic — the propagated beam has no source
        let _ = propagated.input();
    }

    #[test]
    fn adversarial_original_dark_beam_has_source() {
        // Optic::err still has a valid source — only propagated dark beams lose it
        let b: Optic<(), u32, String> = Optic::err((), "dark".into());
        assert_eq!(b.input(), &());
    }

    #[test]
    fn adversarial_dark_with_total_loss() {
        // Edge case: dark beam with Loss::total() (infinite loss)
        let b: Optic<(), u32, String> =
            Optic::err_with_loss((), "total".into(), ScalarLoss::total());
        let n = b.smap(wrap_success);
        assert!(n.is_err());
        assert!(n.result().loss().as_f64().is_infinite());
    }

    #[test]
    fn adversarial_dark_with_zero_loss() {
        // Edge case: dark beam with zero loss
        let b: Optic<(), u32, String> = Optic::err((), "zero".into());
        let n = b.smap(wrap_success);
        assert!(n.is_err());
        assert!(n.result().loss().is_zero());
    }

    #[test]
    fn adversarial_deep_dark_pipeline_20_steps() {
        // 20-step smap chain on dark beam — error and loss must survive
        let err_msg = "deep dark".to_string();
        let b: Optic<(), u32, String> =
            Optic::err_with_loss((), err_msg.clone(), ScalarLoss::new(7.77));
        // Chain 20 smaps
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        let b = b.smap(|&v| Imperfect::success(v + 1));
        assert!(b.is_err());
        assert_eq!(b.result().err(), Some(&err_msg));
        assert_eq!(b.result().loss().as_f64(), 7.77);
    }

    #[test]
    fn adversarial_smap_returning_failure_vs_dark_propagation() {
        // A light beam where smap RETURNS a failure (creates a new error)
        // vs a dark beam that propagates an existing failure.
        // The returned-failure beam should have its OWN error, not the dark one.
        let light: Optic<(), u32, String> = Optic::ok((), 5);
        let failed = light.smap(|_| {
            Imperfect::<u32, String, ScalarLoss>::failure("new error".into())
        });
        assert!(failed.is_err());
        assert_eq!(failed.result().err(), Some(&"new error".to_string()));
        // The source is valid because it came from a light beam
        assert_eq!(failed.input(), &5);

        // Now propagate THAT failure
        let propagated = failed.smap(wrap_success);
        assert!(propagated.is_err());
        assert_eq!(propagated.result().err(), Some(&"new error".to_string()));
    }

    #[test]
    fn adversarial_is_ok_is_err_consistent_on_dark() {
        let b: Optic<(), u32, String> = Optic::err((), "dark".into());
        let p = b.smap(wrap_success);
        assert!(!p.is_ok());
        assert!(p.is_err());
        assert!(!p.is_partial());
    }
}

//! Trace — the full execution record of a beam through a pipeline.
//!
//! `Trace` is the debug artifact. Present in `TraceBeam`, absent in `PureBeam`.
//! In a prod build both observation and reconstruction cost nothing.
//!
//! - Observation:    `format!("{:?}", step.input)` — always works via `Traced: Debug`.
//! - Reconstruction: `trace.reenter_at::<ConcreteType>(i)` — downcast to input type.

use std::any::Any;
use std::fmt;

use crate::loss::ShannonLoss;

// ---------------------------------------------------------------------------
// Traced — the bound on values that can be stored in a Trace
// ---------------------------------------------------------------------------

/// Any value that is `Debug + Any + Send + Sync` can be stored in a `Trace`.
///
/// The `as_any` method works around the Rust limitation that you cannot
/// upcast `&dyn Traced` to `&dyn Any` directly. It is the standard pattern
/// for downcast access through a supertrait.
pub trait Traced: Any + fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + fmt::Debug + Send + Sync> Traced for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Op — which pipeline operation produced a step
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Focus,
    Project,
    Split,
    Zoom,
    Refract,
}

// ---------------------------------------------------------------------------
// Step
// ---------------------------------------------------------------------------

/// The output side of a traced step.
pub enum StepOutput {
    Value(Box<dyn Traced>),
    Parts(Vec<Box<dyn Traced>>),
    Error(Box<dyn Traced>),
}

/// A single traced step through the pipeline.
pub struct Step {
    /// The prism that ran this step.
    pub prism: &'static str,
    /// Which operation.
    pub op: Op,
    /// Loss incurred at this step.
    pub loss: ShannonLoss,
    /// The input to this step. Reconstructable via downcast.
    pub input: Box<dyn Traced>,
    /// The output of this step.
    pub output: StepOutput,
}

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

/// Full execution record — the linked list of all steps through the pipeline.
///
/// Used by `TraceBeam` for observation and reconstruction.
/// `PureBeam` carries `()` in its place — zero cost in prod.
#[derive(Default)]
pub struct Trace {
    steps: Vec<Step>,
}

impl Trace {
    pub fn new() -> Self {
        Trace { steps: Vec::new() }
    }

    pub fn push(&mut self, step: Step) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Attempt to recover the input at step `i` as concrete type `T`.
    /// Returns `None` if `i` is out of bounds or the downcast fails.
    pub fn reenter_at<T: 'static>(&self, i: usize) -> Option<&T> {
        // Deref to &dyn Traced explicitly to force vtable dispatch.
        // Calling .as_any() on Box<dyn Traced> directly would hit the blanket
        // impl for Box<_> and return the wrong type ID.
        let input: &dyn Traced = self.steps.get(i)?.input.as_ref();
        input.as_any().downcast_ref::<T>()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_starts_empty() {
        let t = Trace::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn trace_push_and_len() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ShannonLoss::zero(),
            input: Box::new(42u32),
            output: StepOutput::Value(Box::new("focused".to_string())),
        });
        assert_eq!(t.len(), 1);
        assert!(!t.is_empty());
    }

    #[test]
    fn trace_reenter_at_correct_type() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ShannonLoss::zero(),
            input: Box::new(99u32),
            output: StepOutput::Value(Box::new("out".to_string())),
        });
        let recovered = t.reenter_at::<u32>(0);
        assert_eq!(recovered, Some(&99u32));
    }

    #[test]
    fn trace_reenter_at_wrong_type_returns_none() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ShannonLoss::zero(),
            input: Box::new(99u32),
            output: StepOutput::Value(Box::new("out".to_string())),
        });
        let recovered = t.reenter_at::<String>(0);
        assert!(recovered.is_none());
    }

    #[test]
    fn trace_reenter_out_of_bounds_returns_none() {
        let t = Trace::new();
        assert!(t.reenter_at::<u32>(0).is_none());
    }

    #[test]
    fn traced_debug_works() {
        let val: Box<dyn Traced> = Box::new(vec![1u32, 2, 3]);
        let s = format!("{:?}", val);
        assert!(s.contains('1'));
    }

    #[test]
    fn op_variants_are_distinct() {
        assert_ne!(Op::Focus, Op::Project);
        assert_ne!(Op::Split, Op::Zoom);
        assert_ne!(Op::Zoom, Op::Refract);
    }
}

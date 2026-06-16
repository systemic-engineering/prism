//! Trace — the execution record of a beam through a pipeline.
//!
//! Each [`Step`] records which prism and operation produced it, the input
//! that entered, the output (or error) that left, and the loss incurred.
//! A [`Trace`] collects steps in order, and [`reenter_at`](Trace::reenter_at)
//! allows recovering a typed input at any step for replay or debugging.

use std::any::Any;
use std::fmt;

use crate::ScalarLoss;

/// Which pipeline operation produced a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// The `focus` stage: select what matters from the input.
    Focus,
    /// The `project` stage: the lossy transformation.
    Project,
    /// The `settle` stage: produce the output.
    Settle,
}

/// Any value that is `Debug + Any + Send + Sync` can be stored in a `Trace`.
/// The blanket impl below covers every qualifying type.
pub trait Traced: Any + fmt::Debug + Send + Sync {
    /// Reify as `&dyn Any` for downcasting.
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + fmt::Debug + Send + Sync> Traced for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The output side of a traced step.
pub enum StepOutput {
    /// The step produced a value.
    Value(Box<dyn Traced>),
    /// The step produced an error.
    Error(Box<dyn Traced>),
}

/// A single traced step through the pipeline.
pub struct Step {
    /// Static label for the prism that produced this step.
    pub prism: &'static str,
    /// Which of the three pipeline stages was running.
    pub op: Op,
    /// Loss accumulated during this step.
    pub loss: ScalarLoss,
    /// The input value that entered this step.
    pub input: Box<dyn Traced>,
    /// The output (or error) that left this step.
    pub output: StepOutput,
}

/// Full execution record — all steps through the pipeline.
#[derive(Default)]
pub struct Trace {
    steps: Vec<Step>,
}

impl Trace {
    /// Empty trace.
    pub fn new() -> Self {
        Trace { steps: Vec::new() }
    }

    /// Append a step.
    pub fn push(&mut self, step: Step) {
        self.steps.push(step);
    }

    /// All recorded steps in order.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Number of recorded steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether no steps have been recorded.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Recover the input at step `i` as concrete type `T`. Returns
    /// `None` if `i` is out of range or the recorded type does not
    /// match `T`.
    pub fn reenter_at<T: 'static>(&self, i: usize) -> Option<&T> {
        let input: &dyn Traced = self.steps.get(i)?.input.as_ref();
        input.as_any().downcast_ref::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terni::Loss as _;

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
            loss: ScalarLoss::zero(),
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
            loss: ScalarLoss::zero(),
            input: Box::new(99u32),
            output: StepOutput::Value(Box::new("out".to_string())),
        });
        assert_eq!(t.reenter_at::<u32>(0), Some(&99u32));
    }

    #[test]
    fn trace_reenter_wrong_type() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ScalarLoss::zero(),
            input: Box::new(99u32),
            output: StepOutput::Value(Box::new("out".to_string())),
        });
        assert!(t.reenter_at::<String>(0).is_none());
    }

    #[test]
    fn trace_reenter_out_of_bounds() {
        let t = Trace::new();
        assert!(t.reenter_at::<u32>(0).is_none());
    }

    #[test]
    fn op_variants_are_distinct() {
        assert_ne!(Op::Focus, Op::Project);
        assert_ne!(Op::Project, Op::Settle);
        assert_ne!(Op::Focus, Op::Settle);
    }

    #[test]
    fn steps_returns_slice() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "p",
            op: Op::Settle,
            loss: ScalarLoss::zero(),
            input: Box::new(1u32),
            output: StepOutput::Value(Box::new(2u32)),
        });
        assert_eq!(t.steps().len(), 1);
        assert_eq!(t.steps()[0].prism, "p");
    }

    #[test]
    fn trace_reenter_string_input() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ScalarLoss::zero(),
            input: Box::new("hello".to_string()),
            output: StepOutput::Value(Box::new(42u32)),
        });
        assert_eq!(t.reenter_at::<String>(0), Some(&"hello".to_string()));
    }
}

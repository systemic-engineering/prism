//! Trace — the execution record of a beam through a pipeline.
//!
//! Each [`Step`] records which prism and operation produced it, the input
//! that entered, the output (or error) that left, and the loss incurred.
//! A [`Trace`] collects steps in order, and [`reenter_at`](Trace::reenter_at)
//! allows recovering a typed input at any step for replay or debugging.

use std::any::Any;
use std::fmt;

use imperfect::ShannonLoss;

/// Which pipeline operation produced a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Focus,
    Project,
    Refract,
}

/// Any value that is `Debug + Any + Send + Sync` can be stored in a `Trace`.
pub trait Traced: Any + fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + fmt::Debug + Send + Sync> Traced for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The output side of a traced step.
pub enum StepOutput {
    Value(Box<dyn Traced>),
    Error(Box<dyn Traced>),
}

/// A single traced step through the pipeline.
pub struct Step {
    pub prism: &'static str,
    pub op: Op,
    pub loss: ShannonLoss,
    pub input: Box<dyn Traced>,
    pub output: StepOutput,
}

/// Full execution record — all steps through the pipeline.
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

    /// Recover the input at step `i` as concrete type `T`.
    pub fn reenter_at<T: 'static>(&self, i: usize) -> Option<&T> {
        let input: &dyn Traced = self.steps.get(i)?.input.as_ref();
        input.as_any().downcast_ref::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imperfect::Loss as _;

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
        assert_eq!(t.reenter_at::<u32>(0), Some(&99u32));
    }

    #[test]
    fn trace_reenter_wrong_type() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "test",
            op: Op::Focus,
            loss: ShannonLoss::zero(),
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
        assert_ne!(Op::Project, Op::Refract);
        assert_ne!(Op::Focus, Op::Refract);
    }

    #[test]
    fn steps_returns_slice() {
        let mut t = Trace::new();
        t.push(Step {
            prism: "p",
            op: Op::Refract,
            loss: ShannonLoss::zero(),
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
            loss: ShannonLoss::zero(),
            input: Box::new("hello".to_string()),
            output: StepOutput::Value(Box::new(42u32)),
        });
        assert_eq!(t.reenter_at::<String>(0), Some(&"hello".to_string()));
    }
}

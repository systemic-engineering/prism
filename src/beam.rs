use crate::loss::ShannonLoss;
use crate::oid::Oid;
use crate::precision::Precision;

/// How a beam was recovered after a degraded projection.
#[derive(Clone, Debug, PartialEq)]
pub enum Recovery {
    Coarsened { from: Precision, to: Precision },
    Replayed { from_step: usize },
    Failed { reason: String },
}

/// The pipeline stage a beam has reached.
///
/// Tracks where in the focus → project → split → zoom → refract
/// pipeline a beam currently lives. Initialized to `Initial` on
/// `Beam::new`. Transitions are set explicitly via `with_stage`.
///
/// Note: there is no `Joined` variant. The base trait does not have
/// a `join` operation — gathering populations of beams back into a
/// single beam is the job of meta-prisms in the optics layer, and
/// gather strategies emit `Projected` (the stage of the beam after
/// the gathering project step completes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Initial,
    Focused,
    Projected,
    Split,
    Refracted,
}

/// The trace of a projection through a Prism.
///
/// Always lands. The result is always present. Loss tells you
/// what didn't survive. The beam carries the story of how the
/// result came to be: path, loss, precision, recovery, stage.
#[derive(Clone, Debug)]
pub struct Beam<T> {
    pub result: T,
    pub path: Vec<Oid>,
    pub loss: ShannonLoss,
    pub precision: Precision,
    pub recovered: Option<Recovery>,
    pub stage: Stage,
}

impl<T> Beam<T> {
    /// Create a beam with a result and no loss.
    pub fn new(result: T) -> Self {
        Beam {
            result,
            path: Vec::new(),
            loss: ShannonLoss::zero(),
            precision: Precision::new(0.0),
            recovered: None,
            stage: Stage::Initial,
        }
    }

    /// Whether the projection was lossless.
    pub fn is_lossless(&self) -> bool {
        self.loss.is_zero()
    }

    /// Whether the projection lost information.
    pub fn has_loss(&self) -> bool {
        !self.loss.is_zero()
    }

    /// Whether recovery was attempted.
    pub fn was_recovered(&self) -> bool {
        self.recovered.is_some()
    }

    /// Set the pipeline stage.
    pub fn with_stage(mut self, stage: Stage) -> Self {
        self.stage = stage;
        self
    }

    /// Map the result, preserving the trace.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Beam<U> {
        Beam {
            result: f(self.result),
            path: self.path,
            loss: self.loss,
            precision: self.precision,
            recovered: self.recovered,
            stage: self.stage,
        }
    }

    /// Add a step to the path.
    pub fn with_step(mut self, oid: Oid) -> Self {
        self.path.push(oid);
        self
    }

    /// Set the precision.
    pub fn with_precision(mut self, precision: Precision) -> Self {
        self.precision = precision;
        self
    }

    /// Set the loss.
    pub fn with_loss(mut self, loss: ShannonLoss) -> Self {
        self.loss = loss;
        self
    }

    /// Set recovery.
    pub fn with_recovery(mut self, recovery: Recovery) -> Self {
        self.recovered = Some(recovery);
        self
    }

    /// Add to the existing loss (for boundary crossings).
    pub fn accumulate_loss(mut self, additional: ShannonLoss) -> Self {
        self.loss += additional;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beam_new() {
        let b = Beam::new(42);
        assert_eq!(b.result, 42);
        assert!(b.is_lossless());
        assert!(!b.has_loss());
    }

    #[test]
    fn beam_has_loss() {
        let b = Beam::new(0).with_loss(ShannonLoss::new(1.0));
        assert!(b.has_loss());
        assert!(!b.is_lossless());
    }

    #[test]
    fn beam_map() {
        let b = Beam::new(10).map(|x| x * 2);
        assert_eq!(b.result, 20);
    }

    #[test]
    fn beam_with_step() {
        let b = Beam::new(1).with_step(Oid::new("step-1"));
        assert_eq!(b.path.len(), 1);
        assert_eq!(b.path[0].as_str(), "step-1");
    }

    #[test]
    fn beam_with_precision() {
        let b = Beam::new(1).with_precision(Precision::new(0.05));
        assert_eq!(b.precision.as_f64(), 0.05);
    }

    #[test]
    fn beam_with_loss() {
        let b = Beam::new(1).with_loss(ShannonLoss::new(1.5));
        assert_eq!(b.loss.as_f64(), 1.5);
        assert!(!b.is_lossless());
    }

    #[test]
    fn beam_was_recovered_false() {
        let b = Beam::new(1);
        assert!(!b.was_recovered());
    }

    #[test]
    fn beam_recovery_coarsened() {
        let b = Beam::new(1).with_recovery(Recovery::Coarsened {
            from: Precision::new(0.01),
            to: Precision::new(0.1),
        });
        assert!(b.was_recovered());
        match b.recovered.unwrap() {
            Recovery::Coarsened { from, to } => {
                assert_eq!(from.as_f64(), 0.01);
                assert_eq!(to.as_f64(), 0.1);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn beam_recovery_replayed() {
        let b = Beam::new(1).with_recovery(Recovery::Replayed { from_step: 3 });
        assert!(b.was_recovered());
        match b.recovered.unwrap() {
            Recovery::Replayed { from_step } => assert_eq!(from_step, 3),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn beam_recovery_failed() {
        let b = Beam::new(0).with_recovery(Recovery::Failed {
            reason: "no data".to_owned(),
        });
        assert!(b.was_recovered());
        match b.recovered.unwrap() {
            Recovery::Failed { reason } => assert_eq!(reason, "no data"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn beam_chained_builders() {
        let b = Beam::new("ok")
            .with_step(Oid::new("a"))
            .with_step(Oid::new("b"))
            .with_precision(Precision::new(0.001))
            .with_loss(ShannonLoss::new(0.5));
        assert_eq!(b.path.len(), 2);
        assert_eq!(b.precision.as_f64(), 0.001);
        assert_eq!(b.loss.as_f64(), 0.5);
    }

    #[test]
    fn beam_clone() {
        let a = Beam::new(99).with_step(Oid::new("x"));
        let b = a.clone();
        assert_eq!(b.result, 99);
        assert_eq!(b.path.len(), 1);
    }

    #[test]
    fn beam_accumulate_loss() {
        let b = Beam::new(1)
            .with_loss(ShannonLoss::new(1.0))
            .accumulate_loss(ShannonLoss::new(0.5));
        assert_eq!(b.loss.as_f64(), 1.5);
    }

    #[test]
    fn beam_accumulate_loss_from_zero() {
        let b = Beam::new(1).accumulate_loss(ShannonLoss::new(2.0));
        assert_eq!(b.loss.as_f64(), 2.0);
    }
}

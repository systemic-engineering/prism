use crate::oid::Oid;
use crate::loss::ShannonLoss;
use crate::precision::Precision;

/// How a beam was recovered after a miss or degraded state.
#[derive(Clone, Debug, PartialEq)]
pub enum Recovery {
    Coarsened { from: Precision, to: Precision },
    Replayed { from_step: usize },
    Failed { reason: String },
}

/// A focused result carrying its provenance: path through Oids, loss, precision, and recovery.
#[derive(Clone, Debug)]
pub struct Beam<T> {
    pub result: Option<T>,
    pub path: Vec<Oid>,
    pub loss: ShannonLoss,
    pub precision: Precision,
    pub recovered: Option<Recovery>,
}

impl<T> Beam<T> {
    pub fn hit(value: T) -> Self {
        Beam {
            result: Some(value),
            path: Vec::new(),
            loss: ShannonLoss::zero(),
            precision: Precision::new(0.0),
            recovered: None,
        }
    }

    pub fn miss() -> Self {
        Beam {
            result: None,
            path: Vec::new(),
            loss: ShannonLoss::zero(),
            precision: Precision::new(0.0),
            recovered: None,
        }
    }

    pub fn is_hit(&self) -> bool {
        self.result.is_some()
    }

    pub fn is_miss(&self) -> bool {
        self.result.is_none()
    }

    pub fn is_lossless(&self) -> bool {
        self.loss.is_zero()
    }

    pub fn was_recovered(&self) -> bool {
        self.recovered.is_some()
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Beam<U> {
        Beam {
            result: self.result.map(f),
            path: self.path,
            loss: self.loss,
            precision: self.precision,
            recovered: self.recovered,
        }
    }

    pub fn with_step(mut self, oid: Oid) -> Self {
        self.path.push(oid);
        self
    }

    pub fn with_precision(mut self, precision: Precision) -> Self {
        self.precision = precision;
        self
    }

    pub fn with_loss(mut self, loss: ShannonLoss) -> Self {
        self.loss = loss;
        self
    }

    pub fn with_recovery(mut self, recovery: Recovery) -> Self {
        self.recovered = Some(recovery);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beam_hit() {
        let b: Beam<i32> = Beam::hit(42);
        assert!(b.is_hit());
        assert!(!b.is_miss());
        assert_eq!(b.result, Some(42));
    }

    #[test]
    fn beam_miss() {
        let b: Beam<i32> = Beam::miss();
        assert!(b.is_miss());
        assert!(!b.is_hit());
        assert_eq!(b.result, None);
    }

    #[test]
    fn beam_map() {
        let b: Beam<i32> = Beam::hit(10);
        let mapped = b.map(|x| x * 2);
        assert_eq!(mapped.result, Some(20));
    }

    #[test]
    fn beam_map_miss() {
        let b: Beam<i32> = Beam::miss();
        let mapped = b.map(|x| x * 2);
        assert_eq!(mapped.result, None);
    }

    #[test]
    fn beam_with_step() {
        let b: Beam<i32> = Beam::hit(1).with_step(Oid::new("step-1"));
        assert_eq!(b.path.len(), 1);
        assert_eq!(b.path[0].as_str(), "step-1");
    }

    #[test]
    fn beam_with_precision() {
        let b: Beam<i32> = Beam::hit(1).with_precision(Precision::new(0.05));
        assert_eq!(b.precision.as_f64(), 0.05);
    }

    #[test]
    fn beam_with_loss() {
        let b: Beam<i32> = Beam::hit(1).with_loss(ShannonLoss::new(1.5));
        assert_eq!(b.loss.as_f64(), 1.5);
        assert!(!b.is_lossless());
    }

    #[test]
    fn beam_was_recovered_false() {
        let b: Beam<i32> = Beam::hit(1);
        assert!(!b.was_recovered());
    }

    #[test]
    fn beam_recovery_coarsened() {
        let b: Beam<i32> = Beam::hit(1).with_recovery(Recovery::Coarsened {
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
        let b: Beam<i32> = Beam::hit(1).with_recovery(Recovery::Replayed { from_step: 3 });
        assert!(b.was_recovered());
        match b.recovered.unwrap() {
            Recovery::Replayed { from_step } => assert_eq!(from_step, 3),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn beam_recovery_failed() {
        let b: Beam<i32> = Beam::miss().with_recovery(Recovery::Failed {
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
        let b: Beam<&str> = Beam::hit("ok")
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
        let a: Beam<i32> = Beam::hit(99).with_step(Oid::new("x"));
        let b = a.clone();
        assert_eq!(b.result, Some(99));
        assert_eq!(b.path.len(), 1);
    }
}

//! Gather — strategies for collapsing `Vec<Beam<T>>` into `Beam<T>`.
//!
//! Different strategies make different decisions about how to aggregate
//! loss, combine results, and preserve path provenance. Used by
//! `MetaPrism` as the refract-side collapsing operation.

use crate::{Beam, Precision, ShannonLoss, Stage};

/// A strategy for collapsing `Vec<Beam<T>>` into a single `Beam<T>`.
///
/// Implementations pick how to aggregate:
/// - the result values (concatenate? merge? pick one?)
/// - the loss fields (sum? max? Shannon-add?)
/// - the precision (min? max? average?)
/// - the path provenance (concatenate? merge?)
pub trait Gather<T> {
    fn gather(&self, beams: Vec<Beam<T>>) -> Beam<T>;
}

/// Gather strings by concatenation. Losses sum. Precision is the
/// minimum of all precisions (the weakest link). Paths are taken from
/// the first beam and extended with a synthetic marker.
#[derive(Clone)]
pub struct SumGather;

impl Gather<String> for SumGather {
    fn gather(&self, beams: Vec<Beam<String>>) -> Beam<String> {
        if beams.is_empty() {
            return Beam {
                result: String::new(),
                path: Vec::new(),
                loss: ShannonLoss::new(0.0),
                precision: Precision::new(1.0),
                recovered: None,
                stage: Stage::Joined,
            };
        }

        let mut result = String::new();
        let mut total_loss = 0.0f64;
        let mut min_precision = Precision::new(1.0);
        let first_path = beams[0].path.clone();
        let first_recovered = beams[0].recovered.clone();

        for beam in &beams {
            result.push_str(&beam.result);
            total_loss += beam.loss.as_f64();
            if beam.precision.as_f64() < min_precision.as_f64() {
                min_precision = beam.precision.clone();
            }
        }

        Beam {
            result,
            path: first_path,
            loss: ShannonLoss::new(total_loss),
            precision: min_precision,
            recovered: first_recovered,
            stage: Stage::Joined,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Precision, ShannonLoss, Stage};

    #[test]
    fn sum_gather_concatenates_strings_and_sums_losses() {
        let beams = vec![
            Beam::new("hello".to_string())
                .with_loss(ShannonLoss::new(1.0)),
            Beam::new(" ".to_string())
                .with_loss(ShannonLoss::new(0.0)),
            Beam::new("world".to_string())
                .with_loss(ShannonLoss::new(2.0)),
        ];
        let gather = SumGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "hello world");
        assert_eq!(out.loss.as_f64(), 3.0);
        assert_eq!(out.stage, Stage::Joined);
    }

    #[test]
    fn sum_gather_empty_vec_yields_empty_beam() {
        let beams: Vec<Beam<String>> = vec![];
        let gather = SumGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "");
        assert!(out.loss.is_zero());
    }
}

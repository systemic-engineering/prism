//! Gather — strategies for collapsing `Vec<Beam<T>>` into `Beam<T>`.
//!
//! Different strategies make different decisions about how to aggregate
//! loss, combine results, and preserve path provenance. Used by
//! `MetaPrism` as the refract-side collapsing operation.

use crate::Beam;

// Types go here — see tests for the expected API.

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
        assert!(out.loss.is_lossless());
    }
}

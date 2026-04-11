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

/// Gather by concatenating string results. Losses sum. Precision is
/// the minimum across beams. Path and recovered are taken from the
/// first beam.
///
/// For a string-shaped gather. For numeric types, use `AddGather`.
#[derive(Clone)]
pub struct ConcatGather;

impl Gather<String> for ConcatGather {
    fn gather(&self, beams: Vec<Beam<String>>) -> Beam<String> {
        if beams.is_empty() {
            return Beam {
                result: String::new(),
                path: Vec::new(),
                loss: ShannonLoss::new(0.0),
                precision: Precision::new(1.0),
                recovered: None,
                stage: Stage::Projected,
                connection: Default::default(),
            };
        }

        let first_path = beams[0].path.clone();
        let first_recovered = beams[0].recovered.clone();

        let mut result = String::new();
        let mut total_loss = 0.0f64;
        let mut min_precision = Precision::new(1.0);

        for beam in beams {
            result.push_str(&beam.result);
            total_loss += beam.loss.as_f64();
            if beam.precision.as_f64() < min_precision.as_f64() {
                min_precision = beam.precision;
            }
        }

        Beam {
            result,
            path: first_path,
            loss: ShannonLoss::new(total_loss),
            precision: min_precision,
            recovered: first_recovered,
            stage: Stage::Projected,
            connection: Default::default(),
        }
    }
}

/// Gather by summing via `std::ops::Add`. Generic over any type that
/// has a zero element (Default) and addition. Works for numeric types
/// (i32, f64, etc.) and any user type implementing
/// `Add<Output = Self> + Clone + Default`.
///
/// Losses sum. Precision is the minimum across beams. Path and
/// recovered are taken from the first beam.
#[derive(Clone)]
pub struct AddGather;

impl<T> Gather<T> for AddGather
where
    T: Clone + Default + std::ops::Add<Output = T>,
{
    fn gather(&self, beams: Vec<Beam<T>>) -> Beam<T> {
        if beams.is_empty() {
            return Beam {
                result: T::default(),
                path: Vec::new(),
                loss: ShannonLoss::new(0.0),
                precision: Precision::new(1.0),
                recovered: None,
                stage: Stage::Projected,
                connection: Default::default(),
            };
        }

        let first_path = beams[0].path.clone();
        let first_recovered = beams[0].recovered.clone();

        let mut result = T::default();
        let mut total_loss = 0.0f64;
        let mut min_precision = Precision::new(1.0);

        for beam in beams {
            result = result + beam.result;
            total_loss += beam.loss.as_f64();
            if beam.precision.as_f64() < min_precision.as_f64() {
                min_precision = beam.precision;
            }
        }

        Beam {
            result,
            path: first_path,
            loss: ShannonLoss::new(total_loss),
            precision: min_precision,
            recovered: first_recovered,
            stage: Stage::Projected,
            connection: Default::default(),
        }
    }
}

/// Gather by picking the beam with the highest precision. Generic
/// over any T: Clone + Default — no arithmetic needed, just comparison
/// on precision.
#[derive(Clone)]
pub struct MaxGather;

impl<T: Clone + Default> Gather<T> for MaxGather {
    fn gather(&self, beams: Vec<Beam<T>>) -> Beam<T> {
        if beams.is_empty() {
            return Beam {
                result: T::default(),
                path: Vec::new(),
                loss: ShannonLoss::new(0.0),
                precision: Precision::new(1.0),
                recovered: None,
                stage: Stage::Projected,
                connection: Default::default(),
            };
        }

        let best_idx = beams
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.precision
                    .as_f64()
                    .partial_cmp(&b.precision.as_f64())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        let mut beams = beams;
        let best = beams.swap_remove(best_idx);
        Beam {
            stage: Stage::Projected,
            ..best
        }
    }
}

/// Gather by taking the first beam and discarding the rest. Generic
/// over any T: Clone + Default. Simplest possible gather; mostly
/// useful as a baseline and for testing.
#[derive(Clone)]
pub struct FirstGather;

impl<T: Clone + Default> Gather<T> for FirstGather {
    fn gather(&self, beams: Vec<Beam<T>>) -> Beam<T> {
        let mut iter = beams.into_iter();
        match iter.next() {
            Some(first) => Beam {
                stage: Stage::Projected,
                ..first
            },
            None => Beam {
                result: T::default(),
                path: Vec::new(),
                loss: ShannonLoss::new(0.0),
                precision: Precision::new(1.0),
                recovered: None,
                stage: Stage::Projected,
                connection: Default::default(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Precision, ShannonLoss, Stage};

    #[test]
    fn first_gather_returns_first_beam() {
        let beams = vec![
            Beam::new("first".to_string())
                .with_loss(ShannonLoss::new(1.0)),
            Beam::new("second".to_string())
                .with_loss(ShannonLoss::new(99.0)),
        ];
        let gather = FirstGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "first");
        assert_eq!(out.loss.as_f64(), 1.0);
        assert_eq!(out.stage, Stage::Projected);
    }

    #[test]
    fn first_gather_empty_vec() {
        let beams: Vec<Beam<String>> = vec![];
        let gather = FirstGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "");
    }

    #[test]
    fn concat_gather_concatenates_strings_and_sums_losses() {
        let beams = vec![
            Beam::new("hello".to_string())
                .with_loss(ShannonLoss::new(1.0)),
            Beam::new(" ".to_string())
                .with_loss(ShannonLoss::new(0.0)),
            Beam::new("world".to_string())
                .with_loss(ShannonLoss::new(2.0)),
        ];
        let gather = ConcatGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "hello world");
        assert_eq!(out.loss.as_f64(), 3.0);
        assert_eq!(out.stage, Stage::Projected);
    }

    #[test]
    fn max_gather_picks_highest_precision_beam() {
        let beams = vec![
            Beam::new("low".to_string())
                .with_precision(Precision::new(0.3))
                .with_loss(ShannonLoss::new(5.0)),
            Beam::new("high".to_string())
                .with_precision(Precision::new(0.9))
                .with_loss(ShannonLoss::new(0.1)),
            Beam::new("mid".to_string())
                .with_precision(Precision::new(0.6))
                .with_loss(ShannonLoss::new(1.0)),
        ];
        let gather = MaxGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "high");
        assert_eq!(out.precision.as_f64(), 0.9);
        assert_eq!(out.loss.as_f64(), 0.1);
        assert_eq!(out.stage, Stage::Projected);
    }

    #[test]
    fn max_gather_empty_vec_yields_empty_beam() {
        let beams: Vec<Beam<String>> = vec![];
        let gather = MaxGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "");
    }

    #[test]
    fn concat_gather_empty_vec_yields_empty_beam() {
        let beams: Vec<Beam<String>> = vec![];
        let gather = ConcatGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, "");
        assert!(out.loss.is_zero());
    }

    #[test]
    fn add_gather_sums_i32_beams() {
        let beams = vec![
            Beam::new(10i32).with_loss(ShannonLoss::new(0.5)),
            Beam::new(20i32).with_loss(ShannonLoss::new(0.3)),
            Beam::new(30i32).with_loss(ShannonLoss::new(0.2)),
        ];
        let gather = AddGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, 60);
        assert_eq!(out.loss.as_f64(), 1.0);
    }

    #[test]
    fn max_gather_picks_highest_precision_for_i32() {
        let beams = vec![
            Beam::new(1i32).with_precision(Precision::new(0.3)),
            Beam::new(2i32).with_precision(Precision::new(0.9)),
            Beam::new(3i32).with_precision(Precision::new(0.6)),
        ];
        let gather = MaxGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, 2);
        assert_eq!(out.precision.as_f64(), 0.9);
    }

    #[test]
    fn first_gather_takes_first_i32_beam() {
        let beams = vec![
            Beam::new(42i32),
            Beam::new(99i32),
        ];
        let gather = FirstGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, 42);
    }

    #[test]
    fn add_gather_extensible_to_user_types() {
        #[derive(Clone, Debug, PartialEq, Default)]
        struct Points(i32);

        impl std::ops::Add for Points {
            type Output = Self;
            fn add(self, other: Self) -> Self {
                Points(self.0 + other.0)
            }
        }

        let beams = vec![
            Beam::new(Points(10)),
            Beam::new(Points(20)),
            Beam::new(Points(30)),
        ];
        let gather = AddGather;
        let out = gather.gather(beams);
        assert_eq!(out.result, Points(60));
    }
}

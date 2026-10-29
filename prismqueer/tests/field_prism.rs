//! prism-005 ask 1: a prism whose settle ends in a field.
//!
//! `FieldPrism<B, V>` is any `Prism` whose refracted beam carries a
//! `Field<B, V>`. `ScalarPrism<V>` is a field prism over a point: a single
//! truth. Nothing to implement: a prism is a field prism by what it settles
//! into.

#![cfg(feature = "lapack")]

use prismqueer::field::{Discrete, Field, FieldPrism, Lattice, Least, Point, ScalarPrism};
use prismqueer::{Beam, Optic, Prism};

/// k criteria, settled into a field on k unconnected points.
struct Criteria;

impl Prism for Criteria {
    type Input = Optic<(), Vec<u64>>;
    type Focused = Optic<Vec<u64>, Vec<u64>>;
    type Projected = Optic<Vec<u64>, Vec<u64>>;
    type Refracted = Optic<Vec<u64>, Field<Discrete, Least>>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let v = beam.result().ok().expect("lit").clone();
        beam.next(v)
    }
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let v = beam.result().ok().expect("lit").clone();
        beam.next(v)
    }
    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let v = beam.result().ok().expect("lit").clone();
        beam.next(Field::discrete(v.into_iter().map(Least).collect()))
    }
}

/// One measurement, settled into a field on a point.
struct Reading;

impl Prism for Reading {
    type Input = Optic<(), u64>;
    type Focused = Optic<u64, u64>;
    type Projected = Optic<u64, u64>;
    type Refracted = Optic<u64, Field<Point, Least>>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let v = *beam.result().ok().expect("lit");
        beam.next(v)
    }
    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let v = *beam.result().ok().expect("lit");
        beam.next(v)
    }
    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let v = *beam.result().ok().expect("lit");
        beam.next(Field::point(Least(v)))
    }
}

fn multi_truth<P: FieldPrism<Discrete, Least>>(_: &P) {}
fn single_truth<P: ScalarPrism<Least>>(_: &P) {}

#[test]
fn a_prism_settling_into_a_field_is_a_field_prism() {
    multi_truth(&Criteria);
    let settled = Criteria.settle(Criteria.project(Criteria.focus(Optic::ok((), vec![7, 3, 9]))));
    let field = settled.result().ok().expect("lit");
    assert_eq!(field.meet(), Least(3), "multi-truth: the lawful collapse is the meet");
}

#[test]
fn a_scalar_prism_is_a_field_prism_over_a_point() {
    single_truth(&Reading);
    let settled = Reading.settle(Reading.project(Reading.focus(Optic::ok((), 4))));
    let field = settled.result().ok().expect("lit");
    assert_eq!(*field.value(), Least(4), "single truth: the one value");
    assert_eq!(field.meet(), Least(4).meet(&Least::top()));
}

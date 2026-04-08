//! Fold — the multi read-only optic.
//!
//! A Fold<S, A> extracts zero or more As from an S. No modification
//! side. Think of it as a Traversal with the put-back direction removed.

use crate::{Beam, Prism, Stage};
use std::marker::PhantomData;

pub struct Fold<S, A> {
    fold_fn: Box<dyn Fn(&S) -> Vec<A>>,
    _phantom: PhantomData<(S, A)>,
}

impl<S: 'static, A: 'static> Fold<S, A> {
    pub fn new<F>(fold: F) -> Self
    where
        F: Fn(&S) -> Vec<A> + 'static,
    {
        Fold {
            fold_fn: Box::new(fold),
            _phantom: PhantomData,
        }
    }

    pub fn to_list(&self, s: &S) -> Vec<A> {
        (self.fold_fn)(s)
    }
}

impl<S: Clone + 'static, A: Clone + 'static> Prism for Fold<S, A> {
    type Input = S;
    type Focused = Vec<A>;
    type Projected = Vec<A>;
    type Part = A;
    type Crystal = FoldCrystal<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<Vec<A>> {
        let list = (self.fold_fn)(&beam.result);
        Beam {
            result: list,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
        }
    }

    fn project(&self, beam: Beam<Vec<A>>) -> Beam<Vec<A>> {
        Beam { stage: Stage::Projected, ..beam }
    }

    fn split(&self, beam: Beam<Vec<A>>) -> Vec<Beam<A>> {
        beam.result
            .into_iter()
            .map(|a| Beam {
                result: a,
                path: beam.path.clone(),
                loss: beam.loss.clone(),
                precision: beam.precision.clone(),
                recovered: beam.recovered.clone(),
                stage: Stage::Split,
            })
            .collect()
    }

    fn zoom(&self, beam: Beam<Vec<A>>, f: &dyn Fn(Beam<Vec<A>>) -> Beam<Vec<A>>) -> Beam<Vec<A>> {
        f(beam)
    }

    fn refract(&self, beam: Beam<Vec<A>>) -> Beam<FoldCrystal<S, A>> {
        Beam {
            result: FoldCrystal { _phantom: PhantomData },
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}

pub struct FoldCrystal<S, A> { _phantom: PhantomData<(S, A)> }

impl<S: Clone + 'static, A: Clone + 'static> Prism for FoldCrystal<S, A> {
    type Input = Vec<A>;
    type Focused = Vec<A>;
    type Projected = Vec<A>;
    type Part = A;
    type Crystal = FoldCrystal<S, A>;

    fn focus(&self, beam: Beam<Vec<A>>) -> Beam<Vec<A>> { Beam { stage: Stage::Focused, ..beam } }
    fn project(&self, beam: Beam<Vec<A>>) -> Beam<Vec<A>> { Beam { stage: Stage::Projected, ..beam } }
    fn split(&self, beam: Beam<Vec<A>>) -> Vec<Beam<A>> {
        beam.result
            .into_iter()
            .map(|a| Beam {
                result: a,
                path: beam.path.clone(),
                loss: beam.loss.clone(),
                precision: beam.precision.clone(),
                recovered: beam.recovered.clone(),
                stage: Stage::Split,
            })
            .collect()
    }
    fn zoom(&self, beam: Beam<Vec<A>>, f: &dyn Fn(Beam<Vec<A>>) -> Beam<Vec<A>>) -> Beam<Vec<A>> { f(beam) }
    fn refract(&self, beam: Beam<Vec<A>>) -> Beam<FoldCrystal<S, A>> {
        Beam {
            result: FoldCrystal { _phantom: PhantomData },
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Tree {
        leaves: Vec<i32>,
    }

    #[test]
    fn fold_extracts_list() {
        let leaves_fold: Fold<Tree, i32> = Fold::new(|t: &Tree| t.leaves.clone());
        let tree = Tree { leaves: vec![1, 2, 3] };
        assert_eq!(leaves_fold.to_list(&tree), vec![1, 2, 3]);
    }

    #[test]
    fn fold_focus_produces_list_beam() {
        let leaves_fold: Fold<Tree, i32> = Fold::new(|t: &Tree| t.leaves.clone());
        let beam = Beam::new(Tree { leaves: vec![10, 20] });
        let focused = leaves_fold.focus(beam);
        assert_eq!(focused.result, vec![10, 20]);
        assert_eq!(focused.stage, Stage::Focused);
    }

    #[test]
    fn fold_split_yields_individual_element_beams() {
        let leaves_fold: Fold<Tree, i32> = Fold::new(|t: &Tree| t.leaves.clone());
        let beam = Beam::new(Tree { leaves: vec![5, 6, 7] });
        let focused = leaves_fold.focus(beam);
        let projected = leaves_fold.project(focused);
        let parts = leaves_fold.split(projected);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].result, 5);
        assert_eq!(parts[1].result, 6);
        assert_eq!(parts[2].result, 7);
    }

    #[test]
    fn fold_split_indexes_children() {
        use crate::Oid;

        let leaves_fold: Fold<Tree, i32> = Fold::new(|t: &Tree| t.leaves.clone());
        let beam = Beam::new(Tree { leaves: vec![5, 6, 7] });
        let focused = leaves_fold.focus(beam);
        let projected = leaves_fold.project(focused);
        let parts = leaves_fold.split(projected);

        assert_eq!(parts.len(), 3);
        // Each child should have a path entry with its index
        assert_eq!(parts[0].path.last(), Some(&Oid::new("0")));
        assert_eq!(parts[1].path.last(), Some(&Oid::new("1")));
        assert_eq!(parts[2].path.last(), Some(&Oid::new("2")));
    }
}

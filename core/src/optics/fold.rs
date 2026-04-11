//! Fold — the multi read-only optic.
//!
//! A Fold<S, A> extracts zero or more As from an S. No modification
//! side. Think of it as a Traversal with the put-back direction removed.

use crate::{Beam, Prism, Stage};

#[derive(Clone, Copy)]
pub struct Fold<S, A> {
    fold_fn: fn(&S) -> Vec<A>,
}

impl<S: 'static, A: 'static> Fold<S, A> {
    /// Construct a Fold from a function that extracts zero or more `A`s from `S`.
    ///
    /// # Laws
    ///
    /// Fold has no bidirectional law (it has no `set` or `review` side), so
    /// the caller is only responsible for ensuring the extraction is consistent
    /// and side-effect-free:
    ///
    /// - `fold(s)` returns the same `Vec<A>` for the same `s` (purity)
    /// - `fold(s)` does not panic for any well-typed `s`
    ///
    /// A Fold whose extraction function is impure or partial will produce
    /// non-reproducible results in any pipeline that depends on content
    /// addressing.
    pub fn new(fold: fn(&S) -> Vec<A>) -> Self {
        Fold { fold_fn: fold }
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
    type Crystal = Fold<S, A>;

    fn focus(&self, beam: Beam<S>) -> Beam<Vec<A>> {
        let list = (self.fold_fn)(&beam.result);
        Beam {
            result: list,
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Focused,
            connection: beam.connection,
        }
    }

    fn project(&self, beam: Beam<Vec<A>>) -> Beam<Vec<A>> {
        Beam { stage: Stage::Projected, ..beam }
    }

    fn split(&self, beam: Beam<Vec<A>>) -> Vec<Beam<A>> {
        beam.result
            .into_iter()
            .enumerate()
            .map(|(i, a)| Beam {
                result: a,
                path: {
                    let mut p = beam.path.clone();
                    p.push(crate::Oid::new(format!("{}", i)));
                    p
                },
                loss: beam.loss.clone(),
                precision: beam.precision.clone(),
                recovered: beam.recovered.clone(),
                stage: Stage::Split,
                connection: beam.connection.clone(),
            })
            .collect()
    }

    fn zoom(&self, beam: Beam<Vec<A>>, f: &dyn Fn(Beam<Vec<A>>) -> Beam<Vec<A>>) -> Beam<Vec<A>> {
        f(beam)
    }

    fn refract(&self, beam: Beam<Vec<A>>) -> Beam<Fold<S, A>> {
        // fn pointers are Copy — the optic itself IS the lossless fixed point.
        Beam {
            result: self.clone(),
            path: beam.path,
            loss: beam.loss,
            precision: beam.precision,
            recovered: beam.recovered,
            stage: Stage::Refracted,
            connection: beam.connection,
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

    fn tree_leaves(t: &Tree) -> Vec<i32> { t.leaves.clone() }

    #[test]
    fn fold_extracts_list() {
        let leaves_fold: Fold<Tree, i32> = Fold::new(tree_leaves);
        let tree = Tree { leaves: vec![1, 2, 3] };
        assert_eq!(leaves_fold.to_list(&tree), vec![1, 2, 3]);
    }

    #[test]
    fn fold_focus_produces_list_beam() {
        let leaves_fold: Fold<Tree, i32> = Fold::new(tree_leaves);
        let beam = Beam::new(Tree { leaves: vec![10, 20] });
        let focused = leaves_fold.focus(beam);
        assert_eq!(focused.result, vec![10, 20]);
        assert_eq!(focused.stage, Stage::Focused);
    }

    #[test]
    fn fold_split_yields_individual_element_beams() {
        let leaves_fold: Fold<Tree, i32> = Fold::new(tree_leaves);
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

        let leaves_fold: Fold<Tree, i32> = Fold::new(tree_leaves);
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

    #[test]
    fn fold_is_clone_and_copy() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let f2 = f; // Copy
        let f3 = f2.clone(); // Clone
        let tree = Tree { leaves: vec![1] };
        assert_eq!(f3.to_list(&tree), vec![1]);
    }
}

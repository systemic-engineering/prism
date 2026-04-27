//! Fold — the multi read-only optic.
//!
//! A Fold<S, A> extracts zero or more As from an S. No modification side.
//! Think of it as a Traversal without the put-back direction.
//!
//! As a Prism: focus extracts (S → Vec<A>), project and refract pass through.

use crate::ScalarLoss;
use crate::{Beam, Optic, Prism};
use std::convert::Infallible;

#[derive(Clone, Copy)]
pub struct Fold<S, A> {
    fold_fn: fn(&S) -> Vec<A>,
}

impl<S: 'static, A: 'static> Fold<S, A> {
    /// Construct a Fold from a function that extracts zero or more `A`s from `S`.
    ///
    /// # Laws
    /// - `fold(s)` returns the same `Vec<A>` for the same `s` (purity)
    /// - `fold(s)` does not panic for any well-typed `s` (totality)
    pub fn new(fold: fn(&S) -> Vec<A>) -> Self {
        Fold { fold_fn: fold }
    }

    pub fn to_list(&self, s: &S) -> Vec<A> {
        (self.fold_fn)(s)
    }
}

/// Content address of a Fold: derived from its function pointer.
/// Two Folds with the same function pointer have the same OID.
impl<S: 'static, A: 'static> crate::Addressable for Fold<S, A> {
    fn oid(&self) -> crate::Oid {
        let bytes = (self.fold_fn as usize).to_le_bytes();
        crate::Oid::hash(&bytes)
    }
}

/// Fold implements Prism with Optic.
///
/// Pipeline flow:
/// - focus: extract all elements (S → Vec<A>)
/// - project: identity (Vec<A> → Vec<A>)
/// - refract: identity (Vec<A> → Vec<A>)
impl<S: Clone + 'static, A: Clone + 'static> Prism for Fold<S, A> {
    type Input = Optic<(), S, Infallible, ScalarLoss>;
    type Focused = Optic<S, Vec<A>, Infallible, ScalarLoss>;
    type Projected = Optic<Vec<A>, Vec<A>, Infallible, ScalarLoss>;
    type Refracted = Optic<Vec<A>, Vec<A>, Infallible, ScalarLoss>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let s = beam.result().ok().expect("focus: Err beam").clone();
        let list = (self.fold_fn)(&s);
        beam.next(list)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let v = beam.result().ok().expect("project: Err beam").clone();
        beam.next(v)
    }

    fn refract(&self, beam: Self::Projected) -> Self::Refracted {
        let v = beam.result().ok().expect("refract: Err beam").clone();
        beam.next(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Beam as BeamTrait;

    #[derive(Clone)]
    struct Tree {
        leaves: Vec<i32>,
    }

    fn tree_leaves(t: &Tree) -> Vec<i32> {
        t.leaves.clone()
    }

    // --- Inherent method tests ---

    #[test]
    fn fold_extracts_list() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let tree = Tree {
            leaves: vec![1, 2, 3],
        };
        assert_eq!(f.to_list(&tree), vec![1, 2, 3]);
    }

    #[test]
    fn fold_empty_container() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let tree = Tree { leaves: vec![] };
        assert_eq!(f.to_list(&tree), Vec::<i32>::new());
    }

    // --- Prism trait tests ---

    fn seed<T: Clone>(v: T) -> Optic<(), T, Infallible, ScalarLoss> {
        Optic::ok((), v)
    }

    #[test]
    fn fold_prism_focus_extracts() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let beam = seed(Tree {
            leaves: vec![10, 20],
        });
        let focused = f.focus(beam);
        assert_eq!(focused.result().ok(), Some(&vec![10, 20]));
        // Input is preserved
        assert_eq!(focused.input().leaves, vec![10, 20]);
    }

    #[test]
    fn fold_prism_project_passes_through() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let focused = f.focus(seed(Tree {
            leaves: vec![5, 6, 7],
        }));
        let projected = f.project(focused);
        assert_eq!(projected.result().ok(), Some(&vec![5, 6, 7]));
    }

    #[test]
    fn fold_prism_refract_returns_list() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let focused = f.focus(seed(Tree { leaves: vec![1, 2] }));
        let projected = f.project(focused);
        let refracted = f.refract(projected);
        assert_eq!(refracted.result().ok(), Some(&vec![1, 2]));
    }

    #[test]
    fn fold_prism_is_lossless() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let focused = f.focus(seed(Tree { leaves: vec![1] }));
        assert!(!focused.is_partial());
    }

    /// Fold's split operation is user-space via smap.
    #[test]
    fn fold_split_via_smap() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let focused = f.focus(seed(Tree {
            leaves: vec![10, 20, 30],
        }));
        let sum = focused.smap(|v| terni::Imperfect::success(v.iter().sum::<i32>()));
        assert_eq!(sum.result().ok(), Some(&60));
    }

    #[test]
    fn fold_is_clone_and_copy() {
        let f: Fold<Tree, i32> = Fold::new(tree_leaves);
        let f2 = f; // Copy
        let f3 = f2.clone(); // Clone
        let tree = Tree { leaves: vec![1] };
        assert_eq!(f3.to_list(&tree), vec![1]);
    }

    // --- Addressable tests ---

    #[test]
    fn fold_same_fn_same_oid() {
        use crate::Addressable;
        let a: Fold<Tree, i32> = Fold::new(tree_leaves);
        let b: Fold<Tree, i32> = Fold::new(tree_leaves);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn fold_different_fn_different_oid() {
        use crate::Addressable;
        fn tree_count(t: &Tree) -> Vec<i32> {
            vec![t.leaves.len() as i32]
        }
        let a: Fold<Tree, i32> = Fold::new(tree_leaves);
        let b: Fold<Tree, i32> = Fold::new(tree_count);
        assert_ne!(a.oid(), b.oid());
    }
}

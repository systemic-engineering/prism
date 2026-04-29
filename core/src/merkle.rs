//! Merkle tree — content-addressed tree structure.
//!
//! Every node has data, children, and an Oid.
//! The Oid depends on the data AND the children's Oids.
//! Same content + same children = same Oid. Always.

use crate::oid::{Addressable, Oid};

/// A content-addressed tree. Every node carries data and children.
/// The Oid incorporates both, so identical structure = identical address.
pub trait MerkleTree: Addressable + Clone {
    /// The payload type at each node.
    type Data: PartialEq;

    /// The node's data payload.
    fn data(&self) -> &Self::Data;

    /// The node's children. Sorted by Oid for deterministic tree shape.
    fn children(&self) -> &[Self];

    /// Is this a leaf node?
    fn is_leaf(&self) -> bool {
        self.children().is_empty()
    }

    /// Number of children.
    fn degree(&self) -> usize {
        self.children().len()
    }
}

/// A difference between two merkle trees.
#[derive(Debug, Clone, PartialEq)]
pub enum Delta {
    /// A subtree was added (new Oid).
    Added(Oid),
    /// A subtree was removed (old Oid).
    Removed(Oid),
    /// A node changed from one Oid to another.
    Modified(Oid, Oid),
}

/// Diff two trees. Returns nodes that differ.
/// Identical subtrees are skipped entirely (O(delta) not O(n)).
pub fn diff<M: MerkleTree>(a: &M, b: &M) -> Vec<Delta> {
    if a.oid() == b.oid() {
        return vec![]; // identical subtrees — skip
    }

    let mut deltas = vec![];

    // Data changed at this node
    if a.data() != b.data() {
        deltas.push(Delta::Modified(a.oid(), b.oid()));
    }

    // Compare children by Oid (sorted)
    let a_children = a.children();
    let b_children = b.children();

    let mut ai = 0;
    let mut bi = 0;

    while ai < a_children.len() && bi < b_children.len() {
        let a_oid = a_children[ai].oid();
        let b_oid = b_children[bi].oid();

        match a_oid.cmp(&b_oid) {
            std::cmp::Ordering::Equal => {
                // Same Oid — identical subtree, skip both
                ai += 1;
                bi += 1;
            }
            std::cmp::Ordering::Less => {
                // a_oid < b_oid — a's child was removed
                deltas.push(Delta::Removed(a_oid));
                ai += 1;
            }
            std::cmp::Ordering::Greater => {
                // b_oid < a_oid — b's child was added
                deltas.push(Delta::Added(b_oid));
                bi += 1;
            }
        }
    }

    // Remaining a children were removed
    for child in &a_children[ai..] {
        deltas.push(Delta::Removed(child.oid()));
    }

    // Remaining b children were added
    for child in &b_children[bi..] {
        deltas.push(Delta::Added(child.oid()));
    }

    deltas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::luminosity::Luminosity;
    use crate::store::Store;
    use std::collections::HashMap;
    use terni::{Imperfect, Loss};

    // A simple test tree
    #[derive(Clone, Debug, PartialEq)]
    struct TestNode {
        name: String,
        children: Vec<TestNode>,
    }

    impl Addressable for TestNode {
        fn oid(&self) -> Oid {
            let mut content = self.name.clone();
            for child in &self.children {
                content.push_str(&format!(":{}", child.oid()));
            }
            Oid::hash(content.as_bytes())
        }
    }

    impl MerkleTree for TestNode {
        type Data = String;
        fn data(&self) -> &String {
            &self.name
        }
        fn children(&self) -> &[Self] {
            &self.children
        }
    }

    // --- MerkleTree trait tests ---

    #[test]
    fn merkle_leaf_has_no_children() {
        let leaf = TestNode {
            name: "x".into(),
            children: vec![],
        };
        assert!(leaf.is_leaf());
        assert_eq!(leaf.degree(), 0);
    }

    #[test]
    fn merkle_branch_is_not_leaf() {
        let branch = TestNode {
            name: "root".into(),
            children: vec![TestNode {
                name: "child".into(),
                children: vec![],
            }],
        };
        assert!(!branch.is_leaf());
        assert_eq!(branch.degree(), 1);
    }

    #[test]
    fn merkle_same_content_same_oid() {
        let a = TestNode {
            name: "x".into(),
            children: vec![],
        };
        let b = TestNode {
            name: "x".into(),
            children: vec![],
        };
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn merkle_different_data_different_oid() {
        let a = TestNode {
            name: "x".into(),
            children: vec![],
        };
        let b = TestNode {
            name: "y".into(),
            children: vec![],
        };
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn merkle_different_children_different_oid() {
        let a = TestNode {
            name: "root".into(),
            children: vec![TestNode {
                name: "a".into(),
                children: vec![],
            }],
        };
        let b = TestNode {
            name: "root".into(),
            children: vec![TestNode {
                name: "b".into(),
                children: vec![],
            }],
        };
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn merkle_data_accessor() {
        let node = TestNode {
            name: "hello".into(),
            children: vec![],
        };
        assert_eq!(node.data(), "hello");
    }

    #[test]
    fn merkle_children_accessor() {
        let child = TestNode {
            name: "child".into(),
            children: vec![],
        };
        let parent = TestNode {
            name: "parent".into(),
            children: vec![child.clone()],
        };
        assert_eq!(parent.children(), &[child]);
    }

    // --- diff tests ---

    #[test]
    fn merkle_diff_identical_trees_empty() {
        let a = TestNode {
            name: "x".into(),
            children: vec![],
        };
        let b = TestNode {
            name: "x".into(),
            children: vec![],
        };
        let d = diff(&a, &b);
        assert!(d.is_empty());
    }

    #[test]
    fn merkle_diff_different_data() {
        let a = TestNode {
            name: "x".into(),
            children: vec![],
        };
        let b = TestNode {
            name: "y".into(),
            children: vec![],
        };
        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert!(matches!(&d[0], Delta::Modified(_, _)));
    }

    #[test]
    fn merkle_diff_added_child() {
        let a = TestNode {
            name: "root".into(),
            children: vec![],
        };
        let child = TestNode {
            name: "new".into(),
            children: vec![],
        };
        let b = TestNode {
            name: "root".into(),
            children: vec![child.clone()],
        };
        let d = diff(&a, &b);
        // Data is same ("root"), but children differ
        assert!(d.iter().any(|delta| matches!(delta, Delta::Added(_))));
    }

    #[test]
    fn merkle_diff_removed_child() {
        let child = TestNode {
            name: "old".into(),
            children: vec![],
        };
        let a = TestNode {
            name: "root".into(),
            children: vec![child.clone()],
        };
        let b = TestNode {
            name: "root".into(),
            children: vec![],
        };
        let d = diff(&a, &b);
        assert!(d.iter().any(|delta| matches!(delta, Delta::Removed(_))));
    }

    #[test]
    fn merkle_diff_deep_identical_subtree_skipped() {
        let shared = TestNode {
            name: "shared".into(),
            children: vec![TestNode {
                name: "deep".into(),
                children: vec![],
            }],
        };
        let a = TestNode {
            name: "root".into(),
            children: vec![shared.clone()],
        };
        let b = TestNode {
            name: "root".into(),
            children: vec![shared],
        };
        let d = diff(&a, &b);
        assert!(d.is_empty());
    }

    // --- Store typed over MerkleTree ---

    #[derive(Debug, Clone, Default, PartialEq)]
    struct TestLoss(u32);

    impl Loss for TestLoss {
        fn zero() -> Self {
            TestLoss(0)
        }
        fn total() -> Self {
            TestLoss(u32::MAX)
        }
        fn is_zero(&self) -> bool {
            self.0 == 0
        }
        fn combine(self, other: Self) -> Self {
            TestLoss(self.0 + other.0)
        }
    }

    struct MemStore {
        data: HashMap<Oid, TestNode>,
    }

    impl Store for MemStore {
        type Tree = TestNode;
        type Error = String;
        type L = TestLoss;

        fn get(&self, oid: &Oid) -> Imperfect<TestNode, String, TestLoss> {
            match self.data.get(oid) {
                Some(node) => Imperfect::Success(node.clone()),
                None => Imperfect::Failure("not found".into(), TestLoss::zero()),
            }
        }

        fn put(&mut self, tree: TestNode) -> Imperfect<Oid, String, TestLoss> {
            let oid = tree.oid();
            self.data.insert(oid.clone(), tree);
            Imperfect::Success(oid)
        }

        fn has(&self, oid: &Oid) -> Imperfect<Luminosity, String, TestLoss> {
            if self.data.contains_key(oid) {
                Imperfect::Success(Luminosity::Light)
            } else {
                Imperfect::Success(Luminosity::Dark)
            }
        }
    }

    #[test]
    fn store_put_get_roundtrip() {
        let mut store = MemStore {
            data: HashMap::new(),
        };
        let node = TestNode {
            name: "hello".into(),
            children: vec![],
        };
        let oid = store.put(node.clone()).ok().unwrap();
        let got = store.get(&oid).ok().unwrap();
        assert_eq!(got, node);
    }

    #[test]
    fn store_get_missing_is_failure() {
        let store = MemStore {
            data: HashMap::new(),
        };
        let result = store.get(&Oid::hash(b"nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn store_has_returns_luminosity() {
        let mut store = MemStore {
            data: HashMap::new(),
        };
        let node = TestNode {
            name: "test".into(),
            children: vec![],
        };
        let oid = node.oid();

        let before = store.has(&oid);
        assert_eq!(before.ok(), Some(Luminosity::Dark));

        let _ = store.put(node);

        let after = store.has(&oid);
        assert_eq!(after.ok(), Some(Luminosity::Light));
    }

    #[test]
    fn store_put_tree_with_children() {
        let mut store = MemStore {
            data: HashMap::new(),
        };
        let tree = TestNode {
            name: "root".into(),
            children: vec![
                TestNode {
                    name: "a".into(),
                    children: vec![],
                },
                TestNode {
                    name: "b".into(),
                    children: vec![],
                },
            ],
        };
        let oid = store.put(tree.clone()).ok().unwrap();
        let got = store.get(&oid).ok().unwrap();
        assert_eq!(got.degree(), 2);
        assert_eq!(got, tree);
    }
}

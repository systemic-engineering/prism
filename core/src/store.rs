//! Store — where crystals live.

use crate::merkle::MerkleTree;
use crate::oid::Oid;
use crate::luminosity::Luminosity;
use terni::{Imperfect, Loss};

/// Where crystals live. Every operation returns Imperfect.
///
/// The Store is the third primitive alongside Beam and Prism.
/// - Beam: the value in motion
/// - Prism: the transformation
/// - Store: the persistence
///
/// Typed over a MerkleTree — the store knows its tree shape.
pub trait Store {
    /// The tree type this store persists.
    type Tree: MerkleTree;
    type Error;
    type L: Loss;

    /// Retrieve a tree by address. Partial if some dimensions are dark.
    fn get(&self, oid: &Oid) -> Imperfect<Self::Tree, Self::Error, Self::L>;

    /// Persist a tree. Returns its Oid. Partial if not fully replicated.
    fn put(&mut self, tree: Self::Tree) -> Imperfect<Oid, Self::Error, Self::L>;

    /// Check luminosity at address. Light, Dimmed, or Dark.
    fn has(&self, oid: &Oid) -> Imperfect<Luminosity, Self::Error, Self::L>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merkle::MerkleTree;
    use crate::oid::{Oid, Addressable};
    use crate::luminosity::Luminosity;
    use std::collections::HashMap;

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
        fn data(&self) -> &String { &self.name }
        fn children(&self) -> &[Self] { &self.children }
    }

    /// In-memory store for testing.
    struct MemoryStore {
        data: HashMap<Oid, TestNode>,
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    struct MemoryLoss(u32);

    impl Loss for MemoryLoss {
        fn zero() -> Self { MemoryLoss(0) }
        fn total() -> Self { MemoryLoss(u32::MAX) }
        fn is_zero(&self) -> bool { self.0 == 0 }
        fn combine(self, other: Self) -> Self { MemoryLoss(self.0 + other.0) }
    }

    impl Store for MemoryStore {
        type Tree = TestNode;
        type Error = String;
        type L = MemoryLoss;

        fn get(&self, oid: &Oid) -> Imperfect<TestNode, Self::Error, Self::L> {
            match self.data.get(oid) {
                Some(node) => Imperfect::Success(node.clone()),
                None => Imperfect::Failure(
                    format!("not found: {:?}", oid),
                    MemoryLoss::zero(),
                ),
            }
        }

        fn put(&mut self, tree: TestNode) -> Imperfect<Oid, Self::Error, Self::L> {
            let oid = tree.oid();
            self.data.insert(oid.clone(), tree);
            Imperfect::Success(oid)
        }

        fn has(&self, oid: &Oid) -> Imperfect<Luminosity, Self::Error, Self::L> {
            if self.data.contains_key(oid) {
                Imperfect::Success(Luminosity::Light)
            } else {
                Imperfect::Success(Luminosity::Dark)
            }
        }
    }

    #[test]
    fn store_put_get_roundtrip() {
        let mut store = MemoryStore { data: HashMap::new() };
        let node = TestNode { name: "hello".into(), children: vec![] };
        let oid = node.oid();

        let put_result = store.put(node.clone());
        assert!(put_result.is_ok());

        let get_result = store.get(&oid);
        assert!(get_result.is_ok());
        assert_eq!(get_result.ok(), Some(node));
    }

    #[test]
    fn store_get_missing_is_failure() {
        let store = MemoryStore { data: HashMap::new() };
        let result = store.get(&Oid::hash(b"nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn store_has_returns_luminosity() {
        let mut store = MemoryStore { data: HashMap::new() };
        let node = TestNode { name: "test".into(), children: vec![] };
        let oid = node.oid();

        let before = store.has(&oid);
        assert_eq!(before.ok(), Some(Luminosity::Dark));

        let _ = store.put(node);

        let after = store.has(&oid);
        assert_eq!(after.ok(), Some(Luminosity::Light));
    }
}

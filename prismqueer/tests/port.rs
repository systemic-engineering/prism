//! One port (prism-017 ask 7, prism-019 order item 4): `brain::Backend` is
//! the port, because it is the one with a proof (`docs/spec/brain.md`).
//! `store::Store` is retired, and what it did that was real is done through
//! `Backend`.
//!
//! The capability that moved is the checked read: `Store::get` handed back
//! a value by address and believed the adapter. Through the port, a read is
//! [`brain::recall`]: fetch, rehash, and refuse bytes that aren't the
//! address's (A2, "integrity checked on every read; durability is the
//! adapter's obligation"). It is a free function over any `Backend`, not a
//! trait method, so no adapter can override it and skip the rehash.
//!
//! The migrated caller is the Merkle tree round trip
//! (`merkle::tests::store_put_tree_with_children`): every node persists as
//! a crystal of its canonical bytes, so a node's crystal address *is* its
//! tree address, and the whole tree reads back from its root's address.

use std::collections::BTreeMap;
use std::convert::Infallible;

use prismqueer::brain::{self, Backend, Crystal, Loss, Memory, Purge, Receipt, Recall};
use prismqueer::merkle::{diff, MerkleTree};
use prismqueer::oid::{Addressable, Oid, Scope};

/// A Merkle node: a name and its children. Its address hashes its
/// canonical bytes, which name each child by address.
#[derive(Clone, Debug, PartialEq)]
struct Node {
    name: String,
    children: Vec<Node>,
}

impl Node {
    fn leaf(name: &str) -> Self {
        Node {
            name: name.into(),
            children: vec![],
        }
    }

    fn branch(name: &str, children: Vec<Node>) -> Self {
        Node {
            name: name.into(),
            children,
        }
    }

    /// One line: the name, then each child's address. Never a child's
    /// bytes, so a node's crystal stays one node.
    fn canonical(&self) -> Vec<u8> {
        let mut line = self.name.clone();
        for child in &self.children {
            line.push(' ');
            line.push_str(child.oid().as_str());
        }
        line.into_bytes()
    }

    /// Persist every node, children first, through the port.
    fn persist<B: Backend>(&self, backend: &mut B) -> Result<Oid, B::Error> {
        for child in &self.children {
            child.persist(backend)?;
        }
        let crystal = Crystal::settle(&Scope::public(), self.canonical());
        backend.persist(&crystal)?;
        Ok(crystal.address().clone())
    }

    /// Read a tree back by its root's address. Every node is a checked read.
    fn recall<B: Backend>(backend: &B, address: &Oid) -> Result<Node, Recall<B::Error>> {
        let crystal = brain::recall(backend, &Scope::public(), address)?;
        let line = String::from_utf8_lossy(crystal.bytes()).into_owned();
        let mut words = line.split(' ');
        let name = words.next().unwrap_or_default().to_owned();
        let children = words
            .map(|child| Node::recall(backend, &Oid::new(child.to_owned())))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Node { name, children })
    }
}

impl Addressable for Node {
    fn oid(&self) -> Oid {
        Oid::hash(&Scope::public(), &self.canonical())
    }
}

impl MerkleTree for Node {
    type Data = String;
    fn data(&self) -> &String {
        &self.name
    }
    fn children(&self) -> &[Self] {
        &self.children
    }
}

fn tree() -> Node {
    Node::branch(
        "root",
        vec![Node::leaf("a"), Node::branch("b", vec![Node::leaf("c")])],
    )
}

/// What `Store::put` then `Store::get` did, through `Backend`: the tree
/// persists, and its root's address reads the same tree back.
#[test]
fn a_merkle_tree_persists_and_reads_back_through_the_port() {
    let tree = tree();
    let mut backend = Memory::default();

    let root = tree.persist(&mut backend).unwrap();

    assert_eq!(
        root,
        tree.oid(),
        "a node's crystal address is its tree address"
    );
    assert_eq!(backend.len(), 4, "one crystal per node");
    let back = Node::recall(&backend, &root).unwrap();
    assert_eq!(back, tree);
    assert!(diff(&tree, &back).is_empty());
}

/// What `Store::has` said as `Dark`: an address never persisted is `Lost`,
/// named, never an empty value.
#[test]
fn an_address_never_persisted_is_lost() {
    let backend = Memory::default();
    let never = Oid::hash(&Scope::public(), b"never");
    assert_eq!(
        brain::recall(&backend, &Scope::public(), &never),
        Err(Recall::Lost {
            address: never,
            reason: Loss::Missing
        })
    );
}

/// A backend that swaps two crystals' bytes: each address hands back the
/// other's, and both are well-formed crystals.
struct Swapping {
    held: BTreeMap<Oid, Vec<u8>>,
}

impl Backend for Swapping {
    type Error = Infallible;
    fn persist(&mut self, crystal: &Crystal) -> Result<(), Infallible> {
        self.held
            .insert(crystal.address().clone(), crystal.bytes().to_vec());
        Ok(())
    }
    fn fetch(&self, address: &Oid) -> Result<Option<Vec<u8>>, Infallible> {
        Ok(self
            .held
            .iter()
            .find(|(other, _)| *other != address)
            .map(|(_, bytes)| bytes.clone()))
    }
    fn purge(&mut self, order: &Purge<'_>) -> Result<(), Infallible> {
        self.held.remove(order.address());
        Ok(())
    }
    fn erased(&self, _: &Oid) -> Result<Option<Receipt>, Infallible> {
        Ok(None)
    }
}

/// A2 on the moved read: bytes that don't rehash to the address are
/// `Forged`, never believed, even when they are another real crystal's.
#[test]
fn a_backend_that_hands_back_other_bytes_is_forged_never_believed() {
    let mut backend = Swapping {
        held: BTreeMap::new(),
    };
    let a = Crystal::settle(&Scope::public(), b"reach a".to_vec());
    let b = Crystal::settle(&Scope::public(), b"reach b".to_vec());
    backend.persist(&a).unwrap();
    backend.persist(&b).unwrap();

    assert_eq!(
        brain::recall(&backend, &Scope::public(), a.address()),
        Err(Recall::Forged(a.address().clone()))
    );
    assert_eq!(
        brain::recall(&backend, &Scope::public(), b.address()),
        Err(Recall::Forged(b.address().clone()))
    );

    let mut tree_backend = Swapping {
        held: BTreeMap::new(),
    };
    let root = Node::branch("root", vec![Node::leaf("a")])
        .persist(&mut tree_backend)
        .unwrap();
    assert!(
        matches!(Node::recall(&tree_backend, &root), Err(Recall::Forged(_))),
        "a forged node anywhere in the tree refuses the whole read"
    );
}

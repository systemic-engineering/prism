//! RFC 9162 §2.1: the Merkle tree hash over SHA-256, and the verification
//! of inclusion and consistency proofs, as the RFC's algorithm text gives
//! them.
//!
//! What a leaf *is* stays the caller's: every check takes a leaf hash,
//! `SHA-256(0x00 ‖ leaf)`, so the leaf's bytes (the identity question,
//! prism-019 ask 1) are not pinned here.

use crate::Hash;

/// Why a proof did not verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofError {
    /// Inclusion: `index >= size`.
    IndexOutOfRange,
    /// Consistency: `first > second`.
    SizeOrder,
    /// Consistency from the empty tree: RFC 9162 defines no proof for it.
    EmptyFirst,
    /// Consistency: the path is empty where the algorithm needs one.
    EmptyPath,
    /// The path has more nodes than the tree has levels.
    PathTooLong,
    /// The path ended before reaching the root.
    PathTooShort,
    /// Consistency: the recomputed first root differs from the one given.
    FirstRootMismatch,
    /// The recomputed root differs from the one given.
    RootMismatch,
}

/// `SHA-256(0x00 ‖ leaf)`.
pub fn leaf_hash(leaf: &[u8]) -> Hash {
    let _ = leaf;
    [0; 32]
}

/// `SHA-256(0x01 ‖ left ‖ right)`.
pub fn node_hash(left: &Hash, right: &Hash) -> Hash {
    let _ = (left, right);
    [0; 32]
}

/// MTH({}) = SHA-256 of the empty string.
pub fn empty_root() -> Hash {
    [0; 32]
}

/// MTH over already-hashed leaves (RFC 9162 §2.1.1).
pub fn root(leaf_hashes: &[Hash]) -> Hash {
    let _ = leaf_hashes;
    [0; 32]
}

/// RFC 9162 §2.1.3.2: does `path` prove `leaf_hash` at `index` in the tree
/// of `size` leaves with root `root`?
pub fn verify_inclusion(
    index: u64,
    size: u64,
    leaf_hash: &Hash,
    path: &[Hash],
    root: &Hash,
) -> Result<(), ProofError> {
    let _ = (index, size, leaf_hash, path, root);
    Err(ProofError::RootMismatch)
}

/// RFC 9162 §2.1.4.2: does `path` prove the tree of `first` leaves with
/// root `first_root` a prefix of the tree of `second` leaves with root
/// `second_root`?
///
/// Equal sizes verify with an empty path and equal roots. A first size of
/// zero is refused: RFC 9162 defines proofs for `0 < first < second`, and
/// the empty tree proves nothing about a reader's earlier view.
pub fn verify_consistency(
    first: u64,
    second: u64,
    first_root: &Hash,
    second_root: &Hash,
    path: &[Hash],
) -> Result<(), ProofError> {
    let _ = (first, second, first_root, second_root, path);
    Err(ProofError::RootMismatch)
}

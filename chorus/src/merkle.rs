//! RFC 9162 §2.1: the Merkle tree hash over SHA-256, and the verification
//! of inclusion and consistency proofs, as the RFC's algorithm text gives
//! them.
//!
//! What a leaf *is* stays the caller's: every check takes a leaf hash,
//! `SHA-256(0x00 ‖ leaf)`, so the leaf's bytes (the identity question,
//! prism-019 ask 1) are not pinned here.

use sha2::{Digest, Sha256};

use crate::Hash;

/// RFC 9162 §2.1.1: the leaf prefix.
const LEAF: u8 = 0x00;
/// RFC 9162 §2.1.1: the node prefix.
const NODE: u8 = 0x01;

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
    Sha256::new()
        .chain_update([LEAF])
        .chain_update(leaf)
        .finalize()
        .into()
}

/// `SHA-256(0x01 ‖ left ‖ right)`.
pub fn node_hash(left: &Hash, right: &Hash) -> Hash {
    Sha256::new()
        .chain_update([NODE])
        .chain_update(left)
        .chain_update(right)
        .finalize()
        .into()
}

/// MTH({}) = SHA-256 of the empty string.
pub fn empty_root() -> Hash {
    Sha256::digest([]).into()
}

/// MTH over already-hashed leaves (RFC 9162 §2.1.1).
pub fn root(leaf_hashes: &[Hash]) -> Hash {
    match leaf_hashes {
        [] => empty_root(),
        [one] => *one,
        _ => {
            let k = split(leaf_hashes.len());
            node_hash(&root(&leaf_hashes[..k]), &root(&leaf_hashes[k..]))
        }
    }
}

/// The largest power of two smaller than `n` (n > 1).
fn split(n: usize) -> usize {
    let mut k = 1;
    while k * 2 < n {
        k *= 2;
    }
    k
}

/// "Right-shift both fn and sn equally until either LSB(fn) is set or fn
/// is 0."
fn shift_to_set_or_zero(fn_: &mut u64, sn: &mut u64) {
    while *fn_ != 0 && *fn_ & 1 == 0 {
        *fn_ >>= 1;
        *sn >>= 1;
    }
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
    // 1. leaf_index >= tree_size fails.
    if index >= size {
        return Err(ProofError::IndexOutOfRange);
    }
    // 2. fn = leaf_index, sn = tree_size - 1. 3. r = hash.
    let (mut fn_, mut sn) = (index, size - 1);
    let mut r = *leaf_hash;
    // 4. For each p in inclusion_path:
    for p in path {
        if sn == 0 {
            return Err(ProofError::PathTooLong);
        }
        if fn_ & 1 == 1 || fn_ == sn {
            r = node_hash(p, &r);
            if fn_ & 1 == 0 {
                shift_to_set_or_zero(&mut fn_, &mut sn);
            }
        } else {
            r = node_hash(&r, p);
        }
        fn_ >>= 1;
        sn >>= 1;
    }
    // 5. sn = 0 and r = root_hash.
    if sn != 0 {
        return Err(ProofError::PathTooShort);
    }
    if r != *root {
        return Err(ProofError::RootMismatch);
    }
    Ok(())
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
    if first > second {
        return Err(ProofError::SizeOrder);
    }
    if first == second {
        if !path.is_empty() {
            return Err(ProofError::PathTooLong);
        }
        if first_root != second_root {
            return Err(ProofError::RootMismatch);
        }
        return Ok(());
    }
    if first == 0 {
        return Err(ProofError::EmptyFirst);
    }
    // 1. An empty consistency_path fails.
    let Some((head, tail)) = path.split_first() else {
        return Err(ProofError::EmptyPath);
    };
    // 2. If first is an exact power of 2, prepend first_hash: the seed is
    //    first_hash and every path node follows it.
    let (seed, rest) = if first.is_power_of_two() {
        (first_root, path)
    } else {
        (head, tail)
    };
    // 3. fn = first - 1, sn = second - 1.
    let (mut fn_, mut sn) = (first - 1, second - 1);
    // 4. If LSB(fn) is set, right-shift both until it is not.
    while fn_ & 1 == 1 {
        fn_ >>= 1;
        sn >>= 1;
    }
    // 5. fr = sr = the first value.
    let (mut fr, mut sr) = (*seed, *seed);
    // 6. For each subsequent value c:
    for c in rest {
        if sn == 0 {
            return Err(ProofError::PathTooLong);
        }
        if fn_ & 1 == 1 || fn_ == sn {
            fr = node_hash(c, &fr);
            sr = node_hash(c, &sr);
            if fn_ & 1 == 0 {
                shift_to_set_or_zero(&mut fn_, &mut sn);
            }
        } else {
            sr = node_hash(&sr, c);
        }
        fn_ >>= 1;
        sn >>= 1;
    }
    // 7. fr = first_hash, sr = second_hash, sn = 0.
    if sn != 0 {
        return Err(ProofError::PathTooShort);
    }
    if fr != *first_root {
        return Err(ProofError::FirstRootMismatch);
    }
    if sr != *second_root {
        return Err(ProofError::RootMismatch);
    }
    Ok(())
}

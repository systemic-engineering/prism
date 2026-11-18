//! RFC 9162 §2.1 against the vectors: ledger-vectors.json (every proof over
//! the RFC 6962 leaves, sizes 1..8, from the RFC's text) and ledger-probes.json
//! (transparency-dev/merkle's published probes, good and bad).

use chorus::merkle::{
    empty_root, leaf_hash, node_hash, root, verify_consistency, verify_inclusion, ProofError,
};
use serde_json::Value;

use crate::common::{b64_hash, flip, hash_hex, hex, sha256, Hash};

const VECTORS: &str = include_str!("../../docs/spec/ledger-vectors.json");
const PROBES: &str = include_str!("../../docs/spec/ledger-probes.json");

fn vectors() -> Value {
    serde_json::from_str(VECTORS).unwrap()
}

fn hashes(v: &Value) -> Vec<Hash> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|h| hash_hex(h.as_str().unwrap()))
        .collect()
}

fn leaf_hashes() -> Vec<Hash> {
    vectors()["leaves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| leaf_hash(&hex(l.as_str().unwrap())))
        .collect()
}

struct Inclusion {
    index: u64,
    size: u64,
    leaf: Hash,
    path: Vec<Hash>,
    root: Hash,
}

fn inclusions() -> Vec<Inclusion> {
    vectors()["inclusion"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| Inclusion {
            index: c["index"].as_u64().unwrap(),
            size: c["size"].as_u64().unwrap(),
            leaf: hash_hex(c["leaf_hash"].as_str().unwrap()),
            path: hashes(&c["path"]),
            root: hash_hex(c["root"].as_str().unwrap()),
        })
        .collect()
}

struct Consistency {
    first: u64,
    second: u64,
    path: Vec<Hash>,
    first_root: Hash,
    second_root: Hash,
}

fn consistencies() -> Vec<Consistency> {
    vectors()["consistency"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| Consistency {
            first: c["first"].as_u64().unwrap(),
            second: c["second"].as_u64().unwrap(),
            path: hashes(&c["path"]),
            first_root: hash_hex(c["first_root"].as_str().unwrap()),
            second_root: hash_hex(c["second_root"].as_str().unwrap()),
        })
        .collect()
}

// ── Tree hashes ──────────────────────────────────────────────────────────────────────

#[test]
fn the_empty_tree_is_sha256_of_nothing() {
    assert_eq!(
        empty_root(),
        hash_hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
    assert_eq!(root(&[]), empty_root());
}

#[test]
fn leaf_and_node_hashes_carry_their_prefixes() {
    // transparency-dev NodeHashes()[0][0] and [1][0].
    let h0 = leaf_hash(b"");
    let h1 = leaf_hash(&[0x00]);
    assert_eq!(
        h0,
        hash_hex("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d")
    );
    assert_eq!(h0, sha256(&[&[0x00]]));
    assert_eq!(
        node_hash(&h0, &h1),
        hash_hex("fac54203e7cc696cf0dfcb42c92a1d9dbaf70ad9e621f4bd8d98662f00e3c125")
    );
    assert_eq!(node_hash(&h0, &h1), sha256(&[&[0x01], &h0, &h1]));
}

#[test]
fn mth_matches_the_published_roots_for_sizes_0_to_8() {
    let leaves = leaf_hashes();
    let roots = hashes(&vectors()["roots"]);
    assert_eq!(roots.len(), 9);
    for (n, want) in roots.iter().enumerate() {
        assert_eq!(&root(&leaves[..n]), want, "MTH of size {n}");
    }
}

/// A node's preimage passed off as a leaf: the second-preimage attack the
/// 0x00 / 0x01 prefixes exist to stop.
#[test]
fn an_inner_node_cannot_pose_as_a_leaf() {
    let leaves = leaf_hashes();
    let two = root(&leaves[..2]);
    // The node's children, with and without its 0x01 prefix.
    for prefix in [&[][..], &[0x01]] {
        let mut forged = prefix.to_vec();
        forged.extend_from_slice(&leaves[0]);
        forged.extend_from_slice(&leaves[1]);
        assert_eq!(
            verify_inclusion(0, 1, &leaf_hash(&forged), &[], &two),
            Err(ProofError::RootMismatch)
        );
    }
}

// ── Inclusion (§2.1.3.2) ─────────────────────────────────────────────────────────────

#[test]
fn every_inclusion_proof_up_to_size_8_verifies() {
    let cases = inclusions();
    assert_eq!(cases.len(), 36);
    for c in &cases {
        assert_eq!(
            verify_inclusion(c.index, c.size, &c.leaf, &c.path, &c.root),
            Ok(()),
            "leaf {} of {}",
            c.index,
            c.size
        );
    }
}

#[test]
fn inclusion_at_sizes_1_2_3_7_8() {
    let cases = inclusions();
    for size in [1, 2, 3, 7, 8] {
        let at: Vec<_> = cases.iter().filter(|c| c.size == size).collect();
        assert_eq!(at.len() as u64, size);
        for c in at {
            assert_eq!(
                verify_inclusion(c.index, c.size, &c.leaf, &c.path, &c.root),
                Ok(())
            );
        }
    }
}

#[test]
fn inclusion_for_the_wrong_index_rejects() {
    for c in inclusions().iter().filter(|c| c.size > 1) {
        let wrong = (c.index + 1) % c.size;
        assert!(
            verify_inclusion(wrong, c.size, &c.leaf, &c.path, &c.root).is_err(),
            "leaf {} of {} accepted at {wrong}",
            c.index,
            c.size
        );
    }
}

#[test]
fn an_index_past_the_tree_rejects() {
    for c in inclusions() {
        assert_eq!(
            verify_inclusion(c.size, c.size, &c.leaf, &c.path, &c.root),
            Err(ProofError::IndexOutOfRange)
        );
    }
}

#[test]
fn a_truncated_inclusion_proof_rejects() {
    for c in inclusions().iter().filter(|c| !c.path.is_empty()) {
        let short = &c.path[..c.path.len() - 1];
        assert_eq!(
            verify_inclusion(c.index, c.size, &c.leaf, short, &c.root),
            Err(ProofError::PathTooShort),
            "leaf {} of {}",
            c.index,
            c.size
        );
    }
}

#[test]
fn an_inclusion_proof_with_an_extra_node_rejects() {
    for c in inclusions() {
        let mut long = c.path.clone();
        long.push(c.root);
        assert_eq!(
            verify_inclusion(c.index, c.size, &c.leaf, &long, &c.root),
            Err(ProofError::PathTooLong)
        );
    }
}

#[test]
fn inclusion_under_the_wrong_root_rejects() {
    for c in inclusions() {
        let wrong: Hash = flip(&c.root, 31).try_into().unwrap();
        assert_eq!(
            verify_inclusion(c.index, c.size, &c.leaf, &c.path, &wrong),
            Err(ProofError::RootMismatch)
        );
    }
}

#[test]
fn inclusion_of_the_wrong_leaf_rejects() {
    for c in inclusions() {
        let wrong: Hash = flip(&c.leaf, 0).try_into().unwrap();
        assert!(verify_inclusion(c.index, c.size, &wrong, &c.path, &c.root).is_err());
    }
}

// ── Consistency (§2.1.4.2) ──────────────────────────────────────────────────────────

#[test]
fn every_consistency_proof_up_to_size_8_verifies() {
    let cases = consistencies();
    assert_eq!(cases.len(), 28);
    for c in &cases {
        assert_eq!(
            verify_consistency(c.first, c.second, &c.first_root, &c.second_root, &c.path),
            Ok(()),
            "{} -> {}",
            c.first,
            c.second
        );
    }
}

#[test]
fn consistency_into_sizes_2_3_7_8() {
    let cases = consistencies();
    for second in [2, 3, 7, 8] {
        let into: Vec<_> = cases.iter().filter(|c| c.second == second).collect();
        assert_eq!(into.len() as u64, second - 1);
        for c in into {
            assert_eq!(
                verify_consistency(c.first, c.second, &c.first_root, &c.second_root, &c.path),
                Ok(())
            );
        }
    }
}

#[test]
fn a_truncated_consistency_proof_rejects() {
    for c in consistencies() {
        let short = &c.path[..c.path.len() - 1];
        assert!(
            verify_consistency(c.first, c.second, &c.first_root, &c.second_root, short).is_err(),
            "{} -> {}",
            c.first,
            c.second
        );
    }
}

#[test]
fn consistency_under_the_wrong_roots_rejects() {
    for c in consistencies() {
        let r1: Hash = flip(&c.first_root, 7).try_into().unwrap();
        let r2: Hash = flip(&c.second_root, 7).try_into().unwrap();
        assert!(verify_consistency(c.first, c.second, &r1, &c.second_root, &c.path).is_err());
        assert_eq!(
            verify_consistency(c.first, c.second, &c.first_root, &r2, &c.path),
            Err(ProofError::RootMismatch)
        );
    }
}

#[test]
fn consistency_with_the_wrong_sizes_rejects() {
    for c in consistencies() {
        if c.first > 1 {
            assert!(verify_consistency(
                c.first - 1,
                c.second,
                &c.first_root,
                &c.second_root,
                &c.path
            )
            .is_err());
        }
    }
}

/// RFC 9162 §2.1.4.2 binds the two roots, not the second size: for 16 of
/// the 28 pairs, the proof for (m, n) also verifies as (m, n + 1). The
/// size is bound by the signed checkpoint, and no tree of n + 1 leaves has
/// root MTH(D_n) without a SHA-256 collision across the 0x00 / 0x01
/// domains. transparency-dev's verifier accepts the same (its proof-length
/// check is equal for both sizes). Pinned, so a change shows.
#[test]
fn consistency_binds_the_roots_not_the_second_size() {
    let leaves = leaf_hashes();
    let mut also = vec![];
    for c in consistencies() {
        let up = c.second + 1;
        if verify_consistency(c.first, up, &c.first_root, &c.second_root, &c.path).is_ok() {
            also.push((c.first, c.second));
            if up as usize <= leaves.len() {
                assert_ne!(root(&leaves[..up as usize]), c.second_root);
            }
        }
    }
    assert_eq!(
        also,
        [
            (1, 3),
            (2, 3),
            (1, 5),
            (2, 5),
            (3, 5),
            (4, 5),
            (1, 6),
            (2, 6),
            (3, 6),
            (4, 6),
            (1, 7),
            (2, 7),
            (3, 7),
            (4, 7),
            (5, 7),
            (6, 7)
        ]
    );
}

#[test]
fn consistency_edges() {
    let (r1, r2) = (root(&leaf_hashes()[..1]), root(&leaf_hashes()[..2]));
    // Equal sizes: an empty proof and equal roots.
    assert_eq!(verify_consistency(1, 1, &r1, &r1, &[]), Ok(()));
    assert_eq!(
        verify_consistency(1, 1, &r1, &r2, &[]),
        Err(ProofError::RootMismatch)
    );
    assert_eq!(
        verify_consistency(1, 1, &r1, &r1, &[r1]),
        Err(ProofError::PathTooLong)
    );
    // Backwards.
    assert_eq!(
        verify_consistency(2, 1, &r2, &r1, &[]),
        Err(ProofError::SizeOrder)
    );
    // From the empty tree: no RFC 9162 proof exists.
    assert_eq!(
        verify_consistency(0, 2, &empty_root(), &r2, &[]),
        Err(ProofError::EmptyFirst)
    );
    // An empty path between different sizes.
    assert_eq!(
        verify_consistency(1, 2, &r1, &r2, &[]),
        Err(ProofError::EmptyPath)
    );
}

// ── transparency-dev/merkle's published probes ───────────────────────────────────────────

/// Every hash must decode to 32 bytes; a probe whose bytes don't is
/// rejected before any proof runs (it can't be a SHA-256 value).
fn decode_all(values: &[&Value]) -> Option<Vec<Hash>> {
    values.iter().map(|v| b64_hash(v.as_str()?)).collect()
}

fn path_of(v: &Value) -> Option<Vec<Hash>> {
    match v {
        Value::Null => Some(vec![]),
        Value::Array(a) => decode_all(&a.iter().collect::<Vec<_>>()),
        _ => None,
    }
}

#[test]
fn the_published_inclusion_probes() {
    let probes: Value = serde_json::from_str(PROBES).unwrap();
    let all = probes["inclusion"].as_array().unwrap();
    assert_eq!(all.len(), 98);
    let (mut good, mut bad) = (0, 0);
    for p in all {
        let want_err = p["wantErr"].as_bool().unwrap();
        let verdict = match (
            decode_all(&[&p["leafHash"], &p["root"]]),
            path_of(&p["proof"]),
        ) {
            (Some(h), Some(path)) => verify_inclusion(
                p["leafIdx"].as_u64().unwrap(),
                p["treeSize"].as_u64().unwrap(),
                &h[0],
                &path,
                &h[1],
            ),
            _ => Err(ProofError::RootMismatch),
        };
        assert_eq!(verdict.is_err(), want_err, "{}", p["file"]);
        if want_err {
            bad += 1
        } else {
            good += 1
        }
    }
    assert_eq!((good, bad), (6, 92));
}

#[test]
fn the_published_consistency_probes() {
    let probes: Value = serde_json::from_str(PROBES).unwrap();
    let all = probes["consistency"].as_array().unwrap();
    assert_eq!(all.len(), 98);
    let (mut good, mut bad) = (0, 0);
    for p in all {
        let want_err = p["wantErr"].as_bool().unwrap();
        let verdict = match (
            decode_all(&[&p["root1"], &p["root2"]]),
            path_of(&p["proof"]),
        ) {
            (Some(h), Some(path)) => verify_consistency(
                p["size1"].as_u64().unwrap(),
                p["size2"].as_u64().unwrap(),
                &h[0],
                &h[1],
                &path,
            ),
            _ => Err(ProofError::RootMismatch),
        };
        // The one probe a typed root can't express: equal sizes whose
        // "roots" are 12 equal bytes ("don't care 2"). Equal sizes compare
        // roots only, so it passes there; here nothing but 32 bytes is a root.
        if p["file"] == "consistency/additional/sizes-are-equal-one-and-proof-is-empty.json" {
            assert!(!want_err && verdict.is_err());
            continue;
        }
        assert_eq!(verdict.is_err(), want_err, "{}", p["file"]);
        if want_err {
            bad += 1
        } else {
            good += 1
        }
    }
    assert_eq!((good, bad), (5, 92));
}

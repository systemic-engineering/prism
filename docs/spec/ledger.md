# Ledger: what `ledger-verify` checks, under which assumptions

*prism, 2026-11-17 (prism-019, the verifier crate; decisions in prism-019's Log of 2026-11-16/17 and bullet-up-008). The crate verifies; it never holds, makes or sees a private key. Test vectors: `ledger-vectors.json` (generated from RFC 9162's text by `ledger-vectors.py`) and `ledger-probes.json` (transparency-dev/merkle's published probes, bundled unchanged). `ledger-verify/tests/` holds the crate to both.*

## The crate

`ledger-verify/` is a workspace crate of its own:

- **`#![no_std]` + `alloc`, `#![forbid(unsafe_code)]`.** No I/O, no clock, no RNG, no LAPACK, no dependency on prismqueer.
- **Every dependency without default features**, so nothing that generates keys or reads the OS is linked into the library.
- **Portable, and checked:** the flake's `checks.ledger-portable` builds it in release for `wasm32-unknown-unknown` (a reader's browser) and for `thumbv7em-none-eabihf`, a target with no `std` at all. A dependency that pulled in `std` would fail the second build. It uses a toolchain of its own (the same rustc 1.94.0 with those two targets), so the gate's toolchain and its cache are unchanged.

| Crate | Version | Used for | Standard |
|---|---|---|---|
| `sha2` | 0.11.1 | every hash | FIPS 180-4 |
| `base64ct` | 1.8.3 | base64, canonical only | RFC 4648 §4, §3.5 |
| `ed25519-dalek` | 3.0.0 | `verify_strict` only | RFC 8032 |
| `ml-dsa` | 0.1.2 | ML-DSA-44 `Verify` (Algorithm 3) | FIPS 204 (final); the crate runs Wycheproof's vectors |
| `slh-dsa` | **=0.2.0-rc.5** | SLH-DSA-SHA2-128s `slh_verify` (Algorithm 24) | FIPS 205 (final since 0.1.0) |

**`slh-dsa` is a release candidate, pinned exactly.** Its only stable release, 0.1.0 (2024-08-18: "FIPS 205 Initial Public Draft → FIPS 205 Final"), sits on `signature` 2 / `sha2` 0.10 and itself depends on a `hybrid-array` release candidate. The 0.2 line shares `signature` 3, `sha2` 0.11 and `hybrid-array` 0.4 with `ml-dsa` and `ed25519-dalek`; it has been at rc.5 since 2026-04-28. Move to 0.2.0 when it's released. Its context handling is FIPS 205's: `M′ = 0 ‖ |ctx| ‖ ctx ‖ M`.

## 1. The Merkle log (RFC 9162 §2.1, SHA-256)

```
leaf_hash(d)   = SHA-256(0x00 ‖ d)
node_hash(l,r) = SHA-256(0x01 ‖ l ‖ r)
MTH({})        = SHA-256("")
MTH(D[n])      = node_hash(MTH(D[0:k]), MTH(D[k:n])),  k the largest power of two < n
```

- **Inclusion** (§2.1.3.2) and **consistency** (§2.1.4.2) are the RFC's algorithm text, step by step, with the step numbers in the code.
- **The leaf stays a parameter.** Every check takes a *leaf hash*, so what a leaf is (the identity question, prism-019 ask 1) isn't pinned here.
- **Consistency edges:**
  - equal sizes verify with an empty path and equal roots;
  - `first > second` is refused;
  - **`first = 0` is refused.** RFC 9162 defines proofs for `0 < first < second`, and the empty tree says nothing about what a reader held before.

**A property the vectors pinned: the proof binds the roots, not the second size.**
- For 16 of the 28 pairs up to size 8, RFC 9162's algorithm accepts the proof for (m, n) also as (m, n + 1).
- transparency-dev's verifier does the same: its expected proof length is equal for both sizes.
- **It is harmless here.** The size is bound by the signed checkpoint, and no tree of n + 1 leaves has root MTH(D_n) without a SHA-256 collision across the 0x00 / 0x01 domains (MTH(D_{n+1})'s right child is a node hash where MTH(D_n)'s is a leaf hash).
- `consistency_binds_the_roots_not_the_second_size` pins the 16 pairs, so a change shows.

**Vectors:**
- `ledger-vectors.json`: every inclusion proof (0 ≤ i < n ≤ 8, 36) and consistency proof (0 < m < n ≤ 8, 28) over RFC 6962's eight test leaves. `ledger-vectors.py` generates them from RFC 9162 §2.1.1, §2.1.3.1 and §2.1.4.1 with `hashlib`, and asserts the roots for sizes 0..8 equal transparency-dev's published `RootHashes()`. Its proofs equal the published happy-path probes byte for byte.
- `ledger-probes.json`: transparency-dev/merkle@`c39fbf35` `testdata/{inclusion,consistency}`, 98 + 98 probes (Apache-2.0), unchanged. Every probe's `wantErr` is met.
  - **One exception, stated:** `sizes-are-equal-one-and-proof-is-empty` gives two equal 12-byte "roots". It passes in Go, where equal sizes compare bytes; here a root is 32 bytes by type, so it can't be expressed. It is skipped by name, and only it.

**Soundness (Mara, `research-ledger-design.md` §2.1, §2.3):** a valid inclusion proof for a different leaf, or a verifying consistency proof between forked histories (Lemma A), yields a SHA-256 collision. The domain prefixes keep leaf and node inputs disjoint (`an_inner_node_cannot_pose_as_a_leaf`).

## 2. Checkpoints and the hybrid AND rule

The note text is C2SP `tlog-checkpoint`'s: `origin\n`, `size\n` (ASCII decimal, no leading zeros), `base64(root)\n`. It is parsed strictly:
- canonical base64;
- a 32-byte root;
- an origin of 1..255 bytes with no space or `+`;
- a size that fits a u64.

**Extension lines are refused.** ML-DSA-44 cosignatures don't sign them, so the two signatures would not cover the same statement.

A checkpoint carries two log signatures, as C2SP `signed-note` lines:

| Signature | Type | Signed message | Line |
|---|---|---|---|
| Ed25519 (log signature) | 0x01 | the note text | `key ID ‖ 64-byte signature` |
| ML-DSA-44 (the log cosigning itself) | 0x06 | `CosignedSubtree` (below), `start = 0`, `end = size` | `key ID ‖ u64 timestamp ‖ 2420-byte signature` |

```
struct {
    uint8  label[12] = "subtree/v1\n\0";
    opaque cosigner_name<1..2^8-1>;
    uint64 timestamp;
    opaque log_origin<1..2^8-1>;
    uint64 start;
    uint64 end;
    uint8  hash[32];
} CosignedSubtree;     -- c2sp.org/tlog-cosignature, TLS presentation language
```

ML-DSA-44 is FIPS 204 `ML-DSA.Verify` with an empty context. C2SP names the log as a 0x06 cosigner for itself ("can be used … by the log to authenticate itself").

**The AND rule:** `verify(note, keys)` accepts only if **both** the Ed25519 line and the ML-DSA-44 line, under the given verifier keys, verify. OR would make the post-quantum half decoration (bullet-up-008, Alex). In detail:
- Lines under keys we weren't given are ignored, as signed-note requires.
- Two lines under one (name, key ID) are refused.
- A failure reports which of the two verified: `Unsigned { ed25519, mldsa44 }`.
- The ML-DSA timestamp is returned, never compared to a clock.
- **Key IDs:** vkeys are parsed with signed-note's recommended key ID, `SHA-256(name ‖ 0x0A ‖ type ‖ key)[:4]`, which is checked; a vkey whose ID doesn't match is refused.

**Vector:** C2SP signed-note's Ed25519 example (vkey `example.com/foo+530d903a+…`) parses and verifies, and fails on a changed text. No published ML-DSA-44 (0x06) cosignature vector was found. That half is tested against the C2SP text: the test suite writes its own `CosignedSubtree` encoder from the spec and checks the crate's bytes against it, and signs with seeded `ml-dsa` keys.

## 3. The SLH-DSA anchor

An offline key signs a checkpoint at genesis and periodically after that (bullet-up-008, lean 3). If lattices fall, history up to the last anchor still rests on hashes alone. It is checked separately from the AND rule.

**The format is provisional,** because C2SP assigns SLH-DSA no signature type: FIPS 205 pure SLH-DSA-SHA2-128s over the checkpoint's note text, with the context string `"prism ledger anchor v1"`, for domain separation from anything else the key signs.
- Public key: 32 bytes. Signature: 7856 bytes.
- How an anchor is carried (a note line under 0xff, or a file beside the checkpoint) isn't decided.

**Alex's seal key is a deployment gate** (prism-019, 2026-11-17). Nothing deploys signing with it until a post-quantum seal tool is chosen. This crate only verifies.

## 4. The witness quorum (Lemma B)

Given N witnesses' ML-DSA-44 verifier keys, a tolerance f, and cosignature lines over a checkpoint:

```
accept  ⟺  #{distinct configured witnesses with a valid cosignature}  ≥  Q = ⌊(N+f)/2⌋ + 1
```

- **The cosignature** is C2SP `tlog-cosignature` type 0x06 (`timestamp ‖ ML-DSA-44 over CosignedSubtree`, with the witness's name as `cosigner_name`). Its body format is C2SP's, not provisional.
- **A witness counts once,** however many lines it sent.
- **A line under no configured (name, key ID) counts zero.** That covers an unknown key, and a known name under another key.
- **A known witness's failing line counts zero.** It is reported as `invalid`.
- **A witness set is refused** if Q > N (f ≥ N, or no witnesses), or if it holds one (name, key ID) or one public key twice.
- **Liveness** (Corollary C, N ≥ 3f + 1) is reported in the `Tally`, not required.

The proof is Mara's (`research-ledger-design.md` §2.3, PROVED): two Q-sets share at least 2Q − N ≥ f + 1 witnesses, so an honest one cosigned both checkpoints, and they lie on one history. Meiklejohn et al. Lemma 4.1 is the special case; Malkhi–Reiter is the general counting.

**The quorum is exactly as post-quantum as its cosignature scheme** (Mara, BROKEN on "nothing to break"). Lemma B assumes the cosignatures can't be forged. Here they are ML-DSA-44, so the count rests on Module-LWE/SIS. With Ed25519 cosignatures a quantum adversary forges Q of them.

**Not yet the AND rule for witnesses.** prism-019's Log says the quorum "counts only cosignatures that pass the AND rule". The step that built this crate asked for ML-DSA-44 cosignatures only, which is C2SP's SHOULD. A witness that also signs Ed25519 (0x04) isn't required to. Whether witnesses must sign both is open (below).

## Assumptions, all of them

| Check | Rests on | Post-quantum |
|---|---|---|
| Merkle inclusion and consistency, Lemma A | SHA-256 collision resistance | Category 2 (NIST IR 8547 Table 1) |
| Checkpoint, AND rule | Ed25519 (discrete log) **and** ML-DSA-44 (Module-LWE/SIS) | holds while ML-DSA-44 holds: a forger must break both |
| Anchor | SLH-DSA-SHA2-128s: SHA-256 only | Category 1, hash-only |
| Quorum, Lemma B | ML-DSA-44 unforgeability, at most f dishonest witnesses, honest witnesses checking consistency (C2SP tlog-witness) | as ML-DSA-44 |
| The crates | RustCrypto's implementations match FIPS 204/205 and RFC 8032 | none of them is FIPS 140-validated |

## Tests and mutations

`ledger-verify/tests/` (one binary). Keys are throwaway keypairs from fixed seeds, made in the tests.

| Area | Tests |
|---|---|
| Merkle | 21: published roots 0..8; every vector proof; named runs at sizes 1, 2, 3, 7, 8; the wrong index, a truncated path, an extra node, the wrong root, leaf or sizes reject; the inner-node-as-leaf attack; consistency edges; the size-binding property; both probe sets |
| Note | 8: the C2SP example, key IDs, vkey refusals, canonical base64, key names, the parse rules |
| Checkpoint | 13: the body, its refusals, `CosignedSubtree`; both valid accepts (either order); Ed25519 alone, ML-DSA-44 alone, either one forged, other keys, another checkpoint all reject; duplicate lines, wrong key types and a malformed note are refused |
| Anchor | 5: the sizes, a valid anchor, another checkpoint, flipped bits and a short signature, another key |
| Quorum | 10: Q(7,2) = 5 and Mara's table; 4 reject, 5 accept; 5 with a duplicate, an unknown key, an imposter or a forgery reject; another checkpoint; cosignatures read from a note; unusable sets |

Each mutation was applied, run and reverted:

| Mutation | Tests that fail |
|---|---|
| OR instead of AND (`ed25519 \|\| mldsa44`) | `ed25519_alone_never_verifies`, `mldsa44_alone_never_verifies`, `a_forged_ed25519_beside_a_valid_mldsa44_rejects`, `a_forged_mldsa44_beside_a_valid_ed25519_rejects` |
| Counting lines, not distinct witnesses | `five_with_one_duplicate_rejects` |
| No 0x00 leaf prefix | `leaf_and_node_hashes_carry_their_prefixes`, `mth_matches_the_published_roots_for_sizes_0_to_8`, `an_inner_node_cannot_pose_as_a_leaf` |
| No 0x01 node prefix | 10 Merkle tests, among them every vector proof and both probe sets |

## Not done yet

- **Halevi–Micali commitments** (SHA-256, k = 256; Alex, 2026-11-17) are next, in this crate. Openings are checked, never made.
- **The exit-table verdict** for the quorum (clear / uncertain / unknown / paradox, prism-019 ask 2). The `Tally` carries what it needs (valid, invalid, unknown, Q, live); the mapping isn't defined yet.
- **Whether witnesses must also sign Ed25519** (the AND rule for cosignatures).
- **How an SLH-DSA anchor is carried** in or beside a note.
- **The `@nix` tool** that wraps the crate for spectral.
- **A red test running these vectors under wasm.** Today the crate *builds* for wasm32; it isn't *run* there.
- **The kernel check** (prism-019 ask 4) doesn't belong in this crate's current scope.

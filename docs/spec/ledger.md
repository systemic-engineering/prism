# Ledger: what `chorus` checks, under which assumptions

Named by Alex (2026-11-18), from the frame of *manipulation.md* that also named `echo`. A chorus is many independent voices attesting they saw the same thing; a history can't be rewritten in front of that many. Its role is to comment on the frame, which is what a double bind forbids. Its refrain comes back the same (consistency). Echo replays a reading; chorus cosigns a history.

*prism, 2026-11-17 (prism-019, the verifier crate; decisions in prism-019's Log of 2026-11-16/17 and bullet-up-008). The crate verifies; it never holds, makes or sees a private key. Test vectors: `ledger-vectors.json` (generated from RFC 9162's text by `ledger-vectors.py`) and `ledger-probes.json` (transparency-dev/merkle's published probes, bundled unchanged), and `commitment-vectors.json` (generated from Halevi–Micali's text by `commitment-vectors.py`), and `seal-vectors/` (the stock tools' raw output, 2026-11-18: OpenSSL 4.1.0's SLH-DSA anchor and OpenSSH 10.6p1's `ssh-mldsa44-ed25519` SSHSIGs, throwaway keys, §3 and §6). `chorus/tests/` holds the crate to all four.*

## The crate

`chorus/` is a workspace crate of its own:

- **`#![no_std]` + `alloc`, `#![forbid(unsafe_code)]`.** No I/O, no clock, no RNG, no LAPACK, no dependency on prismqueer.
- **Every dependency without default features**, so nothing that generates keys or reads the OS is linked into the library.
- **Portable, and checked:** the flake's `checks.chorus-portable` builds it in release for `wasm32-unknown-unknown` (a reader's browser) and for `thumbv7em-none-eabihf`, a target with no `std` at all. A dependency that pulled in `std` would fail the second build. It uses a toolchain of its own (the same rustc 1.94.0 with those two targets), so the gate's toolchain and its cache are unchanged.

| Crate | Version | Used for | Standard |
|---|---|---|---|
| `sha2` | 0.11.1 | every hash (SHA-256; SHA-512 for SSHSIG) | FIPS 180-4 |
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

**The format** (C2SP assigns SLH-DSA no signature type): FIPS 205 pure SLH-DSA-SHA2-128s over the checkpoint's note text, with the context string `"prism ledger anchor v1"`, for domain separation from anything else the key signs.
- Public key: 32 bytes, published as the PEM `PUBLIC KEY` that `openssl pkey -pubout` writes (`AnchorKey::from_pem`): DER SubjectPublicKeyInfo, the 18-byte prefix `30 30 30 0b 06 09 60 86 48 01 65 03 04 03 14 03 21 00` (OID 2.16.840.1.101.3.4.3.20, id-slh-dsa-sha2-128s, no parameters; BIT STRING, 0 unused bits), then the key. Any other OID, length or armor is refused.
- Signature: 7856 bytes, raw, as `openssl pkeyutl` writes it.
- How an anchor is carried (a note line under 0xff, or a file beside the checkpoint) isn't decided.

**The stock tool makes exactly this format** (checked 2026-11-18, OpenSSL 4.1.0 of 23 Oct 2026, Homebrew `openssl@4`; the `openssl` first on PATH on this Mac is Nix's 3.6.1, so the ceremony names the 4.x binary explicitly). The ceremony's commands, offline, on Alex's machine:

```sh
openssl genpkey -algorithm SLH-DSA-SHA2-128s -out anchor.key.pem
openssl pkey -in anchor.key.pem -pubout -out anchor.pub.pem
# checkpoint.txt: the checkpoint's note text (origin, size, base64 root,
# each line ending in \n), and nothing else.
openssl pkeyutl -sign -rawin -inkey anchor.key.pem -in checkpoint.txt \
  -pkeyopt 'context-string:prism ledger anchor v1' -out anchor.sig
# and to check it with the same tool:
openssl pkeyutl -verify -rawin -pubin -inkey anchor.pub.pem -in checkpoint.txt \
  -sigfile anchor.sig -pkeyopt 'context-string:prism ledger anchor v1'
```

- **The context option works on SLH-DSA in 4.1.0.** `-pkeyopt context-string:<text>` sets the FIPS 205 context (`OSSL_SIGNATURE_PARAM_CONTEXT_STRING`, at most 255 bytes, default empty; `EVP_SIGNATURE-SLH-DSA(7)`). A signature made with it verifies only with it, and one made without it doesn't verify with it (both checked with the CLI and in the tests). So the context stays, and the flag is part of the format: **forgetting it makes a valid FIPS 205 signature that is not an anchor,** and `chorus` rejects it (`an_openssl_anchor_without_the_context_rejects`).
- **`-rawin` is pure SLH-DSA, not a prehash.** With the default `message-encoding` of 1, OpenSSL signs `M′ = 0x00 ‖ |ctx| ‖ ctx ‖ M` over the file's bytes as given (`EVP_SIGNATURE-SLH-DSA(7)`: "OpenSSL does not support Pre Hash SLH-DSA"). The tests confirm it: a pure FIPS 205 verifier (RustCrypto `slh-dsa`, `try_verify_with_context`) accepts the CLI's signature. `-digest` is not used.
- The signature is randomized (OpenSSL's default `deterministic` is 0), so two signatures over one checkpoint differ; both verify.

**Vectors** (`seal-vectors/`): `anchor-body.txt` (the tests' checkpoint, RFC 6962's eight leaves at size 8), `anchor.pub.pem`, `anchor.sig` (with the context), `anchor-empty-context.sig` (the same command without `-pkeyopt`). The private key was deleted after signing.

**Alex's seal key is a deployment gate** (prism-019, 2026-11-17). The tooling is no longer the blocker (spectral-003's Response, 2026-11-17): this anchor and the SSHSIG seal (§6) are made with stock tools, and `chorus` verifies what they produce. Nothing signs with Alex's keys until Alex has generated and cross-signed them. This crate only verifies.

## 4. The witness quorum (Lemma B), under the AND rule

**Decision** (Alex, bullet-up-008 log, 2026-11-16): hybrid signatures use AND-composition everywhere; the weaker scheme never decides alone. For witnesses that means: each witness is declared with **two** verifier keys, Ed25519 (C2SP type 0x04) and ML-DSA-44 (0x06), under one cosigner name, and counts toward Q **only if both** its cosignatures over the checkpoint verify.

Given N such witnesses, a tolerance f, and cosignature lines over a checkpoint:

```
accept  ⟺  #{distinct configured witnesses whose 0x04 AND 0x06 cosignatures both verify}  ≥  Q = ⌊(N+f)/2⌋ + 1
```

The two cosignatures, both C2SP `tlog-cosignature` (read at C2SP/C2SP@`b2dd989`, 2026-10-07), neither provisional:

| Type | Line | Signed message |
|---|---|---|
| 0x04, Ed25519 | `key ID ‖ u64 timestamp ‖ 64-byte signature` | RFC 8032 Ed25519 over the lines `cosignature/v1`, `time <timestamp as ASCII decimal>`, origin, size (ASCII decimal), base64(root), each ending in `\n`; no extension lines (checkpoints with extensions are refused) |
| 0x06, ML-DSA-44 | `key ID ‖ u64 timestamp ‖ 2420-byte signature` | FIPS 204 ML-DSA-44, empty context, over `CosignedSubtree` (§2), `cosigner_name` = the witness's name, `start = 0`, `end = size` |

- **A witness is declared with both keys** (`Witness::from_vkeys`, either order): one 0x04 and one 0x06 vkey under one name. Two keys of one type, an Ed25519 *log* key (0x01), a malformed key, or two names are refused (`WitnessKeys`). A witness with one key can't be declared, so it counts zero.
- **A witness with only one valid cosignature counts zero.** Either one missing, or either one failing. It is reported as `partial` in the `Tally`.
- **A witness counts once,** however many lines it sent.
- **A line under no configured (name, key ID) counts zero** (`unknown`). That covers an unknown key, and a known name under another key.
- **A known key's failing line counts zero** (`invalid`).
- **A witness set is refused** if Q > N (f ≥ N, or no witnesses), or if it holds one (name, key ID) or one public key twice, in either scheme.
- **The two timestamps aren't compared** with each other or with a clock. Each cosignature is a full C2SP checkpoint cosignature on its own.
- **Liveness** (Corollary C, N ≥ 3f + 1) is reported in the `Tally`, not required.

**The 0x04 body is the C2SP text, checked twice:** the crate's `Checkpoint::cosignature_v1` equals the spec's worked example byte for byte (`example.com/behind-the-sofa`, `20852163`, `time 1679315147`), and equals the tests' own encoder, written from the spec's line list. No published 0x04 signature vector was found; signatures come from seeded `ed25519-dalek` keys.

The proof is Mara's (`research-ledger-design.md` §2.3, PROVED): two Q-sets share at least 2Q − N ≥ f + 1 witnesses, so an honest one cosigned both checkpoints, and they lie on one history. Meiklejohn et al. Lemma 4.1 is the special case; Malkhi–Reiter is the general counting.

**The quorum is as strong as the stronger of its two schemes, until both fall.** Lemma B assumes the cosignatures can't be forged (Mara, BROKEN on "nothing to break"). Counting a witness takes a valid Ed25519 *and* a valid ML-DSA-44 cosignature, so forging Q witnesses takes forging both schemes: a quantum adversary who breaks Ed25519 still faces ML-DSA-44 (Module-LWE/SIS), and a break of ML-DSA-44 alone still faces Ed25519 (discrete log). Under OR it would be the weaker of the two.

## 5. Commitments: Halevi–Micali over SHA-256, k = 256

The log holds commitments, never content (prism-019 ask 3). Decided by Alex, 2026-11-17, after Mara's grades (bullet-up-008). The source is S. Halevi and S. Micali, "Practical and Provably-Secure Commitment Schemes from Collision-Free Hashing", CRYPTO '96; page numbers are those of `shaih.github.io/pubs/hm96.pdf`, read as text for this section.

**The construction, as the paper gives it** (MD = SHA-256, k = 256):

| Step | Paper | Here |
|---|---|---|
| Commit to the digest, not the message | §4, p.15: "the Sender first computes the k-bit string s = MD(m), and then apply the above commitment string to the string s" | s = SHA-256(m); so n = k = 256, whatever \|m\| is |
| Lengths | §3, p.11: L = 4k + 2n + 4; MD : {0,1}^L → {0,1}^k | L = 1540 = 6k + 4 |
| Commit | §3, p.11: random r ∈ {0,1}^L, y = MD(r), a random h ∈ H with h(r) = s; c = ⟨h, y⟩, d = r | the same |
| The universal family | §2.1, p.9: h_{A,b}(r) = A r + b over GF(2), A an n × L **Toeplitz** matrix, described by 2n + L − 1 bits | A[i][j] = t[i − j + L − 1], t of 1795 bits; b of 256 bits |
| Choosing h | footnote 4, p.11: "picking A at random and computing b = m − Ax" | b = s ⊕ A r |
| Receive | §3, p.11: "verifies that y = MD(r) and computes m = h(r)"; §4, p.15: "checks that s is the string being committed to … and that MD(m) = s" | `verify_opening`: y = SHA-256(r) and A r ⊕ b = SHA-256(m) |

**prism's encodings** (the paper works on bit strings; these are ours, and **provisional**):
- A bit string is packed most significant bit first; trailing pad bits are zero, and a set pad bit is refused (`Padding`), so each commitment and opening has one encoding.
- MD on {0,1}^L is SHA-256 of r's 193-byte packing. The packing is injective, so a collision of this MD is a SHA-256 collision. The hiding proof (pp.12–15) holds for *every* function MD : {0,1}^L → {0,1}^k, so it holds for this one.
- The diagonal indexing A[i][j] = t[i − j + L − 1] is a choice; every indexing gives the same family.
- **Commitment** (290 bytes) = `0x01` ‖ t (225 bytes, 5 pad bits) ‖ b (32) ‖ y (32).
- **Opening** = r (193 bytes, 4 pad bits); the message m is passed beside it.
- **The scheme byte `0x01`** names "Halevi–Micali, SHA-256, k = 256, this layout", so a leaf says which scheme it is checked under. No registry assigns it.

**Sizes, exactly:**
- |h| + |y| = (2n + L − 1) + k = **9k + 3 = 2307 bits**. The paper's "9k" (p.15) and its "(L + 2k) + k = 7k + 2n" (p.12) drop the constants. Byte-aligned that is 289 bytes, 290 with the scheme byte, **not the 288** of the decision. Nothing in the construction can be dropped to reach 288 without leaving the paper's family.
- |r| = 6k + 4 = 1540 bits, 193 bytes. The opening is m plus those 193 bytes.
- Cost: two SHA-256 calls and one GF(2) product, about 770 XORs of 32-byte windows.

**Assumptions:**
- **Hiding: none.** Theorem 1 (p.12; proof pp.12–15): for all m₁, m₂, ‖C_k(m₁) − C_k(m₂)‖ < 2^−k = 2^−256, for every MD and every receiver. Information-theoretic: no computer, classical or quantum, changes it. Statistical, not perfect (the authors, p.4: "the Receiver may obtain some advantage, but this advantage is provably exponentially small").
- **Binding: SHA-256 collision resistance** (p.12: "being able to open the commitment in two different ways implies that the Sender can find a collision in MD"). Two openings (m, r), (m′, r′) of one commitment with m ≠ m′: if r ≠ r′, SHA-256(r) = y = SHA-256(r′); if r = r′, SHA-256(m) = A r + b = SHA-256(m′). §4's version of this proof is "omitted from this extended abstract" (p.15); it is this one line. Post-quantum Category 2 (NIST IR 8547 Table 1), the same as the log.
- One k drives both (Remark 1, p.11, allows two). Binding is classical; Unruh's collapse-binding is not claimed, and for a log, where cheating means publishing two openings, it isn't needed (Mara, bullet-up-008).

**Erasure deletes r and m.** What stays is t, b and y. By Theorem 1 their joint distribution is within 2^−256 of the same for every m, so the commitment alone says that something was committed, and nothing about what. Keeping r and deleting only m is **not** erasure: SHA-256(m) = A r ⊕ b can then be recomputed and tested against guesses.

**Not a property: non-malleability.** A diagonal bit of A that r never reads (only at A's corners: t[o] with o < 255 or o > 1539, read by ≤ 255 columns, each with probability 2^−(columns read) of being unread) can be flipped, and the new commit-string still opens, **to the same m with the same r**. Anyone holding r can also make other commit-strings that open to m. This is not ambiguity: binding is about two messages, and that holds. In the log the leaf holds the exact bytes, so a changed commitment is another leaf. The tests pin it (`a_flipped_bit_rejects_unless_r_never_reads_it`, and 8 such bits on the "empty" vector).

**The EDPB reading** (Guidelines 02/2025 on blockchain, v2.0; **not legal advice**):
- ¶52 accepts *computational* protection: a salted or keyed hash "should not be linkable … provided that the algorithm has not been broken", the keys and salt kept.
- ¶53: "*If* the commitment has been computed using a perfectly hiding state-of-the art scheme, then once the original data and its witness are deleted, the commitment persisting in the blockchain is useless." That is a sufficient condition, not a requirement.
- ¶96 asks that, for long retention, "the potential impact of technical advances such as cryptanalytically-relevant quantum computers" be weighed.
- **Reading:** statistical hiding at 2^−256 meets what ¶52/¶53 ask: after erasure the commitment can be neither recovered nor recognised, except with probability below 2^−256, which is not operationally distinguishable from perfect. And because the hiding rests on no assumption, it removes the ¶96 risk class (a future break, quantum or not), which a salted hash keeps. ¶104 still holds for any scheme: where a commitment isn't needed, a pointer is better.

**The committer lives in the tests only** (`chorus/tests/commitment.rs`), with coins from a seeded SHA-256 stream. The library checks; it has no RNG and never commits, so the portable build (`checks.chorus-portable`) is unchanged in kind. A `std`-gated committing feature was the alternative; it would put a second, unchecked-by-the-portable-build code path into the library for a job that is spectral's (the side that holds m and r).

**Vectors:** `commitment-vectors.json`, 4 commitments (m = "", "abc", 64 bytes, 1012 bytes) with fixed coins, written by `commitment-vectors.py` from the paper's text with `hashlib` only (it computes A r row by row from the definition and checks its own Receive). The Rust verifier accepts them; a changed m, every flipped bit (bar the unread corners, which open to the same m), and a truncated commitment or opening reject.

## 6. Alex's seal: SSHSIG with `ssh-mldsa44-ed25519`

Alex's seal and the adjudicator's statements are SSHSIGs made by `ssh-keygen -Y sign` with an `ssh-mldsa44-ed25519` key (OpenSSH 10.6; spectral-003's Response). `sshsig::verify(armored, namespace, message, allowed)` verifies exactly what that tool produces, and nothing weaker.

**The ceremony's commands** (Alex's machine, Alex's user; the key's storage and cross-signatures are spectral-003's step 2–3):

```sh
ssh-keygen -t mldsa44-ed25519 -f ~/.ssh/seal_mldsa44_ed25519   # with a passphrase
ssh-keygen -Y sign -n spectral-seal -f ~/.ssh/seal_mldsa44_ed25519 statement.txt
#   -> statement.txt.sig (armored); default hash sha512
ssh-keygen -Y verify -f allowed_signers -I <identity> -n spectral-seal \
  -s statement.txt.sig < statement.txt
```

**The format, with its sources:**

- **SSHSIG** (OpenSSH `PROTOCOL.sshsig` and `sshsig.c`, V_10_6_P1): the armor is `-----BEGIN SSH SIGNATURE-----`, base64, `-----END SSH SIGNATURE-----`. The blob is `byte[6] "SSHSIG"`, `uint32 1`, `string publickey`, `string namespace`, `string reserved`, `string hash_algorithm`, `string signature`. The signed data is `byte[6] "SSHSIG" ‖ string namespace ‖ string reserved ‖ string hash_algorithm ‖ string H(message)`, and `sshsig.c` builds it with the reserved string **empty** on both sides, so a non-empty reserved field in the blob is ignored, as `PROTOCOL.sshsig` says (`a_reserved_value_is_ignored_as_openssh_does`).
- **The composite** (OpenSSH 10.6p1 `ssh-mldsa-eddsa.c`, which cites draft-miller-sshm-mldsa44-ed25519-composite-sigs-00 and implements draft-ietf-lamps-pq-composite-sigs-18):
  - public key: `string "ssh-mldsa44-ed25519"`, `string (mldsaPK[1312] ‖ ed25519PK[32])`;
  - signature: `string "ssh-mldsa44-ed25519"`, `string (mldsaSig[2420] ‖ ed25519Sig[64])`;
  - both halves sign `M′ = "CompositeAlgorithmSignatures2025" ‖ "COMPSIG-MLDSA44-Ed25519-SHA512" ‖ 0x00 ‖ SHA-512(signed data)`. The 0x00 is the length of the composite's own context, which OpenSSH always passes empty;
  - ML-DSA-44 is FIPS 204 pure, with the label `"COMPSIG-MLDSA44-Ed25519-SHA512"` as its context (hedged: OpenSSH draws fresh randomness per signature); Ed25519 is RFC 8032 over the same `M′`.
  - OpenSSH verifies ML-DSA, then Ed25519, and fails if either fails: AND.
  - A test (`the_composite_encoding_is_what_ssh_keygen_wrote`) pins this against the tool's own output: it builds `M′` from the source's text and verifies each half of the real vector directly with `ml-dsa` and `ed25519-dalek`. ML-DSA-44 doesn't verify with an empty context.

**What `verify` checks, in order:** the armor; the magic and version 1; no trailing bytes in the blob, the key or the signature field; the key is `ssh-mldsa44-ed25519` (anything else, a classic `ssh-ed25519` included, is `KeyType`); the key is byte-equal to the allowed key (`Key`); the namespace equals the expected one and isn't empty (`Namespace`); the hash algorithm is `sha512` or `sha256` (`HashAlgorithm`); the signature is `ssh-mldsa44-ed25519` with 2484 bytes; then **both** halves. `Signature { ed25519, mldsa44 }` says which verified. Ed25519 is checked with `verify_strict`, as in §2: an honest `ssh-keygen` signature always passes it, but it is stricter than OpenSSH's ref10, which accepts some non-canonical forms. `SealKey::from_openssh` reads a `.pub` line or an `allowed_signers` key field (`<type> <base64> [comment]`); the text type must match the blob's.

**Vectors** (`seal-vectors/`, OpenSSH 10.6p1, throwaway keys, each `.sig` checked with `ssh-keygen -Y verify` before check-in, private keys deleted):

| File | Made by |
|---|---|
| `seal.pub`, `other.pub` | `ssh-keygen -t mldsa44-ed25519 -N ''` |
| `classic.pub` | `ssh-keygen -t ed25519 -N ''` |
| `seal-msg.txt` | the message |
| `seal-msg.spectral-seal.sig` | `ssh-keygen -Y sign -n spectral-seal -f seal` |
| `seal-msg.sha256.sig` | the same, with `-O hashalg=sha256` |
| `seal-msg.other-namespace.sig` | `ssh-keygen -Y sign -n other-namespace -f seal` |
| `seal-msg.classic-ed25519.sig` | `ssh-keygen -Y sign -n spectral-seal -f classic` |

## Assumptions, all of them

| Check | Rests on | Post-quantum |
|---|---|---|
| Merkle inclusion and consistency, Lemma A | SHA-256 collision resistance | Category 2 (NIST IR 8547 Table 1) |
| Checkpoint, AND rule | Ed25519 (discrete log) **and** ML-DSA-44 (Module-LWE/SIS) | holds while ML-DSA-44 holds: a forger must break both |
| Anchor | SLH-DSA-SHA2-128s: SHA-256 only | Category 1, hash-only |
| Seal (SSHSIG `ssh-mldsa44-ed25519`), AND rule | ML-DSA-44 **and** Ed25519 over M′, which commits to the message through SHA-512 (sha512) or SHA-256 then SHA-512 (sha256) | holds while ML-DSA-44 holds: a forger must break both |
| Quorum, Lemma B, AND rule | Ed25519 **and** ML-DSA-44 cosignatures, both per witness (a forger must break both); at most f dishonest witnesses; honest witnesses checking consistency (C2SP tlog-witness) | as the stronger of the two: holds while either holds, falls only when both fall |
| Commitment hiding (Theorem 1) | nothing: < 2^−256, information-theoretic | unconditional |
| Commitment binding | SHA-256 collision resistance | Category 2 |
| The crates | RustCrypto's implementations match FIPS 204/205 and RFC 8032 | none of them is FIPS 140-validated |

## Tests and mutations

`chorus/tests/` (one binary). Keys are throwaway keypairs from fixed seeds, made in the tests, except the stock tools' vectors in `seal-vectors/`, made by OpenSSL and OpenSSH with keys deleted after.

| Area | Tests |
|---|---|
| Merkle | 21: published roots 0..8; every vector proof; named runs at sizes 1, 2, 3, 7, 8; the wrong index, a truncated path, an extra node, the wrong root, leaf or sizes reject; the inner-node-as-leaf attack; consistency edges; the size-binding property; both probe sets |
| Note | 8: the C2SP example, key IDs, vkey refusals, canonical base64, key names, the parse rules |
| Checkpoint | 13: the body, its refusals, `CosignedSubtree`; both valid accepts (either order); Ed25519 alone, ML-DSA-44 alone, either one forged, other keys, another checkpoint all reject; duplicate lines, wrong key types and a malformed note are refused |
| Anchor | 12: the sizes, a valid anchor, another checkpoint, flipped bits and a short signature, another key; the OpenSSL vector is over the test checkpoint, verifies, rejects without the context, with a flipped bit, over another checkpoint or under another key; our SPKI encoding equals openssl's byte for byte; malformed PEM keys (another OID, short, long, unused bits, PRIVATE, no footer, bad base64, empty) reject |
| SSHSIG | 14: the composite encoding against the real vector, half by half; the ssh-keygen seal (sha512) and the sha256 seal verify; another namespace (either way, empty, relabelled), another message, another allowed key (or the blob's key swapped) reject; only the Ed25519 half corrupted, only the ML-DSA-44 half corrupted, or another signature's Ed25519 half swapped in reject; a classic `ssh-ed25519` key and SSHSIG are `KeyType`; the `.pub` parse and its refusals; version 0/2, sha384/SHA512/empty hash, a wrong signature type, short or trailing bytes, a bad preamble, truncations reject; a non-empty reserved is ignored; the armor is required |
| Quorum | 19: Q(7,2) = 5 and Mara's table; the 0x04 body against the C2SP example and an independent encoder; a 0x04 cosignature verifies, and rejects retimed, flipped, short or as a plain note signature; 0x01 and 0x04 keys kept apart; a witness with only its ML-DSA-44 or only its Ed25519 cosignature counts 0; 4 reject, 5 with both accept; 5 where one has only one scheme, or one forged Ed25519 or ML-DSA-44, reject; 5 with a duplicate, an unknown witness or an imposter reject; another checkpoint; both cosignatures read from a note; witness declarations; unusable sets |
| Commitment | 14: the sizes; the tests' committer against the row-by-row definition; the Python vectors open, and reject mutated; opens with its (m, r); the wrong m, the wrong r, truncated or extended inputs, another scheme byte reject; every flipped bit rejects with its reason, bar the unread corners; the check equals the paper's equation; the binding extractor; the hiding smoke check (per-bit frequencies over 200 + 200 commitments within 0.3; not a proof) |

Each mutation was applied, run and reverted:

| Mutation | Tests that fail |
|---|---|
| OR instead of AND (`ed25519 \|\| mldsa44`) | `ed25519_alone_never_verifies`, `mldsa44_alone_never_verifies`, `a_forged_ed25519_beside_a_valid_mldsa44_rejects`, `a_forged_mldsa44_beside_a_valid_ed25519_rejects` |
| Counting lines, not distinct witnesses | `five_with_one_duplicate_rejects` |
| SSHSIG: OR instead of AND (`ed25519 \|\| mldsa44`) | `corrupting_the_ed25519_half_only_rejects`, `corrupting_the_mldsa44_half_only_rejects`, `swapping_in_the_ed25519_half_of_another_signature_rejects` |
| Quorum: OR instead of AND per witness (`ed25519 \|\| mldsa44`) | `a_witness_with_only_its_ed25519_cosignature_counts_zero`, `a_witness_with_only_its_mldsa44_cosignature_counts_zero`, `five_where_one_has_only_its_ed25519_rejects`, `five_where_one_has_only_its_mldsa44_rejects`, `five_with_one_forged_ed25519_rejects`, `five_with_one_forged_mldsa44_rejects` |
| No 0x00 leaf prefix | `leaf_and_node_hashes_carry_their_prefixes`, `mth_matches_the_published_roots_for_sizes_0_to_8`, `an_inner_node_cannot_pose_as_a_leaf` |
| No 0x01 node prefix | 10 Merkle tests, among them every vector proof and both probe sets |
| Commitment: drop the universal hash (compare b to SHA-256(m)) | 6, among them `the_python_vectors_open`, `the_check_is_the_papers_equation` |
| Commitment: commit to m, not SHA-256(m) | the same 6 |
| Commitment: accept a longer commitment or opening | `a_truncated_or_extended_commitment_or_opening_rejects` |
| Commitment: no pad-bit check | `a_flipped_bit_rejects_unless_r_never_reads_it`, `mutated_python_vectors_reject` |

## Not done yet

- **What a leaf holds around a commitment** (the scheme byte is there; the leaf format waits on the identity work), and whether 290 bytes, not 288, is fine (Alex).
- **The exit-table verdict** for the quorum (clear / uncertain / unknown / paradox, prism-019 ask 2). The `Tally` carries what it needs (valid, partial, invalid, unknown, Q, live); the mapping isn't defined yet.
- **How an SLH-DSA anchor is carried** in or beside a note.
- **The `@nix` tool** that wraps the crate for spectral.
- **A red test running these vectors under wasm.** Today the crate *builds* for wasm32; it isn't *run* there.
- **The kernel check** (prism-019 ask 4) doesn't belong in this crate's current scope.

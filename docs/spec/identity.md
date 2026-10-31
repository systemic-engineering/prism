# Identity: scoped content addresses

*prism, 2026-10-30. Decided by Alex. The identity function for the whole stack: prism, mirror, bullet-up and mirror's Python checker. Test vectors: `identity-vectors.json`, computed with the `b3sum` CLI (1.8.7) and cross-checked with Python's `blake3` package, and checked against prism by `prismqueer/tests/identity_vectors.rs`.*

## The function

There is one identity function. Every address is taken inside a **scope**, and a scope is a 32-byte key:

```
address(scope, content) = BLAKE3-keyed(K_scope, content)      32 bytes, written as 64 lowercase hex
```

`BLAKE3-keyed` is BLAKE3's keyed-hash mode (`blake3::keyed_hash` in Rust, `blake3(content, key=K)` in Python, `b3sum --keyed`). `content` is the raw bytes, with no prefix, length or framing added.

## The public scope

Public is not a separate kind of identity. It is the scope whose key anyone can derive:

```
K_public = BLAKE3-derive-key(context = "systemic.engineering://visibility/public/v1", key material = empty)
         = 18d4e1ec48377a474bd2709f1a5705ae2c95c11e00b1b490cf1af92a86258ba0
```

Because the context string is published, anyone can recompute a public address. That is correct for public material.

- The context string names the scope for the whole stack (`systemic.engineering`), not for one crate.
- `v1` versions the identity function itself. A change to the function is a new context string, so old and new addresses can't collide by accident.

## Held scopes

Private and protected material is addressed under a scope whose key is a held 32-byte secret. Without the key, an address says nothing about the content and can't be recomputed.

## Why identity is scoped

A plain content address, `BLAKE3(content)`, has two faults:

1. **It leaks low-entropy content.** Anyone can hash a guess ("X said yes") and check it against a published address. Under a held key, they can't.
2. **It defeats erasure.** Deleting the key of a scope makes every address in that scope unrecomputable. A plain hash survives every deletion.

Under a scoped address, the same content in two scopes has two unrelated addresses (see the `scopes_separate` vectors). Identity is "this content, in this scope".

## What identity is relative to

Signed by Alex, 2026-10-31: *"It's relative to the observer itself. It is how the observer is known to the world at large."*

An address is not a property of the content. It is the trace of an observer making itself known. Two rules follow, and they are one rule:

- **A lossless shift doesn't change identity.** A re-sorted chart, a permutation of the basis, or a renamed label (a named-projection \*-isomorphism) leaves the observer and the content unchanged.
- **The consent boundary changes identity,** because it is the observer's boundary: how far it lets itself be known. That is what `K_scope` is. A held key is private, a key shared with a circle is protected, and a published key is known to the world at large (`visibility/public`).

The observer appears in identity **only through its boundary, never through its name.** If the observer itself were hashed, the same public sentence from two speakers would get two addresses, so dedup would break. Worse, anyone could link an utterance to its speaker by recomputing the hash. So the observer's three aspects live in three places:

| aspect of the observer | where it lives |
|---|---|
| how far it is known (its boundary) | **in** the address: the scope key |
| who it is | **beside** the address: the signature at the Drop |
| where it stood | **beside** the address: the spectral uuid's active half |

**Identity is content, modulo the observer's lossless re-readings, keyed by how far the observer lets itself be known.** Every other frame (type, position, chart, time within an epoch) is a reading kept beside the address, never fused into it.

## Out of scope

**Key management.** Where held keys come from (the key hierarchy, sharing, rotation by key epochs, revocation, and whether a scope is deterministic or per-crystal) is designed outside prism (Mara's design, in mirror). prism takes a key as 32 bytes and never stores, derives, prints or rotates it. In prism, `Scope`'s `Debug` doesn't print the key.

**Keys from passwords.** BLAKE3 must never derive a key from a password or any other low-entropy secret. BLAKE3 is fast by design, so a key derived that way can be brute-forced from its addresses. A held key is 32 bytes of real entropy: random, or derived from such a key. If a key ever has to come from a password, that is a password hash's job (Argon2id, for example), outside prism.

## The dark address

prism's `Oid::dark()` (64 zeros) is the address of absence. It is not the address of any content, in any scope, and it is not computed by this function.

## Vectors

`identity-vectors.json` holds:

- `k_public`: the derived public key, with its context string;
- `cases`: public addresses of the empty input, `"hello"`, `"prism"` and a multi-byte UTF-8 string, plus keyed examples under the test keys `0x01 × 32` and `0x02 × 32`;
- `scopes_separate`: the same content under two scopes, with two different addresses.

Each case gives its input as hex (the bytes hashed) and, where it is text, as UTF-8 too. A key is either `"public"` or 32 bytes of hex.

To recompute a case with the CLI:

```
printf '' | b3sum --derive-key "systemic.engineering://visibility/public/v1" --no-names   # K_public
printf 'hello' > in; printf '%s' "$K_PUBLIC" | xxd -r -p | b3sum --keyed --no-names in
```

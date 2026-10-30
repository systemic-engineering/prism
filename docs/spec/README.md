# Specs for ports

*prism-001 (bullet-up handoff), 2026-10-27. Language-neutral specs of prism semantics, with test vectors, so other projects can port the ideas without porting the Rust.*

| File | What |
|---|---|
| `imperfect.md` | `Imperfect`: three cases, loss laws, every operation's rules, what must never happen |
| `imperfect-vectors.json` | 192 cases generated from terni 0.7.0 by `prismqueer/tests/spec_vectors.rs` |
| `gate.md` | The convergence gate for review rounds, taken from settle semantics; proven parts vs. modelling choices |
| `gate-vectors.json` | 9 worked examples, checked against the rule by `prismqueer/tests/gate_vectors.rs` |
| `identity.md` | Identity is scoped: `address = BLAKE3-keyed(K_scope, content)`, the public scope's derived key, why, and what's out of scope (key management) |
| `identity-vectors.json` | `K_public` and 11 addresses computed with `b3sum` (cross-checked with Python's `blake3`), checked against prism by `prismqueer/tests/identity_vectors.rs` |

Regenerate the Imperfect vectors after an intended change in terni:

```
PRISM_SPEC_UPDATE=1 cargo test -p prismqueer --test spec_vectors
```

## Licence

- Apache-2.0, for all repositories (Alex, 2026-10-27). prism's `LICENSE` and imperfect's `LICENSE` carry the full text; terni and prismqueer also declare it in `Cargo.toml`.
- These specs and vectors are Apache-2.0 and can be copied into bullet-up (also Apache-2.0) with attribution.
- imperfect is public, so the submodule is fetched over HTTPS (`.gitmodules`); no deploy key is needed in CI.

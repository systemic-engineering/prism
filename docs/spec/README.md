# Specs for ports

*prism-001 (bullet-up handoff), 2026-10-27. Language-neutral specs of prism semantics, with test vectors, so other projects can port the ideas without porting the Rust.*

| File | What |
|---|---|
| `imperfect.md` | `Imperfect`: three cases, loss laws, every operation's rules, what must never happen |
| `imperfect-vectors.json` | 192 cases generated from terni 0.7.0 by `prismqueer/tests/spec_vectors.rs` |
| `brain.md` | `brain`'s capacity is verdict-neutral: the proof behind the threshold-independence tests, its assumptions (exact recompute, a faithful backend, a blind tenant), and the tests that guard each one |
| `gate.md` | The convergence gate for review rounds, taken from settle semantics; proven parts vs. modelling choices |
| `gate-vectors.json` | 9 worked examples, checked against the rule by `prismqueer/tests/gate_vectors.rs` |
| `identity.md` | Identity is scoped: `address = BLAKE3-keyed(K_scope, content)`, the public scope's derived key, why, and what's out of scope (key management) |
| `identity-vectors.json` | `K_public` and 11 addresses computed with `b3sum` (cross-checked with Python's `blake3`), checked against prism by `prismqueer/tests/identity_vectors.rs` |
| `ruler.md` | The canonical ruler address (`"ruler\0"` ‖ region ‖ question ‖ α ‖ β ‖ clock, public scope; signer and p₀/p₁ never in it) and the calibration crystal that carries p₀/p₁. prism-Reed's lean, Alex's call |
| `metrix.md` | A frame's ruler for length (prism-021 D5/D6, Mara 54c88f72 §8–§9): `Metrix` positive definite, certified by exact LDLᵀ, signed, no default, not over ℤ/n; `Form` of any signature with inertia by symmetric pivoting; isometry defects; relative spectra as exact algebraic multisets, and their caveat |
| `hypothesis.md` | Frames, their composite and the hypothesis over it (prism-021, Mara 54c88f72): `Gl<R>` over exact ℚ/ℤ, the composite's interpretation (agreement, located holonomy, curvature, provenance), the unranked closures per loop, D1/D2 |
| `ledger.md` | What `ledger-verify` checks and under which assumptions (prism-019): RFC 9162 Merkle proofs over SHA-256, checkpoints under Ed25519 **AND** ML-DSA-44, the SLH-DSA-SHA2-128s anchor, Lemma B's witness quorum; crate versions, the mutations, what's not done (Halevi–Micali commitments next) |
| `ledger-vectors.json` | 36 inclusion and 28 consistency proofs (sizes ≤ 8, RFC 6962's test leaves) generated from RFC 9162's text by `ledger-vectors.py`, roots checked against transparency-dev's; checked by `ledger-verify/tests/merkle.rs` |
| `ledger-probes.json` | transparency-dev/merkle's published inclusion and consistency probes (98 + 98, Apache-2.0), bundled unchanged by `ledger-vectors.py --probes` |
| `ruler-vectors.json` | 4 ruler and 3 calibration preimages built by `ruler-vectors.py` from the spec, hashed with Python's `blake3` and `b3sum`, checked against prism by `prismqueer/tests/ruler_vectors.rs` |

Regenerate the Imperfect vectors after an intended change in terni:

```
PRISM_SPEC_UPDATE=1 cargo test -p prismqueer --test spec_vectors
```

## Licence

- Apache-2.0, for all repositories (Alex, 2026-10-27). prism's `LICENSE` and imperfect's `LICENSE` carry the full text; terni and prismqueer also declare it in `Cargo.toml`.
- These specs and vectors are Apache-2.0 and can be copied into bullet-up (also Apache-2.0) with attribution.
- imperfect is public, so the submodule is fetched over HTTPS (`.gitmodules`); no deploy key is needed in CI.

# Specs for ports

*prism-001 (bullet-up handoff), 2026-10-27. Language-neutral specs of prism semantics, with test vectors, so other projects can port the ideas without porting the Rust.*

| File | What |
|---|---|
| `imperfect.md` | `Imperfect`: three cases, loss laws, every operation's rules, what must never happen |
| `imperfect-vectors.json` | 192 cases generated from terni 0.7.0 by `prismqueer/tests/spec_vectors.rs` |
| `gate.md` | The convergence gate for review rounds, taken from settle semantics; proven parts vs. modelling choices |
| `gate-vectors.json` | 8 worked examples, checked against the rule by `prismqueer/tests/gate_vectors.rs` |

Regenerate the Imperfect vectors after an intended change in terni:

```
PRISM_SPEC_UPDATE=1 cargo test -p prismqueer --test spec_vectors
```

## Licence

- terni (`imperfect/Cargo.toml`) and prismqueer (`prismqueer/Cargo.toml`) declare `license = "Apache-2.0"`.
- **Neither the prism repository nor the imperfect repository has a LICENSE file today** (checked 2026-10-27). The declaration lives only in the Cargo metadata.
- These specs and vectors are derived from that Apache-2.0 code and are meant to be under Apache-2.0 too, so they can be copied into bullet-up (also Apache-2.0) with attribution.
- Before copying: adding the Apache-2.0 LICENSE text (and, if wanted, a NOTICE) to both repositories would make that explicit instead of implied. That's a decision for Alex.

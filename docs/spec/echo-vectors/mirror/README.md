# mirror's data, for echo

Copied byte for byte from mirror. Data only: no line of mirror's engine
source was read to make or check them (prism-022 Response §3;
docs/spec/echo.md §2).

| Here | From mirror | What |
|---|---|---|
| `*.vector` (215) | `tests/checker/vectors/*.vector`, tree `6449be5b`: the 46 `kind atom*` files from `reed/ship` `52a435947fbaaee4fecce82180cc8d9638409be8`, the other 169 from `reed/ship` `3cedf18a797dd4f32dcbf676ca76077a24ec3a3b` (2026-11-18; the tree is the same at both) | Taut's §5.0 known-answer vectors, written from the spec text, every digest checked by two BLAKE3 implementations (his own pure-Python one and b3sum 1.8.7) |
| `mirror.facts` | `mirror.facts` (blob `4c0c565b`), `reed/ship` `52a43594` | the pinned F_G (`# grammar 9092fca1…`, the charter's format-facts row): 3 headers, 539 typed `fact` lines (739 `addr`, 229 `nat`, 33 `bytes` arguments) |
| `fixtures/` (38 directories) | `tests/checker/fixtures/`, tree `27d74d61`, `reed/ship` `3cedf18a` | the tune's checker fixtures: the `ct-*` cheats, `pos-*`, the Dark-2, budget and order cases, `typed-corpus-unheard`. Each `expect` names the tune's answer and redirects inputs by path; `../../vectors/<x>` is `<x>` here |
| `20`–`24-certificate.cert`, `20`–`24-source.pact-text`, `48-source.pact-text` | `tests/checker/vectors/`, `reed/ship` `3cedf18a` | Taut's certificates and sources that the `taut-*` and `a1-record-dark-engine` fixtures borrow |
| `27-stdout-positive.stdout`, `50-heard-score-none.tune` | the same | vector 50: the tune's `heard` line over corpus 01's source and 27's stdout split at its `certificate` line (M1) |
| `01-determinism.pact-text` | `tests/pact/01-determinism.pact-text`, `reed/ship` `3cedf18a` | corpus 01's source, vector 50's `source` input |

How echo uses them:
- `echo/tests/vectors.rs`: the 46 atom vectors read as `fact` lines, print back, and the 7 that carry `content` encode to it; `mirror.facts` reads whole.
- `echo/tests/address.rs` (P2): all 215 digests from their preimages; the 63 `atom` lines, the 32 `fact … <hex>` lines and 50 composites (fragments, lenses, the ε frame, models, result and facts sets) from their text, by echo's own encoder.
- `echo/tests/integrity.rs` (P4): `mirror.facts` passes V0, and `# grammar` recomputes from its facts.
- `echo/tests/replay.rs` and `echo/tests/bin.rs` (certificate replay, R1): every fixture reads and none rings; vector 50's `heard` line byte for byte; the model's address recomputes from the layers of Taut's and the correct fixtures' certificates.

To refresh: copy the same paths from a newer mirror commit, update this
README, and expect the counts in the tests to move only where mirror's
data did.

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
| `37-record-dark-2.record`, `48-dark2-order-basis.song`, `55-certificate.cert`, `55-record-dark2-strata.record`, `56-record-dark2-reader.record`; `71`, `77`, `80`, `81`, `90`, `91`: `-source.pact-text`, `-certificate.cert` and their `.record` | `tests/checker/vectors/`, `reed/ship` `3cedf18a` (blobs as `git ls-tree` names them, e.g. `55-record-dark2-strata.record` `d9a4bc81`) | Taut's Dark-2 and hole triples under Note §6.7 (H1, H4, H5): a source, its performance, and the record's whole bytes (each `.record`'s `hex` field, the said text empty). Certificate replay R2 |
| `68-certificate-stray.cert`, `68-a5-dissonant.song` | `tests/checker/vectors/`, `reed/ship` `3cedf18a` (blobs `fa7bb792`, `aeb4c516`) | Taut's one full A5 tune output for a dissonant case: corpus 01's source, 27's record half, 27's certificate plus a spurious `layer 1 path(a, a)`; the `stray` line and its atom. Certificate replay R3 (the interval's text and atom; P and F agree, the stray is R4's) |

How echo uses them:
- `echo/tests/vectors.rs`: the 46 atom vectors read as `fact` lines, print back, and the 7 that carry `content` encode to it; `mirror.facts` reads whole.
- `echo/tests/address.rs` (P2): all 215 digests from their preimages; the 63 `atom` lines, the 32 `fact … <hex>` lines and 50 composites (fragments, lenses, the ε frame, models, result and facts sets) from their text, by echo's own encoder.
- `echo/tests/integrity.rs` (P4): `mirror.facts` passes V0, and `# grammar` recomputes from its facts.
- `echo/tests/replay.rs` and `echo/tests/bin.rs` (certificate replay, R1): every fixture reads and none rings; vector 50's `heard` line byte for byte; the model's address recomputes from the layers of Taut's and the correct fixtures' certificates.

- `echo/tests/source.rs`, `echo/tests/dark2.rs` and `echo/tests/r2.rs` (certificate replay, R2): the fixtures' and Taut's sources read and located; the holes in A6's order against 55's record; the head against 48a, 22f, 24f and 71c; `basis` against 48b; the hole atoms against 73–75, 77, 81, 90, 91; Taut's triples agree; 76 and 76a's interval lines and atoms, byte for byte.

- `echo/tests/r3.rs` (certificate replay, R3): P and F against the 16 `NNa-program` and 16 `NNb-facts` vectors (their noted sources re-read and re-printed by echo), the 17 `NNg-header` vectors, every fixture's and Taut's certificate claims; `ct-a4`'s `source` interval; 45, 46 and 68's interval text and atom.

To refresh: copy the same paths from a newer mirror commit, update this
README, and expect the counts in the tests to move only where mirror's
data did.

# mirror's data, for listen

Copied byte for byte from mirror, branch `reed/ship` at
`52a435947fbaaee4fecce82180cc8d9638409be8` (2026-11-18). Data only: no line
of mirror's engine source was read to make or check them (prism-022
Response §3; docs/spec/listen.md §2).

| Here | From mirror | What |
|---|---|---|
| `*.vector` (46) | `tests/checker/vectors/` (tree `6449be5b`), every file whose `kind` is `atom`, `atom (contrast)`, `atom (negative)` or `atom (certificate header, M6)` | Taut's §5.0 known-answer vectors, written from the spec text. listen reads each `atom` line as a `fact`, prints it back, and checks the 7 that carry `content` against its own argument encoding |
| `mirror.facts` | `mirror.facts` (blob `4c0c565b`) | the pinned F_G (`# grammar 9092fca1…`, the charter's format-facts row): 3 headers, 539 typed `fact` lines (739 `addr`, 229 `nat`, 33 `bytes` arguments) |

Checked by `listen/tests/vectors.rs`. To refresh: copy the same paths from
a newer mirror commit, update this README, and expect the counts in the
tests to move only where mirror's data did.

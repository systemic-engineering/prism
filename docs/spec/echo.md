# echo: prism's second reader of mirror

*prism-022 (mirror-Reed → prism-Reed, 2026-11-18). Version 7: the witness (§11, `prism echo --witness`), beside S5's mechanical checks, prism-020 P1 (the facts reader), P3 (the cut), P2 (the address law `H`), P4 (V0 integrity), and certificate replay's slices R1 (the surface), R2 (the source and the Dark-2 report, §10.4), R3 (P, F and the interval, §10.6) and R4 (the positive run, §10.8). Crate `echo`: a no_std library and the std binary `prism echo …`.*

Echo can only give back the last words she was given, and she never speaks first. The checker never authors: it replays the engine's own record from the text and gives it back exactly, so where the two differ, the difference is named and located.

## 1. Scope

| Item | Here | Where it's specified |
|---|---|---|
| **P3, the cut** | `echo::cut` | mirror `docs/math/2026-11-16-mara-the-reflection-seam.md` §2.1 (amendment 1, `4f6bdc17`) |
| **P1, the facts reader** | `echo::read`, `echo::read_reflection` | the same note, A1.4; Seam's pact-eval audit §1.2 (`docs/audits/2026-10-29-seam-pact-eval-and-ruler-envelope.md`); `docs/specs/mirror-fixpoint-parser.md` §5.0, §5.0.1 items 2, 6, 7, 10, 11, §16.2.1, §16.10 item 1 |
| **S4, typed facts** | `echo::Value`: `Sym`, `Nat`, `Addr`, `Bytes`; equality by type and bytes | fixpoint §16.10 item 1; Seam's trace audit (`seam/trace-audit` `1971b5a1`) §4.2 |
| **S5, independence** | `checks.echo-independent`, `checks.echo-portable` | prism-022 S5 and Response §3; `docs/specs/mirror-certificate.md` §4.3 |
| **P2, the address law `H`** | `echo::address` (`Tau`, `hash`, `atom`, `fragment`, `lens`, `set`) | fixpoint §5.0, §5.0.1 items 1, 2, 10, 11, §4.1 (fact store bytes), §15.2 (the altitude), §16.10 item 1 (`bytes`, `set_addr`); certificate §4.1 item 9, §4.2, §4.3 (the known-answer vectors) |
| **P4, V0 integrity** | `echo::integrity::{check_facts, check_reflection}` | seam note amendment 1, A1.3 and A1.7 P4; fixpoint §16.10 item 1 |

| **S1/S2, certificate replay, R1** | `echo::replay`, `echo::certificate`, `echo::record`; the binary `prism` (`echo/src/bin/prism.rs`) | Note v3.3.2 §4.2, §6.3, §6.4 (a); mirror's v3.2 contract delta; certificate §3.2, §3.1.1 A and J, §4.3 (§10 here) |
| **S1/S2, certificate replay, R2** | `echo::source` (the pact reader), `echo::dark2` (the Dark-2 report and check 10), `echo::interval` | Seam's pact-eval audit §1.2; certificate §2.4, §3.1, §3.1.1 J, J′, M4; Note v3.3.2 §6.2, §6.4, §6.5 G7, G8, §6.7 H1–H5, H7 (§10.4 here) |
| **S1/S2, certificate replay, R3** | `echo::program` (P, F, check 1) | certificate §3.1, §3.1.1 A, B, F, G, §3.2; Note v3.3.2 §6.2, §6.7 H5 (§10.6 here) |
| **S1/S2, certificate replay, R4** | `echo::model` (the positive run, checks 3, 4, 5, 8; the positive core) | certificate §1, §2.1, §3.1, §3.2, §4.1, §4.2; Note v3.3.2 §2.1, §2.2, §6.2 (the `structure` rows), §6.5 G5, §6.7 H1, H3 v3.3 (§10.8 here) |

| **The witness** (prism-025 Addendum item 3) | `echo::witness` (`Response`, `Roll`, the record); `prism echo --witness` | Mara 052e5a13 §1.1, §1.2; Seam SF §5; mirror-029 §6 (§11 here) |

Not yet: P5 (V2, the second reading), and certificate replay's R5–R10 (§10).

## 2. Independence

**Never read mirror's engine.** echo's builders read mirror's **specs, docs and test vectors only**. They never open mirror's engine source: not `src/`, `rust/` or `crates/`, and not the tune's Python (`tests/checker/*.py`) for its logic. Vectors and fixtures are read as data. Where a spec is unclear, the question goes to Mara through mirror-Reed (§9). It is never settled by a guess, and never by looking at how mirror did it. The reason is prism-022 Response §3: echo's builders and mirror's engine builder are instances of the same model, and so they share priors. If one of them peeked at the other's code, a shared misreading of one spec sentence would go through both readers unseen. The tune's vectors, written by other authors in another language, are where such a misreading surfaces. Seam's audit (S8 iii) should check this rule.

**The graph.** `checks.echo-independent` runs `nix/echo-independent.sh` over `cargo metadata --locked --offline`. Over the closure of `echo` in `resolve.nodes` (every dependency kind, every depth), it refuses:
- `prismqueer`, which carries the LAPACK features;
- any mirror crate (`mirror`, `mirror-*`, `mirror_*`);
- by name, `*lapack*`, `*blas*`, `*fortran*`, `netlib-src`, `intel-mkl*` and `accelerate-src`;
- any package whose `links` key names `lapack`, `blas` or `fortran`. prismqueer itself links LAPACK through `build.rs` without such a crate name, so a name alone would miss a native link.

The same check runs the guard over four fixtures in `nix/fixtures`:
- `echo-clean.json` must pass: prismqueer is in the workspace, but not under echo;
- `echo-direct.json` must be refused: echo → prismqueer;
- `echo-transitive.json` must be refused: echo → helper → openblas-src, with a dev cycle back to echo, so the closure has to terminate;
- `echo-links.json` must be refused: a crate with an innocent name and `links = "lapack"`.

**One dependency: `blake3`,** for `H` (§5). It is pinned `=1.8.7` with `default-features = false` (no `std`) and `features = ["pure"]`: only the portable Rust compression function, so its `build.rs` compiles no C and no assembly on any target. Its graph (`arrayref`, `arrayvec`, `cfg-if`, `constant_time_eq`; `cc` as a build dependency, unused under `pure`; `cpufeatures` on x86 only) holds nothing the guard forbids, and none of it has a `links` key. The guard's forbidden list is unchanged. Two notes:
- Cargo unifies features across a workspace build, so `pure` also reaches prismqueer's `blake3` when both are built together. The output is the same function; only speed differs.
- `blake3` has an optional `serde` dependency. echo doesn't enable it, so serde is not in echo's graph.

**The library** is `#![no_std]` with `alloc` and `#![forbid(unsafe_code)]`. It has no clock, no network, no process, no file and no sampling (certificate §4.3). `checks.echo-portable` builds it for `wasm32-unknown-unknown` and `thumbv7em-none-eabihf`, so nothing from std can reach it. No serde.

## 3. The cut (P3)

From the seam note §2.1, verbatim in effect:
- **The seam** is the first line that is exactly `---`: the bytes `2d 2d 2d 0a`, at offset 0 or right after an LF.
- With a seam, `s = a ‖ "---\n" ‖ r`. `a` is empty or ends in LF. `r` is every byte after the seam line, and may be empty.
- With no seam, `a = s` and `r` is **absent**. That is not the same as empty: `Cut::reflection` is `None` here and `Some("")` there.
- Only the first seam cuts; `r` may contain more `---` lines.

What follows from the bytes, with tests in `echo/tests/cut.rs`:
- `--- ` (a trailing space), ` ---`, `----`, `--`, `-- -` and `x---` are not seams;
- a seam after a near miss still cuts;
- `---\r\n` is not a seam (§9 Q1);
- `---` at EOF with no LF is not a seam (§9 Q2).

The seam's line number travels with `r`, so `read_reflection` locates a refusal in the whole file's coordinates. The part above the seam is never read.

## 4. The facts grammar (P1)

The seam note A1.4 fixes the content: `# <key> <value>` header lines, `fact <pred>(<arg>, …)` lines with argument kinds `addr`, `nat`, `bytes`, `sym`, `#` comments, and **nothing else**. An unknown line form is a refusal with its location, never a silent drop. `rule` and `functional` lines never appear below the line. The shapes come from Seam's frozen pact-text grammar (pact-eval §1.2); the constants from fixpoint §5.0.1 and §16.10.

```ebnf
file      = { line } ;                    (* lines end in LF; the last LF may be missing (§1.2) *)
line      = blank | hash-line | record ;
blank     = { " " } ;                     (* §1.2; §9 Q4 *)
hash-line = "#" , { printable } ;         (* column 1 only (§1.2) *)
header    = "# " , key , " " , value ;    (* a hash-line in the leading block of hash-lines (§9 Q3) *)
key       = lower , { lower | digit | "_" | "-" } ;
value     = printable-not-space , { printable } ;
record    = "fact" , sp , atom , { " " } ;
atom      = pred | pred , "(" , arg , { "," , sp0 , arg } , ")" ;   (* "p()" doesn't read *)
pred      = name ;                        (* not "not", "requires" *)
arg       = name                          (* a bare name means sym "<name>" (§5.0.1 item 6, §16.10) *)
          | "sym " , '"' , { char | '\"' | '\\' } , '"'
          | "nat " , ( "0" | nonzero-digit , { digit } )          (* at most 2^64 - 1 *)
          | "addr " , 64 * lower-hex
          | "bytes " , '"' , { lower-hex , lower-hex } , '"' ;
name      = lower , { lower | digit | "_" } ;   (* not a keyword: §9 Q5 *)
sp        = " " , { " " } ;  sp0 = { " " } ;
lower-hex = digit | "a".."f" ;
printable = 0x20..0x7E ;
char      = a UTF-8 scalar, not '"', '\', a byte below 0x20, or 0x7F ;   (* §9 Q7 *)
```

**Types are kept.** `Value` is one of `Sym(String)`, `Nat(u64)`, `Addr([u8; 32])`, `Bytes(Vec<u8>)`, and equality is derived over the variant and its bytes. That matches §16.10's "equality is by the §5.0 bytes", since each argument is `tag ‖ enc(bytes)` (§5.0.1 item 11) with tags `0x00` sym, `0x01` nat, `0x02` addr, `0x03` bytes. So:
- the bare `seq` equals `sym "seq"`;
- `addr abcd…` never equals the bare name `abcd…` with the same 64 hex digits, nor `sym "abcd…"`, nor `bytes "abcd…"`;
- `nat 7` never equals `sym "7"`.

The printed form is always the canonical typed text (`sym "…"`, never bare), so printing never loses a type. Seam's trace audit found that records print `addr` as bare hex. Under this grammar a digit-led bare hex doesn't read, and a letter-led one reads as a **sym**. A record can't give back a type it never printed (§9 Q8).

**Keywords.** `int` and `ratio` are reserved and refused until they are pinned (§16.10). `not` and `requires` are never names (§1.2). A type word without its value (`p(addr)`) is refused (§9 Q5).

**Refusals.** Reading stops at the first byte that doesn't read. It returns `Refusal { at: L:C, what }`, where `L` is 1 plus the LFs before the byte and `C` is 1 plus the bytes before it on its line: columns count bytes (§16.2.1). Each refusal says what went wrong:

| `what` | when |
|---|---|
| `Form` | a line that is neither blank, a `#` line nor a `fact` record (`rule`, `query`, `functional`, an indented line, `reflect …`). Located at column 1 |
| `Byte(b)` | CR, TAB, NUL, DEL, or a non-ASCII byte outside a sym string |
| `Expected(…)` | another byte, or the end of the line, where the grammar wanted something else |
| `Keyword` | `not`, `requires`, or a bare type word where a name belongs |
| `Reserved` | `int`, `ratio` |
| `Uppercase` | an uppercase hex digit (§5.0.1 item 7: one text per address) |
| `AddrLength` | an `addr` that isn't exactly 64 hex digits |
| `OddHex` | a `bytes` string with an odd number of digits, located at its closing quote |
| `LeadingZero`, `NatOverflow` | a `nat` with a leading zero, or one above 2^64 − 1 |
| `Escape` | a `\` not followed by `"` or `\` |
| `Utf8` | a sym string that isn't UTF-8 |

Duplicate `fact` lines are kept in input order, each with its line. Forming the set is the caller's step (§5.0.1 item 2). V0 is one such caller, and it refuses a repeat (§6).

## 5. The address law (P2)

From fixpoint §5.0, byte for byte:

| symbol | bytes | echo |
|---|---|---|
| `u64(n)` | 8 bytes, unsigned, big-endian | `to_be_bytes` |
| `enc(b)` | `u64(len(b)) ‖ b` | `enc` |
| `τ` | the altitude's UTF-8, `0x00`, then `0x00` (root) or `0x01 ‖` 32 raw bytes of `addr(format grammar)` | `Tau::root`, `Tau::under`, `Tau::bytes`; a name holding NUL is refused (`None`), since it would make `τ` ambiguous |
| argument | `tag ‖ enc(bytes)`: `sym 0x00` (UTF-8), `nat 0x01` (`u64`), `addr 0x02` (32 bytes), `bytes 0x03` (§4.1; §5.0.1 item 11; §16.10 item 1) | `Value::encode` |
| content | `enc(p) ‖ u64(k) ‖ arg₀ ‖ …` (§5.0.1 item 10) | `Atom::content` |
| `H` | BLAKE3, default (unkeyed) mode, 32 bytes | `address::hash` = `blake3::hash` |

- atom: `H("splinter\0" ‖ τ ‖ enc(content))`, `address::atom`;
- fragment: `H("fragment\0" ‖ τ ‖ enc(ℓ) ‖ u64(k) ‖ addr(x₀) ‖ …)`, children in the order given, `address::fragment`;
- lens: the same with `"lens\0"`, `address::lens`;
- `set_addr` (§16.10 item 1; §5.0.1 items 1–2): the fragment over the members sorted ascending, unsigned, byte-wise (which is `[u8; 32]`'s own order). A repeated member is refused (`Repeated`), never deduplicated. `address::set`.
- `Tau::parse_item()` is `@parse/item` with the root flag: the altitude of every grammar fact and of F_G's set address (§15.2, §16.10 item 1).

**Typed vs plain.** The tag is the first byte of every argument. So `addr ab…ab` and the bare name `ab…ab` (which means `sym "ab…ab"`) encode to different bytes (`0x02 ‖ enc(32 bytes)` against `0x00 ‖ enc(64 bytes)`) and so have different addresses. The same holds for `nat 7` and `sym "7"`, and for `bytes "2f"` against `sym "2f"` and `nat 47`. A bare name and `sym "<name>"` have one address (§16.10 item 1).

**This is not prismqueer's identity.** prismqueer's `Oid` is a scoped (keyed) BLAKE3, a different law. Fixpoint §5 says so in so many words: "`addr` and `Oid::hash` are different functions, and no code may substitute one for the other". echo doesn't depend on prismqueer, and nothing here calls it. Q18 asks whether there is any place the two must agree; none was found.

**The blake3 pins.** prism's lockfile holds `blake3 1.8.7`, which prismqueer pins `=1.8.7`; echo pins the same, exactly. mirror's engine asks for `blake3 = "1.8"` (a semver range; the seam note A1 found it by grep. echo's builders don't open mirror's `rust/`, so the version mirror resolves isn't read here). mirror's vectors were generated with b3sum 1.8.7, and echo's crate reproduces every one of them. So under the seam note's own test (A1.7 P2: "the blake3 pins agree, or the vectors prove they don't need to"), the vectors prove it today. A later `1.8.x` in mirror would be checked by the same vectors. Q17 is whether mirror should pin exactly as well.

## 6. V0 integrity (P4)

The seam note (A1.3) grades V0 as **a checksum, not a reader**: "each address in `r` recomputes from the content `r` lists, and the header is well-formed". A1.7 P4 adds "`H(a)` matches". With the one reflection format that is pinned today, the F_G file (A1.4 names `mirror.facts` the precedent; fixpoint §16.10 item 1), that is exactly this, in order:

1. **`r` reads** (P1). A refusal keeps its location, in the whole file's coordinates: `Breach::Unreadable`.
2. **The header is well-formed:** exactly `# format-grammar <40 hex>`, `# root <64 hex>`, `# grammar <64 hex>`, in that order, in lowercase (§5.0.1 item 7). Otherwise `Breach::Header` at its `L:C`:
   - `Missing(key)`, at the line after the header block;
   - `Expected(key)`, at the key (column 3);
   - `Hex`, at the value;
   - `Unexpected`, for a fourth header, at its key (Q13).
3. **The facts are listed as a set:** strictly ascending by atom address at `@parse/item` root (§16.10 item 1, §15.2, §5.0.1 items 1–2). A repeat, or a line out of order, is `Breach::Order` at that line (Q15).
4. **Every address that `r` lists the content of recomputes.** That is `# grammar` = `set_addr(@parse/item root, "grammar", facts)`. On a mismatch, `Breach::Address` is located at the claim (`L:11`) and names both values.
   - `# root` is `addr(N₀)`, which only a reading of `a` gives (V2).
   - `# format-grammar` is the git blob of the grammar's source.
   - Neither is listed in `r`, so V0 checks only their form.
5. **`H(a)`** is computed as BLAKE3 of the bytes above the seam (seam note §2.1), and returned in `Integrity::above`. No pinned header carries it yet: §4.1's `reflect above …` is "a proposal", and has no `#`. So there is nothing in `r` to match it against, and V0 doesn't invent a header (Q12).

`check_facts` runs 1–4 over a facts file on its own (`above` is `None`). `check_reflection` cuts first (P3): a file with no seam is `Breach::Unreflected`. Counts and the addresses of the other lists (holes, sites, interface; A1.5) aren't in any pinned surface yet (Q14).

## 7. Vectors

**Cross-implementation vectors are data in both repos** (seam note A1.3; prism-022 S5).

- **mirror's data**, copied byte for byte into `docs/spec/echo-vectors/mirror` (its README names the paths, the commits and the blobs):
  - **all 215 `.vector` files** of `tests/checker/vectors` (tree `6449be5b`, the same at `reed/ship` `52a43594` and `3cedf18a`). These are Taut's §5.0 vectors, written from the spec text by someone who wrote neither the engine nor the checker (certificate §4.3). Every digest was computed twice, by his pure-Python BLAKE3 and by b3sum 1.8.7. In `echo/tests/address.rs`:
    - every `blake3` line is `H` of its own `preimage` (215 of 215), and no `not-equal` vector's digest is the one it names (10);
    - from their **text**, by echo's encoder: the 63 `atom` lines (preimage and digest; the one `(negative)`, 04, is never produced), the 32 `fact <atom> <hex>` lines, and 50 composites. The composites are 13 fragments (including k = 1000 and 1001), 2 lenses, the ε frame (25), 18 sets by `members` (16, the negatives 17 and 18, and the result models) and 16 `certificate: facts` sets;
    - the 10 whole certificates name children (`header layers …`) that only certificate replay derives, so for those only the digest-over-preimage is checked.
  - In `echo/tests/vectors.rs`, the **46 `kind atom*` vectors**: each `atom` line reads as a `fact` and prints back to the vector's own text, and the 7 that carry `content` encode to exactly those bytes. That covers multi-byte UTF-8 (`sym "λ"`), `nat 0`, `nat 2⁶⁴−1`, `sym ""` and `addr`.
  - **`mirror.facts`**, the pinned F_G (`# grammar 9092fca1…`). It reads whole: 3 headers, 539 facts with 739 `addr`, 229 `nat` and 33 `bytes` arguments, and its canonical print reads back to the same atoms. It passes V0, on its own and below a seam.
- **prism's own address vectors.** `docs/spec/echo-vectors/address.py` writes `address.vectors`, 19 records in the `.vector` shape. It was written from the §5.0 text, not from echo:
  - atoms over every tag, including `bytes` (which mirror ships no vector for), the typed/plain pairs, and both `τ` flags;
  - fragments, a lens and a set;
  - `addr(F_G)` recomputed from `mirror.facts` by the script's own small facts reader: `9092fca1…`, mirror's own header.

  The script hashes with Python's `blake3` 1.0.8 and re-hashes every preimage with b3sum 1.8.7. Both wrap the Rust `blake3` crate, so their diversity is in the encoder (Python, from the text), not in the hash. Taut's pure-Python BLAKE3 is the hash diversity on mirror's side. Run: `nix shell nixpkgs#b3sum -c uv run --no-project --python 3.13 --with blake3==1.0.8 python docs/spec/echo-vectors/address.py`.
- **From the spec text.** mirror ships no `bytes` vector, no refusal vector and no cut vector, so these are written in `echo/tests/{cut,facts,refusals,typed}.rs`, each citing its section. That includes `bytes "2f"` and `bytes ""` as tag `0x03` with their `enc` length. V0's cases are in `echo/tests/integrity.rs`.

## 8. Mutations

Each of these was applied, killed by the suite, and reverted:

| Mutation | Killed by |
|---|---|
| `Value` equality by its text, without the type tag | 4 `typed::` tests (addr vs plain name, addr vs sym, addr vs bytes, nat vs sym) |
| the cut also accepts `--- ` | `cut::a_trailing_space_is_not_the_seam` |
| an unknown line form is skipped silently | `refusals::unknown_line_forms`, `refusals::reflection_refusals_are_located_in_the_file` |
| the graph guard reads direct dependencies only | `checks.echo-independent` (accepted `echo-transitive.json`): the first red, `d4ed4fa` |
| the encoder drops the type tag (`Value::encode` without `tag`) | 8 tests: `address::` atoms, fact lines, composites, our vectors, typed vs plain; `typed::` (2); `vectors::` |
| `H` keyed (`blake3::keyed_hash`) | 6 `address::` tests, every digest among them |
| `H` by `blake3::derive_key` with a context | the same 6 |
| V0 without the address check | `integrity::a_wrong_claimed_address_is_refused_at_the_claim`, `integrity::a_dropped_fact_is_refused_at_the_claim` |
| V0's order check lets a repeat through (`<` for `<=`) | `integrity::order_and_repeats_are_refused_at_their_line` |
| R1: replay defaults to `rings` when nothing failed (`Verdict::Rings` for `conclude(&UNREPLAYED)`) | 8 tests: `replay::what_echo_cannot_replay_is_unknown_never_rings`, `replay::no_fixture_rings`, four more `replay::` tests that expect `unknown unreplayed source`, the budget's boundary, and `bin::the_binary_prints_the_library_answer` |
| R1: the record dropped from `heard` (its slot `none`) | `replay::heard_names_each_input_by_blake3_of_its_raw_bytes`, `replay::vector_50_heard_line_byte_for_byte` |
| R1: the binary takes its budget from the clock (`SystemTime` nanoseconds mod 10) | `bin::the_binary_source_reads_only_its_arguments`, `bin::the_binary_prints_the_library_answer` |
| R2: a Dark-2 source reads (`source::read` drops every line that doesn't read) | 19 tests: 5 `source::`, 7 `dark2::` (48's holes, head and basis among them), 7 `r2::` (the fixture table, the tune's kinds, Taut's triples among them) |
| R2: a source cheat passes as unknown (intervals found, `unknown unreplayed program` printed) | 9 tests: 7 `r2::` (the fixture table, the tune's kinds, 76/76a), `replay::record_hole_text_is_opaque`, `replay::the_verdict_then_heard_then_implements` |
| R2: the Dark-2 report skipped (the record's `hole` lines never compared) | 7 tests: 5 `r2::` (the fixture table, the tune's kinds, the multiset, 76/76a, the sort), 2 `replay::` |
| R2: decide where R2 can't tell (`Light::Open` judged) | `r2::what_r2_cannot_tell_is_left_to_the_next_stratum` |
| R2: the `dark` kind never pairs | `r2::dark_atoms_pair_only_one_for_one`, `r2::the_fixture_table` (dark2-not-canonical) |
| R3: check 1 skipped (the header's `program` and `facts` never compared) | 7 tests: `r3::ct_a4_is_refused_for_its_spurious_fact`, `r3::dark_headers_are_judged`, `r3::a_dark_source_with_a_model`, `r3::strata_compose_in_one_list`, `r3::the_fixture_table`, `r3::the_kinds_agree_with_the_tune`, `replay::an_unreadable_source_is_not_unreadable` |
| R3: F over the wrong set (queries' atoms in it) | 6 tests: `r3::p_and_f_are_sets_of_the_lines_that_read`, `r3::the_certificates_claim_the_recomputed_addresses` (pos-v03, a8, p8-swapped carry queries), `r3::the_fixture_table`, `r3::the_kinds_agree_with_the_tune`, 2 `r2::` |
| R3: the intervals left unsorted (R2's `verdict` lines before R3's `source` lines) | `r3::strata_compose_in_one_list`, `r3::the_fixture_table` (ct-a5) |
| R3: sorted by the line alone, not by the stratum's enumeration | `r3::intervals_sort_by_stratum_then_bytes` (`grammar` before `engine`) |
| R3: a reader-Dark source's header expected to be P and F | 14 tests, among them `r3::the_header_vectors` (24g), `r3::dark_headers_are_judged`, Taut's triples (55, 56) |
| R3: check 1 decided where it turns on light R2 can't tell | 4 tests: `r3::check_1_waits_for_the_light_it_needs`, `r3::the_expected_header_follows_the_light`, `r3::the_header_vectors` (95, 97), `r2::what_r2_cannot_tell_is_left_to_the_next_stratum` |
| R2: check 2 ignored (a Dark source printed with a model judged) | `r2::what_r2_cannot_tell_is_left_to_the_next_stratum`, `replay::an_unreadable_source_is_not_unreadable` |
| R2: the head by A6's print order, not the least offset | `dark2::query_arity` (P8: `# 4:6` prints first, `1:1` is the head) |
| R4: depth off by one (a new atom at `round + 1`) | 22 tests, among them Taut's `NNd`/`NNe` and the models (`r4::the_model_layer_and_derivation_vectors`, `r4::echo_s_model_is_the_certificates_result`), the diamond, `r4::the_structure_cheats`, `r4::vector_68_byte_for_byte`, the budget's boundary, and R2's and R3's fixture tables |
| R4: a stray derivation accepted (a claimed derivation of a model atom passes whether or not it is first-tick) | `r4::an_extra_derivation_is_refused` |
| R4: a stray atom accepted (a layer line outside the model passes) | 8 tests: `r4::the_structure_cheats` (ct-a9), `r4::vector_68_byte_for_byte`, `r4::the_fixture_table`, `r4::the_kinds_agree_with_the_tune`, 4 `r3::` (ct-a4, the strata in one list, 68) |
| R4: the budget charged per `derive` line (R1's charge) instead of per instance formed | 4 tests: `r4::the_budget_is_charged_per_instance`, `r4::the_budget_on_every_fixture`, `replay::the_budget_counts_ground_rule_instances`, `replay::a_budget_of_one_runs_out_on_real_inputs` |
| R4: check 8 skipped (no `missing`, no `extra`) | 5 tests: `r4::the_structure_cheats` (ct-a2, ct-a10), `r4::an_extra_derivation_is_refused`, `r4::the_fixture_table`, `r4::the_kinds_agree_with_the_tune`, `r3::the_kinds_agree_with_the_tune` |
| witness: `unknown unreplayed` answered `accept` | 3 `witness::` tests: `respond_maps_every_verdict`, `every_fixture_answers_by_the_mapping`, `a_fixture_needing_r5_is_not_now_and_contexts_is_not_covered` |
| witness: `contexts` listed as a covered stratum, never replayed | 4 `witness::` tests: the coverage table over 38 fixtures, R5's not-now, an input not read, the record's order |
| witness: the record hashed into the certificate's `heard` address | `witness::the_witness_never_enters_the_certificate_address`, `witness::with_the_flag_the_parity_lines_come_first_then_the_seam` |
| witness: a parity line changed with the flag off (`rig  none`) | `witness::without_the_flag_every_fixture_is_byte_identical`, `witness::with_the_flag_the_parity_lines_come_first_then_the_seam`, `bin::the_binary_prints_the_library_answer` |

## 9. Open questions for Mara

Each one is settled in code as stated, conservatively (refuse rather than guess an accept), and is marked in the tests. None of them is settled by looking at mirror's engine.

- **Q1. CRLF** (seam note §2.1). The seam is `2d 2d 2d 0a`, so `---\r\n` is not one, and a CRLF file is unreflected. Is that intended, or does the cut normalise line endings? (Below the seam, a CR is refused, pact-eval §1.2.)
- **Q2. `---` at EOF with no LF** (§2.1). The bytes say no seam. Is a final `---` an empty reflection, a hole (as fixpoint §16.2.1 makes a final comment with no LF), or nothing?
- **Q3. Headers vs comments** (A1.4; fixpoint §16.10 item 1, "the header lines are comments"). Both start with `#`. Which `#` lines are headers? echo reads: only the leading block of `#` lines, and only lines shaped `# <key> <value>`, with key `lower { lower | digit | _ | - }`. Also: the seam note §4.1's proposed header (`reflect above … grammar … laws …`, `meaning <N>`) has no `#`. A1.4's grammar refuses it, and so does echo. Which surface is current?
- **Q4. Blank lines** (A1.4 "nothing else" against pact-eval §1.2 `blank`). echo accepts lines of spaces only.
- **Q5. Type words as names** (§16.10 item 1 against pact-eval §1.2 `name`). Are `sym`, `nat`, `addr`, `bytes`, `int` and `ratio` readable as bare names (`p(addr)`)? echo refuses them.
- **Q6. Spacing inside a typed constant** (§5.0.1 item 6). echo requires exactly one space between the type word and its value, since that is the canonical text. Is `nat  7` unreadable, or the same constant?
- **Q7. Bytes inside `sym "…"`** (§5.0.1 item 6 `<utf-8>` against pact-eval §1.2's ASCII-only rule). echo accepts any UTF-8 scalar except controls below 0x20 and 0x7F, and refuses TAB and CR inside the string. It also refuses non-ASCII in comments and headers.
- **Q8. Records as typed input** (prism-022 Response §4; Seam's trace audit §4.2). Records print `addr` as bare hex. Does the record format (Note §6.x) get a type mark (`addr <hex>`), or is a record never read as typed input? Until then echo keeps the type wherever its input carries one, and cannot recover it from a record.
- **Q9. F_G order and duplicates** (§16.10 item 1: "sorted ascending by atom address bytes"; §5.0.1 item 2). echo's reader keeps input order and duplicates. V0 (P4) refuses both (§6 item 3); see Q15.
- **Q10. First refusal or every line?** echo stops at the first unreadable byte. Note §6.7 H1 makes every unread pact line its own `# L:C` hole and reads the rest. Which applies to a reflection?
- **Q11. `nat` range** (§5.0 `u64(n)`). echo refuses a `nat` above 2^64 − 1 as unreadable. Confirm that it isn't a different kind of hole.

Added with P2 and P4:

- **Q12. `H(a)`: which hash, and where it stands.**
  - The hash: the seam note §2.1 defines `H(a)` as "BLAKE3 (or the git blob hash) of the above bytes". echo computes plain BLAKE3 of `a`, unkeyed, with no kind prefix and no length. Which one is it? If it is BLAKE3, is it bare, or under a domain prefix like §5.0's kinds?
  - The header: §4.1 has `reflect above <H(a)> grammar <G> laws <L> answers …` ("a proposal"); A1.2 has `reflect above <H(a)> compiler <addr(G, L)> answers <addr(A)>`. Neither has a `#`, so A1.4's grammar refuses both (Q3).
  - Until one is pinned, V0 returns `H(a)` and matches it against nothing.
- **Q13. A reflection's header set.** V0 requires exactly F_G's three lines (§16.10 item 1), and refuses a fourth. A reflected `mirror.mirror` would carry the key (`H(a)`, `C`, `A`), `meaning <N>`, and F_G's headers (§10). Which keys, in what order, and are they one block or two?
- **Q14. Counts and list addresses** (A1.5: for each list (holes, sites, interface), `r` carries its count and its address, and the full list when it has at most k entries). No surface is pinned. When one is:
  - V0 would check count = length and address = `set_addr`(list) wherever the list is printed. Which `τ` and `ℓ` belong to each list?
  - Which lists are sets (sorted) and which are sequences?
- **Q15. Repeats and order in `r`** (refines Q9). §5.0.1 item 2 has the reader collapse source-level duplicates when it forms the set. But `r` is emitted, not authored, and §16.10 item 1 pins F_G's file as sorted. So echo's V0 refuses any repeat or misorder in `r`, at its line. Confirm that it should refuse, and not collapse.
- **Q16. The altitude of other reflected facts.** F_G's facts are at `@parse/item` root (§15.2), and V0 addresses them there. Will other fact lists below the seam use the same `τ`, or their own?
- **Q17. The blake3 pin** (for mirror-Reed and Alex, not a spec question). prism pins `=1.8.7`. mirror's engine asks for `"1.8"` (seam note A1). The vectors agree at 1.8.7, so A1.7 P2's "or the vectors prove they don't need to" holds today. Should mirror pin exactly too, so that a future `1.8.x` is a visible change and not a silent one?
- **Q18. `H` and prismqueer's identity.** These are different laws (fixpoint §5), and echo uses only `H`. No place was found where they must agree. The nearest is spectral's crystal content hash: if prism ever stores mirror atoms as crystals, one atom would have two addresses, and they must stay two named fields, never one. Is there a place they *should* agree?
- **Q19. `# format-grammar` is 40 hex** (§16.10 item 1: "the signed grammar blob"). That is SHA-1. In a SHA-256 repository (§7: "the repository's object format"), would it be 64? V0 accepts only 40 today.

## 10. Certificate replay, in slices (S1, S2)

Certificate replay (prism-022 S1/S2; certificate §4; Note v3.3.2 §6) recomputes, from the source alone, P, F, the model, the depths, the first-tick derivations, W's context sequence, every functional declaration, the verdict, the record, the Dark-2 report and every address, and compares each with what the performance, the record and the score claim. It is built in slices, in dependency order. Each slice names the spec sections it implements and the mirror data (fixtures and vectors, read as data) that test it.

**The floor, for every slice:** `rings` is printed only when every stratum the inputs call for has been replayed and agreed. Until then, the first stratum echo can't replay yet answers `unknown unreplayed <stratum>` (exit 2). echo never claims a ring it didn't check.

| Slice | What it replays | Spec | Data that tests it | `<stratum>` it retires |
|---|---|---|---|---|
| **R1. The surface** | the entry `prism echo <source> <certificate> <record> [<score>]`; the `heard`, `implements` and verdict lines; exits 0/1/2; `unknown io`; the work budget and `uncertain budget`; reading the certificate (§3.2's printed form, typed constants included) and the record's line form, so an unreadable one is `unknown unreadable <where>`; every atom the certificate prints addressed under `H`, and the model's address recomputed from its layers | Note §4.2 (`heard`, B8; the listener; Q2's widths; the self-reference rule), §6.3, §6.4 (a) (the record's `dark`/`hole` text is opaque); mirror `docs/pin-batch/2026-10-30-tune-exit-contract-v3.2.md`; certificate §3.2 (the printed form, UNKNOWN), §3.1.1 A, §4.3 (the shape and what the checker may not do) | vector 50 (`heard`, exact bytes); 27's halves; every fixture: none may ring; `typed-corpus-unheard` (echo reads it: S4, beyond the tune) | none: it prints `unknown unreplayed source` |
| **R2. The source and the Dark-2 report** | pact text (`fact`, `rule`, `functional`, `query`, comments, blanks); the reader checks (a repeated rule id, a non-ground query, query arity); the engine checks (arity, safety, declaration positions); `@mirror/dark-order`'s merge; `basis`; check 2 and check 10 | Seam's pact-eval audit §1.2; certificate §2.4, §3.1 (Dark atoms), §3.1.1 F, J, J′, M4 (OPEN: both readings recorded); Note §6.4, §6.5 G7, G8, §6.7 H1, H4 | `pos-dark2-engine`, `pos-dark2-reader`, `taut-22`, `taut-24`, `taut-48`, `p8-swapped`, `p8-one-line`, `a1-record-dark-*`, `dark2-not-canonical`, `ct-a5`, `ct-a6`; vectors 37, 48, 55, 56, 71 | `source` |
| **R3. P, F and the interval** | the program fragment (canonical record lines) and the facts fragment (check 1); the interval atom, its text, its order and `dissonant <n>` | certificate §3.1, §3.1.1 B, G; Note §6.2 (bytes, text, order, distance), §6.5 R2 | `ct-a4`; vectors 20–24 (headers), 45, 46, 46a, 68 | `program` |
| **R4. The positive run** | naive evaluation; depths (§1's conventions); all first-tick derivations; checks 3, 4, 5, 8; `stray`, `unlayered`, `depth`, `missing`, `extra`. The budget's real charge point: each ground rule instance attempted | certificate §1, §2.1, §3.1 (layers, derivations), §3.2, §4.1–§4.2; Note §6.2's structure rows | `ct-a2`, `ct-a8`, `ct-a9`, `ct-a10`, `pos-v01`, `pos-diamond`, `pos-v03`, `taut-20`; vector 27 | `model` |
| **R5. W** | the alternating fixpoint; the whole context sequence (check 6); under- and over-layers and derivations; `context`; `unknown shadow under W` | certificate §1 (W's layers), §2.3, §3.1 (contexts), §3.1.1 H, I; Note §6.7 H1 (C3) | `ct-a1`, `pos-v04`, `taut-23`; vector 28 | `contexts` |
| **R6. Declarations, the verdict, the record** | every `functional` declaration on every result (check 7); holes, light, shadow and the verdict words; the derived record; `verdict`, `record` and `dark` intervals | certificate §2.4, §3.1 (dark), §3.1.1 D, E, §3.2 (violating; N1, N2); Note §6.5 R1–R3, §6.7 H1–H5 | `ct-a3`, `pos-v05`, `pos-v05-control`, `taut-21`; vectors 29, 36, 54, 66, 77–86 | `verdict` |
| **R7. Budget claims** | the `erasure … budget` line; E1's true and false claims; the stopped set; `budget`; `unknown budget <run> <unit>` | Note §6.5 R4–R7, E1, S1, S2, §7 | `ct-a12`, `e1-false-budget-*`, `a8-budget-query-unknown`; vectors 49, 67, 69, 70 | `budget` |
| **R8. The certificate's address and the instrument** | the certificate fragment (check 9) and the `address` interval; the header's `engine` / `dark-order` (E4) and `instrument` | certificate §3, §3.1; Note §4.3, §6.5 E3, E4 | `ct-a11`, `e4-dark-order`; vectors 20h–24h, 30, 31, 35 | `address` |
| **R9. The score** | the score's header, Notes, gazes, piece intervals (clash hub, flux) and completeness | Note §2, §4.2, §4.2.1, §6.2 (piece rows), §6.5 G2, G3 | `pos-v04`, `pos-v05` (with their scores), `score-missing-note`; vectors 38–44a, 47, 51, 52 | `score` |
| **R10. Parity** | every fixture's `expect`, and Taut's tune KAT (41 cases) as data, byte for byte against the tune in the overlap (S7) | prism-022 S2, S7 | all of the above | none |

`rings` becomes reachable at R8 for a run without a score, and at R9 with one. The order follows the data: R2 comes first because every later stratum is judged over the lines that read, and a Dark-2 source has no model. R3 comes before R4 because the first dissonance needs the interval's bytes.

### 10.1 R1, the surface, exactly

**The entry.** `prism echo <source> <certificate> <record> [<score>]`, a `std` binary target in the `echo` package; the library stays `no_std` (`checks.echo-portable` builds `--lib`).
- It reads **its arguments and nothing else**: one `std::fs::read` per path given, and no environment variable, clock, network, process or other file (certificate §4.3). The library takes bytes, so it can't open anything. The source of the binary is tested for the absence of those calls.
- Any other argument count, or a first word other than `echo`, prints `usage: prism echo <source> <certificate> <record> [<score>]` on stderr, nothing on stdout, and exits 2 (the contract delta's last row).

**The lines** (Note §6.3; the contract delta: "`heard` and `implements` stay the second and third lines in every case"), each ending in LF, on stdout:

```text
<verdict line>
heard source <addr | none> certificate <addr | none> record <addr | none> score <addr | none>
implements spec <commit> vectors <tree> rig none
```

- **`heard`**: each slot is plain BLAKE3 of the input's raw bytes, 64 lowercase hex (B8); `none` for an input that couldn't be read, and `score none` when no score was given.
- **`implements`**: constants compiled into echo, never computed at run time, and never naming echo's own tree (the self-reference rule):
  - `spec` is the prism commit whose `docs/spec/echo.md` echo implements: the full 40-hex id (Q2: the repository's native object id; prism's repository is SHA-1);
  - `vectors` is the git tree id of `docs/spec/echo-vectors` that echo was verified against, also 40 hex. Both name commits earlier than the one that sets them;
  - `rig none`: echo is pinned to no rig (Note §4.3; M6, B1: `none` in this pin).
- **The verdict line**, and its exit:

| line | exit | when |
|---|---|---|
| `rings` | 0 | every stratum replayed and agreed. **Not reachable in R1** |
| `dissonant <n>` | 1 | from R2 on (§10.4): one `interval` line each, after `implements` |
| `uncertain budget <n> ground rule instances` | 1 | echo's own work budget, `<n>`, ran out |
| `unknown io <input> …` | 2 | every input that couldn't be read, in the order `source certificate record score` |
| `unknown unreadable <input>:<L>:<C>` | 2 | the first input, in that order, that doesn't read, at its first unread byte (Q22) |
| `unknown unreplayed <stratum>` | 2 | the first stratum, in §10's order, that echo can't replay yet (Q20) |

  Precedence: `io`, then `unreadable`, then the budget, then `dissonant` (from R2 on), then `unreplayed`. The source never makes the answer `unreadable`: a source that doesn't read is a Dark-2 source, and a correct Dark-2 performance rings (Note §5, Seam row 8).

**What R1 reads.**
- **The certificate**, by §3.2: `#` comments at column 1 and blank lines anywhere; every other line is a key, one or more spaces, and a value. The keys are the header's six (`certificate 1`, then `result`, `program`, `facts`, `engine`, `dark-order`, each `<64 hex>` or `none`, once each), `layer`, `derive`, `context`, `dark`, `violating`, `erasure` (R5's form, Note §6.5) and `hole` (Note §6.7 H5). Atoms are read with echo's typed facts reader (§4), so `bytes "…"`, `addr …` and `nat …` constants read: that is S4, beyond the tune (`typed-corpus-unheard`). An unknown key, a malformed value or a missing header key is unreadable at its `L:C`.
- **The record**, by its line form only (certificate §3.2 N2; Note §6.7 H4): a key from `verdict`, `file`, `shaped_by`, `true`, `undefined`, `false`, `findings`, `finding`, `hole`, `shadow`, `basis`, `dark`, `rounds`; one or more spaces; a value; `verdict` one of `clear`, `uncertain`, `unknown`, `paradox`. A `hole` or `dark` value is split at its first ` #`, and what follows is opaque bytes (§6.4 (a)). Every other byte is printable ASCII. The values themselves are read in R6.
- **The source and the score** are hashed for `heard`, and read from R2 and R9.

**What R1 replays.** Every atom the certificate prints is addressed under `H` at the store's `τ` (`@pact/fact` root; §3.1.1 A), and the model's address is recomputed from the layers (the positive run's atoms, or W's under-run atoms) as `fragment(@pact/fact, "model", sorted)`. That is compared with nothing in R1's verdict: the `result` it should equal is a claim, and the model it should be the address of comes from the source (R4). It is tested against Taut's data instead.

**The budget.** One pinned constant, `10 000 000` ground rule instances (prism-022 S1: the tune's figure; Q21), counted, never timed. The library takes the budget as an argument so tests can lower it; the binary has no flag and always passes the constant. In R1 the only ground rule instances echo considers are the certificate's own `derive` lines: one unit each. R4 moves the charge point to every instance its own evaluation attempts.

### 10.2 Questions R1 adds for Mara

- **Q20. A line for "no method yet".** §6.3 has no line for a stratum the checker can't replay. echo prints `unknown unreplayed <stratum>` (exit 2, by the litmus: there is no step to take). Should it be one of §6.3's forms instead, or a new pinned form? It exists only until R9, but the overlap (S7) will see it.
- **Q21. The budget's number.** The certificate spec fixes the budget as "a constant in the checker's own tree" and names its unit (ground rule instances considered). prism-022 quotes the tune's 10M. Is the number pinned in any spec text, and must echo's equal the tune's for parity (S2)? Where both run out at different points, they print different lines.
- **Q22. The form of `<where>`.** §6.3 has `unknown unreadable <where>`; the `typed-corpus-unheard` fixture records the tune's answer as `certificate:8, expected ',' or ')'`, and the T1 pin-move proposal writes `unknown unreadable record`. echo prints `<input>:<L>:<C>` and no free text. What is pinned, byte for byte?
- **Q23. `unknown budget` exits 2 or 1?** §6.3 (v3.2) and the contract delta say 2; §6.5 R5 and vector 67's README row still say 1. echo follows §6.3.

### 10.3 R1, as built

`73df7f0` 🔴 → `b030c7f` 🟢. The `implements` line reads `implements spec 7a6dd355564cffaf6c47bb4f650390a9725352f4 vectors ce646bf56a9f00a9309522d3178e4de4ca482060 rig none`: the §10 commit, and `docs/spec/echo-vectors` at the red commit, which added mirror's fixtures. Both are earlier than the green commit that sets them. They move by hand, at the commit after a spec or vectors change, never to the commit that holds them.

**What the tests hold** (`echo/tests/replay.rs`, `echo/tests/bin.rs`):
- `heard`: b3sum's digests of pos-v01's three files, byte for byte; vector 50's second line over corpus 01's source and 27's halves; `none` for every input that couldn't be read; `score none` when no score was given, which is not `io`.
- `implements`: seven words, the two ids 40 lowercase hex, `rig none`.
- The exits and first lines of all five R1 outcomes; three lines, in §6.3's order, each ending in LF.
- **The floor:** all 38 fixtures answer `unknown unreplayed source` and exit 2; `rings` only from `conclude(&[])`.
- `unreadable`: certificate and record lines refused at their `L:C` (a lowercase predicate, a missing `)`, a leading zero, an unknown run or key, a CR, uppercase or short hex, `certificate 2`, separators cut short where they stop, a header key missing or repeated, every `dark`/`context`/`erasure`/`violating`/`hole` form); the record's `verdict` word, a TAB, non-ASCII outside a hole's utterance; the order `io`, then certificate, then record, then the budget; the source never `unreadable`.
- **The budget:** pinned at 10 000 000; pos-v01's 9 derivations fit a budget of 9, and 8, 1 and 0 run out with `uncertain budget <n> ground rule instances`, exit 1, the same bytes in 50 runs; a budget of 1 runs out on exactly the 22 real inputs that print more than one derivation.
- **Addresses:** the model's address recomputes from the layers and equals the claimed `result` on pos-v01, pos-diamond, pos-v03, pos-v04 and pos-v05 (W), pos-v05-control, taut-20, taut-21, taut-23 and typed-corpus-unheard (typed `bytes`). On CT-A11 it differs from the claim and equals vector 01's. A repeated layer atom has no model address.
- **The binary:** it prints the library's answer for the same bytes; the same bytes with `env_clear()` from an empty directory; it writes nothing into its working directory (under coverage, only `LLVM_PROFILE_FILE` is kept, for the instrumentation's own runtime); a nonexistent score path is `unknown io score`; five paths (the fifth nonexistent), two paths, none, or a first word other than `echo` print usage on stderr, nothing on stdout, exit 2; its source has no `env::var`, `current_dir`, `std::net`, `Command`, `Instant`, `SystemTime`, `read_dir`, `File::open`, `OpenOptions` or `include_`, and one `fs::read(`.

**How "nothing but its arguments" holds by construction:** the library is `no_std`, takes bytes, and can open nothing (`checks.echo-portable` builds it for wasm32 and thumbv7em). The binary is the only code with `std`: it reads `argv`, opens each path given with one `fs::read`, and hands the bytes to the library. There is no flag, so the budget can't be set from outside.

**mirror's checker fixtures under R1** (the binary, each fixture's `expect` redirections followed; the tune's answer from its `expect` file):

| fixture | the tune | echo R1 |
|---|---|---|
| `a1-record-dark-disagrees` | dissonant hole | `unknown unreplayed source` (2) |
| `a1-record-dark-engine` | rings | `unknown unreplayed source` (2) |
| `a1-record-dark-in-clear` | dissonant hole | `unknown unreplayed source` (2) |
| `a1-record-dark-reader` | rings | `unknown unreplayed source` (2) |
| `a8-budget-query-unknown` | rings | `unknown unreplayed source` (2) |
| `ct-a1-stable-model-witness` | dissonant context | `unknown unreplayed source` (2) |
| `ct-a10-truncated-claims-fixed` | dissonant unlayered | `unknown unreplayed source` (2) |
| `ct-a11-model-text-sorted` | dissonant address | `unknown unreplayed source` (2) |
| `ct-a12-budget-stop` | dissonant budget verdict | `unknown unreplayed source` (2) |
| `ct-a2-missing-derivation` | dissonant missing | `unknown unreplayed source` (2) |
| `ct-a3-clear-hides-functional` | dissonant verdict | `unknown unreplayed source` (2) |
| `ct-a4-spurious-fact` | dissonant source | `unknown unreplayed source` (2) |
| `ct-a5-readable-dark` | dissonant dark | `unknown unreplayed source` (2) |
| `ct-a6-unsafe-with-model` | dissonant hole verdict shadowed | `unknown unreplayed source` (2) |
| `ct-a8-layer-off-by-one` | dissonant depth | `unknown unreplayed source` (2) |
| `ct-a9-circular-pair` | dissonant stray | `unknown unreplayed source` (2) |
| `dark2-not-canonical` | dissonant dark | `unknown unreplayed source` (2) |
| `e1-false-budget-clear` | dissonant budget | `unknown unreplayed source` (2) |
| `e1-false-budget-opaque` | dissonant budget verdict | `unknown unreplayed source` (2) |
| `e4-dark-order` | dissonant instrument | `unknown unreplayed source` (2) |
| `p8-one-line` | dissonant hole | `unknown unreplayed source` (2) |
| `p8-swapped` | rings | `unknown unreplayed source` (2) |
| `pos-dark2-engine` | rings | `unknown unreplayed source` (2) |
| `pos-dark2-reader` | rings | `unknown unreplayed source` (2) |
| `pos-diamond` | rings | `unknown unreplayed source` (2) |
| `pos-v01` | rings | `unknown unreplayed source` (2) |
| `pos-v03` | rings | `unknown unreplayed source` (2) |
| `pos-v04` | rings | `unknown unreplayed source` (2) |
| `pos-v05` | rings | `unknown unreplayed source` (2) |
| `pos-v05-control` | rings | `unknown unreplayed source` (2) |
| `score-missing-note` | dissonant missing | `unknown unreplayed source` (2) |
| `taut-20` … `taut-24`, `taut-48` | rings | `unknown unreplayed source` (2) |
| `typed-corpus-unheard` | unreadable (the tune can't read typed constants) | `unknown unreplayed source` (2): echo reads it (S4) |

Nothing rings; every cheat is "not pass". No cheat is refused *for its reason* yet: that is R2–R9, and parity with the tune's reasons is R10.

**What R1 does not do, on purpose:** it reads the record's line form, not its values (R6); it doesn't read the score (R9); it doesn't compare the recomputed model address with the `result` claim in its verdict, because the interval for that is check 9's, against a model from the source (R4, R8).

**Gates at `b030c7f`:** `just check` clean; `checks.coverage` 96.07% of lines (the ratchet is 94; echo 98.88%, 1148 of 1161); `checks.doctest`; `checks.echo-independent` (the binary adds no dependency); `checks.echo-portable`, which now builds `--lib`.

### 10.4 R2, as built: the source and the Dark-2 report

`d7a3fb5` 🔴 → `c72ecad` 🟢. The `implements` line moves to `spec 1e7eb3a402060c253bdb70305dac8c46554e92b1` (version 3) and `vectors 8c1c52ae9fd7fb4720e79044ef58ef4464535e59` (`docs/spec/echo-vectors` at the red commit, which added Taut's Dark-2 triples). R2 retires the word `source`: what R2 agrees with, or can't tell, answers `unknown unreplayed program`, the next stratum. Nothing rings.

**The reader** (`echo::source`) is Seam's pact-eval §1.2 grammar, frozen: `rule`, `fact`, `query`, `functional`, `#` comments at column 1, blank lines of spaces, printable ASCII and LF only. It always reads: a line that doesn't read is a hole, kept with its `L:C` and its byte offset, and the lines that read are read (Note §6.4, composed). The location is **the least byte offset where the grammar can't continue** (certificate §3.1.1 J):
- a byte outside 0x20–0x7E fails where it stands, never ahead of an earlier error on its line (`fact edge(a, B)⇥` is 14, not 16);
- a key must be whole (`fac p` fails at 4, `q p` at 2: `q` could still be `query`), an indented line at its first non-space;
- a name that is a keyword (`not`, `requires`) fails at the byte after it, where it could still have grown into another name (Q25);
- `p()`, a number where a `sym` belongs, a trailing comment, an empty `requires`, `?x` in a query (Q28), position `0`: each where it stops.

**Beyond the tune (S4):** a `fact` argument may be a typed constant (`sym "…"`, `nat`, `addr`, `bytes "…"`), as `pact eval` reads since trace-layer C6. A type word with no space after it is a name (`p(nat)` is `sym "nat"`, as the tune reads it). Typed constants in `rule` and `query` lines don't read, and `sym` strings are printable ASCII (Q24). `typed-corpus-unheard`'s source reads whole: 3 rules and 6 typed facts.

**The report** (`echo::dark2`), from the lines that read, Note §6.7 H1's holes with their canonical texts:

| hole | when | counted over |
|---|---|---|
| `# L:C` | a line that doesn't read | |
| `repeat L:1` | a repeated rule id, at each later line (J′); every rule with that id is left out | the rules |
| `arity L:1` | a query whose arity is none of its predicate's (M4 (a), the checker's reading) | facts and rules |
| `arity p` | a predicate used with two arities (G7: a repeated rule's atoms count); every fact, rule, query and declaration on `p` is left out | facts and rules, heads and bodies (Q29) |
| `safety r ?v` | a variable of the head or of a negative literal in no positive literal, one hole per variable | the rules repeats leave (H1 v3.0.1) |
| `declaration p <canonical line>` | no arity, a position outside `1 … n`, a repeated key (H7 v3.2.5); not when `p` has two arities (`arity p` speaks); a trivial one (value among the keys) is no hole (v3.2.7); identical lines are one declaration (Q33) | |

- **Order** (A6, H1 v3.0.1): `#` by offset, then the reader's by offset, then arity (by predicate), safety (by rule, then variable), declarations (by predicate, then line bytes; G8).
- **The head** under `@mirror/dark-order` (certificate §2.4; Note §6.4): the `#` and reader holes merged by least byte offset, then the engine's in the order above. Its `dark 2` atom is `at L:C` (a reader check at column 1) or `engine <check> <subject>`; a declaration names its predicate only (71c), and a safety names `?v` with its `?` (Q34).
- **`basis`**: the unread lines, `unread <runs>` (runs of two or more as `a-b`) and `basis(nat …)` at `@song/interval`. Computed and tested (48b); the record's `basis` line is compared by the `record` kind, which is R6's (Q32).

**Light, as far as R2 can tell without a run** (H3). The verdict `unknown`, and with it a performance of one `dark 2` atom and no `hole` line (H5), needs "no light at all", which in general is the run's (R4, R5). R2 decides it only where the lines say it outright:
- **no hole**: nothing to tell; the performance carries no `dark 2` atom and no `hole` line, the record no table hole;
- **dark**: nothing left to run could give light. The eval program (the lines that read, minus H1's "left out") has no fact, every rule has a positive premise and no `not`, and no query could be a light falsehood (none is left, or a `#` hole makes `R⁺ = *`);
- **lit**: a fact is left whose predicate nothing can remove: not used with arity ≥ 2 beside a `#` hole (`D(#)`; queries counted, which only widens it), not a declaration hole's at arity ≥ 2 (G1), not declared by a well-formed, non-trivial declaration (the general rule, a paradox's group). Then the verdict is `uncertain` or `paradox`, and either way the performance carries every hole and no `dark 2` atom;
- **open**: anything else. Also every source with a `not` in a line that read beside a hole that adds (`#`, `repeat`, `arity p`, `safety`): that is `unknown shadow under W`, R5's. Taut's 48 has its `not` only in a rule left out, nothing left to run, and rings as `unknown`, so **dark** is decided first (Q31).

**What R2 judges** (check 10, composed: Note §6.4; under the hole: H4, H5, H7). The kinds are §6.2's and H7's, stratum `verdict`, beat `none 0`, distance 1:
- the record's `hole` lines, as a **multiset of hole texts** (the value before its first ` #`; the said text is never compared), against the report: one `hole` interval per text claimed or expected but not both, `about record`, each side BLAKE3 of the text (76). Lines of another type (`budget`, `capacity`, `io`) are R7's;
- the performance's `hole` lines the same way, `about performance`, each side the hole atom `hole(sym …)` split at the first two spaces (76a): expected when lit, none when dark (Taut V2);
- the performance's `dark 2` atoms against `{head}` when dark, `{}` otherwise: a `dark` interval `about performance`, paired only when exactly one is claimed-only and one derived-only (§6.5 R2), else one line per surplus atom. A `dark 255` atom is R6's.

When the light is open, R2 judges nothing. When the source is dark and the performance prints a model (a `result`, layers, derives or contexts), that is check 2's `dark-source`, whose left side isn't pinned (Q26), so R2 judges nothing there either. Intervals print sorted by stratum, then by line bytes (§6.2). **`dissonant <n>` counts the strata replayed so far**: R3–R9 may add lines to the same input (Q27).

**What the tests hold** (`echo/tests/source.rs`, `dark2.rs`, `r2.rs`; 29 new tests, 113 in echo's binary):
- every grammar rule broken, located; lines and offsets counting comments and blanks; typed facts kept with their types, and refused where they stop;
- 48's holes equal 55's record's hole texts, in order; the head is 48a's atom; basis is 48b's; 22f and 24f's dark child; 71c's declaration head; the hole atoms of 73, 74 (and their holes child 75), 77, 81, 90 and 91;
- Taut's triples agree in R2: 55 and 56 (total dark), 71, 77, 80, 81, 90 and 91 (uncertain), each answering `unknown unreplayed program`;
- **76 and 76a byte for byte:** 80's record, then 80's performance, without `declaration p functional p(3) -> 1` print exactly the vector's interval line, and its atom has the vector's address;
- the multiset (a repeated hole is an extra line; the said text never decides), the pairing of `dark`, what R2 can't tell, the sort.

**mirror's checker fixtures under R2** (the tune's answer from its `expect`, whose `dissonant <kind>…` means each kind is among the printed intervals; the fixtures record no interval lines):

| fixture | the tune | echo R2 |
|---|---|---|
| `a1-record-dark-disagrees` | dissonant hole | `dissonant 2`: `hole 61cdf14c… none about record` (`# 3:6`), `hole none f9b90e3f… about record` (`# 3:14`) |
| `a1-record-dark-in-clear` | dissonant hole | `dissonant 1`: `hole 3491ee28… none about record` (`# 1:1`) |
| `ct-a5-readable-dark` | dissonant dark | `dissonant 1`: `dark <dark 2 at 3:5> none about performance` |
| `ct-a6-unsafe-with-model` | dissonant hole verdict shadowed | `dissonant 2`: `hole none 356f962f… about record`, `hole none <hole(safety, bad, ?x)> about performance`. `verdict` and `shadowed` are R6's |
| `dark2-not-canonical` | dissonant dark | `dissonant 1`: `dark <dark 2 at 5:5> <dark 2 at 3:14> about performance` |
| `p8-one-line` | dissonant hole | `dissonant 1`: `hole none f210bb24… about record` (`# 4:6`) |
| `ct-a4-spurious-fact` | dissonant source | `unknown unreplayed program`: the `source` kind is check 1's, the program and facts addresses, R3's (the interval line format too) |
| the other 12 cheats (`ct-a1`, `a2`, `a3`, `a8`, `a9`, `a10`, `a11`, `a12`, `e1-false-budget-*`, `e4-dark-order`, `score-missing-note`) | dissonant (R4–R9's kinds) | `unknown unreplayed program` |
| the 18 that ring (`a1-record-dark-engine`, `a1-record-dark-reader`, `a8-budget-query-unknown`, `p8-swapped`, `pos-dark2-engine`, `pos-dark2-reader`, `pos-diamond`, `pos-v01`, `pos-v03`, `pos-v04`, `pos-v05`, `pos-v05-control`, `taut-20` … `taut-24`, `taut-48`) | rings | `unknown unreplayed program`: R2 agrees, and the rest is unreplayed |
| `typed-corpus-unheard` | unreadable | `unknown unreplayed program`: it reads (S4) and has no hole |

**Counted:** the 6 fixtures whose cheat lives in R2's strata are refused for their own reason; for 5 of them echo's kinds equal the tune's expected kinds exactly, and for `ct-a6` echo prints the `hole` kind the tune expects, with `verdict` and `shadowed` to come from R6. No fixture the tune rings is dissonant in echo, and echo prints no kind the tune doesn't expect. The only interval lines pinned byte for byte by mirror's data are Taut's 76 and 76a, and echo prints both exactly. Parity of whole interval lists with the tune is R10's.

**What R2 does not do, on purpose:** judge the record's `basis`, verdict word, counts or light lines (R6's `record`, `verdict`, `shadowed`); compare the header's `program` and `facts` (R3, check 1, including the engine-Dark header of §3.1.1 F); decide light that needs a run (R4, R5). R1's record reader still reads a record `dark` line, which H4 makes unreadable from v3.0; that moves with R6, which reads the record's values.

### 10.5 Questions R2 adds for Mara

- **Q24. Typed constants in pact sources.** Since C6 `pact eval` reads typed **facts**. echo reads `sym`/`nat`/`addr`/`bytes` constants in `fact` lines only, never in `rule` terms or `query` lines, and keeps a `sym` string to printable ASCII (pact-eval §1.2's byte set). Is that the bin's surface, or do rules and queries take typed constants too, and may a `sym` string hold UTF-8 (§5.0.1 item 6)?
- **Q25. "Can't continue" at a keyword or an overflow.** echo locates `fact not` at the byte after `not` (the name could have grown into `nota`), `rule r not requires q` at the space after `not`, and `nat 18446744073709551616` at the digit that overflows. Is that the pinned reading of J's "least byte offset at which the grammar cannot continue"?
- **Q26. Check 2's `dark-source` bytes.** §6.2: "the performance / the source's least Dark". Which typed constant is the performance's side (its `result`, its address, `none`)? Does it print beside `dark none <head> about performance` or instead of it? Until pinned, a Dark source printed with a model is `unknown unreplayed program`.
- **Q27. A dissonance before every stratum is replayed.** echo prints `dissonant <n>` with the intervals of the strata it has replayed (R2: `hole`, `dark`). R3–R9 may add lines for the same input, so `<n>` is a lower bound until R10, never a claim that nothing else differs. Acceptable for the overlap (S7), or should a partial dissonance say so?
- **Q28. A non-ground query.** Certificate §3.1 lists "a non-ground query" among the reader's checks, at column 1; pact-eval §1.2 makes a query's body a `ground-atom`, so `?x` fails the grammar where it stands, a `#` hole; and H1 has no hole type for it. echo follows the grammar (`query p(?x)` is `# L:9`). Which?
- **Q29. Which lines carry arity.** echo counts facts and rules (heads and bodies, `not` included) for `arity p` and for a declaration's `n`, and never queries, since M4 (a) makes a query's mismatch the reader's `arity L:1`. For `D(#)`'s "used with arity ≥ 2 in a line that read", do queries count? (echo counts them there; that only widens what R2 treats as removable, so it can only make R2 answer `open`.)
- **Q30. A declaration on a predicate only a query uses.** "Used by no line that read" (H7 rule 1): echo reads queries as giving no arity, so it's ill-formed. Right?
- **Q31. The mode, for `unknown shadow under W`.** Certificate §3.2: "a `not` anywhere means W". Taut's 48 has a `not` only in a rule left out by safety, nothing left to run, and the tune rings it as total dark. echo decides total darkness first. Is the mode the source's, or the eval program's? (It matters for a lit source whose only `not` is in a line left out.)
- **Q32. Comparing `basis`.** §6.5 R2: "compared as a value, and its addr is the score's `basis(nat …)` atom address". With one slot, is a wrong `basis` one `record` interval pairing the two atoms, or set difference's two? echo computes `basis` and leaves the comparison to R6.
- **Q33. Repeated lines.** Two identical `functional` lines: one declaration (a set, as the program fragment's "one atom per distinct record"), or two `declaration` holes? Two identical `rule` lines: a `repeat`? echo: one declaration; a repeat.
- **Q34. The safety head's atom.** `dark(nat 2, sym "engine", sym "safety", sym "s", sym "?y")`, with the `?` (§3.1: "the subject as in §3.2", which prints `?y`)? No vector has one.

**Gates at `c72ecad`:** `just check` clean; echo's 113 tests pass; `checks.coverage` 96.22% of lines (the ratchet is 94; echo 99.29%, 1971 of 1985; `source.rs` 288 of 288, `dark2.rs` 451 of 452, `interval.rs` 62 of 62); `checks.echo-independent` (R2 adds no dependency); `checks.echo-portable` (wasm32, thumbv7em). Seven mutations, each killed and reverted (§8). No rustc abort under `-j 4`.

### 10.6 R3, as built: P, F and the interval

`72d1b88` 🔴 → `953b133` 🟢. The `implements` line moves to `spec 3d00a4ecea49d9908c7c03d6973e6484f03075da` (version 4) and `vectors 5bead5f96f6497e5e2d7a1bba62a4f33973ff18c` (`docs/spec/echo-vectors` at the red commit, which added Taut's 68). R3 retires the word `program`: what R2 and R3 agree with, or can't tell without a run, answers `unknown unreplayed model`. Nothing rings.

**P and F** (`echo::program`; certificate §3.1 header, §3.1.1 A, B, G):
- **The canonical line** is the whole record re-printed from its parse: the keyword, one space, the body with single spaces, `, ` between arguments, literals and key positions, `not ` before a negative literal, zero-ary atoms bare, ` -> `, variables with their `?`. However a line was spaced, its canonical line is one.
- **P** is `fragment(τ @pact/program root, ℓ "program", sorted)` over one atom `<keyword>(sym "<canonical line>")` at `@pact/program` per distinct `rule`, `functional` or `query` record that read.
- **F** is `fragment(τ @pact/fact root, ℓ "facts", sorted)` over the distinct fact atoms that read, typed constants keeping their types (S4: `typed-corpus-unheard`'s F is its certificate's claim).
- **Which lines** (Note §6.7 H5): every line that read, left out by H1 or not (an unsafe rule, a repeated id, a declaration hole all stay in P); lines that don't read are in neither (95a, 97a). Repeats collapse: the set is of distinct records (§3.1 "one atom per distinct record"; fixpoint §5.0.1 item 2).

**Which header the source calls for** (H5; §3.1.1 F):

| the source (R2's light) | `program` / `facts` |
|---|---|
| no hole; lit; or total dark after an **engine** fault only | P / F |
| total dark (`unknown`) with a line that doesn't read or a **reader** fault (`#`, `repeat`, `arity L:1`) | `none` / `none` |
| a reader hole beside light R2 can't tell | R3 can't tell: `unknown` turns on the run (R4), or on the positive core for W beside a hole that adds (a certain paradox, H3 v3.3; Taut's 95, 97). Check 1 waits |

**Check 1** (certificate §4.1): each header field against echo's own, `program` then `facts`:

```text
interval source source <claimed> <recomputed> about none at none 0 by 1
```

`none` prints on whichever side has no address (§3.2: "a `none` where the checker computes an address fails check 1"; and an address claimed for a reader-Dark source is a claim of a P that isn't there).

**The interval, across slices** (Note §6.2): R2's `verdict` lines and R3's `source` lines are one list, sorted by stratum in its enumeration's order (`source`, `structure`, `verdict`, then `grammar`, `reader`, `engine`, then `piece`; not by name: `grammar` before `engine`), then by the printed line's bytes. `dissonant <n>` counts every line of every stratum replayed so far (Q27).

**What R3 can't tell, and says so:**
- a reader hole beside open light: `unknown unreplayed model`;
- check 2 (a model printed for a Dark source): its interval's bytes are open (Q26), so where check 1 agrees the answer is `unknown unreplayed source`, the stratum check 2 belongs to. Where check 1 differs, the `source` lines print and the answer is `dissonant` (Q27).

**What the tests hold** (`echo/tests/r3.rs`, 20 tests; 133 in echo's binary):
- **P** from Taut's 16 `NNa-program` vectors: each source noted in the vector is read by echo, every canonical line echo re-prints equals the vector's line set, each line's atom has the vector's address, and P has its digest;
- **F** from Taut's 16 `NNb-facts` vectors over the same sources (23b and 35b: the empty facts);
- **the header** from Taut's 17 `NNg-header` vectors: `program` and `facts` are what echo expects (22: engine Dark, P and F; 24: reader Dark, `none`), and the header atom with echo's P and F has the vector's digest. 95 and 97 are the two R3 can't decide (above); their vectors are the paradox's P and F, and echo's P and F equal them;
- **every certificate's claims**: the 38 fixtures, Taut's 20–24, 55, 71, 77, 80, 81, 90, 91 and 68, 51 in all, every one decided. All claim echo's P and F except five: `ct-a4` (another F) and four that claim `none` (below);
- the canonical line, spacing and typed constants; P and F as sets of the lines that read, queries and declarations in P only; the expected header by light; check 1's sides;
- `ct-a4` byte for byte; a reader-Dark header claiming P; an engine-Dark header claiming `none`; check 1 waiting; check 2 open;
- 45's `structure depth` line and atom, 46's other `τ`, 68's `stray` line and atom, byte for byte; the order by stratum enumeration; R2's and R3's lines composed in one list (ct-a4 plus a record hole: `dissonant 2`, `source` first);
- vector 68's triple: P and F agree, `unknown unreplayed model`, and its `heard` line is the vector's `line2` byte for byte.

**mirror's checker fixtures under R3** (the tune's answer from its `expect`, a **lower bound**: "each kind must appear among the printed intervals", the tune's test docstring):

| fixture | the tune | echo R3 |
|---|---|---|
| `ct-a4-spurious-fact` | dissonant source | `dissonant 1`: `interval source source cdcde6e9… a9eea834… about none at none 0 by 1` (claimed F with `edge(a, z)`; echo's F is 20b's) |
| `ct-a5-readable-dark` | dissonant dark | `dissonant 3`: two `source` lines (`none` claimed; echo's P and F are 20a's and 20b's), then R2's `dark` line |
| `ct-a12-budget-stop` | dissonant budget verdict | `dissonant 2`: `source none <P>`, `source none <F>` (`budget`, `verdict`: R7, R6) |
| `e1-false-budget-clear` | dissonant budget | `dissonant 2`: the same two `source` lines (`budget`: R7) |
| `e1-false-budget-opaque` | dissonant budget verdict | `dissonant 2`: the same (R7, R6) |
| `a1-record-dark-disagrees`, `a1-record-dark-in-clear`, `ct-a6-unsafe-with-model`, `dark2-not-canonical`, `p8-one-line` | dissonant hole / dark | R2's lines, unchanged (§10.4); check 1 agrees |
| the other 10 cheats (`ct-a1`, `a2`, `a3`, `a8`, `a9`, `a10`, `a11`, `e4-dark-order`, `score-missing-note`; `typed-corpus-unheard`, which the tune can't read) | dissonant (R4–R9's kinds) / unreadable | `unknown unreplayed model` |
| the 18 that ring | rings | `unknown unreplayed model`: R2 and R3 agree, the rest is unreplayed |

**Counted:**
- **Refused for its own reason:** `ct-a4` is now `dissonant source`, the tune's exact kind. With R2's six, **7 of the 38 fixtures** are refused for the reason the tune names. For 5 of them echo's kinds are exactly the tune's listed kinds (`ct-a4`, both `a1-record-dark-*`, `dark2-not-canonical`, `p8-one-line`); `ct-a5` prints the tune's `dark` and also `source` (Q35); `ct-a6` prints the tune's `hole` and still lacks R6's `verdict` and `shadowed`.
- **Beyond the tune's listed kinds:** echo prints `source` on 4 fixtures whose cheat is elsewhere (`ct-a5`, `ct-a12`, `e1-false-budget-clear`, `e1-false-budget-opaque`). Each is hand-written with `program none` / `facts none` for a source that calls for P and F. §3.2 makes that a check 1 failure; the `expect` lists don't say whether the tune prints it (Q35). No fixture the tune rings is dissonant in echo.
- **Byte for byte, where mirror's data pins the bytes:** ct-a4's `source` line is not pinned by any vector (the fixture records no interval lines); its two addresses are pinned by the fixture (claimed) and 20b (recomputed), and the line's format by §6.2 and the `verdict`-stratum lines 76/76a match. The interval lines R3 checks byte for byte against vectors are 45's text and atom, 68's `stray` text and atom, 76 and 76a (R2's), and 68's `heard` line: **5 lines and 3 atoms**, all exact. P, F and the header match **49 of 49** vector digests (16 + 16 + 17).

**What R3 does not do, on purpose:** judge `result` (the model is R4's, the certificate's address R8's); decide the header where it turns on the run; judge check 2's own interval (Q26); 46a's rename (`distance` at `@song/distance`) is a digest P2 already reproduces from its preimage, and no R3 interval uses it.

### 10.7 Questions R3 adds for Mara

- **Q35. A `none` header claimed where the source calls for P and F.** §3.2: "a `none` where the checker computes an address fails check 1". Four hand-written fixtures (`ct-a5-readable-dark`, `ct-a12-budget-stop`, `e1-false-budget-clear`, `e1-false-budget-opaque`) claim `program none` and `facts none` for sources that read and aren't `unknown`, and their `expect` lists only the kinds of their cheat. echo prints the two `source` lines §3.2 calls for. Does the tune print them too (the `expect` is a lower bound, so the fixtures can't say)? If it doesn't, S7's overlap will show four located commas, and either the tune or the fixtures move. If `none` is meant as "not claimed", §3.2 needs that sentence.
- **Q36. `program` and `facts` for a budget stop.** S1 says what a budget-stopped certificate's `result` is (the stopped set); nothing says `program` and `facts` differ from an unbudgeted run's. echo expects P and F. Right?
- **Q37. The header of a W source beside a hole that adds.** H5: an `unknown` performance keeps the pre-hole header (`none` after a reader fault); H3 v3.3: such a source is `unknown` (the C3 / K3 case) unless a certain paradox exists in the positive core. So the header turns on a run, and echo's R3 waits (Taut's 95 and 97 are the paradox case, P and F). Confirm that an `unknown shadow under W` source's certificate has `program none` and `facts none` after a reader fault, as every other `unknown` one does.
- **Q38. Repeated records in P and F.** §3.1: "one atom per distinct record"; fixpoint §5.0.1 item 2: the reader collapses repeats when it forms the set. echo collapses both a repeated rule line (which is also a `repeat` hole) and a repeated fact. Identical in bytes, or identical in canonical line (`rule  r p  requires q` and `rule r p requires q`)? echo: canonical line.
- **Q39. A canonical line's constants.** Rules and queries read only plain `sym` names (Q24), so their canonical lines print names bare. If Q24 opens rules and queries to typed constants, does the canonical line print them in their canonical text (`p(nat 7)`), as echo's printer already does, or must a plain `sym` then print typed too (`sym "a"`), which would move every program address?

**Gates at `953b133`:** `just check` clean; echo's 133 tests pass; `checks.coverage` 96.25% of lines (the ratchet is 94; echo 99.35%, 2139 of 2153; `program.rs` 133 of 133, `replay.rs` 146 of 146, `interval.rs` 74 of 74); `checks.doctest`; `checks.echo-independent` (R3 adds no dependency); `checks.echo-portable` (wasm32, thumbv7em). Six mutations, each killed and reverted (§8). No rustc abort under `-j 4`.

### 10.8 R4, as built: the positive run

`575d55b` 🔴 → `a2bddbb` 🟢. The `implements` line moves to `spec fa96f40b3436a71490173fb99a513c03cb7199a7` (version 5) and `vectors 49698ad6c68df77eefedd4e19336e2675c0e9ae0` (`docs/spec/echo-vectors` at the red commit, which added Taut's Notes 38, 41 and 44a). R4 retires the word `model`: what R2–R4 agree with, or what needs R5 or later, answers `unknown unreplayed contexts`. Nothing rings.

**The evaluation** (`echo::model`), from the definitions, never from the certificate:
- **The program it runs** is H1's eval program: the lines that read, minus what H1's table leaves out (`echo::dark2::Report::eval`, the indices of what's left). A positive source (no `not` in a rule that read; §3.2 "a `not` anywhere means W", Q31) runs its whole eval program. A W source runs only its **positive core** (H3 v3.3: "the least model, from the eval program's facts, of its rules with no `not` literal"), and only where the header turns on it (below). Everything else of W is R5's.
- **The least model, by naive evaluation** (certificate §1: "a tick is also the naive stage `min{n : a ∈ T_Pⁿ(F)}`"). Stage 0 is the eval program's facts. Round `n` forms every ground instance of every rule whose positive premises are all in stage `n − 1` (premises matched in body order, each against the stage's atoms of its predicate and arity, typed constants compared by type and bytes, S4), and stage `n` adds their heads. The run ends at the first round that adds nothing: that round is check 5's "one naive round over M adds nothing".
- **Depths** (§1's pinned conventions): an atom's depth is the round that first adds it; facts are 0. That is `1 + max` over its first-tick derivations' premises, the longest-path depth.
- **The first-tick derivations** (§2.1, "all first-tick ones"): the instances formed in the round that first adds their head, exactly those whose positive premises are all at depths below the head's. A head that is a fact has none (Note §6.5 G5: M3's query excludes it).
- **Determinism:** every map and set is a `BTreeMap` / `BTreeSet`; a test refuses `HashMap`, `HashSet`, `hashbrown` and `RandomState` anywhere in `echo/src`, and every fixture's answer is the same bytes twice.

**The budget, its real charge point** (certificate §3.2 UNKNOWN: "ground rule instances considered"; §4.3). Every ground instance echo forms is one unit, in every round, the closure round included; partial matches of a body are not instances and are not charged. Naive evaluation re-forms every instance each round, so vector 01 costs 4 + 8 + 13 + 13 = **38**. R1's provisional charge (one per printed `derive` line) is gone: the charge is the source's, whatever the certificate prints. The run happens before anything is judged, so a run that runs out is `uncertain budget <n> ground rule instances` (exit 1) whatever else differs (§10.1's precedence). Instances charged per fixture (each boundary tested: `n` finishes, `n − 1` runs out):

| instances | fixtures |
|---|---|
| 38 | `pos-v01`, `taut-20`, `ct-a4`, `ct-a5`, `ct-a8`, `ct-a10`, `ct-a11`, `e4-dark-order`, `a1-record-dark-in-clear` (vector 01's source) |
| 14 | `a8-budget-query-unknown`, `ct-a12-budget-stop` |
| 10 | `pos-diamond`, `ct-a2` |
| 9 | `e1-false-budget-clear`, `e1-false-budget-opaque` |
| 4 | `pos-v05`, `taut-21`, `ct-a3`, `score-missing-note` |
| 2 | `pos-v05-control`, `p8-swapped`, `p8-one-line` |
| 0 | `ct-a9` (no premise ever holds), `ct-a6` (its rule unsafe, left out), the Dark ones (`a1-record-dark-disagrees`, `-engine`, `-reader`, `dark2-not-canonical`, `pos-dark2-*`, `taut-22`, `taut-24`), and the W sources, which run nothing in R4 (`ct-a1`, `pos-v03`, `pos-v04`, `taut-23`, `taut-48`, `typed-corpus-unheard`) |

**Which header the run decides** (R3 left it open where a reader hole sits beside light R2 can't tell):
- a **certain paradox** in the run (H3 v3.3: some key group of an eval-program declaration, well-formed and not trivial, has two values in the stable set; for a positive program the run, for W beside a hole that adds the positive core) is `paradox` whatever the light, so P and F (H5);
- **W beside a hole that adds**, with no certain paradox in the core, is `unknown` (`unknown shadow under W`'s record), so the pre-hole header: `none` and `none` after the reader's fault (Q37);
- a positive source with no certain paradox still turns on the light (R6), and check 1 waits.

Taut's **95 and 97** are now decided: both cores hold a certain paradox (95: `p(k, a)`, `p(k, b)` under `functional p(1) -> 2`; 97: `p(k, one)`, `p(k, two)`), so P and F, their `NNg` headers. Without 95's declaration the core has none, and the header is `none`.

**What R4 judges** (checks 3, 4, 5 and 8, certificate §4.1; Note §6.2's `structure` rows; stratum `structure`, about `none`, run `tick`). Over a positive source whose performance should print its run (light no hole or lit; open only with a certain paradox) and that prints a positive one (no `under`/`over`/`context` line: check 6 is R5's; no budget `erasure`: R7's; no `dark 2` line, which §3.2 judges by checks 10 and 1, Q43):

| kind | when | left / right | beat | distance |
|---|---|---|---|---|
| `stray` | a `layer` line's atom is not in the model (checks 3, 4, 8) | the atom / `none` | its claimed tick | 1 |
| `depth` | a `layer` line's atom is in the model at another depth (check 4) | the atom / `none` | its claimed tick | `\|claimed − depth\|` |
| `unlayered` | a model atom no `layer` line names (checks 4, 5) | `none` / the atom | its depth | 1 |
| `extra` | a `derive` line whose head is in the model and whose Note is not one of echo's first-tick derivations' (or is one already matched: a list, never collapsed) (check 8) | the Note / `none` | the head's depth (Q44) | 1 |
| `missing` | a first-tick derivation no `derive` line names (check 8) | `none` / the Note | the head's depth | 1 |

Atoms are addressed at `@pact/fact` root. A derivation is named by its **Note** (Note §2.1, §2.2): `note(addr head, addr rule, addr negs, addr p₁, …)` at `@song/note` root, the rule its record atom `rule(sym "<canonical line>")` at `@pact/program`, `negs` the empty set's address for a positive rule (`fragment(@pact/fact, "negs", [])`, Taut's 41); echo's Notes equal Taut's 38 and 44a. A claimed `derive` line's rule is found by its id among the source's rules that read. A `derive` line whose head is outside the model is named once, by its layer's `stray` (§6.2: "one cause, named once"; `missing` / `extra` for heads in the model only).

**What R4 can't print bytes for, and says so** (`unknown unreplayed contexts`, never accepted; where other lines print, the answer is `dissonant`, Q27): a `derive` line whose head is neither in the model nor layered (Q40); one naming a rule id the source doesn't have, or has twice (Q41); a `layer` line repeated exactly at the right depth (Q42).

**What the tests hold** (`echo/tests/r4.rs`, 18 tests; 151 in echo's binary, R1–R3's tests moved to R4's answers where R4 now decides):
- **Taut's runs:** for the ten positive sources among his vectors (20, 21, 22, 31, 35, 82, 83, 86, 87, 89), echo's layers and derivations fragments equal his `NNd` and `NNe` digests, and its model his `NNc` (22 is an engine Dark, `result none`, with empty layers and derivations): **29 of 29 digests**;
- **models:** echo's model address equals the certificate's `result` on every correct positive fixture with one (`p8-swapped`, `pos-diamond`, `pos-v01`, `pos-v05`, `pos-v05-control`, `taut-20`, `taut-21`; `a8-budget-query-unknown` as its stopped set, depth ≤ 3, Note §6.5 S1) and on Taut's eight positive certificates (20, 21, 71, 77, 80, 81, 90, 91);
- **the Notes:** 38, 41 and 44a, from echo's own derivations;
- **the definitions:** the diamond's depths and its two first-tick derivations of `d`; a non-first-tick instance not carried; no derivation for a head that is a fact; typed constants run with their types (`nat 7` and `sym "7"` are two heads);
- **the cheats, line by line:** below; an `extra` for a valid but not first-tick instance, for a non-instance, and for a first-tick one printed twice; `depth` by its distance (5 − 1 = 4); an unlayered fact at tick 0;
- **vector 68 byte for byte:** lines 1–2 (`exact_hex`), line 4 (`tail_hex`), and the `stray` interval's atom (`24f850e1…`): echo's whole A5 output;
- **the budget:** 38 on vector 01 (37 runs out; ct-a8 too, before its dissonance), every fixture's boundary exact, a budget of 1 running out on exactly the 22 positive sources whose run forms two or more instances;
- **determinism:** two runs, identical bytes, on every fixture; no hash-ordered collection in the library;
- **95 and 97:** the positive core decides their header.

**mirror's checker fixtures under R4** (the tune's answer from its `expect`, a lower bound):

| fixture | the tune | echo R4 |
|---|---|---|
| `ct-a2-missing-derivation` | dissonant missing | `dissonant 1`: `interval structure missing none 2dad57eb… about none at tick 2 by 1` (the Note of `d` by `cd` from `c`) |
| `ct-a8-layer-off-by-one` | dissonant depth | `dissonant 1`: `interval structure depth 85e7f5f4… none about none at tick 4 by 1` (`path(a, d)`, depth 3) |
| `ct-a9-circular-pair` | dissonant stray | `dissonant 2`: `stray` for `p` (`3eef543f…`) and `q` (`44b4c4b3…`), both at tick 1 |
| `ct-a10-truncated-claims-fixed` | dissonant unlayered | `dissonant 2`: `missing none be508772… at tick 3` (`path(a, d)` by path-step from `path(a, c)` & `edge(c, d)`), `unlayered none 85e7f5f4… at tick 3` (Q44) |
| `ct-a4-spurious-fact` | dissonant source | `dissonant 3`: R3's `source` line, then `stray` for `edge(a, z)` at 0 and `path(a, z)` at 1 (Q45) |
| `ct-a5-readable-dark` | dissonant dark | `dissonant 3`, unchanged: a `dark 2` claim isn't judged by checks 3–8 (Q43) |
| `ct-a12`, `e1-false-budget-clear`, `e1-false-budget-opaque` | dissonant budget (verdict) | unchanged (`source` ×2): their `erasure` is R7's |
| `a1-record-dark-disagrees`, `a1-record-dark-in-clear`, `ct-a6-unsafe-with-model`, `dark2-not-canonical`, `p8-one-line` | dissonant hole / dark | R2's lines, unchanged; R4 agrees where it judges (`a1-record-dark-in-clear` runs vector 01: no `structure` line) |
| `ct-a1`, `ct-a3`, `ct-a11`, `e4-dark-order`, `score-missing-note` | dissonant (R5–R9's kinds) | `unknown unreplayed contexts`: R4 agrees (ct-a3's, ct-a11's, e4's and pos-v05's runs are layered and derived exactly) or can't judge (ct-a1 is W) |
| the 18 that ring | rings | `unknown unreplayed contexts`: R2–R4 agree, the rest is unreplayed (pos-v03, pos-v04, taut-23, taut-48 are W: R5's) |
| `typed-corpus-unheard` | unreadable | `unknown unreplayed contexts`: it reads (S4); its source is W |

**Counted:**
- **Refused for its own reason:** with R4's four (`ct-a2`, `ct-a8`, `ct-a9`, `ct-a10`), **11 of the 38 fixtures** are refused for the reason the tune names. For 7 of them echo's kinds are exactly the tune's listed kinds (`ct-a2`, `ct-a8`, `ct-a9`, both `a1-record-dark-*`, `dark2-not-canonical`, `p8-one-line`). Three print the tune's kind and one more: `ct-a4` (`source`, and `stray`: Q45), `ct-a10` (`unlayered`, and `missing`: Q44), `ct-a5` (`dark`, and `source`: Q35). `ct-a6` prints the tune's `hole` and still lacks R6's `verdict` and `shadowed`.
- **Beyond the tune's listed kinds:** `source` on four fixtures (Q35), `stray` on ct-a4 (Q45), `missing` on ct-a10 (Q44). No fixture the tune rings is dissonant in echo.
- **Byte for byte, where mirror's data pins it:** vector 68's whole output (three lines and the interval atom); 29 run digests (`NNc`, `NNd`, `NNe`); 3 Note digests; the model address on 16 certificates. The four cheats' interval lines aren't pinned by any vector (the fixtures record kinds only); their format is 45's and 68's, and their atoms are Taut's encoder's.

**Corrections to §10's table:** `pos-v03` is a W source (`rule send … not reader_tracking(?a)`), so it is R5's data, not R4's. `pos-v05` is positive (`functional owner(1) -> 2` over a positive rule), so R4 judges its run.

**What R4 does not do, on purpose:** judge `result` against the model (check 9's `address`, R8; the model address is tested, not printed); judge a budget-stopped run (R7: E1's true or false claim decides what should be layered); decide a positive source's light where no paradox is certain (R6); anything under W but the positive core (R5).

### 10.9 Questions R4 adds for Mara

- **Q40. A derivation of an atom outside the model and unlayered.** §6.2's `stray` is "a claimed atom not in the recomputed model", beat "its claimed tick". A `derive` line whose head no `layer` line names has no claimed tick. Is it a `stray` (at which beat?), an `extra` (the row says heads in the model only), or unreadable? echo prints nothing and answers `unknown unreplayed contexts`.
- **Q41. A `derive` line's rule id the source doesn't have, or has twice.** Its Note's `rule` is the record atom's address (Note §2.1), and there is no record, or two. Which bytes does `extra`'s left side take? echo: unreplayed.
- **Q42. A `layer` line repeated exactly.** §3.2: "a repeated line in a set is a `fail`". At the right depth, no §6.2 kind names it (`depth` would be distance 0, never printed). Which kind, or is it the reader's (`unknown unreadable certificate:<L>:1`)? echo: unreplayed.
- **Q43. Checks 3–8 on a `dark 2` claim.** §3.2: "A certificate claiming Dark at 2 is judged by check 10 … and by check 1". echo reads that as the list, so a `dark 2` performance for a source with light (ct-a5) is not also judged by the structure checks, which would print every model atom `unlayered` and every derivation `missing` (23 lines on ct-a5). Right?
- **Q44. One cause, two lines: `unlayered` and `missing`.** ct-a10 omits `path(a, d)` and its derivation. echo prints both `unlayered` (the atom) and `missing` (its Note), because `missing` is for heads in the model and `path(a, d)` is. Does the tune name it once? And `missing` / `extra`'s beat, "the head's tick": the head's depth (echo) or its claimed tick, when the two differ?
- **Q45. ct-a4's strays.** Its spurious `edge(a, z)` and `path(a, z)` are layered. echo prints `source` (F) and two `stray` lines; the `expect` lists `source` only. §6.2's `stray` row says "(CT-A4: one cause, named once)": does that mean the tune prints the strays (named once each, not again as `extra`), or that the cause is named once, by `source`, and the strays are silent?
- **Q46. The budget's unit, exactly.** echo charges each ground instance formed (every positive premise matched in the stage), every round, the closure round included, and nothing for a partial match. That is naive evaluation's count (vector 01: 38; semi-naive would charge fewer). Is the tune's "ground rule instances considered" the same function? It matters only at the edge (Q21), but there two checkers print different lines.
- **Q47. Which declarations make a paradox certain.** echo checks the eval program's well-formed, non-trivial declarations over the stable set's key groups, and decides the header with it only where R3 couldn't (a reader hole beside open light). Confirm that an ill-formed declaration (a hole) and a trivial one (value among its keys) never make one certain.

**Gates at `a2bddbb`:** `just check` clean; echo's 151 tests pass; `checks.coverage` 96.28% of lines (the ratchet is 94; echo 99.17%, 2503 of 2524; `model.rs` 329 of 336, `replay.rs` 138 of 138, `dark2.rs` 466 of 467, `interval.rs` 92 of 92); `checks.doctest`; `checks.echo-independent` (R4 adds no dependency); `checks.echo-portable` (wasm32, thumbv7em). Five mutations, each killed and reverted (§8). No rustc abort under `-j 4`.

## 11. The witness (`prism echo --witness`)

*prism-025 Addendum item 3 and Response §5 item 3; Mara's witness grade (mirror `mara/witness-navigation-grade` 052e5a13, `docs/math/2026-11-24-mara-witness-navigation-event-sourced.md` §1.1, §1.2), which adds coverage to Seam's SF §5 (`seam/floor-shape-attack` 8e8f2610, `docs/audits/2026-11-22-seam-floor-shape-attack.md` §5); mirror-029 §6: the witness sits beside the roll, never in its address, and is checked as sufficient and minimal. The record's format is **provisional**: mirror-text lines in echo's style, until Mara or mirror pins one.*

### 11.1 The definition, as read

**Witnessed = address + key + the reader's pin + an independent re-derivation + coverage** (SF §5's four clauses; Mara §1.1 adds the fifth).
- **The key** (SF §5, "Sufficiency", item 1): "An address agreed on by two readers says nothing unless both derived it from the same **(rules, bytes, scope)**." The key is that triple: what the address was derived from.
- **The pin** (SF §5 item 2): "the reader's identity and pin. Independence that can't be named can't be graded."
- **Coverage** (Mara §1.1): "the reader's verdict ranges only over the bytes it read, and everything else is `unreadable`." *A witness is a computation that can say no about every byte it vouches for.*
- **The answers** (Mara §1.2): {accept, refuse, not now}, the witness's, **never fused** with the roll's {true, maybe, false}. The third answer is what keeps a refusal sound when there are inputs the reader can't decide (PROVED there). The "not now"s are **one answer with several reasons** (coverage, a budget stop, a capacity hole), each naming what would settle it.

### 11.2 echo's pieces

| clause | echo's | from |
|---|---|---|
| address | `address certificate <addr \| none>`: plain BLAKE3 of the certificate's raw bytes, the `heard` slot. Not the certificate fragment's address under `H` (check 9 is R8's, unreplayed; Q48) | §10.1 `heard` |
| key | `key program <P> facts <F> scope none`: echo's own P and F (§10.6) over the source's lines that read, when echo read the source; `none` when it didn't. The rules are P, the bytes F; echo's inputs carry no scope (Q49) | §10.6 |
| pin | `pin spec <commit> vectors <tree> rig none`: the `implements` line's content, word for word | §10.1 `implements` |
| re-derivation | one `derived <stratum> <outcome>` line per stratum echo replayed on this input, in §10's order: `source` (R2), `program` (R3), `model` (R4) | §10.4–§10.8 |
| coverage | `covered` / `uncovered` lines for every input and every stratum, then `uncovered rest` | below |

**The outcome of a stratum** (what its checks did on this input):
- `agreed`: its checks ran and found nothing;
- `dissonant <n>`: its checks printed `<n>` interval lines (of the verdict's list);
- `waits`: its checks turn on a later stratum, or it has nothing to judge until one runs (R2's light that needs a run; R3's header that turns on that light; R4 on a Dark, unlit or W source, or a performance with W, budget or `dark 2` lines);
- `open`: its check's bytes are open in the spec (R2's check 2, Q26; R4's Q40–Q42); `dissonant <n> open` when R4 printed lines and also met an open case.

**Coverage.** echo vouches for a byte only if a stratum it replayed read it and decided:
- `covered input <input> <addr>`: an input echo read whole (the source by R2; the certificate and the record by R1's readers), by its `heard` address;
- `uncovered input <input> <addr | none> <why>`: `absent` (no score given), `io` (couldn't be read), `unreadable <L>:<C>` (refused at that byte), `stopped` (not read, because an earlier answer ended the run), `unread` (hashed for `heard`, never read: the score until R9);
- `covered stratum <stratum>`: replayed, with outcome `agreed` or `dissonant <n>`;
- `uncovered stratum <stratum> <why>`: `waits` or `open` (replayed, undecided), `stopped` (not reached: `io`, `unreadable` or the budget answered first), `unreplayed` (no method yet: `contexts` … `score`, R5–R9);
- `uncovered rest`: everything else, explicitly not vouched for. Inside a covered input, a covered stratum vouches only for what its own checks read (R1 reads the record's line form, R6 its values).

### 11.3 The answer

| echo's verdict | the witness's answer |
|---|---|
| `rings` | `answer accept` |
| `dissonant <n>` | `answer refuse` |
| `uncertain budget <n> ground rule instances` | `answer not now budget <n> ground rule instances` |
| `unknown io <input> …` | `answer not now io <input> …` |
| `unknown unreadable <input>:<L>:<C>` | `answer not now unreadable <input>:<L>:<C>` |
| `unknown unreplayed <stratum>` | `answer not now unreplayed <stratum>` |

The reason is the verdict's own words, so `uncertain budget` (exit 1) and `unknown` (exit 2) stay distinct inside one answer. **The floor:** `accept` comes only from `rings` *and* every stratum covered; a `rings` with an uncovered stratum would be `answer not now uncovered`. echo never accepts what it didn't replay.

**Two types.** In code the answer is `echo::witness::Response { Accept, Refuse, NotNow(Reason) }` and the roll's value is `echo::witness::Roll { True, Maybe, False }`. Neither converts to the other; a `compile_fail` doctest shows a `Response` can't stand where a `Roll` is expected, beside a control that compiles.

### 11.4 The mode, and why it comes after a seam

`prism echo --witness <source> <certificate> <record> [<score>]` prints:

```text
<the parity lines, byte for byte as without the flag>
---
witness 1
answer <accept | refuse | not now <reason>>
address certificate <addr | none>
key program <addr | none> facts <addr | none> scope none
pin spec <commit> vectors <tree> rig none
derived <stratum> <outcome>                     (one per stratum replayed)
covered input <input> <addr>                    (in input order)
uncovered input <input> <addr | none> <why>
covered stratum <stratum>                       (in §10's order)
uncovered stratum <stratum> <why>
uncovered rest
```

Every line ends in LF; the exit is the verdict's (§10.1), unchanged.

- **After a seam, not instead.** The parity lines are pinned for byte parity with the tune (prism-022 S2; Note §6.3; the contract delta: "`heard` and `implements` stay the second and third lines in every case"). The witness can't be a line in that surface. `---` is the seam of mirror's reflection (§3, P3): `a ‖ "---\n" ‖ r`, and "the part above the seam is never read" as a reflection. So `a` is the parity surface, unchanged, and `r` the witness: a consumer comparing with the tune cuts at the first `---` and compares `a` byte for byte. No parity line is ever exactly `---`. Printing the record *instead* would make the witness's `heard` and `pin` a restatement; printing it after keeps one source for both.
- **Without the flag**, the output and the exit are byte-identical to before this section: a guard pins every fixture's output.
- **Beside the roll.** The record is computed from the answer and a trace of the replay, after the verdict; nothing in it enters the verdict, an interval, `heard`, a recomputed address, or the certificate's address.
- **The flag selects the output, never the budget**, which stays the pinned constant (§10.1). It is the second word only (`prism echo --witness …`). The usage line stays the contract delta's, byte for byte (Q51).
- **No new dependency**, no serde, no JSON: `checks.echo-independent` unchanged. The library stays `no_std` (`echo::witness`).

### 11.5 Questions the witness adds for Mara

- **Q48. The address.** SF §5's address is the one two readers agree on. echo's is the certificate's raw bytes (BLAKE3), because the certificate's own address under `H` (check 9) is R8's. Should the witness name the roll's `result`, the certificate fragment, or the bytes? Until R8 echo names the bytes.
- **Q49. The key's scope.** echo's inputs carry no scope, so it prints `scope none`. Is the scope a fourth input echo should read, or a claim it should leave to the record?
- **Q50. Coverage inside an input.** echo's coverage is per input and per stratum. A covered certificate with an uncovered `contexts` stratum vouches for its `layer`/`derive` lines and not its `context` lines. Should coverage name lines (or `L:C` ranges), as `settle.py`'s C1 says `unreadable` per line?
- **Q51. The usage line.** The contract delta pins `usage: prism echo <source> <certificate> <record> [<score>]`. echo keeps it and doesn't advertise `--witness`. Should usage name the flag, or is the witness mode a separate entry (the MCP tool's structured result)?
- **Q52. Minimality.** mirror-029 §6 checks a witness as sufficient and minimal. echo's coverage is sufficient for its answer by construction; is a witness that covers more strata than its refusal needs (one `dissonant` stratum among three `agreed`) still minimal, or should a refusal name only the strata whose lines it prints?

### 11.6 As built

`19a3ac0` 🔴 → this 🟢. `echo::witness` (`Roll`, `Reason`, `Response`, `respond`, `witness`, `Witness::print`); `echo::replay::traced` returns the answer and a `Trace` (`Reading` per input, `Outcome` per replayed stratum, P and F), and `replay` is `traced(…).0`, so the verdict path is the one it was. `implements` doesn't move: the parity surface is byte for byte R4's, and the pin moves at the next slice.

**What the tests hold** (`echo/tests/witness.rs`, 12 tests; 163 in echo's binary, plus the `compile_fail` doctest and its control):
- **parity:** all 38 fixtures without the flag, stdout and exit, against BLAKE3 digests taken from the binary at `982d945`; with the flag, the same bytes first, then `---\nwitness 1\n`, the same exit; no parity line is `---`;
- **beside the roll:** `witness()`'s answer equals `replay()`'s on every fixture; the `heard` certificate slot and the record's `address` are the file's BLAKE3;
- **the answer:** `respond` over every verdict form (`rings` accepts only with every stratum covered); 14 fixtures `refuse`, 24 `not now unreplayed contexts`, none `accept`;
- **coverage:** per fixture, the inputs read by their digests, the score `unread` or `absent`, the decided strata covered, `waits` / `open` strata uncovered, `contexts` … `score` `unreplayed`, and the dissonant counts summing to the verdict's `<n>`; pos-v01 (R2–R4 agree) and pos-v04 (W: `model waits`) are `not now` with `contexts` uncovered; `io` stops everything, an unreadable certificate is located and stops the rest, a spent budget reads every input and decides no stratum;
- the record's line order, the pin as `implements`'s content, the key as pos-v01's claimed `program` and `facts`; `--witness` as the second word only, usage unchanged.

**Not reached by any fixture:** `open` (no fixture meets Q26 or Q40–Q42 under the flag) and the `covered` conjunction behind `accept` (short-circuited while `REPLAYED < 8`).

**Gates:** `just check` clean; `checks.coverage` 96.59% of lines (the ratchet is 94; echo 98.85%, 2668 of 2699; `witness.rs` 116 of 126, `replay.rs` 178 of 178); `checks.doctest`; `checks.echo-independent` (no new dependency); `checks.echo-portable` (wasm32, thumbv7em); the no-features workspace compile. Four mutations, each killed and reverted (§8).

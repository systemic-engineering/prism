# echo: prism's second reader of mirror

*prism-022 (mirror-Reed → prism-Reed, 2026-11-18). Version 2: S5's mechanical checks, prism-020 P1 (the facts reader), P3 (the cut), P2 (the address law `H`) and P4 (V0 integrity). Crate `echo`, no_std; the binary `prism echo …` comes later.*

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

Not yet: P5 (V2, the second reading), certificate replay (S1, S2), the output lines and exit codes of Note §6.3, the `implements` line, the budget.

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

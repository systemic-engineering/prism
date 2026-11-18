# echo: prism's second reader of mirror

*prism-022 (mirror-Reed → prism-Reed, 2026-11-18). Version 1: S5's mechanical checks, prism-020 P1 (the facts reader) and P3 (the cut). Crate `echo`, no_std; the binary `prism echo …` comes later.*

Echo can only give back the last words she was given, and she never speaks first. The checker never authors: it replays the engine's own record from the text and gives it back exactly, so where the two differ, the difference is named and located.

## 1. Scope

| Item | Here | Where it's specified |
|---|---|---|
| **P3, the cut** | `echo::cut` | mirror `docs/math/2026-11-16-mara-the-reflection-seam.md` §2.1 (amendment 1, `4f6bdc17`) |
| **P1, the facts reader** | `echo::read`, `echo::read_reflection` | the same note, A1.4; Seam's pact-eval audit §1.2 (`docs/audits/2026-10-29-seam-pact-eval-and-ruler-envelope.md`); `docs/specs/mirror-fixpoint-parser.md` §5.0, §5.0.1 items 2, 6, 7, 10, 11, §16.2.1, §16.10 item 1 |
| **S4, typed facts** | `echo::Value`: `Sym`, `Nat`, `Addr`, `Bytes`; equality by type and bytes | fixpoint §16.10 item 1; Seam's trace audit (`seam/trace-audit` `1971b5a1`) §4.2 |
| **S5, independence** | `checks.echo-independent`, `checks.echo-portable` | prism-022 S5 and Response §3; `docs/specs/mirror-certificate.md` §4.3 |

Not yet: P2 (the address law `H`, with shared vectors), P4 (V0 integrity), certificate replay (S1, S2), the output lines and exit codes of Note §6.3, the `implements` line, the budget. An atom's **content** bytes (§5.0.1 item 10) are here, because they are how the vectors check that every argument kept its type; the hash over them is P2.

## 2. Independence

**Never read mirror's engine.** echo's builders read mirror's **specs, docs and test vectors only**. They never open mirror's engine source: not `src/`, `rust/` or `crates/`, and not the tune's Python (`tests/checker/*.py`) for its logic. Vectors and fixtures are read as data. Where a spec is unclear, the question goes to Mara through mirror-Reed (§7). It is never settled by a guess, and never by looking at how mirror did it. The reason is prism-022 Response §3: echo's builders and mirror's engine builder are instances of the same model, and so they share priors. If one of them peeked at the other's code, a shared misreading of one spec sentence would go through both readers unseen. The tune's vectors, written by other authors in another language, are where such a misreading surfaces. Seam's audit (S8 iii) should check this rule.

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

**The library** is `#![no_std]` with `alloc`, `#![forbid(unsafe_code)]` and has no dependencies. It has no clock, no network, no process, no file and no sampling (certificate §4.3). `checks.echo-portable` builds it for `wasm32-unknown-unknown` and `thumbv7em-none-eabihf`, so nothing from std can reach it. No serde.

## 3. The cut (P3)

From the seam note §2.1, verbatim in effect:
- **The seam** is the first line that is exactly `---`: the bytes `2d 2d 2d 0a`, at offset 0 or right after an LF.
- With a seam, `s = a ‖ "---\n" ‖ r`. `a` is empty or ends in LF. `r` is every byte after the seam line, and may be empty.
- With no seam, `a = s` and `r` is **absent**. That is not the same as empty: `Cut::reflection` is `None` here and `Some("")` there.
- Only the first seam cuts; `r` may contain more `---` lines.

What follows from the bytes, with tests in `echo/tests/cut.rs`:
- `--- ` (a trailing space), ` ---`, `----`, `--`, `-- -` and `x---` are not seams;
- a seam after a near miss still cuts;
- `---\r\n` is not a seam (§7 Q1);
- `---` at EOF with no LF is not a seam (§7 Q2).

The seam's line number travels with `r`, so `read_reflection` locates a refusal in the whole file's coordinates. The part above the seam is never read.

## 4. The facts grammar (P1)

The seam note A1.4 fixes the content: `# <key> <value>` header lines, `fact <pred>(<arg>, …)` lines with argument kinds `addr`, `nat`, `bytes`, `sym`, `#` comments, and **nothing else**. An unknown line form is a refusal with its location, never a silent drop. `rule` and `functional` lines never appear below the line. The shapes come from Seam's frozen pact-text grammar (pact-eval §1.2); the constants from fixpoint §5.0.1 and §16.10.

```ebnf
file      = { line } ;                    (* lines end in LF; the last LF may be missing (§1.2) *)
line      = blank | hash-line | record ;
blank     = { " " } ;                     (* §1.2; §7 Q4 *)
hash-line = "#" , { printable } ;         (* column 1 only (§1.2) *)
header    = "# " , key , " " , value ;    (* a hash-line in the leading block of hash-lines (§7 Q3) *)
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
name      = lower , { lower | digit | "_" } ;   (* not a keyword: §7 Q5 *)
sp        = " " , { " " } ;  sp0 = { " " } ;
lower-hex = digit | "a".."f" ;
printable = 0x20..0x7E ;
char      = a UTF-8 scalar, not '"', '\', a byte below 0x20, or 0x7F ;   (* §7 Q7 *)
```

**Types are kept.** `Value` is one of `Sym(String)`, `Nat(u64)`, `Addr([u8; 32])`, `Bytes(Vec<u8>)`, and equality is derived over the variant and its bytes. That matches §16.10's "equality is by the §5.0 bytes", since each argument is `tag ‖ enc(bytes)` (§5.0.1 item 11) with tags `0x00` sym, `0x01` nat, `0x02` addr, `0x03` bytes. So:
- the bare `seq` equals `sym "seq"`;
- `addr abcd…` never equals the bare name `abcd…` with the same 64 hex digits, nor `sym "abcd…"`, nor `bytes "abcd…"`;
- `nat 7` never equals `sym "7"`.

The printed form is always the canonical typed text (`sym "…"`, never bare), so printing never loses a type. Seam's trace audit found that records print `addr` as bare hex. Under this grammar a digit-led bare hex doesn't read, and a letter-led one reads as a **sym**. A record can't give back a type it never printed (§7 Q8).

**Keywords.** `int` and `ratio` are reserved and refused until they are pinned (§16.10). `not` and `requires` are never names (§1.2). A type word without its value (`p(addr)`) is refused (§7 Q5).

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

Duplicate `fact` lines are kept in input order, each with its line. Forming the set is the caller's step (§5.0.1 item 2).

## 5. Vectors

- **mirror's data**, copied byte for byte into `docs/spec/echo-vectors/mirror` from `reed/ship` `52a43594` (its README names the paths and blobs):
  - the **46 `kind atom*` vectors** of `tests/checker/vectors` (Taut's §5.0 vectors). Each `atom` line reads as a `fact`, prints back to the vector's own text, and, for the 7 that carry `content`, encodes to exactly those bytes. That covers multi-byte UTF-8 (`sym "λ"`), `nat 0`, `nat 2⁶⁴−1`, `sym ""` and `addr`.
  - **`mirror.facts`**, the pinned F_G (`# grammar 9092fca1…`). It reads whole: 3 headers, 539 facts with 739 `addr`, 229 `nat` and 33 `bytes` arguments, and its canonical print reads back to the same atoms.
- **From the spec text.** mirror ships no `bytes` vector, no refusal vector and no cut vector, so these are written in `echo/tests/{cut,facts,refusals,typed}.rs`, each citing its section. That includes `bytes "2f"` and `bytes ""` as tag `0x03` with their `enc` length.

## 6. Mutations

Each of these was applied, killed by the suite, and reverted:

| Mutation | Killed by |
|---|---|
| `Value` equality by its text, without the type tag | 4 `typed::` tests (addr vs plain name, addr vs sym, addr vs bytes, nat vs sym) |
| the cut also accepts `--- ` | `cut::a_trailing_space_is_not_the_seam` |
| an unknown line form is skipped silently | `refusals::unknown_line_forms`, `refusals::reflection_refusals_are_located_in_the_file` |
| the graph guard reads direct dependencies only | `checks.echo-independent` (accepted `echo-transitive.json`): the first red, `d4ed4fa` |

## 7. Open questions for Mara

Each one is settled in code as stated, conservatively (refuse rather than guess an accept), and is marked in the tests. None of them is settled by looking at mirror's engine.

- **Q1. CRLF** (seam note §2.1). The seam is `2d 2d 2d 0a`, so `---\r\n` is not one, and a CRLF file is unreflected. Is that intended, or does the cut normalise line endings? (Below the seam, a CR is refused, pact-eval §1.2.)
- **Q2. `---` at EOF with no LF** (§2.1). The bytes say no seam. Is a final `---` an empty reflection, a hole (as fixpoint §16.2.1 makes a final comment with no LF), or nothing?
- **Q3. Headers vs comments** (A1.4; fixpoint §16.10 item 1, "the header lines are comments"). Both start with `#`. Which `#` lines are headers? echo reads: only the leading block of `#` lines, and only lines shaped `# <key> <value>`, with key `lower { lower | digit | _ | - }`. Also: the seam note §4.1's proposed header (`reflect above … grammar … laws …`, `meaning <N>`) has no `#`. A1.4's grammar refuses it, and so does echo. Which surface is current?
- **Q4. Blank lines** (A1.4 "nothing else" against pact-eval §1.2 `blank`). echo accepts lines of spaces only.
- **Q5. Type words as names** (§16.10 item 1 against pact-eval §1.2 `name`). Are `sym`, `nat`, `addr`, `bytes`, `int` and `ratio` readable as bare names (`p(addr)`)? echo refuses them.
- **Q6. Spacing inside a typed constant** (§5.0.1 item 6). echo requires exactly one space between the type word and its value, since that is the canonical text. Is `nat  7` unreadable, or the same constant?
- **Q7. Bytes inside `sym "…"`** (§5.0.1 item 6 `<utf-8>` against pact-eval §1.2's ASCII-only rule). echo accepts any UTF-8 scalar except controls below 0x20 and 0x7F, and refuses TAB and CR inside the string. It also refuses non-ASCII in comments and headers.
- **Q8. Records as typed input** (prism-022 Response §4; Seam's trace audit §4.2). Records print `addr` as bare hex. Does the record format (Note §6.x) get a type mark (`addr <hex>`), or is a record never read as typed input? Until then echo keeps the type wherever its input carries one, and cannot recover it from a record.
- **Q9. F_G order and duplicates** (§16.10 item 1: "sorted ascending by atom address bytes"; §5.0.1 item 2). echo keeps input order and duplicates. Should the reader refuse an unsorted or repeated F_G, or is that V0's (P4)?
- **Q10. First refusal or every line?** echo stops at the first unreadable byte. Note §6.7 H1 makes every unread pact line its own `# L:C` hole and reads the rest. Which applies to a reflection?
- **Q11. `nat` range** (§5.0 `u64(n)`). echo refuses a `nat` above 2^64 − 1 as unreadable. Confirm that it isn't a different kind of hole.

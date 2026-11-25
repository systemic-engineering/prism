# prism-mcp: prism's floor as a read-only MCP server

*prism, 2026-11-24 (prism-025 Response §1, §2 and §3; Alex on mirror's M1: "No serde."). The crate name `prism-mcp` is provisional; Alex names it. Code: `mcp/` (`prism_mcp::json`, `prism_mcp::server`, the binary `prism-mcp`). Tests: `mcp/tests/`. Vectors: `docs/spec/mcp-vectors/`.*

## 1. What it is

A std binary that speaks the Model Context Protocol over stdio and exposes four of prism's readers, read-only: the verifiers `echo` and `chorus`, and, on prismqueer's certified path (`docs/spec/features.md`), `check` and `measure`.

- **Read-only.** No keys, no signing, no file beyond stdin and stdout, no network, no clock, no environment-dependent behaviour. Every input travels inline in the request. It verifies and never authorizes (prism-025 edge 1): `chorus` returns a grade, never an admission.
- **Three dependencies:** `echo` and `chorus`, both workspace crates, both `no_std`; and `prismqueer = { default-features = false, features = ["exact"] }` for `check` and `measure` (no LAPACK, BLAS or Fortran; ¶6). The library (`prism_mcp`) is `no_std` with `alloc` in its own code: bytes in, bytes out. prismqueer is a std crate, so the build links std. Only the binary touches stdin and stdout.
- **What `exact` costs on the floor:** prismqueer brings a proc-macro crate (prismqueer-projections, with syn, quote and proc-macro2) and terni's macros (terni-macros), the num-* stack (num-bigint, num-rational, num-integer, num-traits), sha2 and hex. All are pure Rust; none is a numeric native library. The tree is in `nix/fixtures/mcp-merged.tree`.
- **No serde, no JSON crate of any kind, never mirror's parser.** The JSON reader and printer are hand-written in `prism_mcp::json` (§3), from RFC 8259 and RFC 7493's text.
- **Never `f64`** (contract rule 7). A source guard scans `mcp/src` for `f32` and `f64`.

## 2. The MCP revision

**2025-06-18**, read from its published text at modelcontextprotocol.io (`/specification/2025-06-18/`: `basic`, `basic/lifecycle`, `basic/transports`, `basic/utilities/ping`, `server/tools`, `changelog`), and JSON-RPC 2.0 from jsonrpc.org/specification. No SDK or library was read.

The site names **2026-07-28** as the current revision. That revision drops the `initialize` handshake (versions travel in each request's `_meta`; `server/discover`) and keeps 2025-11-25 and earlier as the "handshake-based" line. The brief asks for the handshake, so this server implements 2025-06-18, the revision the brief names. Moving to 2026-07-28 is a question for Alex (§9).

What the server takes from the text:

- **Transport, stdio:** "Messages are delimited by newlines, and MUST NOT contain embedded newlines." "The server MUST NOT write anything to its stdout that is not a valid MCP message." Messages are UTF-8. The server writes nothing to stderr.
- **Shutdown:** the client closes stdin; the server exits 0 at end of input.
- **Messages (basic):** a request has `jsonrpc: "2.0"`, an `id` that is a string or an integer and "MUST NOT be `null`", a `method` and optional object `params`. A notification has no `id`, and "the receiver MUST NOT send a response." A response has the request's `id` and exactly one of `result` and `error`; error codes are integers.
- **No batches:** the 2025-06-18 changelog, item 1, "Remove support for JSON-RPC batching". A top-level array is `-32600`.
- **Lifecycle:** `initialize` → the result (`protocolVersion`, `capabilities`, `serverInfo`, optional `instructions`) → the client's `notifications/initialized`. If the client asks for a version the server doesn't support, the server "MUST respond with another protocol version it supports": always `2025-06-18`.
- **Ping:** `ping` → `{}`, in any state.
- **Tools:** the `tools` capability; `tools/list` → `{tools: [...]}`; `tools/call` with `name` and `arguments` → `{content: [...], isError}`. An unknown tool or invalid arguments is a protocol error (`-32602`); a failure while running the tool is a result with `isError: true` ("Tool Execution Errors").

## 3. The wire: `prism_mcp::json`

### 3.1 The grammar

RFC 8259 (§2 to §8), narrowed by I-JSON (RFC 7493) and by an exact-number rule:

- **Values:** `null`, `true`, `false`, numbers, strings, arrays, objects (RFC 8259 §3). Any value may be the top level (§2). Whitespace is space, tab, LF and CR only (§2). Nothing may follow the value but whitespace.
- **Strings** (RFC 8259 §7, RFC 7493 §2.1):
  - an unescaped byte below 0x20 is refused;
  - escapes are `\" \\ \/ \b \f \n \r \t \uXXXX` only;
  - `\uXXXX` in D800–DBFF must be followed at once by `\uXXXX` in DC00–DFFF, and a low surrogate never stands alone;
  - the raw bytes must be strict UTF-8 (RFC 3629, RFC 8259 §8.1): no overlong form, no encoded surrogate, nothing above U+10FFFF;
  - **no noncharacter** (U+FDD0–U+FDEF, and U+xFFFE and U+xFFFF in every plane), raw or escaped (RFC 7493 §2.1).
- **A byte-order mark is refused** (RFC 8259 §8.1: "MUST NOT add a byte order mark"; a reader "MAY ignore" it; this one doesn't).
- **Objects:** member names are unique after unescaping (RFC 7493 §2.3, "MUST NOT contain duplicate names"). A second occurrence is refused at its opening quote.
- **Numbers: exact or refused** (§3.2).

### 3.2 Numbers

The RFC 8259 §6 grammar `-? (0 | [1-9][0-9]*) (. [0-9]+)? ([eE] [+-]? [0-9]+)?`, then:

- **An integer** (no fraction, no exponent) is read as an `i64`. Outside `i64` it is refused (`integer-range`), and `-0` is refused (`negative-zero`): the integer domain has one zero and one spelling for it. A leading zero (`01`, `-01`) is refused by the grammar.
- **A number with a fraction or an exponent** is kept as its exact lexeme (`Decimal`), never converted. It is printed back verbatim. Where an integer is expected (a request `id`, chorus's `inclusion.index`) it is refused as `not-integer`, at the number's first byte: `1e0` and `1.0` are not integers here, though they denote one.
- **Why `i64`:** every integer the protocol carries is a request id or a leaf index; an MCP id is "a string or integer", and chorus's indices are `u64` below 2⁶³ in every log prism keeps. I-JSON §2.2 already warns that integers beyond ±(2⁵³−1) don't interoperate. An arbitrary-precision integer would buy a range nobody sends, and its digits would need their own canonical form. A number that doesn't fit is refused, not rounded.

### 3.3 The limits

- **Size:** a message is at most **8 MiB** (`MAX_BYTES` = 8,388,608 bytes), the newline excluded. A longer line is discarded up to its newline and answered with `-32700`, refusal `size` at offset 8,388,608. the largest file under `docs/spec/echo-vectors` is 64,637 bytes; an ML-DSA-44 vkey is about 1.8 KB of base64.
- **Depth:** at most **64** nested arrays and objects (`MAX_DEPTH`). The 65th opening bracket is refused (`depth`). The reader is recursive descent, so this also bounds its stack.

### 3.4 Refusals

Every refusal is a `Refusal { kind, at }`: a typed kind and the byte offset in the message where reading stopped (0-based).

| kind | at |
|---|---|
| `size` | `MAX_BYTES` |
| `depth` | the opening bracket past the limit |
| `end` | the message length (it ended inside a value) |
| `unexpected` | the byte that can't start or continue the value here |
| `trailing` | the first non-whitespace byte after the value |
| `control` | the unescaped control byte |
| `escape` | the backslash of a bad escape |
| `surrogate` | the backslash of the unpaired `\u` |
| `utf8` | the first byte of the invalid sequence |
| `noncharacter` | the first byte of the code point (raw or escaped) |
| `duplicate` | the opening quote of the repeated name |
| `leading-zero` | the digit after the leading `0` |
| `negative-zero` | the `-` |
| `integer-range` | the number's first byte |
| `not-integer` | the number's first byte (only where an integer is expected) |

### 3.5 The printer

One line, no whitespace, deterministic:

- object members in **byte order of their UTF-8 names** (code-point order), whatever order they were read or built in;
- strings escape `"` and `\`, write `\b \f \n \r \t` for those bytes, `\u00xx` (lowercase hex) for every other byte below 0x20, and everything else as raw UTF-8;
- integers in shortest decimal; decimals as their lexeme.

So `print(read(x))` is canonical (printing it and reading it again gives the same bytes), and identical values give identical bytes. Because LF is always escaped, a printed message never contains an embedded newline (the stdio rule).

### 3.6 Vectors

- **JSONTestSuite** (nst/JSONTestSuite, commit `1ef36fa01286573e846ac449e8683f8833c5b26a`, 2024-11-22), `test_parsing/` copied byte for byte into `docs/spec/mcp-vectors/JSONTestSuite/` with its MIT licence. Every `n_` case must be refused. Every `y_` case must be accepted, except the ones I-JSON or the number rule refuses, each listed with its kind. Every `i_` case's outcome is pinned. Every accepted case round-trips canonically.
- **Hand cases** cite the RFC section they read, and pin the refusal kind and offset.

## 4. Messages

- **Framing:** stdin is split at LF. An empty line is skipped. A CR before the LF is whitespace to the reader. A last line with no LF is read at end of input.
- **A line that doesn't read:** `-32700 Parse error`, `id: null`, `data: {at, refusal}`.
- **Not a request:** `-32600 Invalid Request` for a top-level non-object (an array included: no batches), a missing or wrong `jsonrpc`, an `id` that is `null`, a boolean, a decimal or a structure, a missing or non-string `method`, or a member other than `jsonrpc`, `id`, `method` and `params`. The `id` is echoed when it reads as an id, else `null`.
- **A response from the client** (`result` or `error`, no `method`) is ignored: the server sends no requests.
- **A notification** gets no answer, whatever its method. `notifications/initialized` moves the session to ready; every other notification is ignored.
- **`params`**, when present, must be an object (`-32602` otherwise).
- **Methods:** `initialize`, `ping`, `tools/list`, `tools/call`. Anything else is `-32601 Method not found`.
- **State:** before `initialize`, only `initialize` and `ping` are served; others get `-32000 Server not initialized`. A second `initialize` gets `-32001 Already initialized`. `tools/*` are served once `initialize` has been answered (the client "SHOULD NOT" send them earlier; the server doesn't wait for `initialized` to answer them).
- **`initialize`** needs `protocolVersion` (a string), `capabilities` (an object) and `clientInfo` (an object with string `name` and `version`); otherwise `-32602`. The result is `{capabilities: {tools: {}}, instructions, protocolVersion: "2025-06-18", serverInfo: {name: "prism-mcp", version}}`. No `listChanged`: the list never changes.
- **`tools/list`** takes no cursor (a `cursor` is `-32602`: none was ever issued).
- **The server never panics on input.** Every path out of `handle` is a value; a seeded property feeds it random bytes and mutated messages.

## 5. The tools

Inputs are JSON strings (text), canonical base64 (bytes), JSON integers, or exact rationals as `"p/q"` strings (`check`, `measure`), never paths and never decimals. Unknown arguments, missing required ones and wrong types are `-32602`, with `data: {argument, expected}` (`expected: "none"` for an unknown argument; a nested one is named `inclusion.<name>`). An unknown tool is `-32602` with the message `Unknown tool: <name>` and `data: {tool}`. A non-integer `inclusion.index` adds the reader's `refusal` and `at` to the data.

### 5.1 `echo`

- **Arguments:** `source`, `certificate`, `record` (strings: the files' contents), optional `score` (string), optional `witness` (boolean).
- **What it runs:** the echo library: `replay::replay` with `replay::BUDGET`, or `witness::witness` when `witness` is true. It never spawns the binary.
- **The result:** `isError: false` always: every echo answer, `unknown` and `dissonant` included, is an answer. The text content, in order:
  1. the parity lines exactly as `prism echo` prints them (verdict, `heard`, `implements`, intervals);
  2. with `witness: true`, the record exactly as `prism echo --witness` prints it after those lines (from `---` on);
  3. `exit <code>\n`, the code the binary would exit with.

  So the texts, concatenated, are the binary's stdout followed by `exit <code>\n`: the same bytes `echo/tests/witness.rs` pins as BLAKE3 for its 38 fixtures.
- **`heard`** is echo's own: BLAKE3 of each input's bytes as received, the UTF-8 of the decoded JSON string.
- **The description travels with the tool** (Seam; prism-025 §2). It is built from `echo::replay::STRATA` and `REPLAYED`, so it can't drift from the code:

  > An independent replayer of the positive fragment (R1–R4 of `docs/spec/echo.md` §10: the surface, then the strata source, program, model), sharing blake3 1.8.7 and the spec's authorship lineage with mirror. Strata it can't replay (contexts, verdict, budget, address, score) answer `unknown`, never `rings`.

  A test holds the `1.8.7` against `echo/Cargo.toml`'s pin.

### 5.2 `chorus`

- **Arguments:** `note` (string: the signed checkpoint note), `ed25519` and `mldsa44` (strings: the log's two vkeys, C2SP `name+id+base64`), optional `inclusion` (`{index: integer ≥ 0, leaf: base64, path: [base64, …]}`, all three required).
- **What it runs:** `chorus::checkpoint::verify` under the AND rule, then, if the checkpoint verified, `chorus::merkle::verify_inclusion` of `leaf_hash(leaf)` at `index` against the checkpoint's size and root. Base64 is decoded by `chorus::note::decode_base64` (canonical, `base64ct`, chorus's own dependency); the MCP crate has no base64 of its own.
- **Subject and ruler.** The note and the inclusion proof are what's checked: when they don't hold, the answer is `reject` with `isError: false`. The vkeys are the ruler, and the leaf and path must decode: when a vkey doesn't parse or is the wrong type, or a base64 field isn't canonical, or a path entry isn't 32 bytes, there is no verdict, and the result is `isError: true` with an `error …` line.
- **The text content, one block:**

  ```
  accept | reject <why>
  heard note <b3> ed25519 <b3> mldsa44 <b3> leaf <b3|none> path <b3|none>
  checkpoint <origin> <size> <root base64>     (verified only)
  timestamp <t>                                 (verified only)
  inclusion <index> accept | reject <why> | unchecked | none
  ```

  `<why>` is `note <error>`, `body`, `extensions`, `duplicate`, `unsigned ed25519 <bool> mldsa44 <bool>`, or `inclusion`; an inclusion's own `<why>` is RFC 9162's failure as a word (`index-out-of-range`, `path-too-long`, `path-too-short`, `root-mismatch`, …). A ruler error is one line: `error ed25519 <why>`, `error mldsa44 <why>` (the vkey's parse error, or `key` for the wrong type or length), `error inclusion.leaf base64`, `error inclusion.path <n> base64` or `error inclusion.path <n> length`. `heard` is BLAKE3 (echo's `H`) of each string's UTF-8, and of the decoded leaf and of the decoded path hashes concatenated. `accept` needs the checkpoint and, if given, the inclusion. The timestamp is reported, never compared with a clock.

### 5.3 `check`

`prismqueer::agreement::check`: is a selection inside the consistent readings of exact relations among items?

- **Arguments:**
  - `ring`: `"Z"`, `"ZMod"` or `"QModOne"`; `modulus` (an integer) with `ZMod` and only with it;
  - `relations`: `[[from, to, t], …]`, s(to) − s(from) = t; items are integers ≥ 0;
  - `selection`: `[[item, reading], …]` (an item given twice keeps its last reading, as `SelectionDraft::new` does);
  - optional `items`: items beyond those the relations name;
  - `by`: 64 lowercase hex, the Oid the selection is attributed to.

  A value is a JSON integer in `Z` and `ZMod` (any integer; ZMod reads it mod n) and `"p/q"` in `QModOne` (`-?(0|[1-9][0-9]*)/[1-9][0-9]*`, within i64; reduced by the ring).
- **`by` is a label, never a signature.** `SelectionDraft::seal(by)` is the only road to a `Selection`, and the `Signer` it mints is "a signature *claim* (who, over which address), held and never checked cryptographically" (`crossing.rs`; `algedonic`: "prism checks that a signer exists, never who it is"). The signer sits beside the selection's address, never in it. So the tool takes `by` and prints it, and its description says prism holds no keys and never signs. Sealing here authorizes nothing: what a seal is worth is the receiver's policy.
- **The answer, `isError: false`.** One text block:

  ```
  in | out | needs-ruling
  ring Z | ring ZMod <n> | ring QModOne
  selection <address> by <by>
  edge <a>-<b>#<nth> required <t> found <t'>      (in: every relation, in the graph's order)
  coordinate <item> <x>                          (in: one per component, at its smallest item)
  violated <a>-<b>#<nth> required <t> found <t'>  (out: exactly the broken relations)
  cycle twist <t> vertices <v…> edges <e…>        (needs-ruling: the witness cycles)
  ```

  Edges are written smaller end first, with which parallel edge (`#0`, `#1`, …). Values are integers, ZMod residues, or `p/q` (`p` when q is 1). `out` and `needs-ruling` are verdicts about the subject, the selection and the relations' curvature; `needs-ruling` never picks a reading.
- **No verdict, `isError: true`** (the ruler is broken: there is nothing to check):
  - `error selection missing <item>` / `error selection unknown <item>`: the selection doesn't read exactly the relations' items (`agreement::Refused`);
  - `error ring <relations.k | selection.k> <integer | p/q>`: a value outside the declared ring (a `"p/q"` in ℤ or ℤ/n, an integer in ℚ/ℤ);
  - `error ring modulus <n>`: n < 2.
- **`-32602`:** a missing or unknown argument, an unknown ring, `modulus` missing for ZMod or given for another ring, a row of the wrong length, a negative or non-integer item, a decimal, a string that isn't `p/q`, `by` not 64 lowercase hex.

### 5.4 `measure`

`prismqueer::measured::measure`: a precision matrix estimated exactly over ℚ from integer counts (`docs/spec/measured.md`).

- **Arguments:** `samples` (`[[x₁, …, x_d], …]`, integers), `streams` (d, an integer ≥ 0), `estimator` (`"mle"` or `"unbiased"`), `support` (`[[i, j], …]`, the declared edges), `window` (64 hex: the window ruler's address), `reader` (64 hex: the frame's reader). All six are required.
- **The frame** is `Frame::new(reader, streams)`: prism derives a frame's address from its reader and dimension, so the tool takes those, not a frame address, and prints the address.
- **The window** is named by its address. `measure` took the `Ruler` and read only its address; the tool calls `measured::measure_under(window, …)`, which `measure(&Ruler, …)` now wraps, so both give the same `Measured` (pinned in `prismqueer/tests/measured.rs`). `Measured::seal` still takes the ruler and refuses another.
- **It measures and never seals.** No `by`, no draft, no `Metrix`: a `by` argument is `-32602` ("none"), and no line names a signer.
- **The answer, `isError: false`:**

  ```
  measured
  address <the Measured's address>
  window <window>
  frame <frame address> reader <reader> streams <d>
  method mle | unbiased support <i>-<j> … | none
  samples <n> <the samples' address>
  row <i> <K̂_i0> … <K̂_i(d−1)>
  ```

  Entries are exact: `p` or `p/q` in lowest terms, never a decimal.
- **A refusal is an answer, `isError: false`:** `refused <kind>`, then the window, frame and method lines:
  - `unresolved support <edges>`: the support isn't chordal; no closed form, never approximated;
  - `too-few-samples samples <n> needed <k>`;
  - `not-invertible clique <streams>`;
  - `wrong-shape sample <index> len <len> streams <d>` / `wrong-shape edge <from> <to>`.

  `OtherWindow` and `Draft` come only from sealing, which the tool never does.
- **`-32602`:** a missing or unknown argument, a row that isn't an array of integers (a decimal included), a negative `streams`, an unknown estimator, an edge that isn't two integers ≥ 0, a window or reader that isn't 64 lowercase hex.

## 6. Independence

`checks.prism-mcp-independent` (`nix/prism-mcp-independent.sh`) judges **prism-mcp's own build graph**. It refuses, by name:

- any mirror crate (`mirror`, `mirror-*`, `mirror_*`);
- `*serde*` (serde, serde_json, serde_core, serde_derive, miniserde, nanoserde) and `*json*` (any JSON crate);
- `*lapack*`, `*blas*`, `*fortran*`, `netlib-src`, `intel-mkl*`, `accelerate-src`;
- `prismqueer` with any feature set but exactly `exact`: `default` or `lapack` is refused, with the features named. Its closure under `exact` is held by `checks.prismqueer-exact` (no `cc` but blake3's, nothing native);

and any package whose `links` names lapack, blas or fortran.

### 6.1 Which graph: prism-mcp's resolve, not the workspace's

`cargo metadata` resolves features once for the whole workspace, dev-dependencies included. prismqueer's dev-dependency `terni = { features = ["serde"] }` turns terni's `serde` on in that union, so the union shows a normal edge prismqueer → terni → serde (and serde_core, serde_derive) under prism-mcp, though `cargo build -p prism-mcp` never builds it. Read on the union, the guard would refuse a graph that holds no serde.

So the guard reads

```
cargo tree --locked --offline -p prism-mcp -e normal,build,dev --target all --prefix none -f '{p}|{f}'
```

beside `cargo metadata --locked --offline`:

- **`cargo tree -p prism-mcp`** is cargo's resolver run for prism-mcp alone, as `cargo build` and `cargo test -p prism-mcp` run it: each package with the features *that* build turns on (`{f}`), prism-mcp's own dev edges, and never its dependencies' dev edges (chorus's `serde_json` doesn't appear; nothing below the root needs a dev policy). `--target all` keeps every platform's edges, a superset of any one build. It's stable cargo; `--locked --offline` reads the lockfile and the vendored sources only.
- **`links`** is a manifest key, the same in any resolve, so it comes from the metadata's `packages`, joined by name and version.
- **Read strictly, failing closed:** the first line must be `prism-mcp`, every line must parse as `name vVERSION[ (proc-macro)][ (source)]|features[ (*)]`, and every package must be in the metadata. An empty, truncated or reformatted tree is refused, not passed.

`checks.echo-independent` and `checks.prismqueer-exact` keep the metadata walk (`nix/independent.sh`, policies `all` and `none`), unchanged and exactly as strict.

### 6.2 The fixtures

Trees in `nix/fixtures/mcp-*.tree`, read against `mcp-packages.json` (the metadata's `packages`, with the extra names the fixtures use):

- **`mcp-merged`** passes: the real tree of prism-mcp with prismqueer under `exact`, beside its real workspace-union metadata (`mcp-merged.json`, trimmed to prism-mcp's closure). The check also runs the old reading (`independent.sh`, policy `root`, the same names) on that metadata and requires it to refuse (serde, serde_core, serde_derive): the false positive, kept as a fixture;
- refused, each for its own reason (the check matches the reason, not only the exit):
  - `mcp-lapack`: `prismqueer (features exact,lapack)`;
  - `mcp-default`: `prismqueer (features default,exact)`;
  - `mcp-terni-serde`: terni's `serde` really on, serde in the tree;
  - `mcp-serde-json`: serde_json on a normal edge under chorus;
  - `mcp-json`: a JSON crate not named serde;
  - `mcp-mirror`: a mirror crate behind echo;
  - `mcp-links`: an innocently named crate with `links = "blas"`;
  - `mcp-missing`: a package the metadata doesn't have;
  - `mcp-unparsed`: a line that doesn't parse.

prism-mcp's own dev-dependencies are in the tree, so serde as one is refused as before. (Alex allowed it as a dev-dependency, for a differential test against serde_json; that would be a deliberate change to this guard, not made here.)

**What independence means here** (Seam's finding, prism-025 §3): `echo` is independent of mirror in method, not in toolchain, spec or priors. It shares blake3 1.8.7, the spec's authorship lineage and its builders' model. The tool's description says so.

## 7. Tests

- **JSON:** the JSONTestSuite table; hand cases for a duplicate key (also after unescaping), a lone surrogate (high and low), invalid UTF-8, an overlong form, a BOM, the depth limit (64 passes, 65 refused at the 65th bracket), the size limit, leading zeros, `-0`, a fraction and an exponent where an integer is expected, an integer past `i64`, trailing garbage; each refused with its kind at its offset. The printer: key order, escapes, determinism, canonical round trip.
- **Session:** initialize → initialized → `tools/list` → `tools/call echo` on all 38 fixtures of `docs/spec/echo-vectors/mirror/fixtures`: the texts, concatenated, hash to `echo/tests/witness.rs`'s `PARITY` digests (the binary's stdout ‖ `exit N`, taken at `982d945`). With `witness: true`, they hash to the binary's `--witness` stdout ‖ `exit N`, pinned at `83ec822`. The binary itself is run on a scripted stdin once.
- **check** (`mcp/tests/check.rs`), on `prismqueer/tests/agreement.rs`'s values: the consistent triangle (0 → 1 +3, 1 → 2 +4, 0 → 2 +7) at (10, 13, 17) is `in` with coordinate 10; (10, 14, 17) is `out` on exactly 0-1#0 and 1-2#0; 3 + 4 ≠ 8 needs a ruling (twist ±1); the tritone pair needs a ruling over ℤ (twist 12, the octave) and is `in` mod 12; ℚ/ℤ in and curved; missing, unknown and outside-the-ring are `isError`; malformed is `-32602`. The addresses are the library's own.
- **measure** (`mcp/tests/measure.rs`), on `prismqueer/tests/measured.rs`'s values: (1,2), (2,2), (3,5), (4,7) under MLE give K = [[8, −4], [−4, 20/9]], unbiased [[6, −3], [−3, 5/3]]; the path A–B–C gives K̂_AC = 0; the four-cycle is `unresolved`; too-few-samples, not-invertible and wrong-shape are answers; a `by` is refused. The window is a real ruler's address, and the expected addresses are `measure` under that ruler.
- **Session:** `tools/list` names echo, chorus, check and measure; one session calls each.
- **chorus:** `docs/spec/mcp-vectors/checkpoint.json` (the AND checkpoint of `chorus/tests/checkpoint.rs`, which checks the file) accepts, with its inclusion proof; one corrupted signature byte in either line rejects; a bad vkey is `isError`.
- **Bad input never panics:** a seeded property over random byte strings and over byte mutations of valid messages, through the server's line loop; every output line reads back as a JSON-RPC response.
- **Guards:** no `f32`/`f64` in `mcp/src`; the binary's source makes no file, environment, clock, network or process call.
- **Mutations** (each makes a test fail, then is reverted): accept a duplicate key; read an exponent as an integer; drop `isError` on a tool failure; let `echo` answer `rings` where the library said `unknown`; emit a decimal for a rational; return `isError: true` for `out`; let `measure` take a `by` and print a seal; and, in the guard, accept prismqueer with `lapack`.

## 8. Not here

- `hold` and `flip_point` as tools come next (prism-025 §2 step 4).
- No `structuredContent` and no `outputSchema` yet: the text is the parity surface, and a structured form of it would be a second surface to keep in step.
- No resources, prompts, logging, sampling or cancellation. A `notifications/cancelled` is ignored like any notification: every call finishes before the next line is read.

## 9. Questions

- **For Alex:** the crate's name (`prism-mcp` is provisional). And the revision: stay on 2025-06-18's handshake, or move to 2026-07-28 (per-request `_meta` version, `server/discover`)?
- **For Alex:** the floor now carries prismqueer's proc-macro crates and num-* (§1). Worth it for `check` and `measure`, or should those two live in a second server?
- **For Mara:**
  - `check`: is a selection that misses an item a broken ruler (`isError`, as now) or a subject that fails (`out`)? And is a value outside the declared ring the ruler's fault, the subject's, or a protocol error?
  - `measure`: is naming the window by address enough (`measure_under`), or should the tool take the window ruler's draft and its signer label, so the window is a ruler and not only an address?
  - Is "subject vs ruler" the right line for `isError` (a bad note is `reject`; a bad vkey is no verdict)?
  - Should `heard` for a base64 input name the decoded bytes (as now) or the base64 text received?
  - Should `Decimal` be refused outright (so every number on the wire is an integer), rather than kept as an opaque lexeme?

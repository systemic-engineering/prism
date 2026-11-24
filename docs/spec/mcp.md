# prism-mcp: prism's floor as a read-only MCP server

*prism, 2026-11-24 (prism-025 Response §1, §2 and §3; Alex on mirror's M1: "No serde."). The crate name `prism-mcp` is provisional; Alex names it. Code: `mcp/` (`prism_mcp::json`, `prism_mcp::server`, the binary `prism-mcp`). Tests: `mcp/tests/`. Vectors: `docs/spec/mcp-vectors/`.*

## 1. What it is

A std binary that speaks the Model Context Protocol over stdio and exposes two of prism's verifiers, `echo` and `chorus`, read-only.

- **Read-only.** No keys, no signing, no file beyond stdin and stdout, no network, no clock, no environment-dependent behaviour. Every input travels inline in the request. It verifies and never authorizes (prism-025 edge 1): `chorus` returns a grade, never an admission.
- **Two dependencies:** `echo` and `chorus`, both workspace crates, both `no_std`. The library (`prism_mcp`) is `no_std` with `alloc` too: bytes in, bytes out. Only the binary touches stdin and stdout.
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

Inputs are JSON strings (text) or canonical base64 (bytes), never paths. Unknown arguments, missing required ones and wrong types are `-32602`, with `data: {argument, expected}` (`expected: "none"` for an unknown argument; a nested one is named `inclusion.<name>`). An unknown tool is `-32602` with the message `Unknown tool: <name>` and `data: {tool}`. A non-integer `inclusion.index` adds the reader's `refusal` and `at` to the data.

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

## 6. Independence

`checks.prism-mcp-independent` runs `nix/independent.sh` (the walk `nix/echo-independent.sh` had, now shared) over `cargo metadata --locked --offline` from the root `prism-mcp`. It refuses, by name:

- `prismqueer`, any mirror crate (`mirror`, `mirror-*`, `mirror_*`);
- `*serde*` (serde, serde_json, serde_core, serde_derive, miniserde, nanoserde) and `*json*` (any JSON crate);
- `*lapack*`, `*blas*`, `*fortran*`, `netlib-src`, `intel-mkl*`, `accelerate-src`;

and any package whose `links` names lapack, blas or fortran.

**One difference from echo's walk, and why.** From the root, every dependency kind counts (normal, build, dev). Below the root, dev-dependencies don't: a dependency's dev-dependencies are built only for that dependency's own tests and are never linked into a dependent. This matters because `chorus` has `serde_json` as a dev-dependency for its vector tests, and cargo's `resolve.nodes` lists a workspace member's dev edges. `checks.echo-independent` keeps every kind at every depth, exactly as strict as before: echo's wrapper calls the shared walk with the policy `all`.

The fixtures (`nix/fixtures/mcp-*.json`):

- `mcp-clean.json` passes: prism-mcp → echo, chorus; chorus → serde_json as a dev edge only;
- `mcp-serde-json.json` is refused: prism-mcp → chorus → serde_json, a normal edge;
- `mcp-root-dev.json` is refused: prism-mcp → serde_json as its own dev-dependency;
- `mcp-json.json` is refused: a JSON crate not named serde;
- `mcp-mirror.json` is refused: a mirror crate behind echo;
- `mcp-prismqueer.json` is refused: prismqueer, directly;
- `mcp-links.json` is refused: an innocently named crate with `links = "blas"`.

**What independence means here** (Seam's finding, prism-025 §3): `echo` is independent of mirror in method, not in toolchain, spec or priors. It shares blake3 1.8.7, the spec's authorship lineage and its builders' model. The tool's description says so.

## 7. Tests

- **JSON:** the JSONTestSuite table; hand cases for a duplicate key (also after unescaping), a lone surrogate (high and low), invalid UTF-8, an overlong form, a BOM, the depth limit (64 passes, 65 refused at the 65th bracket), the size limit, leading zeros, `-0`, a fraction and an exponent where an integer is expected, an integer past `i64`, trailing garbage; each refused with its kind at its offset. The printer: key order, escapes, determinism, canonical round trip.
- **Session:** initialize → initialized → `tools/list` → `tools/call echo` on all 38 fixtures of `docs/spec/echo-vectors/mirror/fixtures`: the texts, concatenated, hash to `echo/tests/witness.rs`'s `PARITY` digests (the binary's stdout ‖ `exit N`, taken at `982d945`). With `witness: true`, they hash to the binary's `--witness` stdout ‖ `exit N`, pinned at `83ec822`. The binary itself is run on a scripted stdin once.
- **chorus:** `docs/spec/mcp-vectors/checkpoint.json` (the AND checkpoint of `chorus/tests/checkpoint.rs`, which checks the file) accepts, with its inclusion proof; one corrupted signature byte in either line rejects; a bad vkey is `isError`.
- **Bad input never panics:** a seeded property over random byte strings and over byte mutations of valid messages, through the server's line loop; every output line reads back as a JSON-RPC response.
- **Guards:** no `f32`/`f64` in `mcp/src`; the binary's source makes no file, environment, clock, network or process call.
- **Mutations** (each makes a test fail, then is reverted): accept a duplicate key; read an exponent as an integer; drop `isError` on a tool failure; let `echo` answer `rings` where the library said `unknown`.

## 8. Not here

- `check` and `measure` wait for prismqueer's `exact` split (prism-025 §1); `hold` and `flip_point` as tools come after.
- No `structuredContent` and no `outputSchema` yet: the text is the parity surface, and a structured form of it would be a second surface to keep in step.
- No resources, prompts, logging, sampling or cancellation. A `notifications/cancelled` is ignored like any notification: every call finishes before the next line is read.

## 9. Questions

- **For Alex:** the crate's name (`prism-mcp` is provisional). And the revision: stay on 2025-06-18's handshake, or move to 2026-07-28 (per-request `_meta` version, `server/discover`)?
- **For Mara:**
  - Is "subject vs ruler" the right line for `isError` (a bad note is `reject`; a bad vkey is no verdict)?
  - Should `heard` for a base64 input name the decoded bytes (as now) or the base64 text received?
  - Should `Decimal` be refused outright (so every number on the wire is an integer), rather than kept as an opaque lexeme?

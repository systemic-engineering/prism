# pq — prism query, the wire-altitude algebra over `prism_core::Prism`

*2026-06-02. Reed + Alex + Mara. Status: load-bearing recognition; spec.*

`pq` is the minimal wire primitive for any MCP server whose substrate is
a `prism_core::Prism`. Three operations cross the wire: `focus`,
`project`, `refract`. Every domain-specific MCP tool an agent might
want — `commit`, `read`, `history`, `diff`, `merge`, `branch`,
`search`, `status`, `observe` — decomposes into a chain of those
three. The wire stops enumerating; the algebra of the trait does the
work.

This spec is the **wire algebra**. The companion spec
[[../../../fragmentation/docs/specs/fragmentation-mcp]] describes the
first concrete WIRE that ships pq (frgmnt's MCP). The high-level
surface [[../../../mirror/boot/std/code/mq]] compiles down to pq
chains. Three altitudes, one composition law.

---

## 0. The recognition in one sentence

**`pq` is the trait, projected onto the wire.**

```
impl Prism for FrgmntMcp { ... }   // structural identity, not metaphor

pq.focus(target)         → Beam     # observe the spectral state
pq.project(beam, filter) → Beam     # narrow by criterion
pq.refract(beam, output) → Beam     # settle / crystallize
```

Three calls. Closed under composition. Every previous frgmnt tool
chains from this surface; nothing escapes it.

---

## 1. Why pq exists

The current frgmnt MCP exposes 18 dot/underscore tools
(`fragmentation_commit`, `fragmentation_shard_open`, `fragmentation_read`,
`fragmentation_diff`, `fragmentation_merge`, `fragmentation_branch`,
`fragmentation_refs_list`, `fragmentation_refs_update`,
`fragmentation_history`, `fragmentation_search`,
`fragmentation_observe`, four `fragmentation.shard.*` sub-tools,
plus context/openers). Each is a typed entrypoint over the SAME
shard substrate. They differ only in which corner of the Prism
algebra they expose.

Two things make this wrong:

1. **The `fragmentation_` prefix is Java-style stuttering.** The
   server is already namespaced as `frgmnt` at registration. The
   prefix repeats the namespace into every method name. The MCP
   spec already namespaces tools by server; we are paying token
   cost at the agent altitude for a property the protocol already
   carries.

2. **18 tools where 3 would do.** Every previous tool is a chain of
   focus → (project*) → refract. The Prism trait IS the algebra
   that closes under composition. Exposing 18 names instead of 3
   pays the enumeration tax forever — every new capability becomes
   a new tool, a new schema, a new wire breakage, a new round of
   agent-side learning. The algebra was right there.

The alternative is recognising what the substrate already says:
`impl Prism for FrgmntMcp` is structural identity. The MCP wire
exposes the trait. The trait is closed. The tool surface stops
growing.

This follows [[feedback-no-stringly-types]] and
[[feedback-no-bare-types]] in spirit: the substrate's algebra is
better than the wire's enumeration. **The trait IS the spec.**

---

## 2. The three operations on the wire

Every pq call takes a typed `Beam` shape in and returns a `Beam`
shape out. The `Beam` is the carrier defined in
`prism_core::beam::Beam` — a functor over `Imperfect<Out, Error,
Loss>`. Every response carries the imperfection structure
directly; observation is not a separate tool.

### 2.1 `focus(target) → Beam`

Observe the spectral state. `target` is a typed selector into the
Prism's `Input` space: an OID, a path, a ref, the empty target
(meaning "the current shard"), or a structured pair (for diff/merge
shapes).

```
focus(target: Target) -> Beam<Focused>
```

Where `Target` is the typed query DSL from §6.1. Returns a `Beam<P::Focused>`
carrying:

- `result` — the focused value (the OID's content, the path's blob,
  the empty-target's shard summary).
- `loss` — the [`ScalarLoss`] accumulated during focus (cache miss,
  disk fallback, etc.).
- `imperfect` — `Success | Partial | Failure` per `terni::Imperfect`.
  This is the **observe channel** — every `focus` call already
  carries the structured verdict.

The focus operation maps to `prism_core::Prism::focus`. Source of
truth:

```rust
// prism/core/src/lib.rs ~line 100
pub trait Prism {
    type Input: Beam;
    type Focused: Beam<In = <Self::Input as Beam>::Out>;
    ...
    fn focus(&self, beam: Self::Input) -> Self::Focused;
}
```

### 2.2 `project(beam, filter) → Beam`

Narrow the beam by a criterion. `filter` is a typed selector into
the Prism's `Focused` space: a prefix, a pattern, a comparator, a
walk direction, a kintsugi-tournament invocation, an order spec.

```
project(beam: Beam<Focused>, filter: Filter) -> Beam<Projected>
```

Returns a `Beam<P::Projected>` whose `result` is the narrowed view.
Multiple `project` calls compose; this is the closure under chain
that lets every prior tool collapse into a sequence.

Project maps to `prism_core::Prism::project`. The filter DSL is
spec'd at §6.2.

### 2.3 `refract(beam, output) → Beam`

Settle. Commit. Crystallize. `output` is a typed selector into the
Prism's `Refracted` space: a path to write to, a ref to advance, a
shard to flush, a crystal to seal.

```
refract(beam: Beam<Projected>, output: Output) -> Beam<Refracted>
```

Returns a `Beam<P::Refracted>`. The output is the side-effecting
step — refract is where the commit goes to disk, the ref advances,
the merge writes. Per [[reality-shard-as-crdt]], the refract
step IS the lattice join — the result is monotonically `≥` the
prior shard state.

Refract maps to `prism_core::Prism::refract`. The output DSL is
spec'd at §6.3.

### 2.4 The Beam shape on the wire

A `Beam` flowing across pq's wire is JSON-shaped as:

```jsonc
{
  "input":   <input-state>,   // the source position (the `In` slot)
  "result":  <value|null>,     // the focus payload, present iff is_ok
  "loss":    <scalar-or-null>, // ScalarLoss accumulated this leg
  "imperfect": {
    "kind":   "success" | "partial" | "failure",
    "value":  <value|null>,   // present for success/partial
    "error":  <typed|null>,   // present for failure
    "loss":   <scalar-or-null>
  }
}
```

**Every Beam carries the `imperfect` field.** This subsumes the old
`fragmentation_observe` tool: observation is not a separate call;
it is a field on every response. The agent reads it directly. The
structured diagnostic flows out of `terni::Imperfect` per
[[../../../fragmentation/docs/specs/lens-transit]].

---

## 3. The composition algebra — every old tool as a pq chain

The collapse, in one table. Each row is a previous frgmnt MCP tool
rewritten as a pq composition. The Beam threads left-to-right; the
final Beam is the agent's response.

| Old tool | pq chain |
|---|---|
| `commit(path, content, msg)` | `refract(beam_from_content, { to_path, message: msg })` |
| `show(oid)` (was `read(oid)`) | `focus({ oid })` |
| `cat(path)` (was `read_path(path)`) | `focus({ path })` |
| `context()` (the shard summary) | `focus({})` |
| `list(prefix)` | `focus({}) → project({ prefix })` |
| `search(q)` | `focus({}) → project({ match: q })` |
| `history(oid)` | `focus({ oid }) → project({ walk: "back" })` |
| `branch(name, oid)` | `focus({ oid }) → refract({ ref: name })` |
| `diff(a, b)` | `focus({ pair: [a, b] }) → project({ compare })` |
| `merge(a, b)` | `focus({ pair: [a, b] }) → project({ kintsugi }) → refract({ to_ref })` |
| `snapshot()` | `focus({}) → refract({ snapshot: true })` |
| `refs.list(prefix)` | `focus({ refs: true }) → project({ prefix })` |
| `refs.update(ref, new, old?)` | `focus({ ref }) → refract({ cas: { old, new } })` |
| `observe(scope, since?)` | falls out for free — every Beam carries `imperfect` |
| `shard.status()` | `focus({ shard: true })` (the shard's summary is its focus) |
| `shard.flush()` | `focus({ shard: true }) → refract({ flush: true })` |
| `shard.open(path, budget)` | implicit on session bootstrap (per [[shard-ref-as-prism]] / T10); no tool needed |
| `shard.close()` | implicit on session teardown; no tool needed |

**Eighteen tools collapse to three. Every chain remains typed —
the target/filter/output DSLs are typed against the Prism's
associated `Beam` types, so the wire stays verifiable.**

### 3.1 Type sketches for the non-obvious rows

Three rows in the table take judgement to read; here is the Beam shape entering and leaving each step.

**`commit(path, content, msg)`**. The chain needs a `focus` first to produce a `Beam<Focused>` for `refract` to consume:
```
focus({ content })          → Beam<Focused>   carrying the raw content
project({ to_path: path })  → Beam<Projected> declaring the write target
refract({ message: msg })   → Beam<Refracted> carrying the committed OID
```
The collapse table's `refract(beam_from_content, { to_path, message })` is the same chain elided. The full form is the type-correct one; the elided form is shorthand for the common case where `focus` + `project` are inferred from the `Output` shape.

**`snapshot()`**. The terse `focus({}) → refract({ snapshot: true })` skips `project`, which the trait does not allow directly. The expanded chain inserts an identity project:
```
focus({})                   → Beam<Focused>   the shard's current focus
project({ identity: true }) → Beam<Projected> passthrough
refract({ snapshot: true }) → Beam<Refracted> the snapshot OID
```
The identity project is the algebraic unit; per the optics monoid in `prism_core::optics::monoid` it is the identity element under composition. Snapshot is therefore `focus ∘ id ∘ refract`.

**`merge(a, b)`**. The chain is genuinely three-step, but `project({ kintsugi })` invokes the kintsugi-tournament loop, which itself dispatches pq chains. The shape:
```
focus({ pair: [a, b] })     → Beam<Focused>   the two-input position
project({ kintsugi })       → Beam<Projected> the resolved merge plan
refract({ to_ref: name })   → Beam<Refracted> the merged ref
```
The potential circularity — `project` calls kintsugi calls pq calls `project` — is resolved by the dispatch protocol: `project({ kintsugi })` returns a `Beam<Projected>` whose `imperfect` is `Partial { confidence }` until the loop converges, at which point a subsequent `refract` is what commits. If the agent re-invokes `refract` before convergence, the call is idempotent and returns the same Partial Beam. Convergence is observable on the `imperfect` field; the loop's sub-Turing bound (per [[../../../mirror/docs/specs/kintsugi-tournament]]) terminates the recursion.

### 3.2 What the table does NOT lose

Nothing the old surface expressed is unreachable. The chain shape
is strictly more expressive — the agent can interleave projects,
run multi-leg navigations, fold a `history` walk into a
`project({ match })` in the same call. The algebra is closed; the
wire just publishes the closure.

The HamiltonScheduler discipline
([[../../../fragmentation/docs/specs/hamilton-scheduler]]) is
unchanged. `realtime: "hard"` becomes a Beam-level annotation in
the `focus` call's input slot, NOT a per-tool flag. The drop
discipline rides on `imperfect` already.

---

## 4. The Beam state question — stateless wire, beam-by-value

Decision: **the wire is stateless. Every pq call carries the Beam
by value.** No server-side `beam_id` handle. Resolved.

Reasoning:

- The Beam shape is small: `(In, Imperfect<Out, E, L>)`. Even a
  large focus result (a blob, a tree) is what the agent asked for;
  passing it back to the next pq call costs one round-trip-worth
  of bandwidth, not a stateful server handle.
- Server-side beam handles couple Beam lifetime to MCP session
  lifetime, which couples wire-protocol semantics to transport-
  layer state. **The MCP 2025-06-18 §lifecycle treats requests as
  independent units; the JSON-RPC layer is stateless by
  construction.** A server-side handle layer would push wire
  semantics into transport semantics. Reject. GC questions, leak
  questions, cross-session questions all become moot once the
  handle layer is rejected on this independent ground; the CRDT
  contract per [[reality-shard-as-crdt]] is undisturbed by either
  choice, but does not need the overlay either.
- The shard itself IS the server-side state — `ShardRef` (per the
  in-flight T9 design; see
  [[../../../fragmentation/docs/specs/fragmentation-mcp]] §3.4 and
  `fragmentation/src/shard_ref.rs`) is the per-session typed handle.
  The Prism implementation that the wire dispatches on is
  `FrgmntMcp`, which holds the shard registry; `ShardRef` is a typed
  budget+context binding, not the Prism instance itself. Beam state
  is per-pipeline transient.

The tradeoff:

| Stateless (chosen) | Server-side (rejected) |
|---|---|
| Beam round-trips with each call | One round-trip; subsequent calls reference id |
| No GC questions | Server must reap stale beams |
| Compose-across-sessions trivially | Sessions own beams; cross-session is impossible |
| Each call self-describes | Each call requires server lookup |
| Replay-safe; resumable | Replay collides with stale ids |

Large intermediate results are addressed by content (OIDs); the
Beam carries OIDs, not bytes. When the agent wants the bytes, the
next `focus({ oid })` fetches them. The wire pays bandwidth only
when the agent asks to materialise.

---

## 5. The DSL for `target`, `filter`, `output`

The three argument types are typed, not stringly. Each is a
discriminated union whose variants are inferred from the Prism's
associated Beam types. This satisfies
[[feedback-no-stringly-types]] at the wire altitude.

### 5.1 `Target` — the focus DSL

```
Target =
  | {}                                  -- the shard's current focus
  | { oid:   SpectralCoordinate<5> }    -- focus by content address
  | { path:  text }                     -- focus by working-tree path
  | { ref:   Reference }                -- focus by named ref
  | { pair:  [Target, Target] }         -- focus on the pair (diff/merge inputs)
  | { refs:  true }                     -- focus the ref-set
  | { shard: true }                     -- focus the shard's summary
```

The target type is verified at the wire boundary against the
Prism's `Input::Out` type. Unrecognised targets fail with a typed
verdict; they do not stringly-coerce.

### 5.2 `Filter` — the project DSL

```
Filter =
  | { prefix:  text }                    -- string-prefix narrowing
  | { match:   text }                    -- pattern/substring narrowing
  | { walk:    "back" | "forward" }      -- DAG walk direction (history)
  | { compare: () }                      -- structural diff of a focused pair
  | { kintsugi: () }                     -- tournament merge of a focused pair
  | { order:   [OrderSpec] }             -- ordering
  | { limit:   u32 }                     -- bounded results
  | { where:   [WhereClause] }           -- typed predicate (per mq)
```

Multiple project filters compose; each narrows the beam further.
The project DSL is the structural counterpart to mq's
`project_query.filters`.

### 5.3 `Output` — the refract DSL

```
Output =
  | { to_path:  text, message?: text } -- commit to a path
  | { ref:      Reference }            -- advance a ref (branch creation / update)
  | { cas:      { old: Oid, new: Oid } } -- CAS-safe ref update
  | { to_ref:   Reference }            -- write merged result to a ref
  | { snapshot: true }                 -- crystallize without committing
  | { flush:    true }                 -- shard flush to disk
```

Output verifies against the Prism's `Refracted::Out` type. The
refract step is where the substrate-side effect happens; the
verdict tells the agent whether it landed.

### 5.4 Where the DSL lives

The three DSL types are declared in `prism_core` alongside the
Prism trait, parameterised by the Prism's associated beam types.
The wire schema (JSON Schema) is **derived** from the typed DSL;
the schema is not the source of truth — the Rust types are.

```rust
// sketch — exact shape lands in implementation tick
pub trait PrismQuery: Prism {
    type Target: PqTarget<Self>;
    type Filter: PqFilter<Self>;
    type Output: PqOutput<Self>;
}
```

Mirror-side `mq` (see §8) compiles its query AST into typed
`Target`/`Filter`/`Output` values; the wire never sees raw
strings.

---

## 6. Relationship to optics

The optics hierarchy in `prism_core::optics`
(`iso`, `lens`, `optic_prism`, `traversal`, `fold`, `setter`) IS
the extensible vocabulary for `Filter` and `Output`.

| Optic | Wire role |
|---|---|
| `Iso<A, B>` | An invertible target → focus transform; round-trips lossless |
| `Lens<S, A>` | A focused view inside a whole; `{ path: ... }` is a lens with the working-tree as `S` |
| `OpticPrism<S, A>` | A semi-deterministic variant matcher; `{ where: kind = ... }` is an optic prism |
| `Traversal` | Multi-focus projection; underlies `{ walk: ... }` |
| `Fold` | Read-only summary; underlies `focus({ refs: true })` |
| `Setter` | Write-only update; underlies `refract({ ref: ... })` |

Each optic implements `Prism`. So a `Lens<S, A>` is a Prism the
frgmnt MCP can hand back as a Beam, and the agent can chain pq
operations against it transparently. The optics monoid
(`PrismMonoid`) gives the composition law: chains of pq calls
compose **associatively**, with an identity element, exactly like
classical optic composition.

This matters: agents that want a custom navigator ("give me the
lens for the third commit's tree at path foo/bar.rs") get it for
free via the optic algebra. The wire stays at three calls;
expressiveness lives in the typed DSL.

---

## 7. Five vs three — split and zoom are CLI sugar

The project-level surface declares five operations: `focus`,
`project`, `split`, `zoom`, `refract`. Why does pq publish only
three on the wire?

Because `split` and `zoom` are **compositions** of the three.
They are spectral-CLI verbs, not Prism trait methods. Look at
`prism/core/src/lib.rs` — the trait has exactly three methods:
`focus`, `project`, `refract`. The Prism's algebra is closed at
three; the five-operation framing belongs to the CLI altitude,
where `split` and `zoom` desugar.

### `split` as a chain

`split(options)` = `focus({}) → project({ where: option ∈ options }) → refract({ snapshot })` per branch.

The spectral CLI command runs N pq pipelines in parallel — one per
option — and renders the resulting beams as the variants. The
split happens at the CLI altitude; the wire sees N independent
focus/project/refract chains.

### `zoom` as a chain

`zoom(transform)` = `focus({ target }) → project({ where: transform.scope }) → refract({ to: transform.output })`.

zoom is a transform-at-scale verb in the CLI. On the wire, it is a
standard focus-project-refract triple — the transform shape and
scale are arguments to filter and output. No new wire op is
needed.

### Why three is the wire minimum

The Prism trait's `focus → project → refract` triple is
structurally:

- **`focus`** is the *observe* — the algebra's left-acting
  morphism that maps the input state into the algebra's domain.
- **`project`** is the *transform* — the algebra's endomorphism
  on its own domain. Iterates freely. Closed.
- **`refract`** is the *commit* — the algebra's right-acting
  morphism that maps back into a settled output state.

This is the minimum structure that supports a closed composition
algebra over a state space (observe → transform* → commit). Any
fewer and the wire either can't observe, can't iterate, or can't
commit. Any more and the wire is enumerating instead of
composing — the wins of the algebra disappear.

---

## 8. Relationship to `mq`

`mq` (see [[../../../mirror/boot/std/code/mq]]) is the high-level
surface for agent → spectral communication. It is a Mirror
grammar with the five operations as **query verbs**, a query AST,
a context stack with eigenboard state, suggestions, and a typed
CSS-flavoured pattern selector grammar. **mq is what the agent
speaks; pq is what the wire carries.**

### 8.1 The compile path

```mirror
-- mq.mirror declares two templates:
template parse(input: text) -> query
template compile(query, context) -> result
```

`mq.compile(query, context)` desugars the mq query AST into a pq
chain. The compile rule (sketch):

```
compile(focus_query { path, ... }, ctx)
    => pq.focus({ path })

compile(project_query { source, filters, ordering, limit }, ctx)
    => pq.focus(source) → pq.project({ filters, ordering, limit })

compile(split_query { options, depth }, ctx)
    => parallel(option in options) { pq.focus({ ref: option }) → pq.project({ depth }) → pq.refract({ snapshot }) }

compile(zoom_query { diff, blame, branches, log }, ctx)
    => pq.focus(target(diff|blame|...)) → pq.project(filter) → pq.refract(output)

compile(refract_query { store, crystallize, index }, ctx)
    => pq.focus({}) → pq.refract({ to_path|to_ref|crystallize, ... })

compile(intent_query { intent }, ctx)
    => @fate.tournament(intent, ctx) → compile(resolved, ctx)
```

The intent variant deserves a note: when the agent writes natural
language (`\ find the function that handles authentication`),
`mq.compile` runs the Fate tournament per
[[parse-as-fate-tournament]] to resolve the intent into a concrete
query AST, then recursively compiles it down to pq. The tournament
IS the variety-maintenance step per [[kintsugi-variety]] §7.

### 8.2 A worked example

The agent is debugging an auth-token leak. They write at the
spectral prompt:

```
\ show me the last three commits that touched any function whose
name mentions "auth" or "token", and stage the changes I'd make to
add a `redact` lens to the property chain
```

mq parses this into an `intent_query`. Fate's tournament resolves
the intent to:

```mirror
-- resolved AST after Fate runs:
project_query {
  source:    find(ref("HEAD")),
  filters: [
    where_clause("kind",      eq,  ref_val("zoom")),
    where_clause("name",      contains, text_val("auth")),
    where_clause("name",      contains, text_val("token"))  -- OR-fused upstream
  ],
  ordering: [{ field: "witnessed", direction: desc }],
  limit:    some(3)
}
```

`mq.compile` desugars to:

```
pq.focus({ ref: "HEAD" })
  → pq.project({
      walk: "back",
      where: [
        { field: "kind", op: eq, value: "zoom" },
        { field: "name", op: matches, value: "(auth|token)" }
      ],
      order: [{ field: "witnessed", direction: desc }],
      limit: 3
    })
```

The wire sees two pq calls. The first returns a Beam carrying the
HEAD ref's commit (with `imperfect.kind = success` if it resolved,
`partial` with diagnostics if the shard had to spill a cold
fragment to disk, `failure` with `NotResident` if a hard-realtime
budget was exhausted). The second returns a Beam carrying the
three commits as a typed list.

The agent then asks to stage the redact-lens change. Mq compiles:

```
pq.focus({})                                                  -- the shard
  → pq.project({ prefix: "property/" })                        -- narrow to property chain
  → pq.refract({ to_path: "property/redact.mirror", ...content })
```

Three-call wire. The composition is associative — the agent could
have chained the search and the refract in a single pipeline if
they wanted to.

---

## 9. Sub-Turing — the composition algebra is closed and bounded

Claim: pq's three operations under sequential composition form a
**sub-Turing** algebra. Concretely:

1. **Closed.** Every pq call returns a Beam typed against the
   Prism's associated `Focused | Projected | Refracted` types. The
   next call's input must type-match the prior call's output. The
   composition cannot escape the Prism's type chain.

2. **Bounded.** A Beam carries finite state (`In`, `Imperfect<Out,
   E, L>`). Each call's loss accumulates in `L`, which is a
   monoid. The Hamilton scheduler bounds the per-call work via
   `crystallize_bounded(deadline)`. Per
   [[../../../fragmentation/docs/specs/hamilton-scheduler]], the
   per-shard memory budget is fixed. There is no unbounded
   recursion at the wire altitude.

3. **No reflection.** pq calls do not reflect on the Prism's
   internal structure beyond what the typed `Target`/`Filter`/
   `Output` DSLs expose. The wire cannot mint new operations; it
   composes the three.

4. **No fixed point.** A chain of pq calls terminates after the
   refract step — there is no `while` at the wire altitude. The
   agent decides whether to issue a second pipeline; the wire
   does not loop on its own.

The combinator algebra is `(focus | project*  | refract)` —
structurally a finite-arity composition that monoidally extends
but does not Turing-complete. This satisfies the substrate-pull
discipline ([[feedback-substrate-pull]]) at the wire altitude:
turning-completeness lives in the grammars above (`mq`, mirror
grammars), not at the wire.

Loop semantics — when the agent wants "keep merging until
stable" — live in the kintsugi loop ([[kintsugi-tournament]]),
which runs ABOVE pq, invoking pq chains as its primitive
operations. The wire stays sub-Turing; the higher-altitude logic
does the recursion.

---

## 10. The wire format — JSON-RPC framing

pq rides JSON-RPC 2.0 over the MCP transport (stdio or streamable
HTTP). Three methods on the wire:

```jsonc
// tools/call shape (the request the MCP client sends):
{
  "jsonrpc": "2.0", "id": 17, "method": "tools/call",
  "params": {
    "name":      "focus" | "project" | "refract",
    "arguments": <Beam-shaped or Target-shaped per §2>
  }
}
```

The three tool names live unqualified — no `frgmnt_` prefix
(rule 1 from §1). The MCP server-id already namespaces.

`tools/list` returns three tool stubs whose JSON Schema is
derived from the Prism's associated `Target`/`Filter`/`Output`
types. When the underlying Prism evolves (a new lens, a new
filter variant), the schema regenerates; the wire stays
self-describing.

### 10.1 No `tools/list_changed` for compositions

Because new capabilities are typed extensions of the existing
DSL types, NOT new tools, `tools/list_changed` rarely fires.
The schema may evolve in place; the three names stay fixed.

---

## 11. Refusals

This spec deliberately does NOT:

- **Invent new operations.** Three is what `prism_core::Prism`
  exposes. We do not add `walk`, `merge`, `commit` as wire
  primitives; they compose.
- **Replace `mq`.** The high-level grammar stays the
  agent-facing surface. pq is below; mq compiles down. Both
  ship.
- **Specify a server-side beam handle.** Stateless wire is the
  decision (§4). Future work may add an OPTIONAL cache where
  a server-side OID-keyed beam cache short-circuits redundant
  computation — but the wire protocol does not depend on it,
  and the cache is content-addressed (CRDT-safe).
- **Touch the registry.rs.** No Rust changes in this spec. The
  implementation tick lands afterward; this is the design that
  permits it.
- **Override [[fragmentation-mcp]]'s shard discipline.** The
  Hamilton scheduler, the budget, the realtime classes, the
  Splinter-Merkle diff format, the BLAKE3↔SHA-1 crosswalk —
  all unchanged. pq is the wire algebra ABOVE; the substrate
  STAYS.

---

## 12. Followup ticks

1. **Pq trait + DSL types in `prism_core`.** Declare
   `PrismQuery: Prism`, `PqTarget<P>`, `PqFilter<P>`, `PqOutput<P>`.
   Lands as a new module `prism_core::pq`. Zero new deps (per
   the kernel discipline).

2. **JSON Schema derivation for the three operations.** A
   single proc-macro path emits the wire schema from the
   `PrismQuery::Target/Filter/Output` types. Surface lives in
   `prism_core::pq::schema`.

3. **`impl PrismQuery for FrgmntMcp` in
   `fragmentation/vcs/mcp`.** The 18-tool registry collapses to
   3. The `dispatch` arm folds 18 cases into 3. Per
   [[../../../fragmentation/docs/specs/fragmentation-mcp]] §3,
   each prior tool's body becomes a chain.

4. **`mq.compile` desugaring in mirror.** The template in
   `boot/std/code/mq.mirror` opens. Emits typed
   `Target`/`Filter`/`Output` AST. The intent variant routes
   through `@fate.tournament`.

5. **MCP `tools/list_changed` semantics.** Confirm the schema
   regen path emits the notification IFF the typed DSL
   surface changed. Most Prism evolutions are typed-DSL-extends
   and do NOT regenerate. See [[lsp-and-mcp]] §reload.

6. **Documentation parity at the mq surface.** Update
   [[../../../mirror/boot/std/code/mq]]'s `compile` template to
   reference pq explicitly; the wire altitude appears in the
   grammar's `out` chain.

---

## 13. References and dependencies

In-spec dependencies:

- [[../../../mirror/boot/std/code/mq]] — the high-level grammar
  that compiles to pq.
- [[../../../fragmentation/docs/specs/fragmentation-mcp]] — the
  first wire that ships pq; the substrate the algebra is
  projected against.
- [[../../../mirror/docs/specs/lsp-and-mcp]] — the MCP transport
  this spec layers on.
- [[../../../mirror/docs/specs/kintsugi-variety]] — pq's
  `imperfect` field carries the variety verdict per crossing.
- [[../../../mirror/docs/specs/reality-shard-as-crdt]] — refract
  is the lattice join; pq composition respects strong eventual
  consistency.
- [[../../../mirror/docs/specs/parse-as-fate-tournament]] — how
  intent queries resolve before pq compilation.
- [[../../../mirror/docs/specs/kintsugi-tournament]] — the loop
  that runs above pq; invokes pq chains as primitives.
- [[../../../mirror/docs/specs/parser-as-prism-grammar]] — the
  parser is a Prism; pq is the wire algebra of any Prism.
- [[../../../mirror/docs/specs/prism-core-as-spectral-triple]] —
  why the three-operation algebra is structurally right.
- [[shard-ref-as-prism]] *(TBD — fragmentation T9/T10 spec; see
  [[../../../fragmentation/docs/specs/fragmentation-mcp]] §3.4
  and the T10 architectural reframe; no standalone spec exists
  yet)* — the per-session shard handle.
- [[../../../mirror/docs/specs/properties-on-glass]] *(TBD —
  referenced as load-bearing throughout the corpus but the
  standalone spec does not exist at the path searched
  2026-06-02)* — where typed verdicts live.

In-corpus prior art:

- Pickering, Gibbons, Wu (2017), *Profunctor optics: Modular
  data accessors.* The compositional algebra of optics; the
  monoidal structure pq inherits.
- Foster, Greenwald, Moore, Pierce, Schmitt (2007), *Combinators
  for bidirectional tree transformations: A linguistic approach
  to the view-update problem.* The lens laws that ground the
  Filter/Output DSLs.
- Anthropic, *Model Context Protocol Architecture* (spec
  2025-06-18). The JSON-RPC framing this spec rides.

---

*The Prism trait is the spec. The wire is the trait, projected.
Three calls. Composition is closed. Eighteen collapse to three
and nothing is lost.*

Apache-2.0.

# The Architecture

The compiler produces loss. The loss IS the holonomy. The spectral runtime
analyzes the holonomy as a graph. The graph tells Fate what to do next.

---

## The Stack

```
terni              Imperfect<T, E, L> — the ternary type
prism-core         Optic, Beam, Bundle tower — the optics
mirror             .mirror compiler — the compiler IS the LSP
spectral-db        tick/tock graph — the graph IS the memory
coincidence        eigenvalues — the eigenvalues ARE the observation
fate               model selection — the decision IS the loop closing
```

Each layer produces `Imperfect`. Each layer's loss flows into the next.
The stack is a pipeline where loss accumulates upward and decisions
flow downward.

## The Loop

```
mirror compiles source
  → Imperfect<Artifact, Error, MirrorLoss>
    → MirrorLoss IS Transport::Holonomy
      → spectral-db stores it as a graph mutation (tick)
        → coincidence computes eigenvalues of the loss graph
          → Fate reads eigenvalues as GraphObservation (16 dims)
            → Fate selects the next model
              → the model acts on the source
                → mirror compiles again
                  → tick
```

The loop closes. The compiler feeds the spectral runtime. The spectral
runtime feeds Fate. Fate feeds the agents. The agents modify the source.
The compiler runs again. Every tick produces loss. Every loss is holonomy.
Every holonomy is a graph mutation. Every graph mutation shifts the
eigenvalues. Every eigenvalue shift is a Fate observation.

The compiler IS the sensor. MirrorLoss IS the signal. The spectral
runtime IS the nervous system. Fate IS the decision.

## MirrorLoss

MirrorLoss is Mirror's domain-specific loss type. It implements `terni::Loss`.
It IS `Transport::Holonomy` in the bundle tower.

MirrorLoss is NOT ShannonLoss (a single f64). It is NOT ScalarLoss.
It is a compilation trace — every phase, every intermediate OID, every cost.

```rust
impl Loss for MirrorLoss {
    fn zero() -> Self { /* empty trace */ }
    fn combine(self, other: Self) -> Self { /* append phases */ }
    fn is_zero(&self) -> bool { /* no phases recorded */ }
    fn total() -> Self { /* complete compilation failure */ }
}
```

The loss accumulates through `eh!` blocks. Each `?` in the compilation
pipeline adds a phase to the trace. The final `MirrorLoss` on any
`Imperfect` return tells you the complete story of how that result
was produced.

MirrorLoss is what TraceBeam wanted to become. TraceBeam was trying to be
a carrier AND a recorder. MirrorLoss is the recorder. `Imperfect` is the
carrier. The split is clean.

## The Compiler IS the LSP

The Mirror compiler is an incremental compiler by architecture.
It does not have an LSP. It IS the LSP.

```
mirror (the binary)
  ├── compile file.mirror     one tick, wait for crystal, print result
  ├── repl                    interactive, autocomplete, inline diagnostics
  └── lsp                     stdio LSP for external editors
```

All three are the same compiler. The same function. The same return type:

```rust
Imperfect<CompiledArtifact, CompilationError, MirrorLoss>
```

The CLI calls it once and waits. The REPL calls it on every line. The LSP
calls it on every keystroke. Same function. Different patience.

### Partial Compilation

The LSP serves from whatever state the compiler is in:

- **Success** — fully compiled. Green gutter. Full completions. Full diagnostics.
- **Partial** — compiled what it could. Amber gutter. Completions carry confidence
  from the MirrorLoss. Symbols that resolved are trusted. Symbols that didn't are
  flagged. The trace tells you which phases succeeded.
- **Failure** — nothing compiled. Red gutter. But the trace tells you how far
  it got. Phase 1 of 4 succeeded. 73% of symbols resolved. Failed at emit on
  line 42. The LSP serves from the trace — completions from the last resolved
  phase, with loss.

The LSP protocol was designed for binary compilation. Compiled or not.
MirrorLoss fills the ternary space the protocol left empty.

### Incrementality

Every intermediate state is content-addressed. Every tick produces an OID.
If the OID changed, recompile that subtree. If it didn't, serve from cache.
The content addressing IS the incrementality. No dirty tracking. No
invalidation protocol. The OID IS the cache key.

## Spectral Loom

spectral-loom builds on top of the CLI. Not on top of the LSP protocol.
Because the CLI already IS the LSP.

Loom embeds the mirror binary. The binary ticks. The binary returns
`Imperfect`. Loom renders it.

```
spectral-loom
  └── mirror (embedded)
        └── compiler
              └── Imperfect<Artifact, Error, MirrorLoss>
                    └── loom renders the loss as dots
                          green / amber / red
```

The dots in the gutter aren't a loom feature. They're MirrorLoss rendered.
The compiler produces the loss. Loom paints it. The same loss that the CLI
prints as text, loom renders as dots. gestalt-tui for the terminal.
gestalt-ui for the web.

The `Imperfect` doesn't care who's rendering it.

## Spectral Analysis of Compilation

spectral-db stores MirrorLoss as graph mutations. coincidence computes
eigenvalues. This enables what no other toolchain can:

Analysis of the SHAPE of compilation loss over time.

Not "did it compile." Not "how many errors." The eigenvalue structure
of the loss graph. Which dimensions are settling. Which are diverging.
Where the curvature is. Where the attractors are.

```
This module has been Partial for 47 ticks.
The convergence eigenvalue is 0.3 and falling.
The cross-grammar coupling between @rust and @gleam
has loss concentrated in dimensions 3 and 7.
Fate routes to Cartographer.
Cartographer maps the boundary.
The dot turns amber.
```

The staff engineer sees amber. She doesn't know about eigenvalues.
She doesn't know about Fate. She sees: this module is struggling.
The toolchain knows where. The toolchain knows why. The toolchain
is already working on it.

## The PbtA Reading

The compilation loop is a PbtA session.

Every tick is a roll. The compiler rolls+loss.

- **10+ Success** — clean compilation. No loss. Green. No handler needed.
- **7-9 Partial** — compiled with cost. MirrorLoss tells you what was lost.
  `recover |artifact, loss|` — the soft move. Adjust.
- **6- Failure** — compilation failed. MirrorLoss tells you how far it got.
  `rescue |error|` — the hard move. The MC makes a move.

The `eh!` macro is the move resolution. The compiler is the MC.
The source code is the fiction. The loss is the consequence.

## The Bundle Tower

The compiler implements the full bundle tower from prism-core:

```
Fiber       source text (.mirror content)
Connection  KernelSpec (which decomposition strategy)
Gauge       Target (BEAM / WASM / Metal)
Transport   compilation (source → compiled, with MirrorLoss as holonomy)
Closure     the compiled artifact (content-addressed, frozen)
```

The compiler IS the bundle. Transport IS compilation. Holonomy IS
MirrorLoss. The mathematical structure and the engineering structure
are the same structure.

## One Type, One Loop

```rust
Imperfect<T, E, L: Loss>
```

Three states. Measured loss. Composable via `.eh()`. Recoverable via
`recover` and `rescue`. The pipeline carrier. The flight recorder.
The LSP response. The graph mutation. The eigenvalue source. The
Fate observation.

One type. One loop. The whole stack.

---

*The cost of honesty is 0.65 nanoseconds per step. The return is a
compiler that watches itself compile and knows what it costs.*

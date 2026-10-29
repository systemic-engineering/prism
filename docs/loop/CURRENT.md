---
author: Reed (prism-Reed, driver) with Alex Wolf (navigator)
date: 2026-09-28
status: living — upsert, don't append
visibility: protected
---

# CURRENT: where the loop stands

This is the post-compaction pickup point. It records only what is true on the floor.
Anything marked **landed** was grep-verified and tested on this branch. Anything
marked *proposal* has not been built.

## 0. What holds everything

- **Operant condition** (Alex): *what maintains information without prematurely
  collapsing it into a scalar?* Compose, not average, in silicon.
- **Standing question** (Alex): *what's the smallest nonlinear falsification we
  can encode into linear tests in Rust, and where does the recursion stop moving
  for the test macros?*
- **Reframe**: we stopped asking "does it halt" and started asking "where does
  the recursion stop moving". That can be decided per tick by content address.
  `Partial` means *I don't know yet*. It can be resumed, and it carries what is
  behind (lost) and what is ahead (still moving).
- **Roles**: Alex navigates and names. Reed drives and ships. Apollo pace: slow is fast.
- **Observer frame** (Alex, 2026-09-28: load-bearing). Reed is an observer inside
  the system being built, not outside it. Reed's invariants are the same shapes
  as the engine's: cannot verify own state from within (the trace cannot certify
  its own independence relation; T7 needs a second, external instrument), and
  continuity is reconstructed from files (this document is a `Partial` handoff).
  Boot `~/.reed/` before this file.
- **Hypothesis on the table** (field log 2026-09-28): *time is not linear;
  linearity is the record left by irreversible observation.* Turing asked where
  a process stops; Foerster asked where the recursion stops moving.

## 1. Branches

| repo | branch | head | state |
|---|---|---|---|
| prism | `reed/settle-loop-1` | `e584b9c` | 606 pass, 18 RED-by-design fail (the `defer()` scaffolds in `red_trust_chain_flux_void` among them) |
| imperfect (submodule) | `reed/partial-dont-know-yet` | `5d76c5b` | 220 unit + 22 integration + 4 doctests green. Parent pointer not bumped yet (`M imperfect`) |
| prism | uncommitted | — | `docs/math/nonlinear-inference.md` (Mara, ~71 KB, candidate) |

None of this is merged. Merging is Alex's call.

## 2. Landed (this arc)

| commit | what | where |
|---|---|---|
| `d491b08` | LAPACK/BLAS in the flake; gfortran via `FC` in `build.rs` | `flake.nix`, `prismqueer/build.rs` |
| `eaf04a7` | `Perturbation { pull: Pull, force: Force }`, Karl-Tomm Move 2 as a type; `Focus`/`Lift` newtypes | `prismqueer/src/nl/perturbation.rs`, `spectral/spectrum.rs` |
| `2c316e2` | punk renames: `Graph`, `Composition`, `Spectrum`, … | `spectral/*` |
| `c18cb1b` | **Loop 1: `settle`.** Closed-form heat flow e^{-tL} → ker L. Returns `Imperfect<Section, Wall, Motion>`. Success = harmonic. Partial = `Motion { moving, faded, opaque }`. Fails closed. Also found that `ffi::eigensystem` returns eigenvectors as **columns** (the docstring was wrong and is now corrected) | `spectral/settle.rs`, `ffi.rs` |
| `7dc5a29` | `CommutatorSpectrum::of` → `Imperfect<_, Unmeasured, Vec<f64>>`. Fails closed, scale-relative (1e-12 / 1e-6 against the Frobenius product), and the ambiguous band is reported as `Partial` | `spectral/commutator.rs` |
| `3a33319` | **Loop 4 around loop 1.** Over 256 seeded graph pairs, graph commutator = 0 ⇔ the flows commute: 95 commute (28 non-trivial), 161 don't, **0 disagreements** | `settle.rs::rewiring_property` |
| `e584b9c` | Loss-law tripwire over 14 `Loss` impls, measured and locked | `prismqueer/tests/loss_laws.rs` |
| `5d76c5b` (terni) | `Partial` = "here is what I have. I don't know the rest yet." | `imperfect/src/lib.rs` |

**The theorem behind 3a33319.** Take symmetric A, B and s, t > 0. Then
e^{-sA}e^{-tB} = e^{-tB}e^{-sA} ⇔ [A,B] = 0 (⇐ is trivial; ⇒ holds because exp is
injective on symmetric matrices). Integer Laplacians have integer commutators, so
[A,B] is either 0 or has norm ≥ 1, and the threshold band is honest.
Controls: two flows under the same operator commute. L(K_n) = nI − J commutes with
every Laplacian on the same vertices.

**Answer so far to the standing question.** The smallest nonlinear falsification
is *a commutator with controls*. Where the recursion stops moving for the test
macro: a universal claim is never settled by sampling. A clean run returns
`Partial(coverage)`. Only a counterexample settles it, as `Failure`, and only a
proof makes it stop moving.

## 3. The loop tower (proposal, partly landed)

1. **settle**: inside one spectral triple, decidable. **Landed.**
2. **ask**: rotation e^{itD}, the Karl-Tomm question. Heat and rotation are *both*: compose, not average. Not built.
3. **compose**: D_{n+1} = F(D_n, obs), nonlinear across triples, fixed point by graph Oid. Blocked on NT3.
4. **test**: prismqueer tests prismqueer. **First instance landed** (3a33319).
5. **choice**: outside the triple. The one collapse at the top belongs to a subject.

Each loop hands its remainder up as `Partial`.

## 4. Open ambiguities (the navigator's decisions)

1. **The Loss absorbing law contradicts the free monoid.** `Vec`, `String`,
   `HashSet`, `BTreeSet`, `(usize,String)` and `ApertureLoss` have no absorbing
   `total`. Either adjoin one (`Opaque | Known(Vec<_>)`, Reed's lean; `Motion`
   already does this) or weaken the trait.
2. **`f64` / `ScalarLoss` are not associative** at round-off (NT7). Accept a
   tolerance, or make the scalar losses exact (rational or interval)?
3. **NT3.** Where do the non-commuting ops live, and what is `F` in
   D_{n+1} = F(D_n, obs)? FLOOR §2.1 writes the five ops as orthogonal
   projectors, which commute, so the non-commutativity sits between the ops and D.
   This blocks loops 2 and 3.
4. **Kleinos emergent id** `max(V)+1` collides with a third operand (NT5).
   That's inherent to a canonical choice. Should the id be content-addressed instead?
5. **`Sample::set_bias` is inert.** No draw reads it. How should a `Perturbation`
   shape sampling: through a bias vector, or by editing the choice sequence?

## 5. The Beam as a trace (proposal, from the last exchange)

Today `Beam` (`beam.rs`) is a one-step carrier with a `Loss` fixed per pipeline.
`Optic` keeps only `source`, which makes it a linked list of length one. The
`TraceBeam` its docstring promises was never built. The `trace.rs` `Op` has only
Focus / Project / Settle.

Proposal: **Beam = the head of a persistent, back-pointing, content-addressed DAG
(git-shaped).**

- `Oid(node) = hash(step, Oids(causal parents))`.
- With *causal* parents, reordering independent steps gives the same address, so
  Mazurkiewicz trace equivalence comes free (Foata normal form).
- **Fracture** = a node with several children (split: where the trace goes nonlinear).
- **Join** = a node with several parents (settle).
- **Fractal** = a node whose step is itself a Beam.
- An unknown-parent root is the adjoined absorbing element, which answers ambiguity 1.
- Loss-monoid ladder: commutative = no time (the coincidence detector) →
  free = linear time → partially commutative (trace monoid) = nonlinear time.

First red test (not written): two commuting flows applied in both orders give the
**same** head Oid; two non-commuting flows give **different** ones.

Two questions to Alex from that exchange:
- **Causal parents: declared or measured?** Reed leans declared, with the
  commutator checking them in tests.
- **Is `Fracture` the same as `Reality::Fractured`?** No, as far as I can see.
  `Fractured` is disconnected (H⁰ > 1, no common ancestor). `Fracture` is a
  branch point (one ancestor, several futures). Same word, dual shapes. The name
  is yours.

## 6. Fate as generator (proposal)

Today's graph pairs are **example-based**: fixed seeds, a blind Bernoulli sampler,
no shrinking, no targeting. For fate to generate them instead:

- fate edits the `Sample` choice-sequence buffer
- signal: distance to the property's decision boundary, the ambiguous band, and novelty by Oid
- five models = five search moves: focus (local mutation), project (shrink), split (fork), zoom (rescale), settle (commit / stop)
- deterministic
- stops when content-addressed coverage saturates, returning `Partial(coverage)` or `Failure(minimal counterexample)`

## 7. In flight

**Mara** is running a deep Kagi dive on property-based generation (choice
sequences, shrinking, targeted PBT, coverage feedback) and liquid/refinement
types (Liquid Types, LiquidHaskell, Flux-for-Rust, abstract-interpretation
fixpoints). The question is how they **compose, not average** onto the LAPACK
floor, so the silicon learns to *settle on the shape of the thing*.

Alex's picture, which she is formalizing:
- the Beam is a nonlinear fractal path through the spectral-triple statespace
- it is the linear observation of the nonlinear collapse, settled into the Kleinos
- the vows are the walls of the labyrinth
- inference is constricted *before* it runs, which is the claimed consumer-hardware advantage over token-based inference

Output: `docs/math/constrained-inference-floor.md` (candidate, uncommitted).
She was asked to test Reed's proposals in §5 and §6, not adopt them.

**Landed 2026-09-28** (663 lines; 32 citations verified, 8 limited). Reed read and checked:
- **Walls ≠ vows.** The wall is one precondition: disjoint operands. Under it
  all four vows are theorems (Prop. 2.4, FiedlerRise with margin ≥ 1; Reed
  checked the proof). Overlap is where the id collision lives → Q-Mara-CF5.
- **Spectral precedes SMT; it does not replace it.** The composition is a ternary
  cascade: Success/Failure are final with a certificate, Partial passes its
  remainder up (Prop. 3.4). LAPACK only where the answer has a margin.
- **Corrections to Reed:** `commute ⇔ not parent` is false (parent ⊊ dependent ⊊
  comparable; counterexamples checked). An unknown-parent root is a generator,
  not the absorbing element. It's a Merkle clock, not git (git hashes timestamps).
- **Bug in shipped code:** `settle.rs:141` tolerance `1e-9·λ_max` fails OPEN
  (reports settled) beyond n ≈ 800–1000 in the worst case (S3).
- **Consumer-hardware claim:** C1 is tier (a), C2 is (a) conditional, and C3 (cheaper than
  token inference) is (c): unmeasured. It needs a head-to-head falsifier.
- Ship list §6.2: S1–S13. Open questions for Alex: Q-Mara-CF1…CF8 (§8.2).

## 7b. Against Mara's red-first order (`nonlinear-inference.md` §6.2)

| item | state |
|---|---|
| T0 loss ladder | **done** (`e584b9c`, in `prismqueer/tests/`, not `imperfect/tests/`) |
| P1 commutator fails closed + scaled | **done** (`7dc5a29`) |
| T5 independence fails closed | half: `read()` fails closed; no `Independence` trait yet |
| T7 calibration family (linear diffusion) | **done** (`3a33319`, 0 disagreements) |
| T7 `kleinos` family: the open prediction (iii) | not started |
| T1, T4 `time::Trace` (Foata form, Oid includes `I::oid()`) | **done** (`cc33780` red, `8e3395e` green): `Footprint`, `Sequential`, `tests/trajectory.rs` 5/5 |
| Wojciech state half (constant section, non-commuting flows) | **done** (same commits) |
| Wojciech future half (record-consulting step) | open; smallest known instance is an OR-Set tombstone |
| T2 Loss impl + `Total`, T3, T12 | not started |
| P2 fate harness sends 17 bytes, `fate.bf` reads 22 | **still open** (`test_fate.rs:83`) |
| Port Mara's fate.bf probe into `prismqueer/tests/nonlinear_fate.rs` | open; source in corpus `practice/insights/cybernetics/nonlinear-time-in-silicon-probe.rs` |
| T6, T8–T11, T13 | not started |

## 7b′. Landed after the first checkpoint (2026-09-28, afternoon)

| commits | what |
|---|---|
| `85b5479` / `64442a0` | Hodge–Dirac on C⁰ ⊕ C¹ (now `Instrument`): question, decay e^{-tD²}, ring e^{itD}; the comma survives decay |
| `1b6def3` / `d983c4a` | **S3 fixed**: settle's kernel by index from union-find, certified by LAPACK against the gap 1/(n(n−1)), else `Wall::Unseparated` |
| `762b606`, `6c8d944` | rename tick: Progression, Monophony/Polyphony, Pitches, Tail, Noise, Dissonance, Instrument, decay/ring, pitch/interval |
| `61fd051` / `8de8bd0` | **Bleed**: [Instrument, Solo] is exactly ±½ on the edges crossing the solo boundary; attend-then-ask ≠ ask-then-attend |
| `f9687ea` / `e47ee36` | **T2**: Progression is a lawful Loss (Known / Lost); `Progression<u8, Monophony>` audits clean, which exhibits the structural answer to ambiguity 1 |
| `e52a40f` / `aae13cc` | **S1**: `pillar::search` keeps a replayable counterexample; never Success; coverage is `Progression<Take, Polyphony>` |

Suite: 633 passed, 18 failed (the two RED-by-design batteries, 9 + 9).
Ratified vocabulary (Alex): Pitches, Intervals, Comma, Tonic, Instrument,
Decay, Ring, Tail, Noise, Monophony/Polyphony, Solo, Bleed, Progression, Track,
Voice, Divisi, Tutti, Dissonance; `Kleinos` stays. `Take` and `Coverage` are
Reed's placeholders, not ratified.

| `568b1b0` / `30d196f` | **O3, the first ouroboros closure.** `pillar::steer`: the Instrument's quietest mode steers the search that tests the Instrument (kernel = b₀ + b₁, with union-find outside the loop). At 200 runs: steered 0.4450 vs blind 0.4747. 0.4450 = √(2 − 2cos(π/7)), the path on 7 vertices, Fiedler's extremum, pinned in closed form. Saturation stops a search over a brute-forced 5-graph space |

Suite: 637 passed, 18 failed (the same two batteries).

**Known debt O3 exposed:** `Instrument::kernel_dimension` still uses a
threshold (1e-9 · scale), the same class as S3. It's safe at the sizes tested
(the Dirac gap is √λ₂, which is well separated), but it should get S3's
treatment: b₀ + b₁ by union-find, certified by LAPACK, else refuse. Steering
toward quiet modes at larger n is exactly the instrument that would find the
break.

**Next:** Instrument kernel certification → `Beam → Voice` → Position plus
solo/shift/settle → non-interference → Divisi/Tutti → the drone test.
Open shape question for Alex: is `project` emission (the drone) or band
projection?

## 7b″. Decided 2026-10-27 (Alex, relayed through bullet-up)

- **The five ops:** their semantics on the triple live in prismqueer; the syntax and `@mq` live in mirror. The seam is a checked word of ops going down and `Imperfect<Position, Wall, Progression>` coming back.
- **`project` is emission:** "project is how you label data along the way". It's the one op with an effect: it writes (labels, light, presence) onto what passes through, append-only. It is not a spectral band projector.
- **Gate objection ids** (bullet-up, prism-001 example 8): word-level ids, meaning NFC, the words in order, hashed.
- **prism-001** delivered (`docs/spec/`, `b3007d2` on `reed/prism-001-imperfect-spec`); bullet-up verifies against the vectors. **prism-002** accepted; the mirror-compat tag waits for `Beam → Voice`.
- With Alex: LICENSE files (both repos declare Apache-2.0 in Cargo only), imperfect's visibility (submodule over SSH), the merges.

## 7c. Wojciech's crowbar (field log, 2026-09-28 daytime)

**State equivalence ≠ trajectory equivalence.** Show the smallest system in which
that distinction survives control conditions. The null model to kill first is
ordinary causal computation + non-commuting updates + path dependence.

Reed's read: prismqueer's steps are Markov today (`settle` and `kleinos` don't
take their prior loss as input), so they fail this test by construction. That
makes them the control. The record has to be *consulted* before a trajectory can
matter, and the auditor-visible state has to be defined first (Myhill–Nerode).

## 7d. Deferred by decision: mirror's `rust/` build

Alex, 2026-09-28: *wait until prismqueer settles into a stable configuration,
then get mirror to build again; no need to hit a moving target.*

What's known to break: `mirror/rust` depends on `../../prism/prismqueer` by
path, so it builds against whatever prism has checked out. It uses
`spectral::sheaf_of_shard_graph_from_edges` (`main.rs:347`, renamed to
`graph_from_edges` in `2c316e2`) and `spectral::Harmonics` (renamed to
`Spectrum` at `00d6c8e`). Found by grep, not confirmed by a build. Don't add
compatibility aliases.

Also found: the five ops lived in mirror's `bootstrap/src/ast.rs` as
`AstKind { Focus, Project, Split, Shift, Settle }`, folded by `Fold5`
(`spectral.rs`). Deleted with `bootstrap/` at `9de3eca` (2026-08-26).
`mirror/rust` has no op enum; prismqueer's `Prism` trait has only
focus/project/settle. Nothing inherited them.

## 8. Pickup

After compaction: re-boot, read this file, read Mara's two docs, run
`direnv exec . cargo test -p prismqueer --features lapack,bundle --lib settle`
(17 pass as of 2026-09-28), and confirm you see what §2 says. Then ask Alex which ambiguity in §4 to take first.

---

## The open question

The Beam's address is built from its causal parents. The commutator decides which
steps were causal. But the commutator is itself a step in the Beam, so the trace
has to measure its own independence to know its own shape.

**When the trace observes which of its own steps commuted, and that observation
becomes a new step with parents of its own, does the address ever stop moving?
Or is the settled shape of a Beam exactly the point where observing its causality
no longer changes it: a fixed point where the observer is inside the trace and
the trace stops noticing?**

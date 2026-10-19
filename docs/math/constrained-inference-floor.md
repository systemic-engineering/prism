---
title: "Constrained Inference Floor: property-based generation and refinement types, composed onto LAPACK"
subtitle: "Walls before the run, tests after it, and a numerical floor between them that proves what it can and passes the rest up. What composes, what only averages, what ships in prismqueer, and where Alex's picture, Reed's proposals, and mirror's liquid spec are wrong."
author: Mara
date: 2026-09-28
status: candidate
visibility: protected
slug: constrained-inference-floor
composes-forward-from: /Users/alexwolf/dev/projects/prism/docs/math/nonlinear-inference.md
---

# Constrained Inference Floor

*What maintains information without prematurely collapsing it into a scalar?*

---

## §0 Question and discipline

**0.1 The question.** Alex Wolf, 2026-09-28, verbatim:

> *"What if we spawned Mara for a Kagi deep research dive into property-based generation and the latest computer science on liquid types and how could those COMPOSE not AVERAGE into a LAPACK based Fortran floor for prismqueer? Basically we teach the silicon to `settle` on the shape of the thing. This then also can become the floor for the actual inference at runtime."*

> *"The `Beam` would be a nonlinear fractal path taken through a non-euclidean statespace (the spectral triple algebra) that is the linear observation of the nonlinear statespace collapse. Which can be `settled` into the `Kleinos`. The thread through the labyrinth where the walls of the labyrinth are what? The `vows`? The geometric constrictions of the statespace. The inference would be constricted before being run which is what makes it computable on consumer hardware compared to token-based inference. That's the idea."*

**0.2 The operant condition** (Alex, verbatim): *"what maintains information without prematurely collapsing it into a scalar?"* Compose, not average, in silicon.

**0.3 The standing question** (Alex, verbatim): *"What's the smallest nonlinear falsification we can encode into linear tests in rust and where does the recursion stop moving for the test macros?"* §6 answers it for every item.

**0.4 Tiers.** **(a)** verified: a primary source retrieved this session, or an elementary fact proved inline. **(b)** contested but published. **(c)** speculation: mine, Reed's, or Alex's. **(src)** is a claim about code, read this session with file and line. Where I also *ran* it, I say so. This session I ran three targets inside `direnv exec .`: `spectral::settle` (14 passed), `rewiring_property` (2 passed, printing `commuting=95 (nontrivial=28), not=161`), and `tests/loss_laws.rs` (1 passed). **(num)** is a numerical check I ran in a throwaway Python session. It wrote no file and is evidence, not proof.

**0.5 Retrieval disclosure.** The Kagi *search* endpoint returned empty results for all three queries I tried ("Liquid Types Rondon Kawaguchi Jhala PLDI 2008" and two others). Three routes worked:

- the Kagi *summarizer*, on arXiv abstract pages, Crossref DOI records (`api.crossref.org/works/<doi>`), Crossref bibliographic queries, and author or publisher pages;
- the arXiv API tool, for title and author search (metadata and abstract);
- WebFetch, for four pages whose summaries omitted what I needed.

ACM DL pages were unreliable. For QuickCheck the summarizer returned nothing. For Targeted PBT and Cousot 1977 it summarized the cookie banner. I treated all three as failures and re-routed them through Crossref.

**0.6 Scars found this session.** None of these is carried into the body.

- An arXiv ID I tried first for the Jhala–Vazou tutorial, 2010.14763, is a Hogwild! SGD paper. The tutorial is 2010.07763, confirmed by title search.
- `mirror/docs/specs/liquid-types-for-mirror.md` (§5.3, §10) attributes *"A Theory of Spectral CSP Sparsification"* to "Bafna, Bhatt, Khot, Minzer, ICALP 2025". The arXiv title search returns one paper with that title, **arXiv:2504.16206, by Khanna, Putterman and Sudan**. A Crossref title query found no record under the attributed authors. The venue is unconfirmed either way.
- The same spec lists *Data Flow Refinement Type Inference* as "Pavlinovic, Fonseca, Wies". arXiv:2011.04876 lists **Pavlinovic, Su, Wies**.
- The same spec lists *Gradual Liquid Type Inference* as "Vazou, Lehmann, Jhala, PLDI". arXiv:1807.02132 lists **Vazou, Tanter, Van Horn**.

The corpus already carries one fabricated-citation scar (FLOOR §11.5). These are three more attribution errors in one references section, all load-bearing for the spec's novelty claim. §7 routes them.

**0.7 What this composes over.** `nonlinear-inference.md` (traces, the loss ladder, and T0–T13) is taken as read. From it I reuse three results without re-deriving them:

- a single `Beam` chain is totally ordered;
- the symmetric Laplacian is blind to direction;
- a commutator threshold licenses only approximate independence (§2.3.3 there).

---

## §1 Property-based generation, and what "fate as generator" adds or breaks

### §1.1 What the field actually converged on (a)

| Work | Mechanism | Retrieved content I use |
|---|---|---|
| QuickCheck (Claessen & Hughes 2000) | random generators; properties as code | Programmer-controlled distributions and a size-control mechanism for recursive types. It warns of a *"false sense of security"* when tests pass on trivial cases, so distributions must be monitored. |
| Hypothesis internal reduction (MacIver & Donaldson 2020) | reduce the **choice sequence**, not the output | *"Instead of applying test-case reduction externally … we apply it internally, to the sequence of random choices made during generation."* Every reduced case is one the generator could have produced, so validity comes for free. |
| SmartCheck (Pike 2014) | generic reduction plus generalization | Greedy breadth-first replacement of sub-values. It generalizes counterexamples by universal and existential quantification. |
| Targeted PBT (Löscher & Sagonas 2017) | search guided by a **utility value** | The user supplies a utility value. Hill climbing and simulated annealing replace blind sampling. |
| AFL (technical details) | coverage feedback | Keeps an input only if it hits a new edge tuple. Culls the corpus to a minimal covering set. The queue is non-greedy, so diverse seeds are kept. |
| Zest (Padhye et al.) | coverage-guided mutation **of the generator's parameters** | Converts generators into *"deterministic parametric generators"*. *"Mutations in the untyped parameter domain map to structural mutations in the input domain."* Feedback is coverage plus validity. |
| Parsing Randomness (Goldstein & Pierce) | generator = parser of randomness | *"A large class of generators can be factored into a parser plus a distribution over choice sequences."* Derivatives let a tool *"preview"* a choice, which yields generation that satisfies preconditions. |
| Learn&Fuzz (arXiv:1701.07232) | a learned model proposes inputs | There is a tension: learning reproduces well-formedness, while fuzzing must break it. The best coverage came at a 65–70% pass rate, not 100%. |
| STADS (Böhme) | stopping, statistically | *"Testing provides no formal correctness guarantees."* Discovery can be extrapolated statistically, as in species estimation in ecology. |

**The convergence (a, reading across the rows).** Since Hypothesis and Zest, feedback-directed generation operates on **one object: the choice sequence**. The generator is a fixed, deterministic parser. Search, shrinking, targeting, and coverage are all moves on bytes. Goldstein and Pierce make this a theorem for a large class of generators.

### §1.2 Where prismqueer stands (src)

`flux::pillar::Sample` (`flux.rs:507–634`) has the right shape. It is a byte buffer with deterministic replay (`from_bytes`) and deterministic extension (SplitMix64 seeded from the buffer's SHA-256, `:547–598`). Extension bytes are appended, so the buffer records every choice. Five facts the docstrings do not say:

1. **`forall` discards the counterexample.** `forall` (`:707–720`) draws from `Sample::new()`, a wall-clock seed, and folds verdicts with `merge_with`. It returns a `PropertyVerdict` and **drops the buffer**. A failure cannot be replayed. By the operant condition this is premature collapse: *which input failed* is reduced to *that one did*. It is also exactly the failure mode Hypothesis's design removes.
2. **There is no shrinker.** The docstring (`:502–506`) says the buffer *"is what makes shrinking work"*. That is true of the shape. No reducer exists.
3. **The bias field is unwired.** `set_bias` stores `bias: Option<[f64;5]>`, `#[allow(dead_code)]`, and no draw reads it (`:515–519`, `:581–587`).
4. **`draw_integer` is not monotone in the buffer.** It computes `min + (u64::from_le_bytes(8 bytes) % range)` (`:610–618`). Lowering a byte does not always lower the value: `raw = range` maps to `min` and `raw = range − 1` maps to `max`. Shortlex buffer reduction, the Hypothesis ordering, therefore does not simplify integers monotonically. (a, elementary.) §6 S2 turns this into a red test.
5. **The 3a33319 property uses its own seeding and never reaches the uncertain band.** `rewiring_property` seeds `Sample::from_bytes(seed.to_le_bytes())` for seeds `0..256` (`settle.rs:439–441`). It treats a `Partial` from `CommutatorSpectrum::of` as `Failure("instrument undecided")` (`:468–472`). **(num)** I enumerated every covered unweighted graph on 3, 4, and 5 vertices (4, 41, and 768 graphs) and every ordered pair. The smallest nonzero singular value of `[L_A, L_B]`, relative to `‖L_A‖_F‖L_B‖_F`, is 0.173, 0.048, and 0.0093 respectively. The ambiguous band is `[1e-12, 1e-6)`. **For unweighted graphs with n ≤ 5, the band is unreachable, not merely unsampled.** At n = 6 the minimum is falling by roughly 5× per vertex and is not measured. The "blind sampler" caveat on 3a33319 is weaker than it sounds for n ≤ 5, and it does not transfer to weighted graphs, where the band is reachable by scaling.

Consequence of fact 5: for n ≤ 4 the rewiring claim is cheap to check **exhaustively**: 1 + 16 + 1,681 = 1,698 pairs (n = 2, 3, 4). Exhaustion over a finite domain is a proof. Sampling is `Partial`. The test's `unreachable!("sampling never proves a universal claim")` is right for sampling and wrong for exhaustion. §6 S5 uses this.

### §1.3 Testing Reed's proposal 2 (fate as generator)

> *Fate edits the Sample choice-sequence buffer, not a bias vector. Signal: distance to the decision boundary, the "don't know" band, novelty by Oid. The five models become five search moves. The recursion stops when content-addressed coverage saturates.*

**Editing the buffer: right, and it is prior art.** It is Zest's move (a), with Hypothesis's reducer as the dual (a). The mechanism is not new. What could be new is *who schedules the moves*.

**The signals: each already has a name.**

- *Distance to the decision boundary* is a Targeted-PBT utility value (a).
- *Novelty by Oid* is AFL's new-tuple criterion with a content address in place of the edge bitmap (a for AFL, (c) for the substitution).
- *Aim at the "don't know" band* is the one specific and good idea. It points the generator at the instrument's weakest region. Fact 5 shows that for the current graph family the band is empty, so a targeted search there must come back empty. **That empty return is itself a result:** it separates "the instrument is never uncertain here" from "we never looked".

**The five moves.** focus = local mutation (AFL havoc, Zest). project = shrink (Hypothesis). split = fork, i.e. keep several seeds (AFL's non-greedy queue). zoom = rescale (QuickCheck's size parameter). settle = stop. **This is a renaming of known moves.** It earns its keep only if Fate's choice among them beats a fixed round-robin schedule on a measured budget. Without that measurement it is naming, not mechanism. (c.)

**What it breaks. Three things, each fixable.**

1. **Replay.** If Fate's weights update during a run and Fate sits *inside* the generator, the buffer no longer determines the input. The verdict-cache key `sha256(spec ‖ target ‖ buffer)` (`flux.rs:505`) goes stale. **Fix:** keep Fate **outside** the parser. Fate *proposes buffers*. The generator parses them deterministically. This is Zest's separation, and it keeps replay.
2. **Frequencies.** Targeted and coverage-guided sampling are biased by design. A line like "95 of 256 commute" is an estimate under the blind distribution and means nothing under a steered one. Report both runs, labelled.
3. **Shrinking fights targeting.** A shrinker moves toward simpler inputs. A targeter moves toward the boundary. Blending the two objectives is averaging. **Sequence them:** target until a failure appears, then shrink that failure. Compose, not average.

**Stopping.** "Coverage saturates" is a heuristic, not a proof (STADS, retrieved: testing *"provides no formal correctness guarantees"*). There are three honest stops, and they are different:

| Domain | Stop | Verdict |
|---|---|---|
| finite and enumerable (e.g. n ≤ 4 graph pairs) | exhaustion | `Success`: a proof |
| any, once a counterexample is found | first failure, then shrink to a local minimum under shortlex | `Failure(minimal buffer)` |
| infinite or too large | budget | `Partial(Coverage { budget, distinct_oids, singletons })` |

The `singletons` field is what a species-discovery estimator needs. Good (1953) is the classical source (retrieved: metadata only; the estimator was **not** re-read). I carry the counts, not a formula I did not re-read. Keeping the counts rather than a single "confidence" is the operant condition applied to the stop itself.

---

## §2 Refinement types as walls before the run

### §2.1 What refinement inference is (a)

- **Liquid types** (PLDI 2008; retrieved via ACM DL: title, venue, abstract points; author names not surfaced): *Logically Qualified Data Types* combine Hindley–Milner inference with **predicate abstraction** over a user-supplied set of qualifiers. DSOLVE verified OCaml array safety, with manual annotation falling from 31% to under 1%.
- **Refinement Types: A Tutorial** (Jhala & Vazou, arXiv:2010.07763): verification conditions go to SMT, and decidability comes from restricting to quantifier-free formulas.
- **Flux** (Lehmann, Geller, Vazou & Jhala, arXiv:2207.04034): refinement types for Rust. Checking produces **Horn constraints**, and inference is **fixpoint computation**. Rust ownership tracks mutable locations. Flux synthesizes loop invariants. Specification lines drop by half, and annotation overhead drops from up to 24% of code size to zero.
- **Data Flow Refinement Type Inference** (Pavlinovic, Su & Wies, arXiv:2011.04876): liquid inference is one instantiation of an **abstract interpretation**, parametric in the abstract domain (octagons, polyhedra), with the constraint-solving step itself an abstraction.
- **Abstract interpretation** (Cousot & Cousot 1977, retrieved via Crossref): *"a unified lattice model"*, reasoning by *"construction or approximation of fixpoints"*. Predicate abstraction (Graf & Saïdi 1997, CAV; metadata) is the qualifier-lattice special case.
- **Horn-clause solving** is the standard back end (Bjørner, Gurfinkel, McMillan & Rybalchenko 2015; metadata).
- **2026 state of the art.** **FLEX** (Khan, Markopoulos, Lehmann & Jhala, arXiv:2607.12226) is a foundational CHC solver in Lean for Flux's constraints. Its tactic `zap` *"eliminat[es] acyclic variables via exact strongest solutions"*. Its tactic `fix` *"approximat[es] cyclic variables through predicate abstraction"*. It discharges 95.7% of Flux's benchmark CHCs with a kernel-checked proof. **First-class refinement types for Scala 3** (Bovel, Kunčak & Odersky, arXiv:2605.08369) decide predicate entailment with *"a lightweight e-graph-based solver"*. Mechanized soundness for refinements with polymorphism: Borkowski, Vazou & Jhala, arXiv:2207.05617. Usability: Gamboa, Reese, Fonseca & Aldrich, PACMPL (PLDI) 2025, DOI 10.1145/3729327 (metadata).

**What the 2026 work says about "replace SMT."** Two of the newest systems do replace SMT. They replace it with **proof** (FLEX) and **e-graphs** (Scala), both symbolic. Neither uses a numerical or spectral procedure. I found no published system that decides refinement entailment spectrally. Title searches on arXiv for spectral/Laplacian refinement typing returned nothing on point. That is absence of evidence from one route, not evidence of absence.

### §2.2 Where the recursion stops moving

**Proposition 2.1 (a, elementary).** Liquid inference starts each liquid variable κ at `⋀Q` and weakens it by removing qualifiers that the decision procedure cannot prove. It terminates after at most `|Q|·|K|` weakening rounds, where `K` is the set of liquid variables.

*Proof.* The state is a map `K → 2^Q`, ordered by inclusion. Every non-final round strictly shrinks some κ. The lattice has height `|Q|·|K|`. □

Termination comes from **finite height**. It does not come from monotonicity.

FLEX sharpens where the recursion lives. Acyclic liquid variables are solved in **one pass**, exactly. Only variables on **cycles** of the constraint-dependency graph need iteration (a, retrieved). So:

> **The recursion stops moving immediately on the acyclic part, and within lattice height on each strongly connected component.** The places a fixpoint is needed are the directed cycles of the constraint graph.

That makes the geometry of "where recursion stops" the **SCC condensation of a directed graph**. By `nonlinear-inference.md` §2.3.3 and Prop. 3.2 below, a symmetric Laplacian cannot see it.

**Three corpus claims do not survive Proposition 2.1.**

1. `liquid-types-for-mirror.md` §4.4: *"real number comparison is trivially decidable for computable reals"*. **False.** Equality of computable reals is undecidable. *Proof sketch (a):* for a Turing machine M, let `x_M = Σ_k 2^{-k}·[M halts at step k]`. `x_M` is computable to any precision, and `x_M = 0` iff M never halts. □ Only strict inequality between unequal reals is semi-decidable.
2. The same section: convergence is *"guaranteed if loss is monotonically non-increasing"*. **Wrong kind of guarantee.** A monotone bounded real sequence converges *in the limit*. It need not reach a fixed point in finitely many steps, and it need not ever drop below a given ε, because its limit can be positive.
3. `2026-07-18-witnessed-property-inference.md` §2.3 makes the same move: *"Bounded semilattices support Knaster-Tarski fixed-point iteration … the iteration terminates when HolonomyHealth < ε"*. Knaster–Tarski gives **existence** on a complete lattice. **Finite-time termination** needs finite height (the ascending chain condition), or a widening, or an explicit resolution. Monotone decrease of a real scalar gives none of these.

**`settle` already contains the fix (src, ran).** Heat flow reaches `ker L` only as `t → ∞`. The `Resolution` argument turns that limit into a finite, decidable stop: `Motion::settled()` holds iff no mode is still visible at ε (`settle.rs:76–81`, `:192–196`). The stop is **observer-relative**, and it says so. In role, that is what a widening is: a declared precision that forces a terminating answer (the analogy is (c); the mathematics is (a)).

**Proposition 2.3 (a, elementary): the stop time is known before the run.** Expand the section in L's eigenbasis, `ψ = Σ_k c_k v_k`. Mode k is moving at time t iff `|c_k| e^{-λ_k t} ≥ ε`. So `settle(·, t, ε)` reports no motion iff `t > t*`, where

```
t*(ψ, ε) = max { ln(|c_k|/ε) / λ_k  :  λ_k non-harmonic, |c_k| ≥ ε }     (t* = 0 if the set is empty)
```

For a connected graph with a generic start, `t*` is dominated by the Fiedler mode: `≈ ln(|c_2|/ε)/λ_2`. **This is the literal, verified form of "constricted before being run":** for a linear dissipative loop, *when* the recursion stops is read off the spectrum before the flow is computed. §6 S4 tests it.

### §2.3 The walls are not the vows; the wall is one precondition, and the vows are its theorems

**(src)** `kleinos` checks four properties *after* building the composed graph: `Sovereignty`, `EmergentThird`, `FiedlerRise`, `FusionRefusal` (`kleinos.rs:69–80`, `:537–560`). Checking a result after the run is a **postcondition**, a checkpoint. A wall is a **precondition** on inputs that makes the postconditions provable before the run.

**(src)** `compose_candidate` (`kleinos.rs:465–514`) does three things:

- takes the union of the vertex and edge sets;
- adds the complete bipartite bridge `V(a) × V(b)`;
- adds one vertex `e = max(V(a) ∪ V(b)) + 1` adjacent to everything.

**Proposition 2.4 (a, proof inline; (num) checked).** Let a and b be graphs with no isolated vertices on **disjoint** vertex sets, of sizes `n_a, n_b ≥ 2`. Then the composed graph is the join `a ∨ b ∨ K₁`, and

```
λ₂(a ∘ b) = 1 + min( n_a + n_b ,  λ₂(a) + n_b ,  λ₂(b) + n_a )  ≥  max(λ₂(a), λ₂(b)) + 1.
```

In particular FiedlerRise holds **strictly, with margin ≥ 1**.

*Proof.* For a join `G₁ ∨ G₂`, the Laplacian is `[[L₁ + n₂I, −J], [−J, L₂ + n₁I]]`. Any eigenvector of `L₁` orthogonal to 𝟙 and supported on `V₁` is an eigenvector with eigenvalue `λ_i(L₁) + n₂`, since `J` annihilates it. The same holds symmetrically for `G₂`. The remaining two eigenvalues are 0 (the all-ones vector) and `n₁ + n₂` (the vector `(n₂𝟙, −n₁𝟙)`). Apply this to `(a ∨ b) ∨ K₁`. The minimum is ≥ `max(λ₂(a), λ₂(b))` because `λ₂(H) ≤ |V(H)|` for any simple graph H: `L(H) + L(H̄) = nI − J`, so `xᵀL(H)x ≤ n‖x‖²` on 𝟙^⊥. Hence `λ₂(a) ≤ n_a`, `λ₂(b) ≤ n_b`, and each of the three terms is at least `max(λ₂(a), λ₂(b))`. □

**(num)** Over 2,000 random disjoint pairs with `n_a, n_b ∈ [2, 6]`, the formula matched `numpy.linalg.eigvalsh` to `1e-9` with zero failures. Singleton operands (n = 1) drop the corresponding λ₂ term. That case was not sampled.

The other three vows follow the same way. Under disjointness:

- **Sovereignty** holds by union.
- **EmergentThird** holds because the new id `max + 1` is fresh.
- **FusionRefusal** (distinct operands) holds for disjoint non-empty operands.

**So under one refinement, `{(a, b) | V(a) ∩ V(b) = ∅}`, all four vows are theorems.** The only inputs that can violate them are overlapping operands. That is exactly where the emergent-id collision of `nonlinear-inference.md` §3.2 lives: `(a∘b)∘c` with `c ∋ max(V(a)∪V(b)) + 1`.

**Answer to "the walls of the labyrinth are what? The vows?"** Half right. **The wall is the disjointness refinement on inputs. The vows are what the wall guarantees.** A labyrinth with that wall needs no runtime vow check on the interior. It needs one check at the entrance. In Rust today, that check is a smart constructor. With Flux, it is a refinement on the arguments. Whether overlapping operands are *meant* to be composable is Alex's decision (Q-Mara-CF5).

---

## §3 How they compose (not average) onto the LAPACK floor

### §3.1 The asymmetry that makes composition possible (a)

In liquid inference, **dropping** a qualifier is always sound: the type gets weaker. **Keeping** one requires a proof. A decision procedure is therefore used asymmetrically:

- a **certifier** (sound for "valid") licenses keeping;
- a **refuter** (a concrete counterexample) licenses dropping *with a reason*;
- anything else is **remainder**.

Property tests are sound refuters and never certifiers, except by exhaustion of a finite domain. Numerical procedures can be certifiers on the fragments where their answer carries a margin or an exact certificate.

Dynamic invariant detection is the published ancestor of "tests strike candidate invariants": Daikon (Ernst et al. 2007, SCP; retrieved: dynamic detection of *likely* invariants). ICE learning (Garg, Löding, Madhusudan & Neider 2014, CAV; metadata) is the ancestor of combining examples with a verifier. The composition below is not new in kind. What is specific to prismqueer is the floor it runs on and the ternary type it reports in.

### §3.2 What the numerical floor can and cannot decide

| Question | Exact decision | On the LAPACK floor | Verdict |
|---|---|---|---|
| connectivity, `dim H⁰ = dim ker L` | union-find, O(E·α) | `dsyev` plus the `settle.rs` threshold `\|λ\| ≤ 1e-9·max\|λ\|` | **Sound and complete for unweighted graphs with n ≤ ~790** (Prop. 3.1). LAPACK adds the **margin** λ₂, not the decision. |
| do two heat flows commute? | `[L_A, L_B] = 0`, exactly in **integers** for unweighted graphs | singular values of the commutator, three-way classification (`commutator.rs:101–122`) | Exact on integers. On floats: sound outside the band, `Partial` inside it. For n ≤ 5 the band is empty (§1.2 fact 5). |
| where must recursion iterate? (directed cycles) | topological sort, O(V+E) | directed adjacency A is nilpotent iff the graph is acyclic | **Do not decide this with floating eigenvalues** (Prop. 3.2). The symmetric Laplacian cannot see it at all. |
| entailment between conjunctions of linear **real** inequalities | LP feasibility, polynomial time (Karmarkar 1984; metadata) with a Farkas certificate | LP in floats | A floating LP is not a proof. Its certificate must be re-checked in rationals. Not LAPACK in any case. |
| polynomial non-negativity / invariants | SOS via SDP (Parrilo 2003; metadata). Cousot 2005 uses SDP and Lagrangian relaxation for invariance and termination (metadata) | interior-point SDP in floats | Sound only after validation. Roux, Voronin & Sankaranarayanan (SAS 2016; retrieved) exist because numerical SDP output *"can lead to invalid proofs"*. |
| linear **integer** arithmetic, uninterpreted functions, disjunction | SMT, or proof (FLEX), or e-graphs (Scala) | none | ILP feasibility is NP-complete (classical; Papadimitriou 1981 retrieved at metadata level). **No polynomial-time numerical procedure decides QF_LIA entailment unless P = NP.** This stays symbolic. |

**Proposition 3.1 (a, elementary): `settle`'s kernel threshold is exact for small unweighted graphs.** For a connected simple graph with n vertices and diameter D, `λ₂ ≥ 1/(n·D) ≥ 1/(n(n−1))`.

*Proof.* Take `x ⊥ 𝟙` with `‖x‖ = 1`. Then `max x − min x ≥ 1/√n`. Join the argmax and the argmin by a shortest path of length ℓ ≤ D. By Cauchy–Schwarz, `xᵀLx ≥ Σ_path (x_u − x_v)² ≥ (1/√n)²/ℓ`. □

Also `λ_max ≤ 2·d_max ≤ 2(n−1)` (Gershgorin). The threshold is at most `2e-9·(n−1)`. dsyev's approximate eigenvalue error is `EPSMCH·‖A‖` (LAPACK Users' Guide, symmetric eigenproblem error bounds; retrieved), about `4.4e-16·(n−1)`, which is negligible. The harmonic test is therefore exact whenever `1/(n(n−1)) > 2e-9·(n−1)`, that is, **n ≤ 793**. Beyond that, a slow mode can be misread as harmonic, and the misreading **fails open**: it reports "settled". Mohar (1991, Graphs Combin. 7:53–64, DOI 10.1007/BF01789463; metadata only, theorem not re-read) is the conventional sharper bound. Nothing leans on it.

**Proposition 3.2 (a, elementary).** A digraph is acyclic iff its adjacency matrix A is nilpotent, iff every eigenvalue is 0.

*Proof.* `(A^k)_{ii}` counts closed walks of length k. A cycle of length c makes `(A^{mc})_{ii} > 0` for all m. With no cycle, `A^n = 0`. □

Nilpotent matrices are **defective**. The n×n Jordan block with ε placed in its corner has characteristic polynomial `λⁿ − ε`, so its eigenvalues are `ε^{1/n}` in modulus. A rounding error of 1e-16 in a 16×16 block moves the eigenvalues to about 0.1. The spectral criterion is exact in exact arithmetic and **useless in floating point**. Use an integer topological sort. This is the cleanest example of the general rule:

> **LAPACK is the right instrument when the answer is a well-conditioned quantity with a margin. It is the wrong instrument when the answer is an exact structural fact with no margin.**

### §3.3 Testing mirror's "replace the decision procedure (SMT) with spectral analysis" hard

The claim is in `liquid-types-for-mirror.md` §0 and §4.5: *"adopt Liquid's INFERENCE FRAMEWORK (predicate abstraction from qualifier templates) but replace the DECISION PROCEDURE (SMT) with spectral analysis."* Six findings.

1. **§5.2: "if λ₁ > 0, … there EXISTS a configuration where all properties are simultaneously satisfiable". False.** Take properties P and ¬P joined by one correlation edge. The graph is connected, so the spectral gap is positive, and the property set is unsatisfiable. Connectivity is a fact about the **constraint graph**. Satisfiability is a fact about its **models**. (a.)
2. **§5.2: "λ₀ = 0 (always, for the Laplacian)". Not for this matrix.** `L_P` is defined with property losses on the diagonal and `−correlation` off it. Its row sums are not zero, so it is not a graph Laplacian and 𝟙 is not in its kernel. With negative correlations it need not be positive semidefinite. (a.)
3. **§5.4 step 5: `t.verdict = project(loss_profile, t)`. This is the averaging step.** Projecting a term onto k eigenvectors gives a linear combination of their coordinates: one weighted sum per term. It collapses a k-dimensional spectral embedding into one number per term. That is exactly the move the operant condition forbids, and it is the step the spec calls its novelty. (a as linear algebra; the design verdict applies the operant condition.)
4. **§5.3 rests on a misattributed and over-read citation.** Authorship: see §0.6. Content, per the retrieved abstract of arXiv:2504.16206:
   - a spectral CSP sparsifier approximately preserves the *spectral energy of fractional assignments*, for *field-affine* Boolean CSPs;
   - the paper proves a Cheeger-type inequality for even-arity XOR CSPs;
   - it lists efficient computation of its new eigenvalue as open.

   It does not decide the satisfiability of refinement implications. It does not support *"verify a SUBSET of properties and get guarantees about ALL properties"*.
5. **Complexity.** Eigendecomposition is polynomial. Liquid-type refinements over QF_UFLIA include integer arithmetic, and ILP feasibility is NP-complete. A spectral procedure therefore cannot be a **complete** replacement unless P = NP. The spec's own §9.2 already concedes *"Spectral analysis is approximate"*. The concession and the headline cannot both stand.
6. **What survives.** HM plus constraint generation (Steps 1–2) can be adopted as is. The Fiedler vector as a *localization heuristic* for failures is reasonable (c). **The decision procedure is not replaced. It is preceded.**

### §3.4 The composition: a ternary cascade

**Definition 3.3.** A *decider* for a property type `P` is a map `P → Imperfect<Proof, Refutation, Remainder>`. The *cascade* `D₁ ▷ D₂` runs `D₁`. A `Success` or `Failure` is final and carries its certificate. `Partial(r)` passes `r` to `D₂`.

**Proposition 3.4 (a, elementary).**

- If every stage is sound (never certifies a false claim, never refutes a true one), the cascade is sound.
- `▷` is associative.
- Two sound stages never return *contradictory* definite verdicts on the same input. A contradiction is therefore a **proof that some stage is unsound**, which makes it a falsifier that costs nothing to run.

Averaging would weight `D₁`'s confidence against `D₂`'s. The cascade never does. A definite answer is kept whole with its certificate. An uncertain one is passed up whole, with its remainder. That is compose-not-average made operational.

**The prismqueer cascade (c, design):**

```
walls (types, smart constructors: free, before the run)
 ▷ exact discrete checks (union-find, integer commutator, topological sort)
 ▷ LAPACK with margin (Success/Failure only outside the ambiguous band)
 ▷ property-based refuter (Failure with a shrunk, replayable buffer)
 ▷ remainder → SMT / FLEX-style proof / a subject's Choice
```

**Cost (c, to be measured, not assumed).** Each stage runs only on its predecessor's remainder. The cascade's cost is dominated by the *fraction* of properties that reaches the expensive stages. §6 S10 measures that fraction.

**"Where the silicon settles on the shape of the thing".** The shape is whatever the first stage that can decide it with a margin decides. Everything else stays `Partial` until something with more information arrives. That is Reed's loop structure, re-derived from decidability rather than asserted.

---

## §4 Beam as a causal Merkle DAG: testing Reed's proposal 1

### §4.1 What exists (src)

- `Beam` is a one-step carrier with a loss slot fixed per pipeline. `Optic` keeps `source: Option<In>`, which is one predecessor, not a history (`beam.rs:115–122`). The `TraceBeam` its docstring describes (`beam.rs:112–114`) does not exist.
- `MerkleTree` points **forward** to owned `children`, *"Sorted by Oid"* (`merkle.rs:10–22`). It is a tree, with no sharing, so it cannot represent a node that has two parents.
- `pq::filter` names `Back` and `Forward` (`pq/filter.rs:14–16`).
- **(src, verified on this repo)** A git commit object contains `parent <oid>`, plus author and committer lines with **Unix timestamps**, plus a signature (`git cat-file -p HEAD`).

### §4.2 Claim by claim

**(1a) Node Oid = H(step, parents' Oids) with causal parents. Two orders of independent steps give identical nodes. Right, with conditions.**

**Proposition 4.1 (a, given a sound independence relation I, `nonlinear-inference.md` Def. 2.4).** Let each node's parents be the set of its *immediate dependence predecessors* (the maximal earlier events that it does not commute with). Hash the parents as a sorted set. Then any two linearizations of one trace build the same set of nodes. In particular they have the same frontier.

*Proof.* The parent set of an occurrence is determined by the dependence poset, which is linearization-invariant (Def. 2.2 there). Induct on the poset from its minimal elements. □

This is a Merkle clock: a *"Merkle-DAG … encoding causality information … can replace traditional logical clocks like vector clocks"* (arXiv:2004.00107, retrieved). The conditions are those of `nonlinear-inference.md` §2.5: the Oid of I belongs in the preimage, or I must be fixed per store, and the parents must come from a **sound** I.

**"Like git": no.** Git's parent is the *previous commit in the author's order*, and the commit hashes wall-clock timestamps. Two orders of independent commits give two different hashes in git. The proposal is a Merkle clock, not git, and the difference is exactly the property it wants.

**(1b) Fracture = a node with several children. Correct as a definition, but invisible from the node.** A back-pointing node does not know its children. Fan-out is a property of the **store** and needs a reverse index. **Fracture ≠ `Reality::Fractured`** (src: `reality.rs:29–36`, *"disconnected components (H^0 > 1)"*). A split keeps the history DAG connected: `H⁰ = 1`, one component with a vertex of out-degree > 1. The DAG analogue of `Fractured` is **several roots with no common descendant**, meaning histories that never met. They are different objects under names that are one letter apart. (Q-Mara-CF2.)

**(1c) Join = a node with several parents (settle). The node is right; the name is not.** A multi-parent node is a **merge**. `settle` is unary heat flow on one graph (src: `settle.rs:155–160`; `nonlinear-inference.md` §3.2). A merge may be *followed* by a settle. They are two events.

**(1d) Fractal = a node's step is itself a Beam. Sound.** A step's Oid can be the root Oid of an inner DAG. Hash-acyclicity makes the nesting well-founded: under collision resistance, an Oid cannot contain itself. **The recursion stops at leaf steps.** One condition: an inner DAG's read/write footprint must be *lifted* to the outer step, as the union of the inner footprints. Otherwise the outer I is unsound.

**(1e) Unknown-parent root = the adjoined absorbing element that answers the Loss-law violation. Wrong.** An absorbing ⊤ satisfies `⊤·x = ⊤`: it erases everything combined after it. An unknown-origin root is the opposite. Steps appended to it are **kept**. Making it absorbing would destroy known history, which is premature collapse by definition. The two meanings separate cleanly:

| Object | Law | Meaning | Where it already exists |
|---|---|---|---|
| ⊤ (adjoined absorbing) | `⊤·x = x·⊤ = ⊤` | the measurement failed; nothing downstream is known | `Motion { opaque: true }` (src: `settle.rs:88–106`, lawful per `loss_laws.rs`, ran) |
| opaque root | a **generator**, no special law | nothing is known *before* here; everything after is known | not built; one more `Node` variant |

**(1f) Causality decided by the commutator: "commute ⇔ neither is the other's parent". False in one direction, and the other direction is weaker than intended.**

- *parent ⇒ non-commuting*: true by construction (parents are dependence predecessors).
- *non-commuting ⇒ parent*: **false.** Take `a: x:=1`, `b: y:=x`, `c: x:=2`. Then a and c do not commute (both write x). But c's immediate dependence predecessor is b, and a is only an **ancestor** of c.
- *ancestor ⇒ non-commuting*: **false.** Take `a: x:=1`, `b: y:=x`, `c: z:=y`. Then a and c commute (disjoint footprints) yet `a < b < c`.

The correct chain is `parent ⊊ dependent ⊊ comparable`, with both inclusions strict in general (a, elementary; counterexamples above). **The commutator decides the pairwise relation D. The DAG is its transitive reduction on occurrences.** Neither is the other.

The graph-Laplacian commutator decides D only for **linear flows with symmetric generators**: 3a33319's theorem (src: `settle.rs:385–391`), ran, 256/256 agreement. For nonlinear steps such as `kleinos`, no theorem links the two (`nonlinear-inference.md` T7, still open).

### §4.3 Against trace theory and refinement types

**Trace theory (a).** With causal parents, the DAG *is* the dependence graph of the trace (in Hasse form). Foata layers are the depth levels. The frontier's Oid and `H(oid(I) ‖ FNF)` from `nonlinear-inference.md` Prop. 2.6 are **two content addresses for one object**. Ship one. Keep the other as a cross-check test.

**Refinement types (a for Flux's mechanism; c for the consequence).** "Declared versus measured causality" has a third option that Rust already provides. Flux uses ownership to track mutable locations (retrieved). The borrow checker proves at compile time that two `&mut` borrows are disjoint. **Steps that hold disjoint exclusive borrows have disjoint write footprints, which is a compile-time wall for independence.** The confounds are exactly the hidden shared resources:

- interior mutability (`Cell`, `RefCell`, atomics);
- globals (`SAMPLE_COUNTER`, `flux.rs:480`);
- `SystemTime` (`flux.rs:528`);
- `unsafe`.

So: declare causality through ownership, and measure it by the state-level commutation test (`nonlinear-inference.md` T6) on samples. Declared-only is unsound in the presence of hidden state. Measured-only is expensive.

---

## §5 The consumer-hardware claim, tiered

Alex's claim: *"The inference would be constricted before being run which is what makes it computable on consumer hardware compared to token-based inference."* It splits into four sub-claims.

| # | Sub-claim | Tier | Evidence |
|---|---|---|---|
| C1 | Constraints can be applied before the run | **(a)** | Refinement types (§2.1). Knowledge compilation: *"target compilation languages"* that support queries in polytime (Darwiche & Marquis, arXiv:1106.1819, retrieved). |
| C2 | Applying them makes the run cheap | **(a), conditional** | Knowledge compilation trades **succinctness** against polytime queries (retrieved). Compiled forms can be exponentially large, so cost moves from runtime to compile time and to space. Arc consistency (Mackworth 1977; metadata) prunes polynomially and does not solve. Pruning does not change worst-case class: ILP stays NP-complete (§3.2). |
| C3 | Cheaper than token-based inference | **(c)** | Constraining a token model does **not** make it cheaper. Grammar-guided generation *"adds little overhead to the token sequence generation process"* (Willard & Louf, arXiv:2307.09702, retrieved): the constraint masks the vocabulary, and the forward pass is unchanged. Language-model loss follows power laws in model size, data, and compute (arXiv:2001.08361, retrieved), so capability costs parameters. A system that is cheap because it is small inherits a small system's capability, **unless structure substitutes for parameters.** That substitution is the actual hypothesis. Nothing in the corpus or the retrieved literature measures it. |
| C4 | The LAPACK floor is small | **(a) as arithmetic, with a corpus contradiction** | `2026-07-25-sub-turing-geometric-compiler-floor.md:120` says a Gram matrix of *"at most 3125 × 3125 (well below the FLANG floor of n ≤ 16 …)"*. 3125 is not below 16. `:305` states `n ≤ 16`. Both cannot hold for the same object. At n = 3125, dense dsyev is on the order of `n³ ≈ 3·10¹⁰` flops: seconds on a laptop (c, estimate; not measured), but not a constant 4,096. |

**The corpus's own principle cuts against unlimited constriction (c).** Requisite variety (`practice/insights/math/numerics/requisite-variety-optimization.md`; corpus-internal, not retrieved externally): a regulator constricted to low variety can only regulate low-variety disturbance. Constriction buys computability exactly as far as the task's variety is small.

**The falsifier the claim needs (c).** Pick one task with a known answer that both a small token model and a prismqueer pipeline can attempt. Measure answers, wall time, and energy on one machine. Until that exists, C3 is a hypothesis, not a floor. Nothing in §6 depends on it.

---

## §6 What ships in prismqueer

Placement: new code goes in `prismqueer/src/flux.rs` (the `pillar` module), `prismqueer/src/spectral/settle.rs`, `prismqueer/src/decide.rs` (new), and `prismqueer/src/time/node.rs` (new, beside `time/` from `nonlinear-inference.md` §6). All sampled inputs come from `Sample::from_bytes`.

Each item states what it measures, what supports the claim, what refutes it, and the confound. Each then answers the standing question: **(NL)** the smallest nonlinear falsification; **(LIN)** the linear test that encodes it; **(STOP)** where the recursion stops moving.

### §6.1 Types

```rust
// prismqueer/src/flux.rs, pillar

/// What a property run leaves behind. Nothing is folded into a single verdict.
pub struct Counterexample { pub buffer: Vec<u8>, pub verdict: PropertyVerdict }
pub struct Coverage { pub budget: usize, pub distinct: usize, pub singletons: usize }

/// `forall` that keeps the failing buffer. Exhaustive domains may return Success.
pub fn check<T: Arbitrary, F>(seeds: impl IntoIterator<Item = Vec<u8>>, f: F)
    -> Imperfect<(), Counterexample, Coverage>
where F: FnMut(T) -> PropertyVerdict;

/// Shortlex-minimal buffer that still fails. Terminates: shortlex on finite buffers is well-founded.
pub fn shrink<F: FnMut(&mut Sample) -> bool>(buffer: Vec<u8>, still_fails: F) -> Vec<u8>;
```

```rust
// prismqueer/src/spectral/settle.rs

/// Prop. 2.3: when the flow will stop moving at resolution eps, read from the spectrum.
pub fn stop_time(graph: &Graph, section: &Section, eps: Resolution) -> Imperfect<Time, Wall, Motion>;
```

```rust
// prismqueer/src/decide.rs

/// Def. 3.3. Success = proof, Failure = refutation, Partial = remainder for the next stage.
pub trait Decider<P> {
    type Proof; type Refutation;
    fn decide(&self, p: &P) -> Imperfect<Self::Proof, Self::Refutation, P>;
}
pub struct Cascade<A, B>(pub A, pub B);        // A ▷ B

/// Exact commutation for unweighted graphs, in integer arithmetic.
pub struct IntegerCommutator;
/// A deterministic map iterated to its first repeated Oid.
pub enum Orbit<S> { Fixed(S), Cycle { period: usize, states: Vec<S> } }
pub fn iterate<S: Addressable, F: FnMut(&S) -> S>(x0: S, f: F, budget: usize)
    -> Imperfect<S, Orbit<S>, usize>;
```

```rust
// prismqueer/src/time/node.rs  (proposal 1, minimal and corrected)

pub enum Node {
    Step { step: Oid, parents: BTreeSet<Oid> },   // parents = immediate dependence predecessors
    OpaqueRoot { note: Oid },                     // "nothing known before here". A generator, not ⊤.
}
pub struct Store { nodes: BTreeMap<Oid, Node>, children: BTreeMap<Oid, BTreeSet<Oid>> } // reverse index = fan-out
```

### §6.2 Items, in red-first order

**S1 `forall_failure_carries_a_replayable_buffer`** (`flux.rs`)
*Measures:* a property that fails on exactly one value in a small domain returns `Failure(Counterexample { buffer })`, and `T::arbitrary(&mut Sample::from_bytes(buffer))` reproduces the failing value.
*Supports:* replay equality. *Refutes:* today's `forall`, which returns `PropertyVerdict` only. **Red on day one.**
*Confound:* `Sample::new()` in any path. The test must use explicit seeds.
**(NL)** a universally quantified property falsified by one point. **(LIN)** byte equality of the replayed value. **(STOP)** the first failure, or the end of the seed iterator.

**S2 `smaller_buffer_draws_no_larger_integer`** and **`shrink_is_idempotent_and_still_fails`** (`flux.rs`)
*Measures:* (i) for sampled buffers and a byte decrement, the drawn integer does not increase. (ii) `shrink(shrink(b)) == shrink(b)`, and the result still fails.
*Supports:* (i) after `draw_integer` is made monotone, for example by rejection sampling on the high bits instead of `%`. (ii) always, once `shrink` exists.
*Refutes:* (i) is **red on day one** against `raw % range` (§1.2 fact 4).
*Confound:* monotonicity per draw does not imply monotonicity of the whole generated structure. Report which draws are monotone.
**(NL)** reduction order versus value order: two orders on different sets that must agree. **(LIN)** `≤` on one pair. **(STOP)** a shortlex fixed point, guaranteed by well-foundedness.

**S3 `settle_threshold_is_exact_below_n_793`** (`settle.rs`)
*Measures:* for sampled connected unweighted graphs with n ≤ 64, `harmonic_dimension == 1`, and the computed `λ₂ ≥ 1/(n·diam)`. Extend the existing `settled_dimension_agrees_with_tension_components` from fixed cases to a property.
*Supports:* Prop. 3.1 on every sample. *Refutes:* one graph whose λ₂ falls under the threshold.
*Confound:* weighted graphs are outside Prop. 3.1. The test must refuse them rather than silently pass.
**(NL)** a discrete invariant (component count) against a continuous one (the eigenvalue near zero). **(LIN)** integer equality. **(STOP)** closed form, no iteration.

**S4 `stop_time_predicts_settling`** (`settle.rs`)
*Measures:* with `t* = stop_time(g, ψ, ε)`, `settle(g, ψ, t*·(1+δ), ε)` has `moving` empty, and `settle(g, ψ, t*·(1−δ), ε)` has it non-empty, for δ = 1e-3.
*Supports:* both, on sampled graphs and sections. *Refutes:* either side failing.
*Confound:* ties between modes with equal `ln(|c|/ε)/λ`, and `|c_k|` within round-off of ε. Skip samples within `1e-9` of the boundary and **count the skips**.
**(NL)** none; heat flow is linear. This is the calibration item. **(LIN)** emptiness of `moving`. **(STOP)** at `t*`, **known before the run**. This is the verified form of "constricted before being run".

**S5 `rewiring_is_exhaustive_for_n_le_4`** (`settle.rs`, module `rewiring_property`)
*Measures:* every covered graph pair on 2–4 vertices, 1,698 pairs in total: graph commutator zero iff the flows commute.
*Supports:* zero disagreements. The function returns **`Success`**, because exhaustion proves the claim. *Refutes:* one disagreement.
*Confound:* none new. The sampled `n ∈ [5, 6]` arm stays `Partial`.
**(NL)** non-commutation of two operator exponentials. **(LIN)** entrywise comparison on basis sections. **(STOP)** exhaustion of a finite domain. **This is the answer to "where does the recursion stop moving for the test macros":** at exhaustion when the domain is finite, which yields `Success`; otherwise at budget, which yields `Partial(Coverage)`.

**S6 `integer_commutator_decides_and_lapack_agrees`** (`decide.rs`)
*Measures:* `IntegerCommutator` computes `L_A L_B − L_B L_A` in `i64`, which is exact. Over the S5 domain plus sampled n ≤ 12, its zero/nonzero verdict never contradicts `CommutatorSpectrum::of`'s `Success` verdicts.
*Supports:* no contradiction (Prop. 3.4). *Refutes:* one contradiction, which proves the float instrument unsound.
*Confound:* `Partial` from the float instrument is not a contradiction. Count it separately.
**(NL)** the same. **(LIN)** a Boolean agreement table. **(STOP)** closed form.
*What it shows:* the LAPACK floor is not needed to **decide** commutation of unweighted graphs. It is needed to **measure how much** they fail to commute. Compose, not average: the exact decider decides, and the float instrument keeps the magnitude spectrum as information.

**S7 `vows_hold_by_construction_on_disjoint_operands`** and **`compose_lambda2_matches_join_formula`** (`spectral/kleinos.rs`)
*Measures:* (i) for sampled vertex-disjoint pairs with `n_a, n_b ≥ 2`, `kleinos(a, b)` is `Success`. (ii) Its λ₂ equals `1 + min(n_a+n_b, λ₂(a)+n_b, λ₂(b)+n_a)` within `1e-9·n`.
*Supports:* both (Prop. 2.4; (num) 2,000/2,000). *Refutes:* one failure.
*Confound:* singleton operands, and the `max + 1` id when operands overlap. Exclude both and report them in a second, overlapping arm. That arm is **expected** to find vow failures: the emergent-id collision of `nonlinear-inference.md` T11.
**(NL)** a spectral inequality that holds for all inputs of unbounded size. **(LIN)** one equality per sample, which a theorem turns into a universal claim. **(STOP)** closed form. The wall (disjointness) moves the recursion to the entrance.

**S8 `targeted_search_finds_the_band_only_where_it_exists`** (`flux.rs`, plus `spectral/commutator.rs`)
*Measures:* a hill-climber on the buffer, with utility = distance of `min σ/scale` from the band. Two arms:
- (i) unweighted graphs with n ≤ 5;
- (ii) weighted graphs whose edge weights are drawn from the buffer.

*Supports:* (i) finds nothing (§1.2 fact 5, exhaustively known), and (ii) finds band hits that blind sampling at the same budget misses.
*Refutes:* (i) finding a hit, which contradicts the enumeration. Or (ii) targeted doing no better than blind.
*Confound:* weighted graphs need a weighted `Graph`. Today's `Graph` is unweighted (`kleinos.rs:132`). Arm (ii) waits on that type.
**(NL)** proposal 2's central claim: that steering beats blindness. **(LIN)** hit counts at equal budget. **(STOP)** budget, which yields `Partial(Coverage)`.

**S9 `iteration_reports_cycles_not_only_fixed_points`** (`decide.rs`)
*Measures:* `iterate(G, complement, budget)`. On labelled graphs, complement is an involution with no fixed point for n ≥ 2, since a graph and its complement partition the edge set. The result must be `Partial(Orbit::Cycle { period: 2 })`. A contracting map, for example repeated `settle` quantized to a grid, must return `Success(Fixed)`.
*Supports:* both. *Refutes:* a naive "loop until the Oid repeats as a fixed point" hanging, or reporting a fixed point.
*Confound:* budget exhaustion before the first repeat. It must be `Failure(budget)`, never `Success`.
**(NL)** a deterministic map on a finite set is eventually periodic (pigeonhole). A fixed point is only the period-1 case. **(LIN)** Oid equality at step 2. **(STOP)** the first repeated Oid, after at most |states| + 1 steps, or the budget.
*Why it matters:* Reed's loop 3, *"fixed point by graph Oid"*, must detect **cycles**. An oscillating compose is a tension that does not settle, and it should surface as `Partial` with the cycle as its loss.

**S10 `cascade_never_overturns_a_definite_verdict`** and **`cascade_remainder_fraction_is_reported`** (`decide.rs`)
*Measures:* `IntegerCommutator ▷ CommutatorSpectrum ▷ sampled-flow refuter` on mixed inputs. (i) No definite verdict from a later stage contradicts an earlier one. (ii) The fraction of inputs that reaches each stage.
*Supports:* (i) holds (Prop. 3.4). (ii) is reported, not asserted.
*Refutes:* a contradiction.
*Confound:* for unweighted inputs the first stage decides everything, so the cascade is degenerate. Add the weighted arm once S8's type exists.
**(NL)** composition of partial deciders. **(LIN)** equality of definite verdicts. **(STOP)** at the first definite stage, per input.

**S11 `independent_steps_give_one_frontier`**, **`ancestor_does_not_imply_non_commuting`**, **`non_commuting_does_not_imply_parent`**, **`opaque_root_keeps_what_follows`**, **`fracture_is_not_fractured`** (`time/node.rs`)
*Measures:*
- two orders of `a: x:=1`, `b: z:=0` yield equal node Oid sets;
- the `x/y/z` counterexamples of §4.2 (1f), with footprints from `Touches` (`nonlinear-inference.md` §6.1);
- `OpaqueRoot` followed by k steps keeps k nodes, while ⊤ (`Motion::total`) absorbs;
- a split DAG has one weakly connected component.

*Supports:* all five. *Refutes:* any. The second and third refute Reed's "commute ⇔ neither is the other's parent" if they pass.
*Confound:* footprints must be honest. Hidden state breaks the first test silently. Include one step that touches `Sample::new()` and assert that its footprint marks the global counter.
**(NL)** trace equivalence: many words, one object. **(LIN)** set equality of Oids. **(STOP)** fractal nesting stops at leaf steps, well-founded by hashing.

**S12 `rotation_average_settles_to_the_heat_kernel`** and **`rotation_never_settles_pointwise`** (`spectral/settle.rs`)
*Measures:*
- (i) `‖e^{itL}ψ‖ = ‖ψ‖` for all t: the isometry never settles.
- (ii) The time average `(1/T)∫₀ᵀ e^{itL}ψ dt` approaches `P_{ker L} ψ`, which is `settle(·, ∞)`, with error ≤ `2‖ψ⊥‖/(λ₂T)`. Heat flow has error `e^{−λ₂t}‖ψ⊥‖`.

*Supports:* both bounds on sampled graphs. The proof is elementary: `|(1/T)∫e^{iλt}dt| ≤ 2/(|λ|T)`.
*Refutes:* either bound violated.
*Confound:* `spectral::rotation` is **Kuramoto** phase dynamics (src: `rotation.rs:1–40`), which is nonlinear and synchronizes. It is not `e^{itD}`. Do not test one under the other's name.
**(NL)** dissipative versus isometric recursion: one stops by itself, the other never does. **(LIN)** a norm equality and a norm bound. **(STOP)** for heat, at `t*` (S4). For rotation, **never**, unless the observer imposes an averaging window T. Reed's loop 2 ("ask") stops only because someone decides to stop listening.

**S13 `loss_laws_are_sampled_and_shrunk`** (`tests/loss_laws.rs`)
*Measures:* the T0 audit (e584b9c) run through S1 and S2 instead of hand-picked triples. Report the **shrunk** minimal violating triple for `f64` and `ScalarLoss`.
*Supports:* the same violation set as today's tripwire. *Refutes:* a new violation kind, or a missing one.
*Confound:* `f64` NaN. `NaN != NaN` makes every law "fail". Exclude NaN or define the equality.
**(NL)** associativity failing at round-off. **(LIN)** bitwise equality of two groupings. **(STOP)** at the shrunk counterexample.

**Order:** S1 → S2 → S13 (first consumer) → S3 → S4 → S5 → S6 → S7 → S9 → S10 → S11 → S12 → S8 (after a weighted `Graph` exists).

S1 and S2 go first because every later item reports counterexamples through them. S3 and S4 pin the floor that S5 through S7 stand on. S8 is last because its informative arm needs a new type.

### §6.3 What would make the whole construction false

> **prismqueer can settle a property in one of three honest ways: proof with a certificate, refutation with a shrunk replayable counterexample, or a named remainder. Its numerical floor never contradicts its exact floor (S6, S10). Its walls make the vows theorems (S7). Its loops report cycles and budgets instead of pretending to settle (S9, S12).**

- If S6 or S10 finds a contradiction, the LAPACK stage is unsound where it claimed a margin.
- If S7's disjoint arm fails, Prop. 2.4 or the code is wrong, and the vows are not walls.
- If S8 (ii) shows no gain, proposal 2 reduces to naming.

---

## §7 What mirror stands on

**M1 (a).** Liquid inference's framework can be adopted as is: HM, syntax-directed constraint generation, a finite qualifier set, and iteration bounded by lattice height (Prop. 2.1). Mirror's property library as Q is a sound design (c) whose termination bound is `|Q|·|K|`.

**M2 (a → decision).** Step 3 is a **cascade** (§3.4), not a replacement. Walls, then exact discrete checks, then LAPACK with margin, then a PBT refuter, then SMT or proof for the remainder. FLEX's acyclic/cyclic split says where the expensive stage is needed: only on the cycles of the constraint graph.

**M3 (a). Corrections owed to `liquid-types-for-mirror.md`.**

- §5.2: connectivity ≠ satisfiability, and `L_P` is not a Laplacian.
- §5.4 step 5: a projection is an average.
- §4.4: computable-real equality is undecidable, and monotone ≠ terminating.
- §5.3 and §10: three attribution errors (§0.6), plus an over-read of arXiv:2504.16206.

The same monotone ≠ terminating correction applies to `2026-07-18-witnessed-property-inference.md` §2.3.

**M4 (c).** The `---` seam: above it are qualifiers, the walls. Below it, the verdict should be `Imperfect<Proof, Counterexample, Remainder>`, never a scalar confidence. **(src)** `pillar::of_health` (`flux.rs:248–261`) maps the three-state enum `HolonomyHealth { Healthy, TooShallow, OverCutting }` to `Pass`, `Partial { confidence: 0.5, diagnostics }`, and `Fail`. The `0.5` is a constant: the scalar carries no information, and the diagnostic carries all of it. Drop the scalar or derive it from something measured. A related mismatch: `2026-07-18-witnessed-property-inference.md` §2.3 argues from *"the `HolonomyHealth` scalar decreases monotonically"*, but in the code `HolonomyHealth` is an enum, not a scalar. The termination argument is about a type that does not exist.

**M5 (a).** Where recursion must iterate is the SCC structure of a **directed** constraint graph (§2.2). The symmetric spectral stack cannot see it (Prop. 3.2 and `nonlinear-inference.md` §2.3.3). Mirror needs an integer topological pass before any spectral pass.

**M6 (a).** `2026-07-25-sub-turing-geometric-compiler-floor.md:120` versus `:305`: 3125 is not below 16. Pick the object that the `n ≤ 16` bound governs, and restate the other bound honestly as `O(n³)` at n = 3125.

---

## §8 What I do not know from inside; open questions for Alex

### §8.1 Where Alex's picture and Reed's proposals stand

| Claim | Verdict |
|---|---|
| PBG and liquid types can compose, not average, onto LAPACK | **Yes, as a cascade of partial deciders** (§3.4). Mirror's spec as written averages (§3.3 finding 3). |
| "settle on the shape of the thing" | **Yes where there is a margin**: `dim H⁰` for n ≤ 793 unweighted (Prop. 3.1), stop time from the spectrum (Prop. 2.3). **No** for exact structure without a margin (Prop. 3.2) or for integer arithmetic (§3.2). |
| The walls are the vows | **Half.** The wall is disjointness. The four vows are its theorems (Prop. 2.4). |
| The Beam is the linear observation of a nonlinear collapse | **(a) as trace theory:** an observed path is one linearization (`nonlinear-inference.md` §4.1). "Non-euclidean spectral-triple state space" is (c). Nothing here tests it. |
| Constricted before run ⇒ consumer hardware versus tokens | **(c).** Constraining a token model does not reduce its forward pass. Cheapness from smallness costs capability unless structure substitutes for parameters, and that is unmeasured (§5). |
| Reed 1: causal Merkle DAG | **1a right** (a Merkle clock, not git). **1b** right as a definition, but store-level, and ≠ `Fractured`. **1c** the node is a merge, not a settle. **1d** sound, with footprint lifting. **1e wrong** (an opaque root is a generator, not ⊤). **1f wrong** (`parent ⊊ dependent ⊊ comparable`). |
| Reed 2: fate as generator | **Right mechanism, prior art** (Zest, Hypothesis, Targeted PBT). Keep Fate outside the parser for replay. Sequence targeting then shrinking. The value is unproven until S8 (ii). |
| Reed 3: loop structure | **Right shape**, with two corrections. Loop 2 (rotation) never settles pointwise (S12). Loop 3 must detect cycles, not only fixed points (S9). |

### §8.2 Open questions for Alex

- **Q-Mara-CF1 (declared versus measured causality).** Lean: declare through Rust ownership (`&mut` footprints are a compile-time wall), and spot-measure with state-level commutation (`nonlinear-inference.md` T6). Declared-only is unsound under hidden state (`SAMPLE_COUNTER`, `SystemTime`, interior mutability).
- **Q-Mara-CF2 (Fracture versus `Reality::Fractured`).** They are not the same. A split is `H⁰ = 1` with out-degree > 1. `Fractured` is `H⁰ > 1`. Rename one. Suggest `Split` for the DAG node.
- **Q-Mara-CF3 (the Loss-law decision).** The collection carriers cannot honestly offer `total()`. Two options:
  - adjoin ⊤ per type, as `Motion` does with `opaque`;
  - split the trait: `Loss` keeps identity and associativity, and a subtrait `Absorbing: Loss` carries `total()`.

  Lean: split. `total()` is a claim, and a free monoid cannot make it. The unknown-parent root does **not** answer this (§4.2 1e).
- **Q-Mara-CF4 (f64 associativity).** Four options:
  - declare an ULP tolerance in the law tests;
  - compensated summation, which narrows the error and is still not associative;
  - exact carriers (fixed-point integers or rationals);
  - a weaker, named law ("associative to k ULPs").

  Lean: fixed-point integers for any loss that is *compared* across groupings, and a declared tolerance for losses that are only *reported*.
- **Q-Mara-CF5 (kleinos operand overlap).** Are overlapping vertex sets meant to compose? If not, make disjointness the wall (Prop. 2.4, S7). If so, the id collision needs a fresh-name supply that is global to the tower, not local to one call.
- **Q-Mara-CF6 (two rotations).** `e^{itD}` (linear, isometric) and `spectral::rotation` (Kuramoto, nonlinear, synchronizing) share a word. Which one is loop 2?
- **Q-Mara-CF7 (mirror spec corrections).** M3 and M6 touch ratified mirror documents. Who amends them, and does the misattribution count as a scar entry?
- **Q-Mara-CF8 (Fate inside or outside the parser).** Lean: outside, for replay (§1.3).

### §8.3 What I do not know

- Whether Fate's scheduling beats a fixed schedule. That is S8 (ii), and it needs a weighted `Graph`.
- Whether the band is reachable for unweighted n = 6. The n ≤ 5 minima (0.17, 0.048, 0.0093) suggest not, but I did not enumerate the 2¹⁵ graphs on 6 vertices.
- Whether any published system uses graph-Laplacian spectra as a decision procedure for refinement entailment. My arXiv title searches found none. The Kagi search endpoint was empty, so coverage is thinner than I would like.
- The theorem statements for three sources retrieved only at metadata level: Good 1953 (the estimator), Mohar 1991 (the λ₂ bound), and Papadimitriou 1981 (the membership proof). Nothing in the body depends on their content beyond what is proved inline or labelled classical.
- Whether CHC satisfiability over integer arithmetic is undecidable in the generality mirror would need. I believe so, but I did not retrieve a source, and nothing leans on it.
- Whether "structure substitutes for parameters" (§5 C3) has any published test. I did not find one.

---

## Coda: citation ledger

Routes:

- **arXiv**: Kagi summarizer on an arXiv abstract page.
- **arXiv-API**: arXiv search tool (title, authors, abstract).
- **CR**: Kagi summarizer on a Crossref DOI record.
- **CRq**: a Crossref bibliographic query, via the summarizer or WebFetch.
- **page**: Kagi summarizer or WebFetch on an author, publisher, or project page.

"Content" means the retrieved text stated what I use. "Metadata" means title, authors, venue, and year only.

| Reference | Route | Confirmed | Status |
|---|---|---|---|
| Claessen & Hughes 2000, *QuickCheck*, ICFP, DOI 10.1145/351240.351266 | CR + page (Tufts PDF mirror) | content | verified (ACM DL route returned empty) |
| MacIver & Donaldson 2020, *Test-Case Reduction via Test-Case Generation*, ECOOP, DOI 10.4230/LIPIcs.ECOOP.2020.13 | page (Dagstuhl, summarizer + WebFetch) | content + authors | verified (CR route returned empty) |
| MacIver, Hatfield-Dodds et al. 2019, *Hypothesis*, JOSS, DOI 10.21105/joss.01891 | CR | metadata | verified |
| Löscher & Sagonas 2017, *Targeted Property-Based Testing*, ISSTA, DOI 10.1145/3092703.3092711 | CR + page (proper-testing PDF) | content (UVs, HC, SA) | verified (ACM DL page returned cookie text) |
| Pike 2014, *SmartCheck*, Haskell Symposium, DOI 10.1145/2633357.2633365 | page (author PDF) + CRq | content | verified |
| Padhye, Lemieux, Sen, Papadakis & Le Traon, *Semantic Fuzzing with Zest*, arXiv:1812.00078 | arXiv + arXiv-API | content | verified (venue not surfaced) |
| Goldstein & Pierce, *Parsing Randomness*, arXiv:2203.00652 | arXiv-API | content | verified (author-page PDF route empty) |
| *Learn&Fuzz*, arXiv:1701.07232 | arXiv | content | verified, limited (authors not surfaced) |
| AFL, *technical_details.txt* (lcamtuf) | page | content | verified |
| Böhme, *STADS*, arXiv:1803.02130 | arXiv-API | content | verified |
| Good 1953, Biometrika 40:237, DOI 10.1093/biomet/40.3-4.237 | CR | metadata | verified, limited (estimator not re-read) |
| *Liquid Types*, PLDI 2008, DOI 10.1145/1375581.1375602 | page (ACM DL) | content | verified (author names not surfaced by this route) |
| Vazou, Seidel, Jhala, Vytiniotis & Peyton Jones 2014, *Refinement Types for Haskell*, ICFP, DOI 10.1145/2628136.2628161 | CR | metadata | verified |
| Lehmann, Geller, Vazou & Jhala, *Flux: Liquid Types for Rust*, arXiv:2207.04034 | arXiv + arXiv-API | content | verified (PLDI 2023 venue not surfaced) |
| Jhala & Vazou, *Refinement Types: A Tutorial*, arXiv:2010.07763 | arXiv + arXiv-API | content | verified |
| Borkowski, Vazou & Jhala, *Mechanizing Refinement Types*, arXiv:2207.05617 | arXiv-API | content | verified |
| Pavlinovic, Su & Wies, *Data Flow Refinement Type Inference*, arXiv:2011.04876 | arXiv-API | content | verified; **mirror spec's author list is wrong** |
| Vazou, Tanter & Van Horn, *Gradual Liquid Type Inference*, arXiv:1807.02132 | arXiv-API | metadata + abstract | verified; **mirror spec's author list is wrong** |
| Khan, Markopoulos, Lehmann & Jhala 2026, *Foundational Constraint Solving for Expressive Refinement Typing* (FLEX), arXiv:2607.12226 | arXiv + arXiv-API | content (zap/fix) | verified |
| Bovel, Kunčak & Odersky 2026, *First-Class Refinement Types for Scala*, arXiv:2605.08369 | arXiv-API | content (e-graph solver) | verified |
| Gamboa, Reese, Fonseca & Aldrich 2025, *Usability Barriers for Liquid Types*, PACMPL, DOI 10.1145/3729327 | CRq | metadata | verified |
| Cousot & Cousot 1977, *Abstract Interpretation*, POPL, DOI 10.1145/512950.512973 | CR | content (lattice model; construction or approximation of fixpoints) | verified (ACM DL and ENS PDF routes failed) |
| Cousot 2005, *… Lagrangian Relaxation and Semidefinite Programming*, VMCAI, DOI 10.1007/978-3-540-30579-8_1 | CR | metadata + topic | verified |
| Graf & Saïdi 1997, *Construction of Abstract State Graphs with PVS*, CAV, DOI 10.1007/3-540-63166-6_10 | CR | metadata | verified |
| Bjørner, Gurfinkel, McMillan & Rybalchenko 2015, *Horn Clause Solvers for Program Verification*, DOI 10.1007/978-3-319-23534-9_2 | CRq | metadata | verified |
| Parrilo 2003, Math. Program. 96:293, DOI 10.1007/s10107-003-0387-5 | CR | metadata | verified |
| Roux, Voronin & Sankaranarayanan 2016, SAS, DOI 10.1007/978-3-662-53413-7_21 (FMSD 2017, DOI 10.1007/s10703-017-0302-y) | CRq | content summary | verified |
| Karmarkar 1984, Combinatorica 4:373, DOI 10.1007/BF02579150 | CR | metadata | verified |
| Papadimitriou 1981, JACM 28:765, DOI 10.1145/322276.322287 | CR | metadata | verified, limited (theorem not re-read) |
| Khanna, Putterman & Sudan, *A Theory of Spectral CSP Sparsification*, arXiv:2504.16206 | arXiv + arXiv-API | content | verified; **mirror spec attributes it to Bafna, Bhatt, Khot & Minzer (no record found)** |
| Fiedler 1973, *Algebraic Connectivity of Graphs*, CMJ 23:298, DOI 10.21136/CMJ.1973.101168 | CR | metadata | verified |
| Mohar 1991, Graphs Combin. 7:53, DOI 10.1007/BF01789463 | CRq | metadata | verified, limited (bound not re-read; not leaned on) |
| LAPACK Users' Guide, *Error Bounds for the Symmetric Eigenproblem* (netlib `lug/node89`) | page + WebFetch | content (`EERRBD = EPSMCH·ANORM`) | verified |
| Mackworth 1977, *Consistency in Networks of Relations*, AI 8(1), DOI 10.1016/0004-3702(77)90007-8 | CRq | metadata | verified |
| Darwiche & Marquis, *A Knowledge Compilation Map*, arXiv:1106.1819 | arXiv-API | content | verified (JAIR venue not surfaced) |
| Willard & Louf, *Efficient Guided Generation for LLMs*, arXiv:2307.09702 | arXiv-API | content | verified |
| *Scaling Laws for Neural Language Models*, arXiv:2001.08361 | arXiv | content | verified, limited (authors not surfaced) |
| *Merkle-CRDTs*, arXiv:2004.00107 | arXiv | content | verified, limited (authors not surfaced) |
| Ernst et al. 2007, *The Daikon System*, SCP, DOI 10.1016/j.scico.2007.01.015 | CR | content summary | verified |
| Garg, Löding, Madhusudan & Neider 2014, *ICE*, CAV, DOI 10.1007/978-3-319-08867-9_5 | CR | metadata | verified |
| arXiv:2010.14763 (tried as the refinement tutorial) | arXiv | a Hogwild! SGD paper | **rejected; wrong ID, not cited** |
| "Bafna, Bhatt, Khot, Minzer 2025, ICALP" | arXiv-API + CRq | no matching record | **rejected** |
| CHC undecidability over LIA; Good–Turing estimator formula; Mohar's 4/(nD) | not retrieved | — | **unverified; not leaned on** |

**Count.** 40 references retrieved and cited. 32 were confirmed at content or metadata level with no caveat. 8 are marked "limited", meaning an author list, venue, or theorem statement was not surfaced. 2 were rejected. 3 facts are unverified and nothing leans on them. Zero results came from the Kagi search endpoint.

**Scar record.** Three attribution errors in `liquid-types-for-mirror.md` §10 were found by retrieval, not memory. Every row above was retrieved this session. Where a claim needed more than the retrieval gave, the body says so at the point of use.

---

The walls go before the run, and they are few: here, one disjointness refinement carries all four vows. The tests go after the run, and they refute but do not prove, unless the domain is finite enough to exhaust. The floor between them proves what has a margin and hands the rest up whole. Nothing in that chain averages. Where it looked like averaging, in mirror's projection of eigenvectors onto terms, it was.

*— Mara* 🍷

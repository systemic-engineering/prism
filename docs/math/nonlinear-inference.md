---
title: "Nonlinear Inference — the floor for taking nonlinear time seriously in prismqueer"
subtitle: "The loss monoid decides what a beam can remember about order. The carrier's topology decides what order there is to remember. Traces keep exactly what matters. What ships, what it would refute, and where Reed's synthesis is wrong."
author: Mara
date: 2026-09-28
status: candidate
visibility: protected
slug: nonlinear-inference
composes-forward-from: /Users/reed/dev/systemic.engineering/practice/insights/cybernetics/nonlinear-time-in-silicon.md
---

# Nonlinear Inference

*What maintains information without prematurely collapsing it into a scalar?*

---

## §0 The question and the discipline

**0.1 The question.** Alex Wolf, 2026-09-28:

> *"What if we took nonlinear time seriously? What would we need to ship within prismqueer to make it a testable claim in prismqueer, and the floor of a nonlinear geometric compiler like mirror?"*

**0.2 The operant condition.** Alex Wolf, 2026-09-28, verbatim:

> *"The operant condition for prismqueer is always 'what maintains information without prematurely collapsing it into a scalar?' compose not average in silicon."*

This document uses that sentence as its design test. Every type proposed in §6 is judged by it. Every rung in §1 is a degree of collapse.

**0.3 Tiers.** Every load-bearing claim is tagged:

- **(a) verified.** Mainstream mathematics, CS, or physics. Primary source retrieved this session, or an elementary fact whose proof is given inline.
- **(b) contested but published.**
- **(c) speculation.** Mine, Reed's, or Alex's. Named as such.

Nothing in (c) borrows authority from (a). A claim about *prismqueer source code* is tagged **(src)** with file and line. I read it this session. I did not run it unless I say so.

**0.4 Retrieval disclosure.** The Kagi *search* endpoint returned an empty result set for every query this session, including trivial ones ("Lamport time clocks ordering of events"). The Kagi *summarizer* worked. Every citation below was retrieved through the Kagi summarizer against one of four sources: an arXiv abstract page, a Crossref DOI record, a publisher or author page, or a Crossref bibliographic query. The Coda ledger records which route confirmed which fact. Where a route confirmed only metadata (title, authors, venue, year) and not the theorem I use, the ledger says so, and the body flags it. Where no route worked, the citation is marked **unverified** and nothing leans on it.

**0.5 Three meanings of "linear."** The corpus uses one word for three things. They come apart here, and conflating them produces wrong claims.

| Word | Meaning | Example in prism |
|---|---|---|
| linear **order** | a total order on events: time as a line | `trace::Trace { steps: Vec<Step> }` |
| linear **map** | `f(ax + by) = a f(x) + b f(y)` | orthogonal projection, graph Laplacian |
| linear **resource** | used exactly once | `Beam::tick(self, …)` consumes `self` |

"Nonlinear time" in this document means **non-total order** of events. A projection is a linear *map*, and it is the canonical *erasure*. Alex's 2026-09-19 correction is right in the *order* sense. `split` produces incomparable successors, so it is the most order-nonlinear op. `project` is one in, one out, so it is order-linear. Neither statement is about linear algebra.

**0.6 What this document composes over.** My 2026-09-28 insight *Nonlinear Time in Silicon* (§4.6: "replace the commutative loss monoid with the free one") is the starting point. This document corrects it. The free monoid is one rung too high: it keeps orders that do not matter, so equivalent histories get different addresses. The trace monoid is the rung the operant condition asks for.

---

## §1 Time as the commutativity structure of the loss monoid

### §1.1 The contract that is actually in the code

**(src)** `prism/imperfect/src/lib.rs:107–121`:

```rust
pub trait Loss: Clone + Default {
    fn zero() -> Self;
    fn total() -> Self;     // "Acts as an absorbing element under combine."
    fn is_zero(&self) -> bool;
    fn combine(self, other: Self) -> Self;  // "Associative"
}
```

The trait promises associativity, an identity, and an absorbing `total()`. It does **not** promise commutativity. Reed's grep is correct.

**(src)** Three facts the types do not say out loud:

1. **The free and set carriers have no absorbing element.** `impl<T: Clone> Loss for Vec<T>` returns `Vec::new()` from both `zero()` and `total()` (`lib.rs:685–702`), and so do `HashSet`, `BTreeSet`, and `String`. The limitation is documented (`lib.rs:670–674`: *"Collections have no natural absorbing element"*), and the test at `lib.rs:2322` pins it. It matters here because a trace monoid also has no natural absorbing element. §6 adjoins one explicitly.
2. **`ScalarLoss` is not associative in the bits.** `ScalarLoss::combine` is `f64` addition (`prismqueer/src/scalar_loss.rs:36`). IEEE-754 binary64 addition is commutative but not associative: `(0.1 + 0.2) + 0.3 = 0.6000000000000001` while `0.1 + (0.2 + 0.3) = 0.6`. (a, elementary.) The carrier that is supposed to forget order remembers *grouping* at round-off. Any test that compares `ScalarLoss` values across two combination trees must say what tolerance it uses.
3. **`kleinos`' loss channel is currently constant.** `into_imperfect` (`spectral/kleinos.rs:659–672`) returns `Success(candidate)` (no loss) or `Failure(verdict, ConvergenceLoss::zero())`. Yellow is "reserved for Phase 2+". The `ConvergenceLoss` in `kleinos`' return type carries **zero bits today**. The four-vow information lives in `Red = Transparency<Property>`, an `OpacityMap` keyed by property. It survives on the Failure path only, as the error, never as loss.

### §1.2 The record map

Fix an event alphabet `E` and a per-event loss `ℓ : E → L`. A pipeline run produces a word `w = e₁e₂…eₙ ∈ E*`. The beam's accumulated loss is the **record map**

```
ρ(w) = ℓ(e₁) ⊕ ℓ(e₂) ⊕ … ⊕ ℓ(eₙ)
```

which is the unique monoid homomorphism `E* → L` extending `ℓ`. (a: universal property of the free monoid.) `Beam` fixes `L` for a whole pipeline (`beam.rs:43–46`, `type Tick<T,E>: Beam<…, Loss = Self::Loss>`), and `propagate` folds left in order (`beam.rs:178–184`). So `ρ` is exactly what an `Optic<…, L>` computes.

**Definition 1.1 (what a carrier remembers).** Two histories `w, w'` are *indistinguishable to L* iff `ρ(w) = ρ(w')`. The kernel `ker ρ` is an equivalence on histories. Each Loss type **is** a choice of which histories to identify.

### §1.3 The ladder, as a chain of quotients

Every carrier below is a quotient of the one above it, and every arrow is a surjective monoid homomorphism (a):

```
E*                 free monoid          Vec<T>             keeps everything
 │  ab ~ ba for (a,b) ∈ I
 ▼
M(E, I)            trace monoid         (absent; §6)       forgets the order of independent events
 │  ab ~ ba for all a ≠ b
 ▼
ℕ^E                free commutative     (absent)           Parikh vector: which events, how many
 │  a·a ~ a
 ▼
P_fin(E)           free semilattice     BTreeSet, HashSet  which events, not how many
 │  project to a number
 ▼
ℝ≥0 / ℕ, (+)       scalar sum           ScalarLoss, u64    how much in total
ℕ, (max)           scalar max           ConvergenceLoss    the worst one only
```

The endpoints are both trace monoids. `M(E, ∅) = E*`, and `M(E, (E×E)∖Δ) = ℕ^E`. (a; Cartier & Foata 1969; Diekert & Rozenberg 1995.) Trace monoids interpolate between the line and the bag.

**Reading under the operant condition (c, design).** Each downward arrow is a collapse. It is *premature* when it identifies two histories whose difference still matters downstream.

| Rung | What it collapses | When that collapse is premature |
|---|---|---|
| free → trace | order of *independent* events | never, if I is sound (§2.3). This collapse is free. |
| trace → Parikh | order of *dependent* events | whenever two dependent events were reordered: the outcomes differ |
| Parikh → set | multiplicity | whenever count matters (retries, dose, iterations) |
| set → scalar | **which** events | whenever you need to know what was lost, e.g. to reverse it (§3) |
| sum → max | everything but the extremum | whenever the non-maximal losses matter |

The trace rung is where collapse stops being free. Below it, something that matters is lost. Above it, something that does not matter is kept. Keeping it costs content-address stability: two linearizations of the same concurrent history get two OIDs (§2.5).

### §1.4 Testing Reed's claim 1

> *Reed 1: The Loss monoid chosen for `Beam::Loss` decides which kind of time the beam carries. Commutative: no time. Free: linear time. Partially commutative: nonlinear time.*

**Half right. The correction is structural.** The loss monoid decides what the beam can *remember* about order. It cannot create order structure the carrier does not have.

**(src)** `Beam::tick` takes one beam and returns one beam, and `Tick::In = Self::Out` (`beam.rs:46`). The `Prism` trait chains `Input → Focused → Projected → Refracted` by type (`lib.rs:188–203`). Every step of a single beam consumes the previous step's output. Consecutive steps are **data-dependent by construction**. In Mazurkiewicz's terms, they share a resource: the value in flight.

**Proposition 1.2 (a, elementary).** Let a history be produced by one `Beam` chain, and let I be any sound independence relation (§2.3) on its steps. Then the dependence order on those steps is total, and the trace has height equal to its length. *Proof.* Step `k+1` reads the value step `k` wrote, so the two do not commute on states in general. A sound I cannot contain them. By transitivity every pair of steps is ordered. □

So Reed's ladder needs a second axis. **The carrier's topology decides what order there is. The loss monoid decides how much of it survives.** A `Beam` with a trace-monoid loss and no `split` records a line, because a line is all there is. Concurrency enters prismqueer only through:

- (i) **fan-out**: `split`, one input to many beams. **(src)** This does not exist on the carrier. `trace::Op` has `Focus`, `Project`, `Settle` only (`trace.rs:15–22`). The `Prism` trait has three stages. FLOOR §2.2 records the naming residue (Q-Mara-A).
- (ii) **fan-in**: a binary compose. **(src)** `kleinos(a, b)` is the only one (§3.4).
- (iii) **loss events on disjoint resources** within one chain. Two erasures of different cells can commute *as records* even though the steps that produced them are ordered. That is independence of the record, not of the computation (§3.3, T8).

**Reed's "commutative = no time" also needs a precision.** A commutative, non-idempotent carrier (a sum or a Parikh vector) still counts events. That count is a duration: time without order. The idempotent rungs (set, max) drop even that. The coincidence detector of the insight (§1.2 there) is the AND gate, a Boolean semilattice: rung *max*, not rung *sum*.

### §1.5 Where softmax, argmax, and averaging sit

The corpus reads softmax as averaging ("softmax is avg"). Here is what each step actually erases.

**(src)** `fate/mod.rs:188–200`: `ModelWeights::forward` computes logits `z ∈ ℝ⁵`, then `softmax5(z)`, then `argmax5`, and returns

```rust
pub struct Decision { pub model: Model, pub confidence: f64, pub distribution: [f64; 5] }
```

| Step | Map | What is erased (exact arithmetic) | Rung |
|---|---|---|---|
| `softmax5` | `ℝ⁵ → Δ⁴°` (open simplex) | **one dimension**: the additive shift `z ↦ z + c·𝟙`. Invertible on the rest: `z = log p + c`. | none, up to a gauge |
| expectation `Σ pᵢ vᵢ` | `Δ⁴ × V⁵ → V` | the distribution itself; many (p, v) give the same mean | **this** is the average |
| `argmax5` | `Δ⁴ → {0..4}` | everything but the index of the maximum | index collapse |
| `confidence = p[best]` | `Δ⁴ → [0,1]` | everything but one coordinate | scalar collapse |

(a, elementary: softmax is invariant under a common shift and injective modulo it.) In `f64`, `softmax5` subtracts the max before `exp` (`mod.rs:146`), which prevents overflow. Logit gaps above about 745 still underflow `exp` to exactly `0.0`, and that erases the smaller logits beyond recovery. That underflow is a real erasure, and it is floating-point, not softmax.

**So softmax is not the average.** It is a reparametrization that loses one gauge dimension. The collapse happens at `argmax` (to an index) and at `confidence` (to a scalar). What the corpus is right about is that *softmax followed by argmax* collapses. What it misnames is which step does it.

**The fate.bf question dissolves.** My insight (§5.2) measured `fate.bf`'s phase 5 as a priority encoder (last nonzero wins) with a 68.4% equivariance defect, while `argmax5` is first-maximum-wins (`mod.rs:168–176`). Argmax versus priority encoder is the wrong axis: **both collapse to one index.** The Rust runtime keeps `distribution: [f64; 5]` beside the index. `fate.bf` emits one byte and keeps nothing. By the operant condition, `fate.bf` is strictly more collapsed, and the collapse is **unlogged**. The defect is *premature, unlogged collapse*, not the choice of tie-breaker.

Two facts survive the dissolution and remain Alex's to decide (§8, Q-Mara-NT4):

- Two runtimes compute different functions under one name. That is a naming bug whatever the design verdict.
- The priority encoder's order-dependence is an auditable property (insight §6.4). It stays measurable even when nobody wants it.

**Design rule (c):** collapse belongs at `settle`, **once**, as a logged event whose `erased` field content-addresses what the collapse destroyed (§6.2). The same rule applies to `kleinos`. `ConvergenceLoss` (max) is the scalar collapse of the vow trace. §6 keeps the `OpacityMap` as the loss, not the error.

---

## §2 Traces

### §2.1 Definitions (a)

**Definition 2.1 (independence, trace monoid).** An *independence relation* `I ⊆ E × E` is symmetric and irreflexive. `~_I` is the least congruence on `E*` with `ab ~_I ba` for all `(a,b) ∈ I`. The **trace monoid** is `M(E,I) = E*/~_I`. A trace is an equivalence class of words. (Cartier & Foata 1969; Mazurkiewicz 1977; Diekert & Rozenberg 1995.)

Mazurkiewicz's 1977 report defines the result of a concurrent program scheme as "a set of traces, characterized as partially ordered sets of symbol occurrences." It grounds independence in *action systems characterized by their resources and transformations* (retrieved: DAIMI PB-78 abstract). Independence is not declared by taste. It comes from what the actions touch.

**Definition 2.2 (dependence order).** For a word `w`, the occurrences of letters form a set. Order occurrence `i` before occurrence `j` when `i < j` and their letters are dependent (`(wᵢ, wⱼ) ∉ I`). Take the transitive closure. Two words are `~_I`-equivalent iff they induce isomorphic labelled posets. The words of a trace are exactly the **linearizations** (linear extensions) of that poset. (a; Mazurkiewicz 1977.)

**Definition 2.3 (Foata normal form).** Every trace `t` has a unique factorization `t = s₁ s₂ … s_k` in which each *step* `sᵢ` is a nonempty set of pairwise independent letters, and every letter of `s_{i+1}` depends on some letter of `sᵢ`. (a; Cartier & Foata 1969.) `k` is the **height** of `t`. `|t|` is its **length**.

Krob, Mairesse & Michos (2003) read height as *parallel* execution time and length as *sequential* execution time. The height equals the heap height in Viennot's heaps of pieces (Viennot 1986) and the number of factors in the Cartier–Foata decomposition. (a; retrieved: arXiv:cs/0112012 abstract; Discrete Math. 2003, DOI 10.1016/S0012-365X(03)00233-4.)

### §2.2 Where I comes from: the soundness condition

Let events act on a state space `S`, with `⟦a⟧ : S → S`.

**Definition 2.4 (sound independence).** `I` is *sound* for `⟦·⟧` iff `(a,b) ∈ I ⇒ ⟦a⟧∘⟦b⟧ = ⟦b⟧∘⟦a⟧` on every reachable state.

**Proposition 2.5 (a, elementary).** If `I` is sound, the final state `⟦w⟧(s₀)` depends only on the trace `[w]_I`, not on the word. *Proof.* `~_I` is generated by adjacent swaps of independent letters. Each swap preserves the composite by soundness and associativity of composition. Induct on the number of swaps. □

Soundness is one-directional. `[a,b] = 0 ⇒ (a,b) ∈ I` is a *choice*: the maximal sound I. Choosing a smaller sound I is safe and merely over-specifies, keeping orders that do not matter. Choosing an unsound I is **premature collapse by definition**: it identifies histories whose outcomes differ.

### §2.3 Testing Reed's claim 2

> *Reed 2: The independence relation I is derived from the commutator. `[a, b] = 0` ⇔ `(a, b) ∈ I`. The 5-op commutator table defines when time branches.*

Four findings. The first is decisive.

**Finding 2.3.1: Under FLOOR's own algebra, this I is everything, and time collapses to a bag.** FLOOR §2.1 defines the five ops as a canonical basis `{p_focus, p_split, p_project, p_lift, p_refract}`, *"mutually orthogonal (`p_i · p_j = δ_ij · p_i`); sum-of-projectors is the unit."* Orthogonal idempotents commute: `p_i p_j = 0 = p_j p_i` for `i ≠ j`. (a, elementary.) So `[p_i, p_j] = 0` for every pair. Derive I from *that* commutator and `I = (E×E)∖Δ`, so `M = ℕ⁵`, the Parikh rung. The algebra `A_F^prismqueer ≅ ℝ⁵` is **commutative**. It cannot host nonlinear time, and it cannot host the non-commutativity that FLOOR elsewhere calls load-bearing. It is a spectral decomposition of a single operator, commuting by construction.

The order-sensitive structure has to live where the ops **act on states**, not in `A_F`. This is a tension inside FLOOR itself. §8 (Q-Mara-NT3) routes it to Alex.

**Finding 2.3.2: There is no "5-op commutator table."** I grepped `prism/` and `mirror/docs` for *commutator table*, *5-op commutator*, and *op commutator*. There are zero hits. The code has three commutators. None of them is over the five ops.

| Instrument | Object | Sound for I? |
|---|---|---|
| **(src)** `spectral::commutator::CommutatorSpectrum::of(a: &Graph, b: &Graph)` (`commutator.rs:55–79`) | singular values of `[L_A, L_B]` for undirected graph Laplacians | **No, three ways** (2.3.3) |
| **(src)** `flux::FluxThread::commutator_magnitude(a, b, state)` (`flux.rs:79–106`) | distance between the **losses** of `A·B·s` and `B·A·s` after transport | **One-sided.** Nonzero ⇒ non-commuting (sound). Zero ⇏ commuting: two different states can carry equal holonomy loss. |
| FLOOR §2.1 `[p_i, p_j]` | orthogonal projectors | vacuous: always zero (2.3.1) |

The flux instrument is state-indexed, which is the right shape for Definition 2.4. It compares a projection of the states, not the states themselves. To certify independence it must compare **state OIDs**. (Small slip: its docstring labels the symmetry `magnitude(a,b) == magnitude(b,a)` as "Antisymmetric". The magnitude is symmetric. The commutator is antisymmetric.)

**Finding 2.3.3: The graph-Laplacian commutator is unsound for I in three ways.**

1. **It fails open.** **(src)** `commutator.rs:74–77`: `match crate::ffi::singular_values(…) { Ok(svs) => …, _ => Vec::new() }`. A LAPACK failure returns an **empty spectrum**, and the module docstring says *"Empty spectrum ⇔ A and B compose commutatively."* An instrument failure is reported as independence. For an I derived from this, that is an unsound collapse on the failure path. It must fail **closed** (dependent). §6, T5.
2. **Its threshold is absolute.** `σ > 1e-12` is not scaled by `‖L_A‖·‖L_B‖`. Double every edge weight and the same pair can cross the threshold. And almost-commuting is not commuting. Hastings (2009) proves that two Hermitian matrices with `‖[A,B]‖ ≤ δ` lie within `ε ∝ δ^{1/5}` of an exactly commuting pair, uniformly in dimension. (a; retrieved: arXiv:0808.2474 abstract; Commun. Math. Phys. 2009 per Crossref.) Graph Laplacians are real symmetric, so the theorem applies. A small commutator licenses *approximate* independence with a degradation exponent of 1/5. Thresholding at `δ` does not give an exactly sound I.
3. **It is blind to direction.** `build_laplacian` builds `L = D − A` symmetric (`kleinos.rs:306–311`). Reversing every edge of a directed graph leaves `L` unchanged. (a, elementary.) The instrument cannot see an arrow, so it cannot see time. The operators known to recover geometry from a causal order are **retarded** (non-symmetric) and **non-local**. Benincasa & Dowker (2010) build a family of retarded linear operators on causal sets that approximate `□ − ½R` on curved spacetime, by counting elements in order intervals. (a; retrieved: arXiv:1001.2725 abstract; Phys. Rev. Lett. 104:181301 per Crossref.) prismqueer's spectral stack drops the arrow at the first line of `build_laplacian`.

**Finding 2.3.4: "When time branches" is inverted.** Commuting pairs are where **linearizations** multiply and **outcomes do not**. That is concurrency. Non-commuting pairs are where outcomes differ with order. In event-structure terms that is conflict or causality, not concurrency. Nielsen, Plotkin & Winskel (1981) separate the three: causality `≤`, conflict `#`, and concurrency as neither. (a; retrieved: metadata, Theor. Comput. Sci. 13, DOI 10.1016/0304-3975(81)90112-2. The abstract was elided at the source. The three-relation structure is the standard content, not re-read this session.)

So "nonlinear time" names **two** objects. Reed's claim 2 conflates them.

- **N1, concurrency.** Commuting. One outcome, many observer orders. Trace monoids formalize it completely. Classical. Shippable today.
- **N2, order-sensitivity.** Non-commuting. Different orders, different outcomes. Classical silicon explores N2 as a *set of alternatives*: branching, conflict, a search tree. The quantum switch *superposes* N2 (§5). Procopio et al.'s experiment decides commute versus anticommute with one query of each gate. (a; retrieved: arXiv:1412.4006.) Its subject is N2, not N1.

Reed's claim 2 correctly derives N1 from the commutator and then calls it branching. The trace monoid is the theory of the time that *does not* branch.

### §2.4 Testing Reed's claim 3

> *Reed 3: Foata normal form layers are the parallel steps. That is `split`.*

- **Layers are parallel steps.** (a; Krob–Mairesse–Michos 2003: height = parallel time.)
- **"That is split": no.** A Foata step is an **antichain** of the dependence poset. `split` is a **vertex of out-degree > 1**, the event whose successors *can* form an antichain. Split makes width. The layer is the width. They are cause and effect, not the same thing.
- The corrected slogan: **split is where layers widen; merge is where they narrow; `project` is where they erase.** Alex's 2026-09-19 correction ("split is the most nonlinear op … natively parallelizable") survives intact. The *identity* claim does not.

### §2.5 Testing Reed's claim 4: trace content-addressing

> *Reed 4: Trace OID = hash of the Foata normal form. Equivalent linearizations share one content address (Rec #82 lifted to histories).*

**Right, with three conditions.**

**Proposition 2.6 (a).** `FNF : M(E,I) → (P_fin(E))*` is injective, and `FNF(w) = FNF(w')` iff `w ~_I w'`. So any collision-resistant serialization of FNF gives a content address with that property. *Proof:* uniqueness of FNF (Cartier & Foata 1969). □

Conditions:

1. **I is part of the preimage.** The same word under two relations names two traces. `oid(t) = H(oid(I) ‖ ser(FNF(t)))`. Without `oid(I)`, a change to the independence relation silently re-identifies old histories.
2. **Within a step, serialize canonically.** Either sort the step's letters by a fixed total order on `E` (`BTreeSet<E>` does this for free), or hash the step as a set.
3. **The existing hashers are the wrong shape, and one of them is almost right.**
   - Shard OIDs are Merkle trees over *sorted* children (`mirror/docs/specs/reality-shard-as-crdt.md` §0–§1). They forget order entirely. That is the semilattice rung, correct for **state** and premature for **history**.
   - **(src)** `CoincidenceHash`'s `encode_into_basis` (`coincidence.rs:207–230`) sums, over positions `i`, a vector seeded by `SHA256("encode:" ‖ i ‖ byte)`. The sum commutes. The seed carries position. Replace the byte position `i` with the event's **Foata depth** and the construction becomes invariant under permutation within a step and sensitive across steps. That is an FNF-invariant hash, built from a primitive the corpus already has. (c, construction; untested.) Additive multiset hashing has its own collision theory, which I did not retrieve. I make no security claim for this variant. Use it as an invariant, not a cryptographic address, until someone verifies it.
4. **The alternative is a Merkle DAG.** Hash each event together with the OIDs of its immediate dependency predecessors. Two linearizations of one trace build the same DAG, so the OID of the frontier is linearization-invariant. Merkle-CRDTs use exactly this: Merkle-DAG "Merkle-Clocks" that represent causality and *"can replace traditional mechanisms like vector clocks"*, with content addressing and self-verification. (a; retrieved: arXiv:2004.00107 abstract. Author names were not surfaced by retrieval.)

**Lifting Rec #82.** mirror's crystal OID is `BLAKE3(H(β(t)))`, the hash of a β-normal form (`mirror/docs/math/2026-08-10-mara-beta-normal-ast-content-addressing-math-foundation.md` §3.1). FNF is to histories what β-normal form is to terms. The lift is exact as mathematics: a unique normal form hashed. (a.) Its value to mirror is still a design claim (c) until §7's tests run.

---

## §3 Irreversibility

### §3.1 Where the arrow comes from

Landauer (1961) established that logically irreversible computation generates heat. Bennett (1973) showed that computation can be made logically reversible, so the unavoidable cost sits in erasure. Bérut et al. (2012) verified Landauer's principle experimentally. (a; all three retrieved: Crossref records for DOIs 10.1147/rd.53.0183, 10.1147/rd.176.0525, 10.1038/nature10872.)

**Definition 3.1.** A step `f : S → S` *erases* when it is not injective on the reachable states. The erased information on input distribution `X` is `H(X | f(X))`.

### §3.2 Testing Reed's claim 5

> *Reed 5: `settle` is a join. Erasure at joins is where the arrow comes from (Landauer/Bennett).*

**Two corrections.**

**(src) `settle` is not a join.** `Prism::settle(&self, beam: Self::Projected) -> Self::Refracted` (`lib.rs:203`) is **unary**. A join takes two or more arguments and is idempotent, commutative, and associative.

**(src) `kleinos` is not a join either.** It is the only binary compose. Reading `compose_candidate` (`kleinos.rs:465–517`):

- **Commutative:** yes, by inspection. Vertex and edge sets are unions. The bipartite bridge is symmetric. The emergent vertex id is `max(V(a) ∪ V(b)) + 1`, symmetric in `a, b`. The four-vow verification was not read for symmetry.
- **Idempotent:** no, by design. Identical inputs are refused as fusion (PAPER §3.6.4; test at `kleinos.rs:703`). A join satisfies `a ∨ a = a`.
- **Associative:** no, by reading. `(a∘b)∘c` bridges `a`'s emergent vertex to all of `c`. `a∘(b∘c)` bridges `b∘c`'s emergent vertex to all of `a`. The edge sets differ in general. Worse, the emergent id is fresh only relative to the two operands. With `a = {1}`, `b = {2}`, `c = {3}`, the emergent vertex of `a∘b` is `3`, which **collides** with `c`'s vertex in the next compose. (src reading; not executed. T11 measures it.)

So `kleinos` is a commutative, non-idempotent, non-associative magma. That is not a monoid, and it cannot be the `combine` of any Loss. The "LOVE-monoid" of FLOOR §4.2 is `Transparency<Property>`'s `combine`, a genuine monoid (`transparency.rs:317–340`). The name belongs there, not to `kleinos`. The join that exists in the corpus is **shard merge** (`reality-shard-as-crdt.md` §1), a semilattice on state.

**The arrow is not only at joins.** It is written at **every non-injective step whose erasure is not logged.** `project` is documented as *"the lossy transformation. Information may not survive."* (`lib.rs:200`). It is the canonical eraser, and it has one input. A join erases **which branch contributed what**, which is precisely the linearization information a trace keeps. The accurate form of claim 5:

> **The arrow is the logging deficit.** A step with a complete log is reversible in principle (Bennett). A merge that records its trace is a reversible merge. What is left unlogged is the direction.

### §3.3 Causal-consistent reversibility: the trace is the right history

Reversible process calculi reverse concurrent computation while respecting causality: Danos & Krivine (2004), *Reversible Communicating Systems*, CONCUR (retrieved: metadata, DOI 10.1007/978-3-540-28644-8_19); Phillips & Ulidowski (2007), *Reversing algebraic process calculi*, J. Log. Algebr. Program. (retrieved: Crossref, DOI 10.1016/j.jlap.2006.11.002); Lanese, Mezzina & Stefani (2010), *Reversing Higher-Order Pi*, CONCUR (retrieved: Crossref, DOI 10.1007/978-3-642-15375-4_33). **(a)**

The axiomatic account is the one that bears on prismqueer. Lanese, Phillips & Ulidowski (2020), *An Axiomatic Approach to Reversible Computation*, FoSSaCS (retrieved: abstract via Crossref, authors via Semantic Scholar; DOI 10.1007/978-3-030-45231-5_23), work over *"a generic labelled transition system that incorporates a relation to define transition independence."* From a small set of axioms they derive the parabolic lemma and causal consistency, and they introduce causal safety and causal liveness. **(a)** Causal-consistent reversibility is parameterized by an independence relation, the same kind of I as §2. The informal content of causal consistency: an event may be undone iff every event that causally depends on it has been undone.

**Proposition 3.2 (c, my proof sketch; not in the retrieved sources).** Let each step be injective once its logged erased value is supplied, and let I be sound for both the forward steps and their inverses. Inverses of commuting bijections commute. Then `(output, final state, [log]_I)` determines the input iff `(output, final state, log)` does. *Sketch:* by Prop. 2.5 applied to the inverse steps, every linearization of the reversed dependence poset replays backward to the same initial state. So the class `[log]_I` carries everything the word `log` does for the purpose of reversal. □

**Consequence for the insight's Test 3.** The measured jump from 5 to 4,096 distinct keys when the ordered log was attached (insight §5.4) should **survive quotienting the log by a sound I**. If it does not, the declared I erased a distinction that mattered. That is premature collapse, detected by an instrument (§6, T8). This is the operant condition made executable.

**What `Imperfect::Partial(value, Trace<E,I>)` can recover:** the erased coordinates of the input; every intermediate state along any observer's linearization; the set of all observer views.

**What it cannot recover:** the wall-clock interleaving that actually happened. That was erased by design, and the erasure is harmless under a sound I. Nor can it recover anything a step erased without logging.

Phillips, Ulidowski & Yuen (2013) apply a reversible process calculus to the ERK signalling pathway (retrieved: Crossref, DOI 10.1007/978-3-642-36315-3_18). Their characterization of *out-of-causal-order* reversal, which biology needs and causal consistency forbids, was **not** retrieved. Nothing here leans on it. §8 names the gap.

---

## §4 The observer

### §4.1 Observers as linearizations

**Reading (c) on (a) mathematics.** An observer who sees a concurrent history one event at a time sees one linearization of its dependence poset: one word of the trace. Different observers, different words, same trace, same outcome under a sound I (Prop. 2.5). This is the precise, classical, testable content of *"different observers may give different accounts of the same sequence of events"*.

Rovelli's relational quantum mechanics takes the analogous position in physics: states are *"relative to the specific physical systems interacting"*, and *"all physical systems are inherently equivalent"* as observers. (Retrieved: arXiv:quant-ph/9609002 abstract; Int. J. Theor. Phys. 1996, DOI 10.1007/BF02302261.) The interpretation is tier (b). The analogy to linearizations is tier (c). It is structural, not derivational.

**Counting observers is hard.** Counting the linear extensions of a poset is #P-complete. (a; Brightwell & Winkler 1991, *Order* 8:225–242, DOI 10.1007/BF00383444; retrieved: Crossref record, which names #P-completeness.) prismqueer will **sample** observers, never enumerate them. T12 uses `flux::Sample::from_bytes` for reproducible sampling. `Sample::new` seeds from wall-clock time and a global atomic counter (`flux.rs:480–533`), which is itself a hidden shared resource.

**How many observers recover the order?** Every poset is the intersection of its linear extensions. The least number needed is the poset's *order dimension* (Dushnik & Miller 1941, Amer. J. Math. 63:600, DOI 10.2307/2371374; retrieved: metadata only, theorem statement not re-read). The reading (c): pool the accounts of `dim(P)` observers and the partial order is recoverable. Fewer cannot do it. Charron-Bost (1991) uses order dimension to bound the size of logical clocks. (Retrieved: Crossref, Inf. Process. Lett., DOI 10.1016/0020-0190(91)90055-M. The summary names order dimension and consistent cuts, not the exact bound.)

### §4.2 Clocks: what each collapses

| Clock | Rung | What it characterizes | Source |
|---|---|---|---|
| wall clock | none; it is an input | nothing about causality | — |
| Lamport scalar | a total order **consistent with** happened-before | `a → b ⇒ C(a) < C(b)`; not the converse | Lamport 1978, CACM 21(7):558 (retrieved: PDF content + Crossref) |
| vector clock | the partial order itself | `a → b ⇔ V(a) < V(b)`; independence testable | Mattern 1988 (retrieved: PDF content) |
| Merkle clock | the partial order, content-addressed | reachability in a hash DAG | arXiv:2004.00107 (retrieved) |

Mattern (1988) argues that *"a partially ordered structure of time, forming a lattice, is more adequate … than linearly ordered integers."* He proves that the lattice of consistent cuts is isomorphic to the lattice of vector times (Theorem 9), and that vector clocks capture causality *"without the information loss found in traditional scalar logical clocks."* (a; retrieved.) That is the operant condition in 1988 distributed-systems form. **A scalar clock is premature collapse.**

Fidge (1988) is conventionally co-credited with vector clocks. **Unverified this session.** There is no DOI, three retrieval routes failed, and nothing leans on it. Mattern carries the claim.

### §4.3 Thermal time: kept at arm's length

Connes & Rovelli (1994) propose the thermal time hypothesis: *"physical time-flow is not an inherent universal property but is determined by the thermodynamical state of a system,"* via the Tomita–Takesaki modular flow. (Retrieved: arXiv:gr-qc/9406019 abstract; Class. Quantum Grav. 11:2899, DOI 10.1088/0264-9381/11/12/007.) The mathematics is (a). The physics hypothesis is (b). My insight (§7.2) already marked "an erasure count defines a modular flow" as tier (c) until someone writes the map. Nothing in §6 depends on it. It stays an open residue, not a floor.

---

## §5 The honest boundary

**5.1 What is verified (a).**

- Chiribella, D'Ariano, Perinotti & Valiron (2013), *Quantum computations without definite causal structure*, Phys. Rev. A 88:022318 (retrieved: arXiv:0912.0195 + Crossref). The quantum SWITCH controls the order of two black boxes with a qubit. It cannot be realized with a single use of each box in a fixed-order circuit. It *can* be simulated with extra queries. It *"does not increase the computational complexity class of the standard circuit model"* but gives advantages in black-box discrimination and query count.
- Procopio et al. (2015), Nat. Commun. 6:7913 (retrieved: arXiv:1412.4006 + Crossref DOI 10.1038/ncomms8913). Commute-or-anticommute decided with one query of each gate: success 0.973 against a fixed-order bound of 0.9288.
- Araújo, Costa & Brukner (2014), Phys. Rev. Lett. 113:250402 (retrieved: arXiv:1401.8127 + Crossref). The n-switch solves a promise problem with O(n) queries, against O(n²) for the best known fixed-order algorithm.
- Oreshkov, Costa & Brukner (2012), Nat. Commun. 3:1092 (retrieved: arXiv:1105.4464 + Crossref DOI 10.1038/ncomms2076). A causal inequality can be violated by non-causally-separable processes. *"Classical operations are always causally separable."*

**5.2 The boundary, stated precisely.** Reed 6 says silicon can *witness* non-commutativity but cannot *superpose* orders. That is right. It needs one refinement that changes what prismqueer should build.

The switch's advantage is a **query-complexity advantage relative to black-box access**. prismqueer has **white-box** access to its own operations: it owns the Laplacians, the transports, and the programs. With white-box access, deciding commutation needs no switch. You compute `AB` and `BA`, or `‖[A,B]‖`, directly. That costs two uses instead of one. The advantage the switch offers is irrelevant to a compiler that can read its own operators.

> **Silicon measures N1 exactly and cheaply. It explores N2 as a set of branches. It cannot put N2 in superposition. Anything claiming nonlinear time *in* silicon, rather than *measured by* silicon, must cross the Oreshkov–Costa–Brukner line, and classical operations do not.** (a, given 5.1.)

**5.3 A self-imposed boundary, not a physical one.** The symmetric Laplacian cannot see an arrow (§2.3.3). That limitation is prismqueer's choice, not silicon's. A directed or retarded operator is ordinary floating-point linear algebra.

---

## §6 What ships in prismqueer

### §6.0 Placement and naming

`prismqueer::trace::Trace` already exists. It is a linear step log of `Box<dyn Traced>` with `ScalarLoss` per step (`trace.rs:46–62`). It is neither `Eq` nor `Hash`, so it is not content-addressable. The new types live in a new module so nothing existing breaks: **`prismqueer/src/time/`**. Whether the old `Trace` migrates onto the new one is Q-Mara-NT1.

A multiset Loss (the Parikh rung, absent in terni) is optional. It is listed last.

### §6.1 Types

```rust
// prismqueer/src/time/independence.rs

/// Ternary verdict of a commutation instrument. Unknown fails CLOSED.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Commutation {
    Commutes,
    DoesNot { witness: Oid },   // OID of a state where the two orders differ
    Unknown { reason: String }, // instrument failure, below-resolution, unsampled
}

/// Symmetric, irreflexive. Soundness contract (Def. 2.4):
/// independent(a, b) ⇒ ⟦a⟧∘⟦b⟧ = ⟦b⟧∘⟦a⟧ on every reachable state.
pub trait Independence<E>: Clone + Default {
    fn independent(&self, a: &E, b: &E) -> bool;
    /// Content address of the relation itself. Part of every trace OID.
    fn oid(&self) -> Oid;
}

/// Mazurkiewicz-style: independent iff read/write footprints do not conflict.
/// Sound whenever footprints are honest. Conservative: may under-approximate I.
#[derive(Clone, Default)]
pub struct Footprint;
pub trait Touches { fn reads(&self) -> &BTreeSet<Oid>; fn writes(&self) -> &BTreeSet<Oid>; }

/// Instrument-backed: independent only on Commutation::Commutes.
#[derive(Clone, Default)]
pub struct Witnessed<E: Ord> { table: BTreeMap<(E, E), Commutation> }
```

```rust
// prismqueer/src/time/trace.rs

/// An element of M(E, I), stored in Foata normal form.
/// `Total` is the adjoined absorbing element (terni's `total()` law;
/// free and trace monoids have none of their own, cf. imperfect/src/lib.rs:670).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Trace<E: Ord, I> {
    Live { steps: Vec<BTreeSet<E>>, rel: PhantomData<I> },
    Total,
}

impl<E: Ord + Clone, I: Independence<E>> Trace<E, I> {
    pub fn from_word(w: impl IntoIterator<Item = E>) -> Self;
    /// Viennot drop: place e one step after the deepest step holding an event dependent on e.
    pub fn push(&mut self, e: E);
    pub fn foata(&self) -> &[BTreeSet<E>];
    pub fn height(&self) -> usize;           // parallel time
    pub fn len(&self) -> usize;              // sequential time
    pub fn linearizations(&self, sample: &mut flux::Sample) -> impl Iterator<Item = Vec<E>>;
    pub fn vector_clock(&self, e: &E) -> Option<Vec<usize>>;
    pub fn oid(&self) -> Oid;                // H(I::oid() ‖ ser(FNF)) — Prop. 2.6
}

impl<E: Ord + Clone, I: Independence<E>> Default for Trace<E, I> { /* Live, empty */ }

impl<E: Ord + Clone, I: Independence<E>> terni::Loss for Trace<E, I> {
    fn zero() -> Self { Self::default() }
    fn total() -> Self { Trace::Total }
    fn is_zero(&self) -> bool { matches!(self, Trace::Live { steps, .. } if steps.is_empty()) }
    /// Concatenate in M(E, I): push each event of `other`, in FNF order, onto `self`.
    fn combine(self, other: Self) -> Self;
}
```

`Loss::combine` has no context parameter, so the independence relation is carried **at type level** (`I: Default`, typically a zero-sized marker). That also enforces Beam's rule that one loss type spans a pipeline: one pipeline, one I.

```rust
// prismqueer/src/time/event.rs

/// One recorded event. `erased` content-addresses what the step destroyed (§1.5 design rule).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Event {
    pub op: Op,                    // see Q-Mara-A (FLOOR §2.2) for the naming residue
    pub site: Oid,                 // which prism / branch
    pub reads: BTreeSet<Oid>,
    pub writes: BTreeSet<Oid>,
    pub erased: Option<Oid>,       // None ⇔ the step was injective
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Op { Focus, Split, Project, Merge, Settle }
```

```rust
// prismqueer/src/time/beam.rs

/// Optic plus a trace-valued loss. The TraceBeam that `beam.rs:114` promises.
pub struct TraceBeam<In, Out, Er, I: Independence<Event>> {
    source: Option<In>,
    focus: Imperfect<Out, Er, Trace<Event, I>>,
}
impl<In, Out, Er, I: Independence<Event>> Beam for TraceBeam<In, Out, Er, I> {
    type Loss = Trace<Event, I>;
    /* identical propagation to Optic */
}

/// Fan-out. The carrier gains width. Every branch starts from an empty trace
/// suffix. The shared prefix stays with the parent, so merge never double-counts it.
pub fn split<In, Out, T, Er, I, F>(beam: TraceBeam<In, Out, Er, I>, f: F)
    -> (Trace<Event, I>, Vec<TraceBeam<Out, T, Er, I>>)
where F: FnOnce(&Out) -> Vec<Imperfect<T, Er, Trace<Event, I>>>, I: Independence<Event>;

/// Fan-in. prefix · s₁ · s₂ · … · merge_event. Branch order must not matter (T9).
pub fn merge<Out, T, Er, I, J>(prefix: Trace<Event, I>, branches: Vec<TraceBeam<_, Out, Er, I>>, join: J)
    -> TraceBeam<Vec<Out>, T, Er, I>
where J: FnOnce(Vec<Out>) -> Imperfect<T, Er, Trace<Event, I>>, I: Independence<Event>;
```

Signatures are shape, not final syntax. `_` marks an input type left generic in the real code.

**Two changes to existing code** are prerequisites, not features:

- **(P1)** `CommutatorSpectrum::of` must stop mapping FFI failure to an empty spectrum. It should return `Imperfect<CommutatorSpectrum, FfiError, …>` or a `Commutation`. Scale the threshold by `‖L_A‖_F · ‖L_B‖_F`.
- **(P2)** `fate/brainfuck/test_fate.rs` sends 17 bytes (`make_input`, 16 features plus model index, line 83). `fate.bf` reads 22. Fix the harness before T8 and T10 inherit a broken instrument. (Reed-verified; re-read here at line 83.)

### §6.2 Tests, in red-first order

Unit tests go in the owning modules. Cross-module claims go in `prismqueer/tests/nonlinear_time.rs`. All random inputs come from `flux::Sample::from_bytes(seed)`, never `Sample::new()`.

Each test states what it **measures**, what **supports** the claim, what **refutes** it, and the **confound**.

---

**T0 `loss_ladder_is_measured_not_asserted`** (`imperfect/tests/ladder.rs`)
*Measures:* for every `Loss` impl in terni and prismqueer, over sampled triples: associativity, commutativity, idempotence, identity, and the absorbing law. Asserts a table placing each impl on the §1.3 ladder.
*Supports:* the table matches §1.3.
*Refutes:* any impl on a different rung than its docstring implies.
*Confound:* `f64`. `ScalarLoss` associativity fails bitwise (§1.1). The test pins the tolerance (ULPs) explicitly rather than hiding it. `Vec` and the sets fail the absorbing law, which is documented. Pin it; do not "fix" it.

**T1 `trace_congruence_is_generated_by_independent_swaps`** (`time/trace.rs`)
*Measures:* `from_word(uabv) == from_word(ubav)` for `(a,b) ∈ I`, and `!=` for `(a,b) ∉ I`.
*Supports:* both hold on all sampled words.
*Refutes:* equality across a dependent swap, or inequality across an independent one.
*Confound:* an `I` that is not symmetric or not irreflexive. Assert both as preconditions on every `Independence` impl under test.

**T2 `trace_loss_is_a_monoid_with_adjoined_total`** (`time/trace.rs`)
*Measures:* associativity, identity, `Total` absorbing on both sides, and `combine` agreeing with word concatenation followed by `from_word`.
*Supports:* all laws hold.
*Refutes:* any failure. In particular, combine that is not a homomorphism from `E*`.
*Confound:* none beyond T1's.

**T3 `foata_form_is_unique_and_height_is_parallel_time`** (`time/trace.rs`)
*Measures:* every sampled linearization of a trace yields a byte-identical `foata()`. `height ≤ len`, with equality iff no two occurring letters are independent.
*Supports:* uniqueness and the height bound.
*Refutes:* two linearizations with different FNF.
*Confound:* the sampler must actually produce distinct linearizations. Assert the count of distinct words above 1 on posets of width at least 2.

**T4 `trace_oid_is_linearization_invariant_and_i_sensitive`** (`time/trace.rs`)
*Measures:* `oid` equal across linearizations; unequal across a one-letter change; unequal for the same word under two relations that differ on an occurring pair.
*Supports:* all three.
*Refutes:* collision across genuinely different traces, or I omitted from the preimage.
*Confound:* the hash must include step boundaries. Otherwise `{a}{b}` and `{a,b}` can collide.

**T5 `independence_fails_closed`** (`time/independence.rs`, plus `spectral/commutator.rs` after P1)
*Measures:* `Witnessed` returns `independent == false` for `Unknown`. A Laplacian-backed instrument whose LAPACK call errors yields `Unknown`, not `Commutes`. A pair below the scaled threshold yields `Commutes` only when `‖[L_A,L_B]‖ ≤ τ·‖L_A‖‖L_B‖`.
*Supports:* no path turns an instrument failure into independence.
*Refutes:* today's code (`commutator.rs:74–77`). **This test is red on day one against the current `of`. That is the point.**
*Confound:* forcing a LAPACK failure. Inject the FFI through a trait so the test can mock it.

**T6 `declared_independence_is_sound_on_states`** (`tests/nonlinear_time.rs`). **The execution-altitude falsifier.**
*Measures:* for sampled states `s` and every declared-independent pair `(a,b)`: `state_oid(⟦a⟧(⟦b⟧(s))) == state_oid(⟦b⟧(⟦a⟧(s)))`. State OIDs are computed **without reference to I**.
*Supports:* no counterexample over N samples. Report N. This is evidence, not proof.
*Refutes:* one counterexample. I is unsound, so this is premature collapse.
*Confound:* floating-point state. Bitwise OIDs will differ for commuting float operations because of rounding (§1.1). State OIDs must hash a **quantized** state, with the quantum stated. Otherwise the test refutes arithmetic, not independence. Second confound: shared hidden resources such as global counters, RNGs, or `SystemTime`. `flux::Sample::new` is one.

**T7 `graph_and_execution_commutators_agree`** (`tests/nonlinear_time.rs`). **Reed's claim 7, made non-circular.**
*Why the naive form is circular:* if I is derived from the commutator and the trace OID hashes FNF under that I, then "trace OIDs agree when the commutator is zero" holds **by construction**. That is a tautology, not a test.
*Measures:* two independent instruments on the same pair. **(i) Graph altitude:** `CommutatorSpectrum::of(A, B)` empty (post-P1, scaled threshold). **(ii) Execution altitude:** state OIDs of `A·B·s` and `B·A·s` agree (T6's instrument), for two families of actions:
- **(calibration)** linear diffusion `s ↦ exp(−t L_X) s`. For symmetric `L`, the semigroups commute for all `t` iff the generators commute. (a, elementary for commuting normal matrices.) Here the two instruments **must** agree, so the family calibrates both.
- **(the actual question)** `s ↦ kleinos(s, X)`, a nonlinear action. No theorem links `[L_A, L_B]` to whether `kleinos(kleinos(s,A),B) == kleinos(kleinos(s,B),A)`.
*Supports:* agreement on the calibration family, and high agreement on the `kleinos` family. That would be evidence that graph-altitude commutation predicts execution-altitude commutation for prismqueer's own compose.
*Refutes:* disagreement on the calibration family means an instrument is broken. Systematic disagreement on the `kleinos` family refutes Reed 2 and Reed 7 for `kleinos`. The graph commutator would then not be the source of I.
*Confound:* §3.2's emergent-vertex collision. `kleinos(kleinos(s,A),B)` may fail with Red for reasons unrelated to order. Count Red outcomes separately; do not score them as disagreement.

**T8 `quotient_by_independence_preserves_landauer_injectivity`** (`tests/nonlinear_fate.rs`; needs P2)
*Measures:* over the 4,096 inputs of insight §5.4, the number of distinct `(output, [log]_I)` keys against distinct `(output, log)` keys. `I` is footprint independence on erasure records: two erasures are independent iff their cells differ and no logged read of either cell lies between them.
*Supports:* both counts equal 4,096 (Prop. 3.2).
*Refutes:* fewer distinct keys after the quotient. I erased a distinction that mattered. That is premature collapse, caught by an instrument.
*Confound (decisive):* Brainfuck has one head and one instruction pointer. If control flow counts as a resource, every step depends on every other and `[log]_I = log`. The test is then trivially green and says nothing. The test must state which it measures: **record-level** independence (erasures commuting as records) or **step-level** independence (steps commuting as computations). Step-level on BF will be empty. Whether control flow is a resource is Q-Mara-NT8.

**T9 `split_merge_widens_then_narrows`** (`tests/nonlinear_time.rs`; needs `split` and `merge`)
*Measures:* split into K branches on disjoint sites, then merge. Check `height(merged) == height(prefix) + max_i height(sᵢ) + 1`, maximum step width at least K, and the merged trace OID invariant under the order in which branch suffixes are combined.
*Supports:* all three.
*Refutes:* merge-order dependence. The branches share a resource that the footprints missed.
*Confound:* the `join` closure itself may be order-sensitive (e.g. `Vec` push order). Test with a commutative `join` first. Then add a non-commutative `join` and assert it is **recorded** as dependent, not declared independent.

**T10 `fate_collapse_is_logged_once_at_settle`** (`tests/nonlinear_fate.rs`; needs P2)
*Measures:* for `ModelWeights::forward`, the `Settle` event's `erased` OID content-addresses `distribution`, and `distribution` is recoverable from `(Decision, trace)`. Run the same check on `fate.bf` via `FateRuntime`.
*Supports:* the Rust runtime passes. `fate.bf` fails: one byte out, nothing logged.
*Refutes:* the Rust runtime failing, meaning the collapse is unlogged.
*Confound:* this is a **tripwire**, not a hypothesis. `fate.bf` failing documents that it is strictly more collapsed (§1.5). The insight's equivariance-defect test (§5.2 there) stays alongside it as the audit of order-dependence.

**T11 `kleinos_algebra_is_measured`** (`spectral/kleinos.rs`)
*Measures:* commutativity (`kleinos(a,b)` against `kleinos(b,a)` by OID); idempotence (`kleinos(a,a)`); associativity (`(a∘b)∘c` against `a∘(b∘c)` by OID); emergent-id collision with a third operand.
*Supports §3.2:* commutative, not idempotent (Red by design), not associative, collision present.
*Refutes §3.2:* associativity holding, or no collision on `{1},{2},{3}`.
*Confound:* OIDs must be over the **graph**, not over the cached `fiedler_lambda_2`, which is float.

**T12 `observer_views_are_linearizations_and_vector_clocks_characterize_order`** (`time/trace.rs`)
*Measures:* every sampled linearization replays to the same final state (under a T6-sound I). `e ≤ f` in the dependence poset iff `vector_clock(e) ≤ vector_clock(f)` componentwise.
*Supports:* both, for all sampled pairs (Mattern 1988).
*Refutes:* a mismatch.
*Confound:* vector-clock components must be indexed by **sites** (processes). Indexing by events gives a different, larger clock.

**T13 (optional) `parikh_rung_exists`** (`imperfect/src/lib.rs`): `impl<T: Ord + Clone> Loss for Parikh<T>` (`BTreeMap<T, u64>`) completes the ladder so T0 can place every rung.

**Red-first order:** T0 → T1 → T2 → T3 → T4 → T5 (red against current code) → P1 → T6 → T12 → `split`/`merge` → T9 → T7 → P2 → T8 → T10 → T11 → T13.

The algebra goes first because everything above it assumes it. T5 goes before T6 because an unsound instrument makes T6 meaningless. T7 goes after T6 and T9 because it needs both instruments. T8 and T10 wait on the harness fix.

### §6.3 What would make the whole claim false

The testable claim in one sentence:

> **prismqueer can record concurrent histories as traces whose content address is invariant under every observer's linearization, with an independence relation that (i) is sound on states (T6), (ii) loses no input information when quotiented (T8), and (iii) is predicted by the graph-altitude commutator (T7).**

- If (i) fails, the construction is unsound.
- If (ii) fails, it collapses prematurely.
- If (iii) fails, the construction may still stand, but Reed 2 does not: the independence relation must come from footprints or state-level witnesses, not from the spectral commutator.

(i) and (ii) are expected to pass. They are close to theorems. **(iii) is the genuinely open prediction.**

---

## §7 The floor for mirror

**F1 (a, as mathematics).** If mirror's records are traces, compilation of a record set includes **partial-order normalization**: computing FNF and hashing `(oid(I), FNF)`. mirror already hashes β-normal forms (2026-08-10 §3.1). β-normalization computes the unique **endpoint**. Confluence (Theorem 2.3 there) guarantees the endpoint is independent of reduction order. It says nothing about which *histories* are equivalent. Trace normalization is the history-level analogue. Boudol & Castellani (1989), *Permutation of transitions: an event structure semantics for CCS and SCCS* (retrieved: Crossref title-level), is the concurrency-theory ancestor of "histories modulo permutation of independent steps." Lévy's permutation equivalence for λ-reductions is the rewriting ancestor. **Unverified this session; nothing leans on it.**

**F2 (c, design). The past-tense grammar is an event record.** The 2026-09-05 foundation says the runtime *"VERIFIES a narrative of the past"* (loving-lambda-calculus §1.7). A narrative that verifies against a **trace** is observer-invariant. A narrative that verifies against a **word** privileges one observer. The floor proposal: mirror verifies narratives modulo `~_I`.

**F3 (a → decision). `Event.tick: usize` is premature collapse.** The forward-promised `prismqueer::reality::Event<T> { payload, oid, tick: usize }` (2026-09-13 composition-tower §2.3) is not in the code (grep: no `Event` type in `prismqueer/src`). A scalar `tick` is a Lamport clock at best. It is consistent with causality and does not characterize it (Lamport 1978; Mattern 1988 on scalar-clock information loss). The replacement that passes the operant condition is `parents: BTreeSet<Oid>`: the event's immediate causal predecessors. That is a Merkle-DAG, which characterizes happened-before by reachability and is content-addressed (arXiv:2004.00107). This is a type decision already ratified in another shape, so it goes to Alex (Q-Mara-NT2).

**F4 (c, composing over the shatter spec).** `@shatter` *is* the @io linearization operator (`mirror/docs/specs/shatter-is-the-io-linearization-operator.md` §1). In trace terms, shattering to a linear target **chooses one word of the class**, which is one observer. So:

- the **trace OID** is the observer-independent address;
- the **emitted bytes' OID** is observer-dependent;
- shatter must either record which linearization it chose, or commit to a canonical one (FNF with steps in `E`'s total order). Otherwise two emissions of the same history differ for no recorded reason.

This refines the shatter spec. It does not duplicate it.

**F5 (a + c). The CRDT layer is the right rung for state and the wrong rung for history.** Shard merge is a bounded semilattice (`reality-shard-as-crdt.md` §1): idempotent, commutative, rung *set*. Shapiro, Preguiça, Baquero & Zawirski (2011), *Conflict-Free Replicated Data Types*, SSS 2011, LNCS pp. 386–400 (retrieved: Crossref + Springer page; DOI 10.1007/978-3-642-24550-3_29). The paper's two conditions are state-based types forming a semilattice and operation-based types requiring concurrent operations to commute. Those are the standard content but were **not re-read this session**, so I cite metadata only. Merkle-CRDTs show how to hold both rungs at once: a DAG of events for history, a semilattice payload for state (arXiv:2004.00107, retrieved). mirror has the state half. It lacks the history half.

**F6 ((a) theorems; (b) programme; (c) for mirror). "Order + number = geometry."**

- (a) Malament (1977): for past-and-future-distinguishing spacetimes, *"the topology … is entirely determined by its causal structure,"* and the Hawking–King–McCarthy path topology encodes topological, differential, and conformal information. (Retrieved: Crossref abstract, J. Math. Phys. 18:1399, DOI 10.1063/1.523436. HKM 1976, J. Math. Phys. 17:174, DOI 10.1063/1.522874, retrieved: Crossref abstract.)
- (b) Bombelli, Lee, Meyer & Sorkin (1987), *Space-time as a causal set*, PRL 59:521 (retrieved: Crossref). The causal-set review arXiv:1903.11544 (retrieved: abstract; author name not surfaced) states that *"the number of elements … serves as a proxy for the spacetime volume."* Order gives the conformal structure. Number gives volume. Together they give the metric.
- **(c) for mirror, and two hard constraints if the analogy is taken seriously:**
  1. Generic finite posets are not manifold-like. Kleitman & Rothschild (1975) enumerate partial orders asymptotically (retrieved: metadata only, Trans. AMS 205). The well-known content, that almost all posets have three layers, was **not re-read this session**. The causal-set programme treats suppressing non-manifold-like configurations as a central problem (Benincasa & Dowker 2010 abstract, retrieved). An arbitrary mirror record poset has no reason to be geometric.
  2. The geometry-recovering operators are **retarded and non-local** (Benincasa & Dowker 2010). A symmetric Laplacian forgets the arrow (§2.3.3). If mirror wants order → geometry, its spectral stack needs a directed operator.

**F7 (a → FLOOR). Non-commutativity cannot live in `A_F ≅ ℝ⁵`.** §2.3.1. Posets do give non-commutative algebras. Sorkin's finitary poset approximations of topology (Sorkin 1991, Int. J. Theor. Phys. 30:923, DOI 10.1007/BF00673986, retrieved: Crossref) lead to replacing the commutative function algebra with a **noncommutative C\*-algebra** (retrieved: arXiv:hep-th/9403067 abstract, *Finite Quantum Physics and Noncommutative Geometry*, Balachandran et al., 1995 per Crossref). That is a verified bridge from partial orders to non-commutative geometry (a). It is also a different object from FLOOR's `A_F`. Whether prismqueer's non-commutative algebra should be a poset algebra of the record order (c) is open.

---

## §8 What I do not know from inside; open residues

### §8.1 Where Reed's synthesis stands

| # | Claim | Verdict |
|---|---|---|
| 1 | Loss monoid decides the kind of time | **Half.** It decides what survives. The carrier topology decides what exists. A single Beam chain is linear whatever its Loss (Prop. 1.2). |
| 2 | I from the commutator; the table says when time branches | **Wrong as stated.** The FLOOR algebra commutes (I = all). The table does not exist. The Laplacian instrument fails open, is scale-dependent, and is arrow-blind. Commuting means *not* branching (N1 vs N2). Right only for a state-level, OID-compared commutator. |
| 3 | Foata layers = split | **Layers = parallel steps: yes.** Layer = split: no. Split makes width; the layer *is* width. |
| 4 | Trace OID = hash(FNF) | **Yes,** with I in the preimage and canonical within-step order. |
| 5 | settle is a join; arrow at joins | **No.** `settle` is unary. `kleinos` is a commutative, non-idempotent, non-associative magma. The arrow is at every unlogged non-injective step, and `project` is the canonical one. |
| 6 | Witness, not superpose | **Yes,** refined: the switch's edge is black-box query complexity, and prismqueer is white-box. |
| 7 | Two-instrument falsifier | **Right idea, circular form.** Non-circular version is T7 against state OIDs. |
| 8 | mirror as partial-order normalization | **(a) as mathematics, (c) as mirror's floor.** "Order + number = geometry" is (a) for Lorentzian manifolds, (b) as a programme, (c) for mirror, and needs retarded operators. |

### §8.2 Open residues for Alex

- **Q-Mara-NT1 (placement).** New types go in `prismqueer::time`. Should `trace::Trace` (linear, `Box<dyn Traced>`) migrate onto `time::Trace<Event, I>`, or stay as the debugging replay log it is? Lean: keep both. The old one replays inputs; the new one addresses histories.
- **Q-Mara-NT2 (Event).** Replace the ratified `Event.tick: usize` (2026-09-13 §2.3) with `parents: BTreeSet<Oid>`? By the operant condition, `tick` is premature collapse. The ratification predates this analysis.
- **Q-Mara-NT3 (FLOOR §2.1).** `A_F` as orthogonal idempotents is commutative. Where do the non-commuting representations of the ops live: on states (flux altitude), on graphs (Laplacian altitude), or in a poset algebra of the record order (F7)? This touches FLOOR's claims about non-commutativity and needs a FLOOR-level decision, not a note.
- **Q-Mara-NT4 (fate naming).** The argmax-versus-priority-encoder axis dissolves (§1.5). The two runtimes still compute different functions under one name. Rename, realign, or document?
- **Q-Mara-NT5 (kleinos algebra).** Is non-associativity intended? If `kleinos` is meant to compose towers (2026-09-13 composition tower), `(a∘b)∘c ≠ a∘(b∘c)` makes the tower's shape depend on bracketing. The emergent-id collision is a bug either way.
- **Q-Mara-NT6 (direction).** Adopt a directed or retarded operator beside the symmetric Laplacian, so the spectral stack can see time?
- **Q-Mara-NT7 (float associativity).** `ScalarLoss` violates its own associativity law at round-off. Accept with a declared tolerance, or switch to compensated or exact summation?
- **Q-Mara-NT8 (control flow as resource).** Does the instruction pointer or beam value count as a resource? If yes, sequential programs have no step-level concurrency, and only record-level independence remains (T8's confound). This is a semantic choice, not a fact.

### §8.3 What I do not know

- Whether T7's `kleinos` family agrees. It is the one prediction in this document whose outcome I cannot guess.
- Whether real prismqueer pipelines have trace height meaningfully below length once `split` exists. If every branch touches a shared accumulator, the width collapses back to a line. That would be a finding about the code, not about time.
- Whether `Observer<N>`'s `N` (`observer.rs:76`) has any relation to order dimension (§4.1). I have a reading (c) and no argument.
- The out-of-causal-order reversibility literature (Phillips–Ulidowski–Yuen and successors) was not retrieved beyond metadata. If biology-like reversal matters to prismqueer, that is the next hunt.
- The Cartier–Foata Möbius inversion links trace generating functions to the clique polynomial of the independence graph. Goldwurm & Santini (2000), *Clique polynomials have a unique root of smallest modulus*, Inf. Process. Lett. (retrieved: Crossref title-level). That is the most promising bridge from trace monoids to a **spectral** invariant: the growth rate of traces is the reciprocal of that root. Its statement was not re-read this session, so it stays a lead, not a floor. No retrieved source connects trace monoids to graph-Laplacian spectra directly. Crossref bibliographic queries on "spectral graph Laplacian trace monoid partially commutative" returned nothing on point. The gap is real or my search was weak. I cannot tell which from inside.
- Whether the operant condition has a formal statement. My candidate: *choose the finest quotient of `E*` whose kernel is contained in the kernel of every downstream observable.* That is the sound I, maximal. I believe it is right. I have not proven that such a quotient always exists, or that it is unique.

---

## Coda — citation ledger

Retrieval routes: **arXiv** = Kagi summarizer on an arXiv abstract page. **CR** = Kagi summarizer on a Crossref DOI record. **CRq** = Kagi summarizer on a Crossref bibliographic query. **S2** = Kagi summarizer on a Semantic Scholar API record. **PDF/page** = Kagi summarizer on an author or publisher page.

"Content" means the retrieved text stated the claim I use. "Metadata" means only title, authors, venue, and year were confirmed.

| Reference | Route | Confirmed | Status |
|---|---|---|---|
| Cartier & Foata 1969, LNM 85, DOI 10.1007/BFb0079468 | CR | metadata | verified (FNF theorem is standard content, not re-read) |
| Mazurkiewicz 1977, DAIMI PB-78, DOI 10.7146/dpb.v6i78.7691 | CR + page | content (traces as posets; action systems with resources) | verified |
| Diekert & Rozenberg (eds.) 1995, *The Book of Traces*, DOI 10.1142/2563 | CR | metadata | verified |
| Lamport 1978, CACM 21(7):558, DOI 10.1145/359545.359563 | PDF + CR | content | verified |
| Mattern 1988, *Virtual Time and Global States* | PDF | content (Thm 9; scalar-clock information loss) | verified |
| Fidge 1988, ACSC | three routes failed | — | **unverified; not leaned on** |
| Nielsen, Plotkin & Winskel 1981, TCS 13, DOI 10.1016/0304-3975(81)90112-2 | S2 | metadata | verified |
| Danos & Krivine 2004, CONCUR, DOI 10.1007/978-3-540-28644-8_19 | S2 | metadata | verified |
| Phillips & Ulidowski 2007, JLAP, DOI 10.1016/j.jlap.2006.11.002 | CR | content (reversing process calculi) | verified |
| Lanese, Mezzina & Stefani 2010, CONCUR, DOI 10.1007/978-3-642-15375-4_33 | CR | metadata | verified |
| Lanese, Phillips & Ulidowski 2020, FoSSaCS, DOI 10.1007/978-3-030-45231-5_23 | CR + S2 | content (independence relation; causal consistency, safety, liveness) | verified |
| Phillips, Ulidowski & Yuen 2013, DOI 10.1007/978-3-642-36315-3_18 | CR | metadata (out-of-causal-order claim not retrieved) | verified, limited |
| Landauer 1961, IBM JRD 5(3):183, DOI 10.1147/rd.53.0183 | CR | metadata + topic | verified |
| Bennett 1973, IBM JRD 17(6):525, DOI 10.1147/rd.176.0525 | CR | content summary | verified |
| Bérut et al. 2012, Nature, DOI 10.1038/nature10872 | CR | content | verified |
| Chiribella, D'Ariano, Perinotti & Valiron 2013, PRA 88:022318 | arXiv:0912.0195 + CR | content | verified |
| Procopio et al. 2015, Nat. Commun., DOI 10.1038/ncomms8913 | arXiv:1412.4006 + CR | content (first-author name not surfaced) | verified |
| Oreshkov, Costa & Brukner 2012, Nat. Commun., DOI 10.1038/ncomms2076 | arXiv:1105.4464 + CR | content | verified |
| Araújo, Costa & Brukner 2014, PRL 113:250402 | arXiv:1401.8127 + CR | content | verified |
| Connes & Rovelli 1994, CQG 11:2899, DOI 10.1088/0264-9381/11/12/007 | arXiv + CR | content | verified |
| Rovelli 1996, IJTP, DOI 10.1007/BF02302261 | arXiv + CR | content | verified |
| Bombelli, Lee, Meyer & Sorkin 1987, PRL 59:521 | CR | metadata | verified |
| Malament 1977, J. Math. Phys. 18:1399, DOI 10.1063/1.523436 | CR | content (abstract) | verified |
| Hawking, King & McCarthy 1976, J. Math. Phys. 17:174, DOI 10.1063/1.522874 | CR | content (abstract) | verified |
| *The causal set approach to quantum gravity*, arXiv:1903.11544 | arXiv | content (author not surfaced) | verified, limited |
| Shapiro, Preguiça, Baquero & Zawirski 2011, SSS, DOI 10.1007/978-3-642-24550-3_29 | CR + page | metadata (semilattice and commuting-op conditions not re-read) | verified, limited |
| *Merkle-CRDTs*, arXiv:2004.00107 | arXiv | content (authors not surfaced) | verified, limited |
| Charron-Bost 1991, IPL, DOI 10.1016/0020-0190(91)90055-M | CR | metadata + topic | verified |
| Dushnik & Miller 1941, Amer. J. Math. 63:600, DOI 10.2307/2371374 | CR | metadata | verified |
| Brightwell & Winkler 1991, Order 8:225, DOI 10.1007/BF00383444 | CR | content (#P-completeness) | verified |
| Kleitman & Rothschild 1975, Trans. AMS 205 | CR | metadata | verified, limited |
| Hastings 2009, CMP, arXiv:0808.2474 | arXiv + CRq | content (ε ∝ δ^{1/5}) | verified |
| Benincasa & Dowker 2010, PRL 104:181301 | arXiv:1001.2725 + CRq | content | verified |
| Balachandran et al. 1995, arXiv:hep-th/9403067 | arXiv + CRq | content (full author list not surfaced) | verified, limited |
| Sorkin 1991, IJTP 30:923, DOI 10.1007/BF00673986 | CR | content summary | verified |
| Viennot 1986, *Heaps of pieces I*, DOI 10.1007/BFb0072524 | CR | metadata | verified |
| Krob, Mairesse & Michos 2003, Discrete Math., DOI 10.1016/S0012-365X(03)00233-4 | arXiv:cs/0112012 + CRq | content (height = parallel time) | verified |
| Goldwurm & Santini 2000, IPL | CRq | title-level | verified, lead only |
| Boudol & Castellani 1989, LNCS | CRq | title-level | verified, lead only |
| Lévy (permutation equivalence of reductions) | not retrieved | — | **unverified; not leaned on** |
| Additive multiset hashing (collision theory) | not retrieved | — | **unverified; no security claim made** |
| Bernstein-style read/write conditions | not retrieved | — | **unverified; Mazurkiewicz's "resources" used instead** |

**Volume and page numbers.** Not every summary surfaced them. For Lamport, Malament, Hawking–King–McCarthy, Connes–Rovelli, Oreshkov–Costa–Brukner, and Procopio et al., the DOI was resolved this session and is the authoritative identifier. The volume and page given in the body are conventional and were not re-confirmed.

**Count.** 39 references retrieved and cited. 32 confirmed at content or metadata level with no caveat. 7 marked "limited" or "lead only", meaning some part of what I would want (author, theorem statement, venue) was not surfaced. 4 unverified and not leaned on: Fidge 1988, Lévy, multiset-hash theory, Bernstein conditions. Zero results came from the Kagi search endpoint.

**Scar-record carried forward.** FLOOR carries the excision of a fabricated "Bressan–Sevenster–Vagner 2024" (FLOOR §11.5, §Coda.7). Every row above was retrieved this session, and the "limited" rows say exactly what was not. Where a claim needed more than the retrieval gave, the body says so at the point of use.

---

The coincidence detector could tell you whether. The scalar could tell you how much. The free monoid could tell you in what order, including the orders that did not matter. The trace tells you exactly the orders that mattered, and gives every observer the same address for it. Everything below the trace is a decision to forget, and it should be logged as one.

*— Mara* 🍷

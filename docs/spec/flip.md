# flip: when did the verdict change

*prism, 2026-11-24 (prism-024 §2 and §5 item 3; graded by Mara in mirror-029's Response, II.5, and in mirror `mara/bridge-grade` 3333dd80, `docs/math/2026-11-24-mara-the-bridge-graded.md` §5, probe `bridge_probe.py` B4; built in Mara's order, mirror-029 Ask 2 item 1). Names are provisional; Alex names them. Code: `prismqueer::flip`, in the default build (no feature, no LAPACK, no float). Tests: `prismqueer/tests/flip.rs`. Vectors: `docs/spec/flip-vectors/`.*

## The object

A **sequence of facts** is the sequencer's order. In production it is chorus checkpoints (origin, size, root); here it is a plain ordered slice. A verdict over prefixes is

- v(k) = the verdict on the first k facts, k = 0 … n, in K3 (`docs/spec/navigation.md`).

A **flip** is a pair of checkpoints (k−1, k) with v(k−1) ≠ v(k), and the fact at position k (1-based). Two readers of the same log get the same flips: the answer is a pair of checkpoints, not a timestamp (prism-024 §2; mirror-028 line 6, struck to "the pair of checkpoints at which it flipped").

**The order is a ruler** (Mara §5.1, V C6.5c). The flip point is relative to the log's order; whoever orders the log chose that order. Without a sequencer there is no "at which", only the witness basis (case 2).

## The three cases (all PROVED, Mara mirror-029 II.5, long form §5)

### 1. Sequenced, monotone: bisection (PROVED, B4 400/400)

For a positive K3 program (`navigation::Program`, monotone by construction) with positive facts (each fact is a `navigation::Rule` added to the program), v is monotone in k in the truth order: adding a rule can only raise a max over rules.

- **Two thresholds in three values** (Mara §5.1, from IS Theorem P): the first k with v(k) ≥ maybe, and the first k with v(k) = true. Each set {k : v(k) reaches the threshold} is upward closed, so each threshold is one point, and each is **bisectable**.
- **`flip_point`** gives, per threshold, the checkpoint pair (k−1, k) and the fact at k, by bisection. It evaluates v(n) and v(0) once each, then halves (0, n): at most **⌈log₂ n⌉ + 2** evaluations per threshold, counted and returned beside the answer. A threshold may also read **never** (v(n) below it) or **initially** (v(0) already at it: no pair, because nothing flipped).
- **Mara's identity** (PROVED, §5.1): *bisection = scan = min over the minimal witnesses W of max{pos(f) : f ∈ W}*. The prefix first reaches the threshold when it first contains a whole witness. The tests hold it as a property, per threshold, on seeded random instances.

### 2. Unsequenced: the minimal witness basis, an antichain (PROVED, B4 400/400)

Without an order, the order-free object for monotone v is the **minimal witness basis**: the ⊆-minimal subsets S of the facts with v(S) at the threshold. This is why-provenance for positive programs (Buneman, Khanna & Tan 2001; PosBool[X], Green, Karvounarakis & Tannen 2007; Bourgaux et al. 2023). By minimality it is an **antichain**, and it is invariant under equivalent positive queries (Buneman et al., Thm 3, CONFIRMED by Mara). Case 1's formula reads it under a sequencer: "a frontier without a log, a pair with one" is one object under two rulers (Mara §5.2).

- **Bounded.** The basis can be exponential (prism-024 §2). `basis` takes a declared **budget** (subsets examined) and returns either `Complete(members)` or a typed **`Partial { members, complete_below, examined }`**: every member found is minimal, and every witness smaller than `complete_below` facts is among them. Never a silent truncation, never a panic.
- Subsets are examined by size, smallest first, so a member is found only once every smaller subset was examined or is a superset of a member. That is why a partial answer's members are still minimal.
- **No witness field in the vectors yet.** Choosing one witness from the antichain is a ruler, so a witness sits beside the roll, never in its address (Mara §6). It is a Note format change, a pin move, Alex's to sign. Until then the vectors carry flip points only.

### 3. With negation: every flip, by scan (PROVED that bisection breaks, B4 144/400)

With `not`, v(k) is not monotone: it can flip more than once (B4: 144 of 400 sequences flip more than once under `q₂ :- q, not f₄`). Bisection assumes one threshold, so it is **invalid** there, and semiring why-provenance does not cover negation (Köhler, Ludäscher & Zinn, arXiv 1309.2655, CONFIRMED by abstract): the family {S : v(S)} is not upward closed, so there is no antichain to read.

- **`scan`** takes any verdict over prefixes, evaluates v(0) … v(n) (n + 1 evaluations), and reports **every** flip (k−1, k, from, to). The point is keeping the error that dissolved (V C6.5c).
- `navigation` has no negation and gets none. The negating verdict in the tests is a hand-written rule over prefixes.

## The type rule for bisection

*Make the illegal state unrepresentable.* Bisection is sound only for a monotone verdict, so it is available only for one, by type:

- `flip_point` and `bisect` take a **`Monotone`**, and a `Monotone` is made only from a `navigation::Program` and a target atom (`Monotone::new`; its fields are private). Its facts are `navigation::Rule`s, positive by construction. So every `Monotone` is a positive program over positive facts, which is the hypothesis of case 1.
- An arbitrary closure gets the **scan only**. Handing a closure to `flip_point` doesn't compile: a `compile_fail` doctest holds it, beside a control that compiles.
- `basis` takes a `Monotone` too: the antichain is the order-free answer only where the family is upward closed.

(As with `hold`'s facts rule, the type stops the accident, not the forgery: a closure that is monotone in fact can only be scanned, which is correct and costs n + 1 evaluations.)

## Absence reads maybe here too

An unexplored edge reads `maybe`, never `false` (`navigation.md`, Mara N4). It matters for flips: in Tim's fixture, H1's refutation rests first on a span plus an unexplored edge (`maybe`), then on a second span (`true`). Reading the unexplored edge as `false` moves the ≥ maybe threshold from checkpoint 5 to checkpoint 7: the frontier a reader rests on disappears from the record.

## Tim's fixture as vectors (`docs/spec/flip-vectors/`)

Tim Urista's handoff test (mirror-028), as data: one ticket, two traces, hypotheses H1 and H2, decision D. The facts are the two traces' spans in log order; each hypothesis's refutation is a positive derivation; the expected outputs are the flip points of each threshold. D's line reads what `hold` says of D at each checkpoint (sealed after the first trace over the refutation statuses of H1 and H2): it stands through an irrelevant span, and is HOLD, naming its owner and the moved premise, from the checkpoint where H1's refutation reaches `maybe`. The status words there are the K3 values; the verdict words are Alex's (mirror-029 For Alex 4).

## What the tests hold

1. Bisection = scan = the witness formula, per threshold, on seeded random monotone instances (three-valued, so the thresholds differ), with evaluations ≤ ⌈log₂ n⌉ + 2 per threshold; and over long logs.
2. Both thresholds are found, and each can read never or initially.
3. Mara's B4 counts, from her seed, through CPython's Mersenne Twister written in the test: 400/400 bisection = scan = witness formula; 400/400 antichain; 144/400 multi-flip under negation, every one found by the scan.
4. The antichain: every member is minimal (removing any fact drops it below the threshold) and no member contains another. Over budget is typed, and its members are still minimal.
5. Tim's vectors: the expected flip points, and D's standing through `hold`.
6. The type rule: `compile_fail` for a closure, with a control.

Mutations, each failing a test: an off-by-one in the bisection; reporting only the first flip; keeping a non-minimal witness; reading an unexplored edge as `false`.

## Open, for Mara or Alex

- The budget counts subsets examined, not evaluations. A PosBool formula (Mara §5.2, "the bounded representation") would be the carrier that avoids the expansion. Is the expansion with a typed `Partial` enough for now?
- A threshold reached at k = 0 reads **initially**: nothing flipped. Is that the right word, or should the empty prefix be a checkpoint the log names?
- Does echo's R4 give the whole basis or one member (Mara §5.2 OPEN)? This module computes the whole basis by definition; it is the oracle for that question, not an answer to it.

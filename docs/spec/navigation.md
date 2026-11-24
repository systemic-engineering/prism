# navigation: the K3 floor a navigator reads over

*prism, 2026-11-24 (identity handoff `prism-025` Addendum item 1; graded by Mara in mirror `mara/witness-navigation-grade` 052e5a13, `docs/math/2026-11-24-mara-witness-navigation-event-sourced.md` §2.1–§2.3, probe `wnes_probe.py`, cited N1–N5). Names are provisional; Alex names them. Not "D2": prism's D2 is Hypothesis. Code: `prismqueer::navigation`, in the default build (no feature, no LAPACK, no float). Tests: `prismqueer/tests/navigation.rs`.*

## The algebra: K3

K3 = {false < maybe < true}, with

- ⊕ = max, ⊗ = min, 0 = false, 1 = true.

**PROVED** (Mara §2.1, N1: 27/27 triples): a commutative semiring, ⊕-idempotent, and **0-closed**: true ⊕ a = true, so a* = true. It is the bottleneck (widest-path) semiring restricted to three values. GraphBLAS ships it as `MAX_MIN` (**CONFIRMED**, SuiteSparse `GB_control.h`), over F, U, T as 0, 1, 2.

Two orders on the same three values:

- **truth** (≤t): false < maybe < true;
- **knowledge** (≤k): maybe below both false and true; false and true incomparable. Settling a `maybe` (maybe → true, or maybe → false) raises knowledge.

## The carrier: `Membership`, with set-valued payloads

*"`Membership` is the carrier, not the algebra"* (Mara §2.1). The carrier is terni's `Membership<I, O, U>` (re-exported as `algedonic::Membership`): `In` = true, `Undecided` = maybe, `Out` = false. It has no order, meet or join of its own; K3 is a separate type over it.

- **Payloads are sets** (`BTreeSet`, under union). Lifted ⊕ and ⊗ keep the K3 value of max or min, and the union of the payloads of the operands that carry that value. So a meet of two `In`s carries both certificates, and a join of two `Out`s carries both witnesses (a cut). Picking one certificate from the set is a ruler (Mara, BG §6), so the floor never picks.
- The lifted operations are commutative, associative and idempotent, and the carrier map to K3 is a homomorphism for both. The semiring laws (distributivity, identities) are stated on the K3 value, not on the payloads.

## For atoms: K3-Datalog over hyperedges

A ground rule `h :- b₁, …, bₖ` is a hyperedge (Mara §2.1, after M3 2026-10-29; GKT 2007, positive K3-annotated Datalog). A body item is an atom or an edge label.

- **A rule's value** is the ⊗ (min) over its body. The empty body is 1 (true).
- **An atom's value** is the ⊕ (max) over its rules. An atom with no rule is 0 (false), in a program whose edges are all explored.
- **The value is the least fixed point** in the truth order, reached by chaotic iteration from all-false. Because ⊕ is idempotent and every operation is monotone on a finite lattice, every fair order reaches the same fixed point (Cousot 1977; **N3**).
- **Payloads are a second, separate fixed point**, taken once the values are fixed: each atom's payload is the union over its rules that reach its value, each rule's the union over its body items that reach the rule's value, least under inclusion. Two phases keep the payload order-free, too.

Graph navigation (shard to shard along `in @` edges) is the special case where every rule is `v :- u, edge(u, v)`: the matrix case, a MAX_MIN closure. It's expressed through the same rules here; no separate matrix closure is kept.

## The two laws

1. **Monotone in both orders** (Mara §2.2, N2: min and max ≤t-monotone 36/36, ≤k-monotone 25/25; Fitting's result for Kleene's strong connectives, CONFIRMED). So the fixed point is ≤k-monotone in the edge labels: **settling a `maybe` input never flips a decided output** (N5: 0 flips over 2,000 graphs). Settling needs no coordination for navigation's verdicts. What *can* move a decided verdict is growth: an added edge (absent → true) can raise a `false` (N5: 471 raised by one added edge). That is the closed-world read, and it is what needs a seal.
2. **An unexplored edge reads as `maybe`, never `false`** (Mara §2.3, N4). A navigator that stops early reads every edge it hasn't explored as `maybe`. Then the partial fixed point is a ≤k-lower bound of the full one (2,000/2,000), and **0** decided verdicts flip. Read as `false` ("default deny" as a value), **3,191** flip. Default-deny is a rule for what you do, never for what you conclude: `false` is earned by a seal on the frontier.

## Absence is its own constructor

An edge is either read, with its `Membership` label, or **unexplored**, naming itself. The unexplored constructor reads as `Undecided`, carrying its own name in the payload, so a `maybe` says which frontier edges it rests on. There is no path from "unexplored" to `Out`: a lookup that finds nothing becomes unexplored, not `false`. Reading absence as `false` is a closed-world step, and it belongs to whoever seals the frontier, never to the floor.

## What the tests hold (Mara's N1, N3, N4, N5)

- **N1:** the semiring axioms, ⊕-idempotence and 0-closure on all 27 triples of K3; plus ACI and the homomorphism for the lifted operations on the carrier.
- **N2** (the law behind N5): min and max ≤t-monotone 36/36, ≤k-monotone 25/25.
- **N3:** the fixed point is independent of rule order: 500/500 graphs × 5 orders.
- **N4:** unexplored as `maybe`: a ≤k-lower bound 2,000/2,000 and 0 flips; unexplored left out (read as `false`): 3,191 flips.
- **N5:** settling every `maybe` edge at random: 0 decided verdicts flip; one added `true` edge: 471 `false` verdicts raised.

**The data is Mara's, the code is not.** The graphs are her probe's: 7 nodes, each ordered pair an edge with probability 0.3, a uniform label, source node 0; for N4 each edge seen with probability ½. To reproduce her exact counts the tests regenerate her graphs from her seeds (31, 32, 33) through CPython's Mersenne Twister, written in the test from MT19937's definition and CPython's documented draws (`random()`, `getrandbits`, `_randbelow`), in integers. Her closure and counting are not ported: the fixed point is this module's, and the counts are checked against her output.

## Open, for Mara or Alex

- Lifted ⊗ doesn't distribute over lifted ⊕ on payloads (an `Out` witness can be kept on one side and not the other). Is ACI plus the value homomorphism the law Mara meant, or should the payload be a provenance polynomial (GKT) rather than a set?
- An atom with no rule reads `false` with an empty witness. That is right inside a fully explored program; a navigator must write `h :- unexplored(e)` for an atom whose derivations it hasn't read. Should a body atom with no rule be refused instead, so that closed-world reads are always explicit?

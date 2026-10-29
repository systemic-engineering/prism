# The convergence gate

*prism-001, 2026-10-27. The gate rule for bullet-up's M3 review pipeline, taken from prism's settle semantics. Worked examples: `gate-vectors.json`, checked against this rule by `prismqueer/tests/gate_vectors.rs`.*

## The rule in one paragraph

A draft goes through review rounds. In each round every observer returns an `Imperfect`: `Success` (no objection), `Partial` with a list of objections, or `Failure` (it couldn't review). The round's **state** is the draft's address plus the set of everything still open: every objection from every observer, and every observer that couldn't review. Nothing is averaged and nothing is voted away: one observer's objection is enough to keep an item open. The review has **converged** when a round's state is exactly the previous round's state. Converged with nothing open is **green**. Converged with something open is a **kept disagreement (a comma)**: not green, and it goes to Alex as a question. Out of rounds while the state is still changing is **still moving**: not green, also a question. If nobody could review at all, the gate **fails closed**.

## Definitions

- **Observer result:** `Success`, `Partial(_, items)`, or `Failure(error, _)`.
- **Open items of a round:** `"<observer>: <item>"` for every objection, and `"<observer>: did not review (<error>)"` for every observer that failed. A set: order doesn't matter, duplicates count once.
- **State of a round:** (draft address, open items).
- **No further change:** this round's state equals the previous round's state, exactly.
- **Budget:** the maximum number of rounds.

## The gate's answer, as an `Imperfect`

| Situation | Gate result | Green? | Goes to Alex as |
|---|---|---|---|
| converged, nothing open | `Success(draft)` | yes | nothing |
| converged, items open | `Partial(draft, {kept: items})` | no | "These disagreements didn't settle: …" |
| out of rounds, still changing | `Partial(last draft, {moving: items})` | no | "This didn't settle in N rounds. Open now: …" |
| every observer failed in a round | `Failure(unmeasured, {unmeasured: items})` | no | "Nobody could review this." |

**Green means `Success` and nothing else.** `Partial` is red, and it isn't a failure either: it's "here is the draft, and here is what's not settled."

**Default budget: 4 rounds** (up to three working rounds plus one that confirms nothing changed). bullet-up's own expectation is that most drafts settle within three.

## Worked examples

All eight are in `gate-vectors.json`:

1. Two identical clean rounds → green.
2. The same objection twice on an unchanged draft → kept: `["a: tone too formal"]`.
3. An objection, a new draft with no objections, the same again → green on the new draft.
4. A different objection every round until the budget (3) runs out → moving: `["a: z"]`.
5. One observer times out twice → kept: `["b: did not review (timeout)"]`. A missing observer is never dropped quietly.
6. Every observer fails → fail closed.
7. A budget of 1 → still moving, even with no objections: one round can't show a fixed point.
8. The same objection, reworded in round two, then repeated → the rewording counts as change; it converges in round three, kept.
9. The same objection with different spacing → not a change (same words in the same order); converged in round two, kept.

## What's proven in prism, and what's a choice for bullet-up

**Proven in prism** (tests you can run):

| Principle | Where |
|---|---|
| Sampling can't settle a universal claim; a clean search is `Partial`, never `Success` | `prismqueer/tests/search.rs` |
| Stop when nothing new appears for a while (saturation with patience) | `prismqueer/tests/steer.rs` |
| Settling keeps what no assertion can resolve (the comma survives) | `prismqueer/tests/instrument.rs`, `decay_keeps_the_comma` |
| When the instrument can't measure, refuse instead of guessing | `prismqueer/src/spectral/settle.rs` (`Wall::Unseparated`), `commutator.rs` (`Unmeasured`) |
| Resuming equals not stopping, so a review can pause between rounds | `settle.rs`, `resuming_equals_not_stopping` |
| Collecting objections as a set is lawful (union is associative, has an identity) | the loss laws, `prismqueer/tests/loss_laws.rs` |

**Choices for bullet-up** (reasonable defaults, not theorems):

- **What counts as "the same".** The draft's address plus each objection's **word-level id** (decided by bullet-up, 2026-10-27): normalise the text to Unicode NFC, take its words in order, hash them. So spacing and Unicode encoding don't count as change (example 9), but a reworded objection does (example 8). Where this line is drawn decides where the review stops. It's the observer's boundary, written down. The reference evaluator compares words in order; it doesn't apply NFC itself, so the examples stay in ASCII.
- **One confirming round** (patience 1). More rounds of patience make green slower and more certain.
- **Budget 4.**
- **A failed observer counts as an open item.** The alternative, ignoring it, would let a crashed reviewer turn red into green.
- **Fail closed only when every observer fails.** A partial outage is red (`Partial`), not a failure.
- **Kept vs. moving** are separated in the loss so Alex can see which question they're being asked. Both are red.
- **No majority rule.** One dissent keeps an item open. That's the "compose, don't average" line: the comma goes to a human instead of being outvoted.

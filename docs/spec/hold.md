# hold: a decision stands while what it read stands

*prism, 2026-11-24 (prism-024 §1 and §5 item 2; graded by Mara in mirror-029's Response, I.1, and in mirror `mara/bridge-grade` 3333dd80, `docs/math/2026-11-24-mara-the-bridge-graded.md` §1; built in Mara's order, mirror-029 Ask 2 item 1). Names are provisional; Alex names them. Code: `prismqueer::hold`, in the default build (no feature, no LAPACK, no float). Tests: `prismqueer/tests/hold.rs`.*

## The law

*A decision sealed at premise projections (p₁ … pₖ) is HOLD exactly when some pᵢ's projection address now differs from the one sealed. Only its owner's seal over the current tuple clears it.*

```
standing(decision, current outputs) -> Stands | Hold { moved: [premise ids], owner }
reseal(held, new seal, current outputs) -> Ok(decision) | Refused { NotOwner | OtherDecision | OldTuple | NotCurrent }
```

- **A premise** is a declared derivation's output that a decision reads. It has a name (which derivation), a **status** (the verdict word the decision acts on), and things that sit beside the status: the stage (λ, ρ), the cites, the facts head. Its **full output address** covers all of them.
- **The premise projection** is what a decision reads of an output, and its address is what the decision seals. It is one named choice in the code (below).
- **A decision** is a draft (the rule it decides, the tuple of projections it read) sealed by its owner. Its address is `"decision\0" ‖ projection name ‖ rule ‖ the tuple`. The owner's signer sits **beside** that address, never in it (`ruler.md`): one judgment sealed by two people is one address and two signers.
- **Standing** projects each current output the decision declared and compares it with the sealed one. Equal everywhere: it stands. Otherwise it is HOLD, naming every premise that moved and the owner who can re-seal. A declared premise with no current output counts as moved: absence is not the sealed projection.
- **Clearing** takes a new seal. It is refused unless it is the held decision's owner's, over the same rule and premise names, and over the *current* tuple. A seal over the sealed (old) tuple is `OldTuple`; over any other tuple, `NotCurrent`, naming the premises that don't match.

## Which half is a theorem, and which is policy

Mara's grade (mirror-029, I.1; long form §1.1–§1.2), in Model D: declared premises, deterministic derivations, collision-free addresses, and the decision reads its premises only through their projection addresses.

- **The standing half is PROVED** (Mara's Lemma H): unchanged projection addresses mean the decision's inputs are unchanged, so its seal still covers exactly what it read. It is a lemma of incremental soundness (mirror `mara/incremental-soundness` 4909412b, **IS §5**: equal keys mean equal rolls), with the decision's key K(D) = H(rule ‖ addr(p₁) ‖ … ‖ addr(pₖ)). That is early cutoff: a fact that moves no projection leaves every seal standing.
- **The HOLD half is policy, not a theorem.** In IS a key miss means recompute. A decision isn't a function of its premises; it is a person's act over them. So a miss means *ask the owner to re-seal*. That is the authority rule (rev 36; convergence_ci RT2).
- **The "iff" over-HOLDs.** An address can move while what the decision acted on hasn't. The law is sound and conservative, never exact: exactness would need a semantic check (IS §3.3). The projection decides how much it over-HOLDs.
- **Undeclared premises are outside the law** (Mara §1.2 item 4). A decision that read something it didn't declare isn't covered by Lemma H.
- **The seal must name the tuple it covers** (Mara §1.2 item 5), so a re-seal that doesn't name the current tuple clears nothing.

## The projection: status alone (flagged for Alex)

**Mara's lean, provisional:** the premise projection is the **status alone**. λ, ρ, the cites and the head sit beside it. A stage move is then reported, never a HOLD.

- In code it is one trait, `Projection`, and one implementation, `StatusAlone`, which every decision uses by default. Including λ is a new implementation (or an edit of that one), with its own name in every projected address, so decisions sealed under two projections never share an address. The law itself doesn't change.
- *Dissonance (Mara, kept):* someone may want to re-seal when a hypothesis was refuted *earlier* than they thought (ρ moved). That is a different ruler. It should be declared, not defaulted.
- **Why the head must never be in it** (Mara §1.2 item 3, the B1 probe): a facts head moves on *every* fact, so a projection carrying it HOLDs every dependent decision on every fact, the storm early cutoff exists to prevent. Over Mara's log of 21 facts with one flip and one later second refutation: **21 HOLDs** with `facts_head` inside, **2** with `cites` inside, **1** with the status alone. The tests reproduce those three numbers through `standing`.

## Facts never enter a decision

A decision can be sealed only over projections of derivation outputs, never over a fact set. A `Premise` is made only by projecting an `Output`; a `Facts` value is not an `Output` and has no conversion to a `Premise`. The draft's constructor takes premises only, so handing it facts doesn't compile. A `compile_fail` doctest holds it, beside a control that compiles. (A caller can still write a fact set's address into an output's status string. The type stops the accident, not the forgery; the forgery is an undeclared premise.)

## What the tests hold (Mara's Ask 2 item 1)

1. An irrelevant fact leaves every seal standing, while each output's head (beside) moves.
2. A relevant fact HOLDs exactly the dependents: D1 reads p1, D2 reads p1 and p2, D3 reads p3; a fact that moves only p1 HOLDs D1 and D2, never D3, each naming its owner and p1.
3. A stage move without a status move doesn't HOLD: the full output address moves, the projection doesn't. Flagged for Alex.
4. A projection carrying `facts_head` storms: B1's 21 / 1 / 2, documenting.
5. Only a seal naming the current tuple clears: a non-owner, the old tuple and a non-current tuple are refused; the owner over the current tuple stands.

## Open, for Alex or Mara

- The projection: status alone, or with λ (above).
- A missing current output counts as moved. Should it be its own case (`Unread`), since it is a `maybe`, not a new value?
- The verdict words in the status are Alex's (mirror-029 For Alex 4). Here a status is any string.

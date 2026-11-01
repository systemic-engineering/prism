# brain: capacity is verdict-neutral (the proof behind `6f892fd`)

*prism-Reed, 2026-11-01. Written for Mara's G-5 (prism-012 Log): a kind-2 choice must be
"earned by a named proof, guarded by a named test". The tests in `tests/brain.rs`
(threshold independence, the 5×5 lums sweep) **measure** the claim. This note **proves** it,
and names the assumptions it rests on. Each assumption is either checked at run time or
stated as the tenant's obligation.*

## The claim

Fix a tenant: an evaluator that drives a `Brain` with a sequence of calls (`deposit`,
`contains`, `scan`, `commit`, `settle`, `crystal`). Run the same tenant on the same input
under two capacities c₁ and c₂ (and any two rulers). Then:

1. every call returns the same value in both runs;
2. `lums()` and `lums_of(r)` agree after every call;
3. the settled crystals are byte-identical and have the same addresses.

So `Capacity` (with the ruler that orders eviction) is **kind 2**: it can't change a verdict.

## Assumptions

- **A1 (exact recompute).** For every relation r and pattern p, the tenant's `Recompute`
  returns exactly the committed facts of r matching p that the brain was ever given, each
  with its original deposit tick. It returns nothing more (soundness) and nothing less
  (completeness). *This is the tenant's obligation.* For mirror's parser it is Lemma G
  (deposits are the original naive ranks) together with a window-local re-run.
- **A2 (faithful backend).** `Backend::fetch` returns the bytes `persist` was given.
  *This is checked on every read.* `crystal()` rehashes what it reads back and returns
  `Recall::Forged` on a mismatch.
- **A3 (blind tenant).** The tenant's control flow depends only on values returned by
  `deposit`, `contains`, `scan`, `commit`, `settle` and `crystal`. It never reads
  `stats()` or hot bytes. *This is the tenant's obligation.*

## The implementation facts the proof uses (`prismqueer/src/brain.rs`)

- **F1.** Eviction (`press`) runs only inside `commit` and `settle`. It removes rows from the
  hot tier only, and only rows with a tick ≤ the committed tick. It never touches the
  counters or the crystals' bytes. With no `Recompute`, it evicts no facts at all.
- **F2.** `restore` inserts recomputed rows that match the pattern, aren't already hot,
  and have a tick ≤ committed, keeping each row's returned tick. It never touches the
  counters.
- **F3.** `deposit` first calls `contains`. Only if that returns `None` does it insert at
  the open tick and add one to the pending counters.
- **F4.** `contains(f)` returns the hot tick if f is hot. Otherwise it restores with f as
  a full pattern, then reads the hot tier.
- **F5.** `commit` moves pending into the lit counters, then may press (F1).
- **F6.** A crystal's address is `Oid::hash(scope, bytes)`, computed from its bytes alone.

## Proof

Let K (the *knowledge*) be the set of facts ever given to `deposit` and accepted, each
with its tick. Let H ⊆ K be the hot tier. Both depend on the run; K, we claim, doesn't
depend on the capacity.

**Invariant**, after every call, in both runs:
- (i) K and every tick in it are the same in both runs;
- (ii) H ⊆ K, with K's ticks;
- (iii) the counters are the same in both runs;
- (iv) every call so far returned the same value in both runs.

By induction on the calls. Initially K = H = ∅ and all counters are 0.

- **`contains(f)`.** If f ∈ H, it returns f's tick, and by (ii) that is K's tick. If
  f ∉ H, `restore` asks the recompute for the committed facts matching f exactly. By A1
  that is {f} if f ∈ K and f is committed, otherwise ∅. F2 inserts it with its K-tick.
  A fact of the open tick is never evicted (F1), so it is still in H. So `contains(f)`
  returns `Some(t)` iff (f, t) ∈ K, which is independent of H, so of the capacity.
  K and the counters are unchanged, and (ii) is kept, since F2 inserts only K-facts at
  K-ticks.
- **`deposit(f)`.** By F3 and the case above, it inserts iff f ∉ K, which is the same in
  both runs. When it inserts, K gains (f, open), and both runs' pending counters gain 1.
  The open tick is the same in both runs by (iv) (the number of commits so far). So
  (i)–(iv) hold.
- **`scan(r, p)`.** `restore` brings, by A1, every committed K-fact matching p into H.
  Open-tick K-facts are already in H (F1). So it returns exactly K ∩ p, with K's ticks,
  in tuple order, which is independent of H.
- **`commit`.** By F5 the counters move identically. `press` changes only H, within K (F1),
  so (i), (iii) and (iv) hold.
- **`settle(bytes)`.** The address depends on the bytes alone (F6). The bytes are the
  tenant's, and by A3 and (iv) they are the same in both runs. Persisting, then evicting
  the cached copy, changes nothing a later `crystal()` returns, by A2 (checked).
- **`crystal(a)`.** It returns the cached bytes, or the backend's bytes rehashed to a
  (A2). These are identical whichever way they're obtained.

So (iv) holds for every call, which is claim 1. Claim 2 follows from (iii), because
`lums` reads only the counters. Claim 3 follows from the `settle` case. ∎

**Why lums is right, not only equal.** By F3 the counters grow by exactly one per element
added to K, and F1 and F2 never move them. So lums = |K at the committed tick|: evicted
facts still count, and restored facts are never counted twice.

## Where it breaks: what each assumption costs

- **Without A1-soundness:** a recompute that returns a fact never deposited makes
  `contains` answer `Some` in the small-capacity run only, and the runs diverge. Partly
  checkable at run time: a full-pattern restore that leaves `evicted ≠ 0`, or a restore
  that over-subtracts, means the recompute is wrong. A checked mode for this is a
  follow-up.
- **Without A1-completeness:** a recompute that misses a fact makes `deposit` accept it a
  second time, at a later tick, so K and lums diverge. *This is what the "restore counted
  again" mutation in `tests/brain.rs` breaks, and what test 4 guards.*
- **Without A3:** a tenant that branches on `stats()` can make capacity visible. That is
  outside the claim by definition.

## The named tests that guard the proof

| test (`prismqueer/tests/brain.rs`) | guards |
|---|---|
| `threshold_independence_tiny_and_unbounded_give_the_same_light` | F1–F5 as implemented, end to end, with eviction proven to happen |
| `warmer_never_truer_priority_and_threshold_never_change_the_light` | the ruler's freedom (any eviction order) |
| `lums_are_capacity_independent_evicted_facts_still_count` | claim 2 over 25 capacity × ruler runs |
| `lums_never_double_count_a_restored_fact` | F2 (restore never counts); A1-completeness on its own exact recompute |
| `an_uncommitted_fact_is_never_evicted` | F1's open-tick clause |
| `without_a_recompute_no_fact_is_ever_forgotten` | F1's no-recompute clause |
| `a_backend_that_loses_or_forges_is_named_never_believed` | A2's check |

**Status:** Capacity's verdict-neutrality is **proved relative to A1–A3**. A2 is checked on
every read. A1 and A3 are the tenant's obligations, so a tenant that can't meet A1 keeps
Capacity at **kind 3**, a signed ruler, until it can.

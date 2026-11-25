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
  *Integrity checked on every read; durability is the adapter's obligation.* Every
  read through the port is `recall` (which `crystal()` uses on a cache miss): it
  rehashes what it reads back and returns `Recall::Forged` on a mismatch, and
  `Recall::Lost` when the bytes are gone, never `Ok` with other bytes. Integrity is
  prism's to check; keeping the bytes is the adapter's to promise.
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
- **`crystal(a)`.** It returns the cached bytes, or `recall(backend, scope, a)`: the
  backend's bytes rehashed to a (A2). These are identical whichever way they're
  obtained.

So (iv) holds for every call, which is claim 1. Claim 2 follows from (iii), because
`lums` reads only the counters. Claim 3 follows from the `settle` case. ∎

**Why lums is right, not only equal.** By F3 the counters grow by exactly one per element
added to K, and F1 and F2 never move them. So lums = |K at the committed tick|: evicted
facts still count, and restored facts are never counted twice.

## One port (prism-017 ask 7, prism-019)

`Backend` is prism's one content-store port; `store::Store` is retired. What `Store`
did that was real is done through `Backend` without changing the trait:

- **`Store::put`** is `Backend::persist` of a `Crystal`. The address comes from the
  bytes (F6), never from the adapter.
- **`Store::get`** is `brain::recall(backend, scope, address)`: fetch, rehash, refuse.
  It is a free function over any `Backend`, **not a trait method**, so no adapter can
  override it and skip the rehash. `Brain::crystal` reads through it on a cache miss,
  so there is one checked read, not two that could drift.
- **`Store::has`** (luminosity) and the typed get (a `MerkleTree` back from an
  address) had no caller outside their own test doubles, and are deleted. A presence
  check that doesn't read the bytes would believe the adapter, which A2 forbids; the
  checked form is `recall(..).is_ok()`.
- **`merkle::diff`** was never a store capability. It stays in `merkle`, over trees.
  A Merkle tree persists through the port as one crystal per node, whose canonical
  bytes name its children by address, so a node's crystal address is its tree address.

**Why the proof still holds.** The trait's methods are unchanged, so A2 still names
the same obligation (`fetch` returns what `persist` was given), and `crystal(a)`
returns exactly what it did before: `recall` is the same fetch-then-rehash, moved out
of `Brain` so a read outside a brain is checked the same way. No step of the
induction reads the backend except through `crystal`, so nothing else changes.

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

| test (`prismqueer/tests/brain.rs`; `port::` is `tests/port.rs`) | guards |
|---|---|
| `threshold_independence_tiny_and_unbounded_give_the_same_light` | F1–F5 as implemented, end to end, with eviction proven to happen |
| `warmer_never_truer_priority_and_threshold_never_change_the_light` | the ruler's freedom (any eviction order) |
| `lums_are_capacity_independent_evicted_facts_still_count` | claim 2 over 25 capacity × ruler runs |
| `lums_never_double_count_a_restored_fact` | F2 (restore never counts); A1-completeness on its own exact recompute |
| `an_uncommitted_fact_is_never_evicted` | F1's open-tick clause |
| `without_a_recompute_no_fact_is_ever_forgotten` | F1's no-recompute clause |
| `a_backend_that_loses_or_forges_is_named_never_believed` | A2's check |
| `port::a_backend_that_hands_back_other_bytes_is_forged_never_believed` | A2's check on `recall`, the read outside a brain |
| `port::a_merkle_tree_persists_and_reads_back_through_the_port` | the one port: `Store`'s round trip, through `Backend` |

**Status:** Capacity's verdict-neutrality is **proved relative to A1–A3**. A2 is checked on
every read. A1 and A3 are the tenant's obligations, so a tenant that can't meet A1 keeps
Capacity at **kind 3**, a signed ruler, until it can.

## Keyed scopes and erasure (prism-026 items 1, 7, 2; mirror-036 E1)

*prism-Reed, 2026-11-25. The design is prism-026's Response. This run builds items 1, 7
and 2 and E1's reason on `Lost`. Mara's F8 gate stands: **no personal data enters
`brain`** until items 1–7 are red-tested. Every test here uses synthetic bytes and
throwaway keys.*

### The key is the caller's capability (item 1)

prism holds no keys. A keyed scope needs its key at computation time, so the key is
**passed in by the caller for the duration of a call**: never stored, never derived from
a master inside prism, never persisted, never logged.

- **`ScopeKey`** (`prism_laws::oid`) holds the 32 bytes. It is **move-only**: no `Clone`,
  no `Copy`, no `Display`, and a `Debug` that prints `ScopeKey(held)`. Its bytes are
  **zeroed on drop**. Its only constructor, `ScopeKey::take(&mut [u8; 32])`, copies the
  caller's buffer and zeroes it, so the caller's copy doesn't outlive the handover.
- **There is no `ScopeKey` for the public scope.** `take` refuses `K_public`'s bytes
  (`IsPublic`): anyone can derive them, so they are nobody's capability.
- **`Scope::keyed(&ScopeKey) -> Scope<'k>`** borrows the key. A `Scope` holds a
  reference, never a copy, so a keyed scope can't outlive its key: the borrow checker
  enforces "for the duration of a call". `Scope::public()` is a `Scope<'static>`, and
  `Brain` stores only that.
- **What is stored is the scope's public id, `ScopeId`**:
  `BLAKE3-derive-key("systemic.engineering://scope-id/v1", K_scope)`. It's one-way, so
  it doesn't reveal the key. It's domain-separated from every content address, so no
  crystal's bytes can collide with it, as `Oid::hash(keyed, b"scope")` could. It is 32
  bytes and nothing else. An `Erasure`, a receipt and a backend's tombstone hold a
  `ScopeId`, never a `Scope` or a key.

### What an Erasure can and can't claim

`Erasure::mint(key: ScopeKey, epoch, census)` (since item 3, `Erasure::mint(key, epoch,
&backend)`) takes the key **by value**. Once it
returns, the key value is gone from this process and its bytes are zeroed. The type
makes using the key after the mint unrepresentable (`compile_fail`, E0382).

- **It can't prove that no copy exists elsewhere.** Nothing can, cryptographically. So an
  Erasure is **the holder's attested act**: in `crossing`'s grades it is **Sealed**,
  never Law. The receipt carries that grade.
- **The public scope is unerasable by type.** There is no `ScopeKey` for it, so no
  `Erasure` can name it: `Erasure::mint(Scope::public(), ..)` doesn't compile, and
  `ScopeKey::take` refuses `K_public`'s bytes.
- **The census (as built in this run; replaced in item 3).** An Erasure covered exactly the crystals the holder showed it while the
  key was still live: each must rehash to its address under the key. A crystal that
  doesn't is refused (`Stranger`), and the key is handed back unconsumed. So an Erasure
  can purge only its own scope's crystals, never the public scope's or another
  holder's. Without the census, one holder's erasure could delete anyone's bytes,
  because an address doesn't say which scope it's in. Item 3's per-scope index took
  the census's place (below, "The census, replaced").
- **A dangling verdict is never erasure evidence.** It is never final (`compile_fail`).

### The receipt (item 7)

An Erasure produces a `Receipt`:

- **What it names:** the scope's public id and an `Epoch`, the holder's count of when the
  erasure was attested. An epoch is not a clock, since prism reads no time. **It never
  names the person.** No field, and no address input, is derived from the holder's
  identity beyond the `ScopeId`.
- **Its address:** `Oid::hash(Scope::public(), scope_id ‖ epoch)`. The preimage is
  exactly 40 bytes: the 32 raw bytes of the `ScopeId`, then the epoch as a big-endian
  `u64`. It's in the public scope, so anyone holding the receipt can recompute it.
- **The signer sits beside the address, never in it** (`ruler.md`). The receipt is a
  `crossing::Claim` with no proof and one seal, so its grade is `Sealed` by
  construction. The seal's signer label is the **scope id itself**: the holder appears
  through its boundary (the scope), never its name (`identity.md`). When item 6 lands,
  a per-scope pseudonym may take that place.

### Purge, and `Lost`'s reason (item 2, E1)

- **`purge(backend, &erasure)`** is a free function, like `recall`. For each covered
  address it hands the backend a `Purge` order, which only this function can build.
  The order tells the backend to delete the bytes and keep the receipt as the
  address's tombstone.
- **`Backend` gains two required methods:** `purge(&Purge)` and `erased(&Oid)`, which
  returns the tombstone's receipt. They're required, not defaulted, because a default
  would either delete nothing or forget the receipt. Deleting across replicas, outside
  git, is the adapter's obligation (mirror-036 §3), as durability is under A2.
- **`Recall::Lost` gets a reason:** `Lost { address, reason }`, where `reason` is a
  `Loss`, either `Erased(Receipt)` or `Missing`.
  - `recall` asks for the tombstone **first**. An erased address reads
    `Lost(Erased(receipt))` even if a faulty backend kept the bytes. A purge is final,
    so it fails closed.
  - Otherwise, no bytes read `Lost(Missing)`: a durability fault, never a lawful act.
  - The two never compare equal, and only `Erased` carries a receipt.
  - An edge from Y's world into a purged X reads `Lost(Erased(receipt))`. Y never needs
    X's key to read it.
- **The three states stay apart in the types:**
  - **`Dangling`** (in `Reach`) is "not reached as of", never final.
  - **Dark** is an address this brain can't see into: a set in `Reach`.
  - **`Lost`** is a `Recall` error, final when its reason is `Erased`.
  - None of them converts into another (`compile_fail`).
- **The purge law** (`tests/brain.rs`, run over every backend prism has: `Memory`, the
  independently written test backend `Ledger`, and `Counted`, which wraps `Memory`. A
  `Hoarding` backend, which keeps purged bytes, shows the read failing closed):
  - after `purge(erasure)`, every covered address reads `Lost(Erased(receipt))`;
  - and `fetch` holds no bytes for it;
  - the crystals of another keyed scope, and of the public scope, still read back;
  - persisting an erased address again stores nothing.

### Why A1–A3 still hold

A purge deletes bytes that only a keyed scope could have settled. `Brain` works only in
the public scope, and the public scope has no Erasure, so no step of the proof above
meets a purge. A2 gains one clause: a tombstoned address reads `Lost(Erased)`, never
bytes.

### What a tombstone keeps, and a question for Mara

A tombstone maps an address to its receipt. After a purge, the backend still holds the
set of X's (keyed, now unrecomputable) addresses and how many there were, under X's
scope id. That is what lets Y's edge read `Erased` instead of `Missing`. Whether the
count is itself a leak is Mara's to grade. The alternative is a tombstone per scope,
with Y's edges carrying the target's scope id.

## A keyed scope's own state (prism-026 items 3, 4, 5)

*prism-Reed, 2026-11-25. The second run of prism-026. F8 stands: synthetic bytes and
throwaway keys only. Items 3, 4 and 5 put a keyed scope's index, light and root **in the
scope itself, through `Backend`**, so the same erasure that purges its crystals purges
them. prism keeps none of it in memory and holds no key.*

### Slots: a scope's mutable state, under its key

A content address can't hold state that changes (a list that grows, a root that
advances). So a keyed scope keeps its state in **slots**: named places whose address
only the key holder can compute.

- **The address:** `Oid::slot(scope, label) = BLAKE3-derive-key("systemic.engineering://slot/v1", K_scope ‖ label)`,
  64 hex. It's the same shape as `ScopeId`: one-way, and in neither the content
  domain (a keyed hash) nor the scope-id domain. No crystal's bytes can land on a
  slot, and a slot doesn't reveal the key. Without the key, a slot address is as
  unlinkable as the scope's crystal addresses.
- **The port:** `Backend` gains one required method, `write(&Slot)`: replace the bytes
  under a slot address. `Slot` has no public constructor; only prism's keyed-scope
  operations build one. Writing a tombstoned slot stores nothing (a purge is final).
  A slot reads back through `fetch`, by a checked read that asks `erased` **first**,
  so an erased slot reads `Lost(Erased(receipt))` even on a backend that kept its
  bytes.
- **What a slot read can't check, and what it can.** A slot's bytes are mutable, so
  they can't be rehashed to their address the way a crystal's can. Their
  faithfulness is the adapter's obligation, like durability under A2. Malformed
  bytes read `Forged(slot)`, never believed. And every address a slot *names* is
  checked when it's used: a listed crystal is read by `recall` (rehash), the mint
  rehashes each one under the live key, and the root's address is recomputed from
  the root's bytes on every read.

A scope's slots:

| slot (label) | holds | item |
|---|---|---|
| `manifest` | the crystals settled in the scope, in settle order | 3 |
| `references ‖ c` | the references declared for crystal `c` at `settle_with` (a union) | 3 |
| `light` | each deposited fact's slot address and its deposit tick | 4 |
| `root` | the current root's bytes | 5 |
| `chain` | every root's address, in order (only when chained) | 5 |

### `Held`: a scope, for the duration of a borrow

`Held::open(&mut backend, &key)` borrows the backend and the key. It keeps nothing
beside the borrow: no cache, no index in memory, and no key (it holds a
`Scope<'k>`, which borrows the key, so a `Held` can't outlive it). Every operation
reads and writes the scope's slots through `Backend`. It isn't a `Brain`: it holds
nothing hot, so `Capacity` doesn't apply, and the capacity proof above is untouched.
`Brain` still works only in the public scope.

### Item 3: the per-scope reference index

- **The lists live beside the crystal, in its own scope.** `Held::settle_with(bytes,
  references)` persists the crystal, appends it to `manifest` if it's new, and writes
  its declared references into its own `references ‖ c` slot. Settling the same
  bytes again adds the new references to the old ones, as `Brain::settle_with` does.
- **The index is a projection, rebuilt.** `Held::index()` reads `manifest`, then each
  crystal's `references` slot, and returns the map. Nothing caches it. Rebuilding it
  twice gives the same index; rebuilding it after a purge reads `Lost(Erased)`.
  - A scope never settled into has no `manifest`: its index is **empty**.
  - A purged scope's index is **`Lost(Erased(receipt))`**, never empty: a purge and
    a scope never used don't read the same (E1, again).
- **No global cross-world index.** Every list is in a slot under its own scope's key.
  No slot is written outside the scope, and no scope's index names another scope's
  crystal unless its holder declared it as a reference.
- **The public scope keeps today's behaviour.** `Brain`'s index (`references`) stays
  in memory, outside `Capacity`, and never shrinks: cooling, a keyed purge on any
  backend, or settling again never removes an entry. The public scope has no
  `Erasure`, so nothing could license a shrink.
- **Flagged for Alex: built pending confirmation.** prism-026 asked Alex to confirm
  retiring "the index lives outside `Capacity` and never shrinks" for keyed scopes.
  That isn't confirmed yet. The keyed index is built purgeable (it's in the scope's
  slots, so the erasure covers it) because erasure is meaningless if the index
  survives it. If Alex says no, the lists would have to leave the scope's slots, and
  an erasure would leave them behind: that needs its own design, and Mara's grade.

### The census, replaced

`Erasure::mint(key, epoch, &backend)`: the census is no longer the caller's. Under the
live key, the mint rebuilds the scope's state and covers:

- every crystal in `manifest`;
- every root in `chain`, and the current root;
- every slot: `manifest`, each `references ‖ c`, `light`, `root`, `chain`.

The `Stranger` refusal keeps its meaning: anything not in the scope under the live key
is refused, and the key comes back unconsumed.

- A listed crystal (or chained root) whose bytes the backend holds must rehash to its
  address **under the live key**. If it doesn't, it's another scope's:
  `Stranger(Foreign(address))`.
- If the backend fails, `Stranger(Backend(error))`: still unconsumed.
- A listed crystal whose bytes are missing is covered. It's named in the scope's own
  `manifest`, which only the key holder's process writes, and a person's erasure
  can't be blocked by a lost disk. (If a faulty adapter listed another scope's
  address there and also lost its bytes, that address would read `Erased` instead
  of `Missing`. It already had no bytes.)
- **It can't be stale.** The mint reads the backend at the moment it consumes the key.
  Once the key is consumed, nothing more can be settled in the scope in this process.
  A copy of the key elsewhere could settle more; that's the same unprovable copy the
  Sealed grade already names.
- A scope already purged mints again (a copy of the key, say) and covers its slots
  only. Its crystals are already tombstoned.

### Item 4: lums and deposit ticks, per scope

- `Held::deposit(fact, tick)` records a fact's deposit tick in `light`. It returns
  `false`, and keeps the first tick, if the fact was already deposited, as
  `Brain::deposit` does.
- `Held::tick_of(fact)` is the tick it was deposited at.
- `Held::lums(as_of)` is the number of distinct facts deposited at or before `as_of`:
  `Brain::lums`'s count, per scope and persisted.
- **A fact is named by its slot address,** `Oid::slot(scope, "fact\0" ‖ bytes)`, never by
  its bytes. The backend holds no fact's plaintext, only one-way names and ticks.
- **Erased with the scope:** `light` is a slot, so the mint covers it. After a purge,
  `lums`, `tick_of` and `deposit` read `Lost(Erased(receipt))` on every backend.
- The public `Brain`'s lums stay as they are: counters in memory, never part of an
  address.

### Item 5: a per-scope root, with an optional chain

- **Every new crystal settled in the scope advances the root.** The root's bytes are
  `root ‖ settled ‖ previous root` (one line each; no previous root at the first
  settle). Its address is `Oid::hash(keyed, bytes)`. So the root commits to the
  scope's whole settle history, as a hash chain, whether or not the history is
  kept. Settling the same bytes again doesn't advance it.
- **The `root` slot holds the root's bytes.** Its address is recomputed on every read,
  never believed.
- **The chain is optional.** `Held::open(..).chained()` also persists each root as a
  crystal in the scope and appends it to `chain`: the person's own history,
  walkable from the head. Unchained, a root's bytes exist only in the slot, and
  earlier roots are nowhere.
- **Purged with the scope.** The mint covers the current root, every chained root and
  both slots. After a purge, `root()` and `chain()` read `Lost(Erased(receipt))`, and
  so does any edge into a root: Y reading X's root, or any of X's chained roots,
  without X's key, reads `Lost(Erased(receipt))`.

### The mutations, each caught

- **A global cross-scope index entry** (the `manifest` slot taken in the public scope,
  so every scope shares one list): one scope's index names another's crystals.
- **A purge that leaves lums** (`light` left out of the census): lums still read after
  the purge.
- **A purge that leaves the root chain** (the chained roots left out of the census): a
  chained root still reads back.
- **A mint from a stale census** (the manifest's last crystal left out, as if the
  census were taken one settle early): a crystal settled before the mint survives it.

## The line between prism and the adapter (prism-029, "the gap")

prism's keyed scope is **keyed addressing**, not encryption.

- **prism gives unlinkable addresses.** `Oid::hash(Scope::keyed(k), bytes)` and
  `Oid::slot(Scope::keyed(k), label)` make a scope's addresses unrecomputable and
  unlinkable without its key. **The content is still plaintext**: a crystal's bytes,
  a slot's lists of addresses and ticks, a root's bytes.
- **Encryption at rest is the key holder's adapter's job**: mirror's, per mirror-036,
  running for X with X's key, through `@io`. **Never prism's.** prism holds no keys and
  sees one only for the duration of a call. If prism ever seals envelopes, `K_enc` is
  its own type, never a field of `Scope` (`identity.md`).
- **Recall's integrity check in a keyed scope needs the key.** A crystal's address is
  the keyed hash of its *plaintext*. If the backend stores ciphertext, `recall`'s
  check (fetch, rehash, compare) has to decrypt before it rehashes, so it needs the
  key at read time. That fits "the key is passed per call", but it means a keyed
  scope can't be read, or verified, without its holder present.
- **So after erasure, nobody can read or verify the scope.** That is the point of
  `Erasure::mint`: once the key is consumed, nothing in the scope can be recomputed,
  decrypted or checked again, and the tombstones say why.
- **Deterministic encryption within one scope** would keep dedup inside the scope
  (equal plaintext, equal ciphertext), leaking only equality within that scope.
  That's the adapter's choice to make.
- **prism builds no encryption.** This section states the line; it doesn't move it.

## Next (not in this run)

- **Alex:** confirm (or refuse) the keyed index being purgeable, retiring "never
  shrinks" for keyed scopes only (item 3, above).
- **Item 6.** Held until mirror's pin is signed and Alex answers "per scope or per
  (role, scope)".
- **A keyed `Brain`.** `Brain` still works only in the public scope. `Held` carries a
  keyed scope's persisted state; a keyed hot tier, if it's ever needed, would take
  its `ScopeKey` per call, never at construction.
- **For Mara:** a backend sees which slots are written together, and when. Like the
  tombstone count, that links a scope's slots to each other (never to the person, and
  never across scopes). Each write rewrites a whole slot, which is O(n) per settle;
  fine at this size, named here.

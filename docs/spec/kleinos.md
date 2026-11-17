# kleinos: two parties glued along declared spans

*prism, 2026-11-17 (prism-021 D4). Sources: Mara's kleinos note on mirror branch `mara/frames-hypothesis`, **34f70d6d** (`docs/math/2026-11-17-mara-kleinos-is-a-pushout-along-spans.md`, §3 the construction, §4 the third and Fiedler, §6 "For prism"), its probe **df9cccc3** (`spans_probe.py`, 16/16: S3c/S3d collisions, S7 the 2k count), and the frames note **54c88f72** §6.3 (laws L1–L6). Span identity from bullet-up-011 (emit commit ‖ receive commit). Decided by Alex as D4 (mirror STATE #77–#78). Names are provisional and Mara's. Code: `prismqueer::kleinos`; tests: `prismqueer/tests/kleinos.rs`.*

## What changed

The old `kleinos` was a **join**: it bridged every vertex of one operand to every vertex of the other and added a cone vertex joined to all of them. Those are relations nobody signed. Its strict λ₂ rise is fixed by graph sizes and certifies faster *averaging*, not composition (54c88f72 §6.2).

- It is now `prismqueer::spectral::join`, with module docs saying exactly that. It was renamed, not aliased. Its one caller, `spectral::infer_via_rotation`, runs Kuramoto on it, so it stays there until someone asks for rotation over a lawful composite.
- The name `kleinos` belongs to the pushout below.
- `love` never existed in prism. The only mention is a comment in `shard.rs` pointing at mirror's `rust/src/love.rs`.

## Two layers

**The event layer is a category.**
- **`Commit`** is a full hash: 40 or 64 lowercase hex.
- **`SpanId`** = emit commit ‖ receive commit, both full. It is derived from two addresses and nothing measured: no timestamp, no duration. Its order carries the direction. A span from a commit to itself is refused (`SameCommit`): that is an identity, not an event.
- **`Label`** is the 8+8 hex display form (the live OTel span id). It is `Eq` and `Display`, but on purpose neither `Ord` nor `Hash`, so it cannot key a map (a compile_fail doctest checks this). Two distinct spans can share a label (S3d).
- **`Path`** is an arrow of the category. `identity(c)`, `span(id)`, and `then(next)`, which is defined only when `next` starts at the commit where this one ends (lineage). There is no inverse (L7; a compile_fail doctest checks this).
- Not checked here: that the emit commit is an ancestor of the receive commit. That needs the commit DAG.

**The reading layer is a groupoid.**
- A **`Span<R>`** is a `Relation<R>` witnessed by its span id. `RelationDraft::along(id, emitter, receiver, reading)` is sealed by the record's holder: *attributed at both ends, sealed by one holder* (34f70d6d D4), not signed at both ends.
- The relation's address is the plain relation address followed by `"span\0" ‖ u64le(len) ‖ emit hex ‖ u64le(len) ‖ receive hex`. The transport's encoding is self-delimiting, so a span relation and a plain relation never share a preimage. A plain relation's address is unchanged.
- The reading is a `Gl<R>`: how the receiver's frame reads what the emitter's frame sent. A reading that doesn't invert never becomes a span, because `Gl::new` refuses it (`Singular`, `NotUnit`).
- **`Pushout::read_back(id)`** is the inverse transport on the same edge: what the emitter's frame makes of the receiver's reading. It is not an event, has no id, and leaves `events()` unchanged.

## The construction

`kleinos(a: &Party<R>, b: &Party<R>, spans: &[Span<R>]) -> Result<Pushout<R>, KleinosError>`.

- **`Party`** is a set of frames, the relations among them and the faces it reports. It may be empty: the empty party is the unit (L3). It names only its own frames (`UnknownFrame`), and it is canonical: frames by address, relations and faces by (address, signer), each once.
- **The glue is a keyed union.**
  - Frames with one address are one frame (overlapping parties glue).
  - Every relation and every span is its own edge, keyed by its address. A span's key includes its full id, so k spans between two frames are k parallel edges, even with one reading and colliding labels.
  - Nothing else is added: no cone, no bridge.
- **Refused, typed, never a panic:**
  - a span to a frame neither party holds (`UnknownFrame`), so a span into the empty party is refused: a span needs a frame on each side;
  - one span id with two pairs of endpoints (`Collision`: a collision, not an edge), including against a span already inside a glued party;
  - one id, one pair of endpoints, two readings (`TwoReadings`);
  - anything `hypothesis::compose` refuses (`Composition`).
- One span given twice (same endpoints, same reading) is one event and one edge.
- **The composite is `compose`'s, and the hypothesis is `hypothesize`'s** (`Pushout::hypothesize`), on the glued party. kleinos recomputes neither. `Pushout::oid()` is the composite's address. `Pushout::party()` is the glued party, so it can be glued again.

## The emergent third

`Pushout::third()` is **the set of loops the spans add**: b₁(pushout) − b₁(a ∪ b) of them (Mayer–Vietoris, L5).

- **How they are read.** A spanning forest grows from the parties' relations (by address), then from the fresh spans (by id). Each span that closes a cycle closes one loop. The loop's walk is the span itself, then the forest's path from its receiver back to its emitter.
- **Holonomy.** Each loop carries its holonomy as an exact `Gl`. A **comma** is a loop whose holonomy isn't the identity (`Pushout::commas`).
- Between two one-frame parties, k spans add k − 1 loops, all bigons.
- **What the third is not.**
  - Loops made only by gluing overlapping parties are in the composite's holonomy, not in the third. They are not loops the spans added.
  - The trace DAG is not the third. It is the third's ground, kept by address.
- Under `Pushout::hypothesize`, every comma becomes an open loop with its unranked menu (`hypothesis.md`).

## λ₂, a reading

`Pushout::fiedler()` measures λ₂ of the room multigraph (L₀ = δ*δ, dsyev) on demand.

- It is sealed with machine and method, and it is never in the address.
- Between two parties joined by k spans, L = [[k, −k], [−k, k]] and **λ₂ = 2k: it counts messages**, whatever the readings say (S7).
- It is not a law, and nothing is decided by it.

## Tests (`prismqueer/tests/kleinos.rs`)

**Mara's test 2.**
- Parties A and B, with the reply chain c1 → c2 → c3: span A→B read as I, then span B→A read as `not`.
- `not` is ℤ/2's regular representation, the 2×2 swap over ℤ (det −1, a unit).
- Expected and observed:
  - two events, by full id;
  - two parallel edges, both spans;
  - one added loop, a bigon, with holonomy exactly `not`;
  - one comma;
  - `hypothesize` gives `Partial` with one open loop, grounded on the pushout's address.

**Falsifiers.**
- B→A read as I: one loop and no comma (`Success`).
- Dropping B→A: zero loops.
- S3d: `aaaa0000…f` → `bbbb1111…0` and `aaaa0000…e` → `bbbb1111…1` share the label `aaaa0000bbbb1111`, yet they are two events, two edges and one bigon.

**Refusals.** Every `KleinosError` variant, the two `GlError`s, and the idempotent duplicate. A span into the empty party, from either side, is `UnknownFrame`.

**λ₂.** For k = 1…5, with B→A read as I or as `not`: λ₂ = 2k (to 1e−9), and k − 1 loops.

**L1–L7.** Seeded `flux::pillar::forall` properties, 96 worlds each.
- **What a world is.** Three parties of one to three frames. Sometimes a frame is shared, so parties overlap. Relations are read as I, `not`, −I or a shear.
- **Its spans.** Spans run a–b, b–c and a–c. They are drawn from a pool of commits whose 8-hex prefixes collide, so labels collide while full ids differ (S3c/S3d). Half of them chain on from the last receive commit, giving paths that compose and parallel routes between one pair of frames: colliding paths (Seam 6601cd62 §13). L1 asserts that some world had colliding labels.
- **The laws as tested.**
  - **L1, associativity:** (a ⊔ b) ⊔ c and a ⊔ (b ⊔ c) give one address and one event set, both equal to the plain union.
  - **L2, commutativity:** a ⊔ b = b ⊔ a, third included.
  - **L3, the unit:** the empty party is the unit (the frames note's L3; `hypothesize`'s "empty input = unit"). ∅ ⊔ a = a = a ⊔ ∅: one address with the party alone, the same party and events, and no third, on a drawn party and on a glued one that carries spans. Alongside: with no spans, the result is the union, with no third and no events, and identities are units for `Path::then`.
  - **L4, sovereignty:** every relation and every edge is a's, b's or a span's. Every edge between a frame only a holds and a frame only b holds is a span.
  - **L5, the third:** |third| = b₁(pushout) − b₁(a ∪ b). Every loop is closed and is closed by a fresh span. commas = the loops with non-identity holonomy.
  - **L6, idempotence and no fusion:** a ⊔ a = a with no third. Events = distinct full ids, even under label collisions.
  - **L7, forward only:** `then` is defined iff r₁ = e₂, and it is associative. Reading back inverts the reading and makes no event.

**Mutations.** Each was applied, run against `kleinos::` and reverted:
- **Keying by the short label** (encoding the label instead of the full ids). Fails 4 tests: the S3d falsifier, λ₂, L1 and L6.
- **Accepting a duplicate id with different endpoints.** Fails 2 tests: the collision test and the across-a-glued-party collision.
- **Adding a cone edge nobody declared.** Fails 10 tests, including the main test and L1–L6.
- **Refusing an empty party** (the old `EmptyParty`). Fails 2 tests: the L3 unit law and the span into the empty party.

## Open, for Mara and Alex

- **The fusion-refusal *policy* (L6) is not implemented.** a ⊔ a = a is the law. The old join refused identical operands; the pushout doesn't.
- **Faces are triangles only**, as in `hypothesis`. A party cannot report a filled bigon over two spans yet.
- **Loops from overlap gluing** are in the composite, not the third (see above). Whether they belong to "the emergent third" is a reading for Mara.
- **Signing at both ends** needs per-party keys on the two commits. Today a span is sealed by one holder.

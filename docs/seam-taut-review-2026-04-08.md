# Seam + Taut Joint Review — Prism Optics Layer

**Date:** 2026-04-08
**Reviewers:** Seam (security, soundness, integration) + Taut (performance, hot paths, allocation)
**Subject:** prism crate at HEAD `7be9952`, branch `reed/prism-composition-foundation`
**Test count:** 121 tests passing (verified — 118 lib + 3 integration)

## Summary

**Seam:** The layer compiles, the tests are green, but the tests do not witness
what the documentation claims. Four of the six "classical optics" are in one
form or another *degenerate* as Prisms — either (a) their Prism impl is
disconnected from the stored closure (Setter), (b) they carry their refutation
channel outside ShannonLoss in direct violation of the spec (OpticPrism), or
(c) they silently drop provenance when crossing a level boundary (MetaPrism,
Traversal.split). The monoid-law tests prove associativity of integer addition
on `CountPrism.count`; they do **not** prove the `Compose<P1, P2>` struct is a
monoid. And the `MetaPrism` that landed is not the one the spec asked for — its
inner type parameter silently changed from `P: Prism` to `T`, so the "meta-prism
lifts an inner prism's split" design is not expressible with the types as
shipped. This is not a refactor away from a good core; this is a core that will
teach its lessons in production if merged as-is.

**Taut:** The shape is functional and the test suite runs in ~0s debug, so
there is no urgent hot path problem — but the code is shipping unforced
pessimism that will compound. Every optic kind boxes its closures (`Box<dyn Fn>`),
which forces a `*Crystal` marker twin per optic and duplicates six Prism impls
that do nothing except clone a path Vec and flip a Stage enum. The Crystal
types are functionally useless: they remember nothing and do nothing, and they
exist because `Box<dyn Fn>` isn't `Clone`. A generic `Optic<F>` that
monomorphises the closure would eliminate the entire twin hierarchy, halve the
visible type surface, and let `Compose` actually be tested. Split paths clone
parent Vecs O(depth × children); the Beam struct is rebuilt field-by-field at
every stage transition when `with_stage` already exists; and `Stage::Joined`
sits in the enum with no producer anywhere in the base trait since `join` was
removed. The line exists. We haven't run it yet.

**Joint:** The optics layer ships a working skeleton, but the skeleton has load-bearing
holes in exactly the places Reed was going to put weight on — composition,
refutation, and meta-level gathering — and the test suite green-lights them
because the tests were adapted to match what compiled rather than what the spec
specified.

## Findings

### Critical (must fix before merging to main)

---

**C1 — `MetaPrism` silently changed shape from the spec and lost the inner prism**

- **Lens:** Both (Seam: architectural regression, Taut: blocks the boot-fold use case)
- **Severity:** Critical
- **Location:** `src/optics/meta.rs:22` and commits `a250d75` → `8becf30`
- **Description:** The spec (`docs/.../2026-04-08-prism-monoid-optics-design.md` §Layer 3) defines
  `MetaPrism<P: Prism, G: Gather<P::Part>>` — a meta-prism parameterised by an
  **inner prism P** that supplies the split, plus a gather strategy.
  `focus` calls `self.inner.split(inner_beam)`. That is how the meta-prism is a
  cross-level homomorphism from Level 0 to Level 1 and back.

  The implementation is `MetaPrism<T, G: Gather<T>>`. The inner prism has been
  *removed entirely*. `Input = Vec<Beam<T>>`, `focus` is a no-op stage-flip, and
  `project` just calls `self.gather.gather(beam.result)`. There is no inner-prism
  lifting anywhere in the file.

  This means the spec's headline use case — *"`MetaPrism<inner, SumGather>` gathers
  them back into a single beam representing the sum type"*, where `inner` is
  a real domain prism whose split produces the population — is not expressible
  with the types as shipped. The current `MetaPrism` is, structurally, a `Gather`
  wrapped in a Prism trait jacket.

  The implementation report flags a "Tasks 11/12 inline correction" in the plan
  where the original test was wrong (it fed `Beam<Vec<Beam<String>>>` to a
  function expecting `Beam<T>`). The correct response to a type-impossible
  test is to fix the test to match the spec. The response taken was to fix the
  type parameter until the test compiled. That regressed the spec.
- **Recommended fix:** Restore `MetaPrism<P: Prism, G: Gather<P::Part>>` with the
  inner prism. Write an `apply`-level test that actually chains `StringPrism.split
  → MetaPrism<StringPrism, SumGather>.project` as the spec's Layer 3 test section
  describes. Do not land until the round trip goes through a real inner prism.
- **Disagreement:** None. Seam sees a broken invariant; Taut sees a broken use case.

---

**C2 — `OpticPrism` leaks refutation through `Option` and fabricates data on None**

- **Lens:** Seam (primary), Taut (agrees: Default bound is cargo-culting)
- **Severity:** Critical
- **Location:** `src/optics/optic_prism.rs:11,38,57-76`
- **Description:** The spec (§Error handling) is explicit: *"no tagged error types, no
  Option/Result. Refutation and failure are encoded as `ShannonLoss` in the Beam."*

  `OpticPrism` violates this in three ways at once:
  1. `preview_fn: Box<dyn Fn(&S) -> Option<A>>` — `Option` in the public
     constructor signature. Every caller now has to think about Some/None.
  2. `type Focused = Option<A>` — the refutation channel is in the *type
     parameter* of the Beam, which is exactly the thing the base crate's design
     comment says must not happen.
  3. In `project`, when `preview` returns None, the implementation synthesises
     `A::default()` as the beam result and sets loss to infinity. An infinite
     loss is a correct signal, but a downstream reader who only checks
     `beam.result` (or who treats infinite loss as an edge case the way
     agents sometimes treat "0 failed" as "passed" — see Seam's tensions on
     verification surface, 2026-04-07) will consume **fabricated data with no
     indication at the value level**. This is exactly the false-positive class
     Seam crystallized as a threat-model entry.

  The `S: Default + A: Default` bound is a downstream symptom: impls only need
  `Default` because the code has to make something up. That's the giveaway.

- **Recommended fix:** Change `preview_fn` to `Fn(&S) -> Beam<A>` (returning
  `Beam<A>` with infinite loss on the refutation case; the closure authors the
  refutation directly into the Beam rather than via Option). Alternatively, keep
  the Option shape at the closure boundary but make `Focused = A` with the
  refuted case carrying a `ShannonLoss::infinite()` Beam through a sentinel —
  but do **not** call `A::default()`. If there is no sentinel, the correct
  behaviour is not to produce a beam at all (fall back to the split empty-vec
  convention). Drop the `A: Default` bound.
- **Disagreement:** None. Both lenses want this gone.

---

**C3 — The monoid law tests do not witness the laws on `Compose<P1, P2>`**

- **Lens:** Seam
- **Severity:** Critical
- **Location:** `src/optics/monoid.rs:298-342` (tests) and `src/optics/monoid.rs:29-67` (Compose impl)
- **Description:** The plan claims: *"monoid laws as property tests … identity law
  … associativity."* The tests that landed:
  - `count_prism_identity_law_left/right` — tests `CountPrism::compose`, which is
    literally `self.count + other.count` on a `u64` field. This is a test that
    integer addition has an identity of 0. It is a tautology about u64.
  - `count_prism_associativity` — same. Tests that u64 addition is associative.
  - `compose_chains_two_id_prisms_via_refract` — does **not** use `Compose`.
    It constructs a single `IdPrism<String>` and calls focus/project/refract on
    it. The comment in the test admits the original test was type-impossible
    and the "fix" was to run a single prism and assert on that.
  - `compose_type_chains_crystal_to_input` — asserts `Compose::new` compiles.
    Does not invoke `refract`.

  **There is no test anywhere that exercises `Compose::refract` end-to-end.**
  The `Compose<P1, P2>::refract` implementation at `monoid.rs:61` runs
  `self.first.refract(beam)` and then calls `self.second.focus/project/refract`
  on the resulting `Beam<P1::Crystal>`. This control flow is untested. A bug in
  the intermediate chaining — wrong stage transition, path dropped, loss not
  accumulated — would not be caught.

  This matters because `Compose` is the only place where two real prisms
  actually meet. Every other test is self-loop IdPrism. The monoid structure
  the spec is selling does not have its composition primitive tested.
- **Recommended fix:** Write a `Compose<StringPrism, IdPrism<StringPrism>>`-style
  chain where `P1::Crystal` really is a different thing from `P1` (the existing
  `StringPrism.Crystal = StringPrism` self-loop doesn't suffice — `Compose` there
  is indistinguishable from running `StringPrism` twice). Even better: write the
  identity and associativity laws against **`Compose` itself**, not
  `CountPrism.count`. If `CountPrism` isn't expressive enough to witness the laws
  on `Compose`, that is a sign `CountPrism` is the wrong witness and should be
  rewritten.
- **Disagreement:** Taut adds: also untested because `Compose<P1, P2>` needs real
  monomorphization to expose its cost; the generic impl is paying for a test
  that never fires.

---

**C4 — `Setter`'s Prism impl is disconnected from its stored modify closure**

- **Lens:** Both (Seam: misleading API, Taut: dead code on the Prism surface)
- **Severity:** Critical
- **Location:** `src/optics/setter.rs:34-55`
- **Description:** `Setter<S, A>` stores a `modify_fn`. The inherent method
  `Setter::modify` calls it. The `Prism for Setter` impl **never invokes
  `modify_fn`**. `focus`, `project`, `split`, `zoom`, `refract` are all
  identity pass-throughs that just flip the stage. So:

  ```
  let s = Setter::new(|b, f| Box2 { count: f(b.count), ..b });
  let out = apply(&s, box2);  // out.result has the ORIGINAL count, not modified
  ```

  This means `Setter` has a Prism impl that satisfies the trait signature and
  does nothing with the data. A downstream user who treats it as a prism will
  see surprising no-op behaviour; the test file only exercises
  `count_setter.modify(...)` directly, not the Prism pipeline.

  This is worse than the "degenerate but correct" pattern the report describes —
  it's a Prism impl that silently drops its semantic payload.
- **Recommended fix:** Either make the Prism impl actually run the modify with a
  no-op inner function (so refract transforms `S` by applying `modify(s, &|a| a)`
  as an identity check), or delete the Prism impl entirely and surface Setter
  only as an inherent-method optic. Current state is the worst of both.
- **Disagreement:** None.

---

### Major (should fix soon)

---

**M1 — `Traversal.focus` applies the map, then `split` loses the index provenance**

- **Lens:** Seam (provenance), Taut (O(n·depth) clone on the split hot path)
- **Severity:** Major
- **Location:** `src/optics/traversal.rs:39-67` (and mirrored in TraversalCrystal)
- **Description:** `focus` collects `beam.result.into_iter().map(self.map_fn).collect()`.
  No index recording. `split` then emits N child beams, each cloning
  `beam.path.clone()` with no index step pushed. A Traversal over `["alpha", "beta",
  "gamma"]` produces three `Beam<String>`s whose paths are indistinguishable.
  If you recombine them via a Gather, you cannot tell which child came from
  which position. That's a content-addressing story the base crate is trying
  to tell and this optic silently breaks.

  Taut side: every child clones the parent path. For a traversal at depth D
  over N elements, that's O(N·D) Vec allocation per split call. The fix for
  Seam's issue (push an index Oid per child) is also free for Taut because
  the clone happens anyway.
- **Recommended fix:** In `Traversal::split` (and `Fold::split`, which has the
  same bug), enumerate and push `Oid::new(i.to_string())` on each child. Same
  pattern as `StringPrism.split` in `lib.rs:209-223`.

---

**M2 — `MetaPrism::split` wraps the singleton beam and **drops** the input beam's path/loss/precision**

- **Lens:** Seam
- **Severity:** Major
- **Location:** `src/optics/meta.rs:60-69`
- **Description:**
  ```rust
  fn split(&self, beam: Beam<T>) -> Vec<Beam<Beam<T>>> {
      vec![Beam {
          result: beam,
          path: Vec::new(),
          loss: crate::ShannonLoss::new(0.0),
          precision: crate::Precision::new(1.0),
          recovered: None,
          stage: Stage::Split,
      }]
  }
  ```
  The outer beam is constructed with **fresh zero loss, fresh full precision,
  empty path**. Any accumulated history from upstream is erased. The inner beam
  is preserved because it moved into `result`, but the outer envelope is pristine.

  This is not "degenerate but safe" — it's a provenance reset. If you were
  trying to audit where a value in a MetaPrism split came from, you would be
  looking at a lie. Seam's `refract when the finding is reproducible` principle
  says this is reproducible: every call to `split` loses history.
- **Recommended fix:** Carry `beam.path/loss/precision/recovered` into the outer
  Beam fields, keep the inner beam in `result` as before.

---

**M3 — `Iso` tests do not witness the invertibility law for arbitrary user Isos**

- **Lens:** Seam
- **Severity:** Major
- **Location:** `src/optics/iso.rs:166-179`
- **Description:** `Iso::new` accepts any two functions, with no constraint
  whatsoever. `Iso::new(|s: String| s + "x", |s| s)` is a well-typed "Iso" that
  violates both laws. The test `iso_round_trip` proves the *particular* pair
  `{chars().collect, into_iter().collect}` is invertible. It does not prove,
  for any other Iso a user might construct, that the law holds.

  This is the standard problem with closure-parameterised optics: the type
  system cannot enforce the law, so the law must be witnessed by a debug-mode
  property test inside the refract path, or documented with a loud WARNING on
  `Iso::new` saying "you must prove invertibility; we cannot." The current code
  does neither.
- **Recommended fix:** Add a `Iso::new_checked<A: PartialEq + Clone>` that, in
  debug builds, runs the round-trip on a sample and asserts. Or at minimum, add
  `# Safety`-style rustdoc to `Iso::new` making the law a caller obligation.
  (Same issue applies to `Lens::new`, `OpticPrism::new`, `Fold::new`.)

---

**M4 — `Stage::Joined` is dead in the base trait but still enumerable**

- **Lens:** Both (Seam: code smell, Taut: affects Beam layout and pattern-match exhaustiveness)
- **Severity:** Major
- **Location:** `src/beam.rs:19-26`
- **Description:** Commit `6db59ca` removed `join` from the base trait. The
  Stage enum still carries `Joined`. The only producers now are `SumGather`,
  `MaxGather`, `FirstGather`, which set `stage: Stage::Joined` when emitting their
  result. But nothing in the base trait transitions to or from `Joined`, so its
  presence in the base crate's public Stage enum is confusing.

  Taut: `Stage` is `#[derive(Clone, Copy, Debug, PartialEq, Eq)]` with 6 variants.
  Dropping `Joined` would not shrink the byte (it'll still be 1 byte rounded),
  but removing it forces `Gather` implementations to invent a real Stage
  transition (probably `Projected`) which is the honest move.
- **Recommended fix:** Either (a) delete `Stage::Joined` and have gather
  strategies return `Stage::Projected`, or (b) reintroduce `join` as a first-class
  operation. Not both. Current state is the worst of both.

---

**M5 — The "integration test" does not integrate Compose**

- **Lens:** Seam
- **Severity:** Major
- **Location:** `tests/optics_integration.rs`
- **Description:** The spec's §Data flow describes
  `Compose::new(abyss_prism, pathfinder_prism)` as the boot-fold pattern. The
  integration test file at the branch's head contains three tests, none of
  which use `Compose`. The first runs a bare `Traversal.focus`. The second
  runs a bare `MetaPrism.focus + .project`. The third calls `apply(&meta,
  population)`. There is no Compose, no chaining, no cross-optic composition.
  This is not an integration test; it's three unit tests in a separate file.
- **Recommended fix:** Write a test that composes at least two optics whose
  Crystal types differ from their Input types, with a non-trivial transform,
  and asserts the combined pipeline produces the expected result.

---

### Minor (worth knowing)

---

**m1 — Every `*Crystal` type duplicates the same five no-op Prism methods**

- **Lens:** Taut (primary), Seam (agrees: copy-paste surface)
- **Location:** `iso.rs:111-159`, `lens.rs:100-121`, `traversal.rs:93-128`,
  `optic_prism.rs:102-123`, `setter.rs:57-80`, `fold.rs` (same pattern)
- **Description:** Six distinct Crystal types exist solely because `Box<dyn Fn>`
  is not Clone. Each has an identical-looking Prism impl: focus/project/split/zoom/refract
  that flip the stage enum and otherwise pass through. This is ~300 lines of code
  that exists to satisfy `Crystal: Prism<Crystal = Self::Crystal>` and does nothing
  at runtime beyond rebuilding a Beam struct.

  The cleaner design is a single generic `PhantomCrystal<Marker>` type with one
  Prism impl, parameterised by a marker type. All six crystals collapse to one.
  Binary size drop is measurable; compile-time drop is nicer; humans reading the
  code stop wondering why six near-identical types exist.
- **Recommended fix:** Introduce `pub struct PhantomCrystal<M>(PhantomData<M>)`
  with a single blanket Prism impl. Replace IsoCrystal/LensCrystal/etc. with
  type aliases.

---

**m2 — `Beam` is rebuilt field-by-field at every stage transition when `with_stage` exists**

- **Lens:** Taut
- **Location:** Pervasive — e.g. `monoid.rs:113-121`, `iso.rs:54-64`, many more
- **Description:** The idiom throughout the optics module is:
  ```rust
  Beam {
      result: ...,
      path: beam.path,
      loss: beam.loss,
      precision: beam.precision,
      recovered: beam.recovered,
      stage: Stage::Focused,
  }
  ```
  This is six field moves per call. `beam.rs:72` already provides
  `with_stage(mut self, stage) -> Self` which is one mut + one move. The same
  is true for `map` which handles the result field. Using
  `beam.map(|x| new_result).with_stage(Stage::Focused)` is equivalent, more
  ergonomic, and the compiler generates the same code — but it removes ~200
  lines of struct-literal boilerplate that has to be manually kept in sync if
  Beam ever grows a field.

  If Beam adds a new field tomorrow, every one of these ~50 struct literals
  has to be updated. The `..beam` sugar is used in some places (iso.rs:69,
  lens.rs:69) and not in others — no consistency. Seam notes: inconsistency
  is where drift hides.
- **Recommended fix:** Mandate `..beam` or `with_stage(...)` throughout. Forbid
  full struct-literal rebuilds in a lint or a style guide.

---

**m3 — `Gather` strategies are concretely implemented only for `String`**

- **Lens:** Both
- **Location:** `src/optics/gather.rs:26, 69, 108`
- **Description:** The implementation report flags this as intentional. Fine —
  but the trait is generic in T and the `impl Gather<String>` is hardcoded. A
  generic `impl<T: Add + Clone> Gather<T> for SumGather` would do most of this
  work. Right now `MetaPrism<i32, SumGather>` does not type-check despite the
  type parameters looking generic.
- **Recommended fix:** Make SumGather generic over `T: Clone + Add<Output = T>
  + Default` (or a `Monoid<T>` trait to keep it honest). Same for Max/First.

---

**m4 — `ShannonLoss::is_lossless()` alias**

- **Lens:** Seam (readability)
- **Location:** `src/loss.rs` (not read — flagged from report)
- **Description:** The plan used `is_lossless()` throughout; the actual method
  is `is_zero()`. The report suggests an alias. Seam concurs — `is_lossless` is
  the semantically right name in this codebase's ontology, and the current name
  invites copy-paste from numerical code that uses `is_zero` for a different
  meaning.
- **Recommended fix:** Add `pub fn is_lossless(&self) -> bool { self.is_zero() }`.

---

### Interesting observations (no action required)

- **i1 — TDD was followed; the red commits compile-fail, the green commits
  pass.** That discipline is visible in the commit log. What was *not* caught
  is that some of the red tests were wrong specs (not wrong implementations),
  and the green response was to weaken the test rather than fix it. This is the
  class of failure that Seam's verification-surface tension already names:
  "report X passes" does not mean "X was specified correctly." The remedy is
  spec → test derivation with a human checkpoint before green, not more TDD.

- **i2 — The `Crystal: Prism<Crystal = Self::Crystal>` fixed-point bound
  actually holds** in every optic because they all set `Crystal = *Crystal<T>`
  where `*Crystal::Crystal = *Crystal<T>`. Taut: that does monomorphise to six
  distinct types per (A,B) pair, but it's free at runtime since the crystals are
  phantoms. Seam: the fixed point is enforced structurally by the type alias,
  not by any runtime check. A malicious impl could set
  `type Crystal = SomethingUnrelated` as long as *that* type also has a
  self-crystal. The constraint is honest about what it enforces (one level of
  fixed-point), not two.

- **i3 — 121 tests in <1 second** (debug build, verified). There is no urgent
  Taut fire. The allocation patterns noted above will matter when the boot fold
  runs on hundreds of files, not on the current tests. But the shape is already
  wrong; fix it before it's load-bearing.

## Specific items the dispatcher asked about

**Seam lens items:**

- *Type-system holes around recursive `Crystal: Prism<Crystal = Self::Crystal>`:*
  Holds in every shipped optic. Enforced structurally via type aliases, not runtime.
  See i2. No exploitable hole found.

- *Sub-Turing preservation (Compose / MetaPrism / closure optics):*
  `Compose` is a one-step forward pipeline, not recursive. `MetaPrism` has no
  self-reference. The `Box<dyn Fn>` closures in Iso/Lens/Traversal/etc. are
  a Turing-complete escape hatch by nature (you can put an infinite loop inside
  a closure), but this is unavoidable with the Rust function type and is
  documented as a non-goal in the spec. The Rust compiler cannot make those
  closures sub-Turing; that check lives at the grammar layer in `conversation`,
  not here. **No issue inside the optics layer itself.**

- *Monoid laws actually witnessed:* **No.** See C3. Tests prove integer
  addition is a monoid on u64. They do not prove `Compose<P1, P2>` is one.

- *Refutation leaks (`Option`/`Result`/panic-on-None):*
  `OpticPrism` leaks Option into its constructor and its Focused type. See C2.
  Also `.unwrap_or(std::cmp::Ordering::Equal)` in MaxGather (`gather.rs:89`) —
  swallows NaN precision as Equal, which is benign but is a hidden semantic
  decision worth documenting.

- *`Box<dyn Fn>` lifetime tightness:* Every closure is `'static`. That is the
  normal Rust choice and it's not cargo-culted — it's forced by the fact that
  these closures are stored in structs that are themselves constructed and
  returned from functions. Cannot be tightened without a lifetime parameter
  on every optic. **No issue.**

- *Crystal pattern soundness:* The marker types are sound (they do carry the
  correct type-level information for the fixed-point bound), but they are
  redundant — see m1. A single `PhantomCrystal<M>` would be equally sound and
  cut the surface by six.

- *Integration with base trait / bypass paths:* `Compose::refract` directly
  calls `first.refract` then `second.focus/project/refract`, **skipping
  `first.focus/project`** because the base trait's `apply` is not used.
  Seam flag: the semantics of `Compose` are "run first fully, then run second
  fully," but the code runs *only* first's refract before handing off. If
  `P1::refract` assumed `focus` and `project` had already run, the state
  coming in via `Compose::focus → Compose::project → Compose::refract` might
  be different from what `Compose::refract` ends up feeding to the second
  prism. This deserves a dedicated test.

- *Misuse: ill-formed optics that look well-formed:* Yes — see M3 (any
  non-invertible `Iso`), also applies to `Lens::new` (no view-set law check),
  `OpticPrism::new`, and the entire `Gather` trait (no constraint that
  `gather(singleton(b)) == b`).

- *MetaPrism self-referential split wrapping input in singleton:* See M2.
  Wraps correctly but drops outer Beam provenance. The shape is "right in
  spirit, wrong in execution."

- *The four adaptations from the report:* Reviewed in the report context:
  1. `compose_type_chains_crystal_to_input` replaced with a no-op existence
     check — papered over. C3.
  2. `is_lossless` → `is_zero` — real API gap, correct fix. m4.
  3. Closure type inference in Traversal tests — ergonomics, not a bug.
  4. `with_stage_projected` helper missing — direct field mutation used
     instead. That's fine locally but see m2 — the codebase is inconsistent on
     stage transitions, and the fix should go in `beam.rs`, not in ad-hoc
     mutation.

- *The 30 commits:* Red→green cadence is clean. The one suspicious commit is
  `8becf30` where `MetaPrism<P, G>` (red) silently became `MetaPrism<T, G>`
  (green). That is C1's origin. No hook-bypassing or force-pushes visible.

**Taut lens items:**

- *Allocation pressure:* Every optic construction allocates two `Box<dyn Fn>`s
  (~16 bytes each plus closure env). At call time, each op is a virtual
  dispatch through the boxed fn. In the boot-fold use case (100 files × ~5
  ops per file = 500 virtual calls), this is negligible. In any inner loop
  (per-Beam-in-Vec at Traversal level), it compounds linearly.
  **Recommended alternative:** generic `Optic<F, G>` with `F: Fn + 'static,
  G: Fn + 'static` — monomorphises, no box, no vtable. Trade: compile time
  grows. Worth it; benchmark after C1-C4 land.

- *Cloning:* `CountPrism.clone()` for the assoc-law tests is three clones for a
  `u64` — free. `MetaPrism::refract` clones `self.gather` which is a ZST for
  Sum/Max/First — free. Path clones in `Traversal::split` are O(N·depth)
  per call and are the only real hot spot. See M1.

- *Monomorphization from `Compose`:* `Compose<A, B>`, `Compose<Compose<A,B>, C>`,
  etc., generate distinct LLVM types. For a linear chain of 100 composed
  prisms, that's 100 unique types, each emitting a ~30-line refract body. Not
  a crisis (Rust handles 10k-type crates routinely) but worth measuring once
  the boot fold is real.

- *`Vec` allocation in split:* Correct concern. Every `split` call is a fresh
  `Vec`. For a MetaPrism wrapping a singleton, the Vec has capacity 1 and
  allocates anyway. A `SmallVec<[Beam<T>; 4]>` would eat the singleton case
  inline. Defer until there's a bench to point at.

- *Beam clone in split path inheritance:* Yes, O(depth × children). See M1 —
  Seam's fix also removes Taut's concern because both require rewriting the
  split emission site.

- *Stage transitions rebuild the whole Beam:* See m2.

- *Crystal pattern optimization elimination:* Verified — every `*Crystal` is
  a `PhantomData`, size 0, zero runtime cost. The cost is *compile-time*
  and *source-code-reading* time. See m1.

- *Trait dispatch via `&dyn Fn`:* See m1 + allocation bullet.

- *Integration test timing:* 3 tests in <1s debug, no signal to extract.
  `cargo test --release` will be similar.

- *Hot-path identification:* The real hot path for the mirror use case is
  `focus → project → refract` per file during boot. `Compose::refract` is the
  top frame once composition is used. Today that path is untested (C3).

**Cross-pollination items:**

- *`Box<dyn Fn>` as both concerns:* Both lenses agree: boxing is a pessimism
  and a maintenance burden (because of the Crystal twin hierarchy). Both
  lenses want the same fix — monomorphise via generic type parameters. Seam
  adds: once generic, the Crystal can often be `Self`, restoring the
  spec-intended fixed-point without marker types.

- *`Stage::Joined` still existing:* See M4. Seam: smell. Taut: layout-neutral
  but exhaustive-match-noise. Joint recommendation: delete.

- *Crystal marker type-level lipstick:* Seam: soundly enforces the bound, so
  not pure lipstick, but could be one generic marker. Taut: free at runtime,
  costly at compile and read time. Joint: unify into `PhantomCrystal<M>`.

- *Recursive `Crystal: Prism<Crystal = Self::Crystal>` bound:* Seam: one
  level of fixed-point only — doesn't catch a malicious impl that uses a
  different self-crystal type, but no observed violation. Taut: no measurable
  monomorphization explosion because the six Crystal types are the same five
  lines of Prism impl each and LLVM dedups most of it.

- *`MetaPrism::project` doing gather + stage mutation:* Both lenses prefer
  gather owning the stage transition (return a `Stage::Projected` beam
  directly). That's one less place where stage can drift. Currently the code
  emits `Stage::Joined` from gather and then overrides to `Stage::Projected`
  in meta::project — two layers disagreeing about which stage the result is
  in, and the outer wins silently. That's the kind of inconsistency Seam
  flags as drift-habitat.

## Cross-lens tensions

The biggest Seam/Taut disagreement is about **the Crystal pattern**.

- **Seam** wants the Crystal types kept, because they document "this is the
  fixed point" at the type level and a future impl author sees
  `type Crystal = FooCrystal` and knows what's being promised. Deleting them
  means the promise lives only in prose.

- **Taut** wants them unified into a single `PhantomCrystal<M>` because six
  copies of the same five-method impl is readable only on the first pass —
  on the third pass, a reader no longer knows whether Iso and Lens have
  *intentionally* different crystals or just copy-paste-different ones.

The synthesis is: keep `Crystal: Prism<Crystal = Self::Crystal>` as the trait
bound, but let most optics set `type Crystal = PhantomCrystal<Self>` via a
single blanket impl. The type-level fingerprint is preserved (via the `M`
parameter) and the code duplication is gone. Both lenses sign off on that.

Secondary disagreement: **`OpticPrism`'s infinity-loss fabrication**. Taut was
initially fine with the trick (it's a branchless encoding: always produce a
beam, always produce a value). Seam refused — see C2 — because silent
fabrication is exactly the class of bug Seam's own tensions already flag as
load-bearing. Seam wins this one by appeal to the verification-surface
requirement. Taut concedes with the note that the fixed version should still
be branchless if possible.

## Recommended next actions

1. **Block merge.** C1-C4 must be addressed before this branch goes to main.
   The tests pass, but they pass for the wrong reasons on at least three of
   the four criticals.

2. **Rewrite `MetaPrism` to carry the inner prism** (C1). This is the
   largest fix — likely 30 minutes of focused work plus a new test that
   actually uses `StringPrism.split` through the meta layer. Start here
   because everything else assumes a meta-prism shape that the shipped code
   does not have.

3. **Write a real `Compose::refract` test** (C3). Parameterise it on two
   distinct prism types where `P1::Crystal != P1` (introduce a `UpperPrism`
   whose crystal is `StringPrism`, for instance). Assert identity and
   associativity against this composition, not against `CountPrism.count`.

4. **Fix `OpticPrism`** (C2). Either change the signature to closure-produces-Beam
   or provide a type-level sentinel that doesn't require `Default`. Do not
   land a refutation channel that returns fabricated values at the value level.

5. **Decide `Setter`'s Prism impl shape** (C4). Either connect it to
   `modify_fn` or delete the Prism impl and expose Setter inherently.

6. **M1 + M2 + M3 as a batch** — they're all "the split/focus code loses
   provenance or allows law violations." Same fix shape: enumerate with
   index oids, carry parent state into constructed beams, add law-check
   hooks on optic constructors.

7. **Refactor: single `PhantomCrystal<M>`** (m1). Cleanup pass. Reduces
   surface area by ~300 lines.

8. **Write a real integration test** (M5) that composes at least two optics
   with type-distinct Crystals and an inner prism, end-to-end.

## Sign-off

**Seam:** not yet. Will sign after C1, C2, C3, C4, M1, M2 are addressed and
the tests on `Compose` and `MetaPrism` exercise the actual composition shape
described in the spec, not a degenerate substitute. The layer is currently
readable as "plausibly works" only because the tests don't ask the hard
questions.

**Taut:** not yet. The allocation and monomorphization concerns are all
deferrable — but deferrable rests on the composition primitive actually being
correct, and right now I can't measure `Compose` because there's no working
`Compose` to measure. Sign after C1 and C3 at minimum; m1/m2 are nice-to-have
but I'd run the branch in anger to see where the boot fold actually hurts.

Neither of us is angry. The skeleton is the right shape at the joint level
(five ops, fixed-point, feature-gated). The flesh on the joint is where the
holes are. Fix C1-C4, then come back for the rest.

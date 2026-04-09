# Seam + Taut Re-Review — Prism Optics Layer (2026-04-08)

**Prior review:** `docs/seam-taut-review-2026-04-08.md` (same date)
**Subject:** prism crate at HEAD `8f81d67`, branch `reed/prism-composition-foundation`
**Test count verified:** 132 lib tests + 6 integration tests, 0 failed, 0 ignored. Runs in <1s debug.
**Build:** `nix develop /Users/alexwolf/dev/projects/mirror -c cargo test --features optics` — green.

> Procedural note: the insight document at
> `/Users/reed/dev/systemic.engineering/practice/insights/cosmology/nested-bundles-and-the-runtime-unification.md`
> was denied read permission in this session. The physics-of-the-ship
> assessment below is therefore grounded in the dispatcher's framing of the
> claim, not the full text. A follow-up pass with the insight doc in hand is
> warranted; nothing in the current review blocks on it.

## Summary

**Seam:** The four critical blockers are addressed in a way that the old
tests would not have caught and the new tests do. `MetaPrism<P: Prism, G>`
has its inner prism back (C1); `OpticPrism` takes refutation through
`ShannonLoss::infinite()` with a closure-author-chosen sentinel instead of
`Option<A>`/`A::default()` (C2); `Compose::refract` is witnessed by a real
`Compose<MarkerPrism, IdPrism<MarkerPrism>>` chain and a 3-deep
associativity test (C3); `Setter::refract` actually calls `modify_fn`
with an identity inner and records the witnessed `S` in a non-phantom
crystal (C4). M1/M2/M3/M4/M5 are likewise addressed. The surface that
remains questionable — the PhantomCrystal unification and the sealed
`Accumulate` workaround — are not blockers, but they are documented as
new concerns below. **Seam signs, conditional on the two minor
follow-ups listed at the end.**

**Taut:** The composition primitive is now measurable because it exists
as a real code path that real tests exercise. `Compose::refract` runs
`first.refract → second.focus → second.project → second.refract` in a
visible control flow. No measurements have been run yet — the deferrable
allocation concerns (m1 path-clone hot path, m2 beam-rebuild pattern,
monomorphisation of `Compose<Compose<...>, _>` chains) are all still
deferrable, and Taut still wants to run the branch in anger against the
real boot fold before betting on the shape. The PhantomCrystal unification
resolves the six-crystal surface area without introducing a runtime cost.
**Taut signs.**

**Joint:** The four criticals and five majors named in the prior review
are addressed; the minor follow-ups are worth filing but do not block
the merge to main or the next major step.

## Per-finding status

### Critical

**C1 — MetaPrism with inner prism — ADDRESSED.**
`src/optics/meta.rs:34` declares `MetaPrism<P: Prism, G: Gather<P::Part>>`
with `inner: P` and `gather: G`. `focus` at line 67 builds an inner beam
and calls `self.inner.split(inner_beam)`. `Input = P::Projected`,
`Focused = Vec<Beam<P::Part>>`, `Projected = P::Part`. The headline use
case (`MetaPrism<WordsPrism, SumGather>` delegating to `WordsPrism.split`)
is witnessed by `meta_prism_uses_inner_split_and_gathers` at line 236 and
by the integration test `meta_prism_over_sum_gather_collapses_population`.
Commits: `d7f82c7` (🔴) + `06af4f8` (🟢).

**C2 — OpticPrism refutation in ShannonLoss, not Option/Default — ADDRESSED.**
`src/optics/optic_prism.rs:11-16` shows the three-closure shape:
`match_fn`, `extract_fn`, `review_fn`. No `Default` bound on `S` or `A`.
`Focused = A` (line 78), no `Option`. `focus` at line 83 always calls
`extract_fn`, then sets `loss = ShannonLoss::new(f64::INFINITY)` on
mismatch (line 88). The test `optic_prism_focus_nonmatch_produces_infinite_loss_without_default`
(line 181) proves the sentinel is the closure author's chosen `-1` rather
than `i32::default()` (which would be `0`). The `Shape` test enum at
line 132 explicitly has **no `Default` derive**, which would have
prevented compilation under the old design. The `# Laws` rustdoc on
`OpticPrism::new` documents the caller obligation and the
"check loss before reading result on infinite-loss beams" invariant —
this addresses Seam's verification-surface concern from the prior review.
Commits: `fc925e3` (🔴) + `9552e62` (🟢).

> New concern, not blocking: the inherent `extract(&self, s: &S) -> Option<A>`
> convenience method still exists (line 63). It is marked as internal
> convenience API, and it is not on the `Prism` trait surface, so it does
> not violate the prior C2 finding. But a downstream reader who imports
> `OpticPrism` and reaches for `extract` will still see `Option`. Consider
> renaming to `try_extract` or documenting "this is not the prism surface"
> on the method itself, not just in module prose.

**C3 — Monoid laws witnessed on Compose itself — ADDRESSED.**
`src/optics/monoid.rs:426-538` contains three tests exercising real
`Compose::refract`:
- `compose_runs_first_then_second_refract` — verifies
  `Compose<MarkerPrism, IdPrism<MarkerPrism>>` reaches `Stage::Refracted`
  with the right crystal type and path survival. Crucially,
  `P1::Crystal = MarkerPrism` is genuinely distinct from `P2::Input = MarkerPrism`
  only at the nominal level, but `P2 = IdPrism<MarkerPrism>` has a distinct
  `Crystal` type (`IdPrism<MarkerPrism>`), so the chain actually crosses
  a type boundary.
- `compose_path_survives_through_both_prisms` — path/loss/precision
  survive the composition.
- `compose_associativity_via_distinct_prism_chain` — a 3-prism chain
  `(MarkerPrism, IdPrism<MarkerPrism>, IdPrism<IdPrism<MarkerPrism>>)`
  is composed two ways (`(a.b).c` vs `a.(b.c)`) and the two concrete
  `Compose<...>` types are run inline. Observable fields (stage, path,
  loss, precision) are asserted identical between groupings.

The old `CountPrism` identity/associativity tests at line 355-382 still
exist — they were not removed, only supplemented. Seam is fine with that:
they now read as smoke tests for the `PrismMonoid` trait, not as the
whole story. Commit: `2a7afde` (♻️).

**C4 — Setter::refract connected to modify_fn — ADDRESSED.**
`src/optics/setter.rs:57-72` shows `refract` calling
`(self.modify_fn)(beam.result, &|a| a)` and recording the result in
`SetterCrystal { witnessed: modified, ... }`. Note `SetterCrystal` is
**not** unified into `PhantomCrystal<M>` (unlike the five other optics);
it carries real data (`witnessed: S`). The test
`setter_refract_runs_modify_with_identity_and_witnesses_value` at line 122
asserts that `refracted.result.witnessed == input` — which, because
the identity inner makes modify a no-op at the A level, is the right
observable for "the closure ran and produced a value we can inspect."
Commits: `7236379` (🔴) + `6242d1d` (🟢).

> New concern, not blocking: `SetterCrystal` leaks `S` forward across
> the Prism boundary. Downstream users who pattern-match on
> `refracted.result.witnessed` can observe the whole `S`. This is not
> a security issue — the same `S` was in the input beam — but it does
> give Setter's refract crystal a semantically richer signal than the
> other five optics. Document it, or accept it as a deliberate feature
> of Setter being the one write-side optic.

### Major

**M1 — Traversal/Fold split carry index provenance — ADDRESSED.**
`src/optics/traversal.rs:76-93` enumerates and pushes
`Oid::new(format!("{}", i))` on each child's path. `fold.rs:82` does
the same. Tests `traversal_split_indexes_children` at line 152 and the
equivalent in `fold.rs` assert `parts[i].path.last() == Some(&Oid::new("i"))`.
Commits: `d0abfa4` (🔴) + `caa2e3c` (🟢).

**M2 — MetaPrism::split/focus/project carry outer beam provenance — ADDRESSED.**
`src/optics/meta.rs:117-133` builds the outer wrapper Beam with
`path: beam.path`, `loss: beam.loss`, `precision: beam.precision`,
`recovered: beam.recovered` — parent values, not fresh zeros.
`meta_prism_split_carries_parent_provenance` at line 269 asserts the
path, loss=2.5, precision=0.7 survive. `focus` (line 67) and `project`
(line 98) also thread the outer envelope forward, with tests at line 312
and 333 confirming. Commit: `06af4f8`.

**M3 — Laws sections on closure-parameterised optic constructors — ADDRESSED.**
`# Laws` rustdoc sections present on `Iso::new`, `Lens::new`,
`OpticPrism::new`, `Traversal::new`, `Setter::new`, and `Fold::new`.
Each names the caller obligation the type system cannot enforce (e.g.,
Iso round-trip, Lens view/set/setset, Traversal purity, Setter
modify-identity and modify-compose, OpticPrism review/extract round-trip
and sentinel contract). Seam's original M3 recommendation was
"document or debug-check"; the doc path was taken. That's acceptable —
the prior review explicitly said doc-level was fine. Commit: `98dbd23`.

> No `Iso::new_checked` debug-mode property variant was added. That is
> the nice-to-have; the doc fix discharges the blocker.

**M4 — Stage::Joined resolution — ADDRESSED (deleted).**
`src/beam.rs:24-31` — `Stage` has exactly five variants: Initial,
Focused, Projected, Split, Refracted. A ripgrep for `Joined` finds
zero references in `src/`, only historical mentions in `docs/` and
the design plan. Gather strategies now emit `Stage::Projected`
(`gather.rs:80, 105, 125, 144, 160, 169`). Commit: `c7fe545`.

**M5 — Integration test exercises Compose end-to-end — ADDRESSED.**
`tests/optics_integration.rs` gained three Compose tests:
`compose_two_idprisms_end_to_end_through_full_pipeline` (line 122)
chains `Compose<IdPrism<String>, IdPrism<IdPrism<String>>>` across a
type boundary and runs focus → project → refract. `compose_preserves_path_through_both_idprisms`
(line 149) asserts path survival. `meta_prism_full_pipeline_splits_gathers_and_refracts`
(line 181) runs the headline `MetaPrism<WordsPrism, SumGather>` through
`apply`. All pass. Commit: `8f81d67`.

### Minor

**m1 — PhantomCrystal unification — ADDRESSED.**
`src/optics/phantom_crystal.rs` defines `PhantomCrystal<M: 'static>` with
a single blanket `Prism` impl. `Iso`, `Lens`, `Traversal`, `OpticPrism`,
and `Fold` all use it via per-optic marker types (e.g.,
`OpticPrismMarker<S, A>`, `TraversalMarker<A, B>`). `Setter` kept its own
`SetterCrystal` because it carries real data. Type-level fingerprinting
is preserved via `M`. Seam's original synthesis ("keep the fixed-point
bound, collapse the impl to one blanket") was taken verbatim. Commit:
`ad4c1db`.

> New concern, not blocking: see "New concerns" section below on whether
> the marker types could be unified into `PhantomData<Self>` and save
> another layer of type surface.

**m2 — Beam rebuild consistency — ADDRESSED.**
`..beam` sugar is now used consistently in pass-through sites (e.g.,
`Beam { stage: Stage::Focused, ..beam }` in `iso.rs`, `lens.rs`,
`phantom_crystal.rs`, and `monoid.rs::IdPrism`). Sites that carry
provenance forward with transformations still use field-by-field
builds, which is the honest choice because they are doing more than a
stage flip. The inconsistency Seam flagged is resolved. Commit: `ad4c1db`.

**m3 — Gather strategies generic over T — ADDRESSED (with workaround).**
`MaxGather` and `FirstGather` are generic over `T: Clone + Default`.
`SumGather` uses a sealed `Accumulate` trait with explicit `NotString`
markers for eight numeric types. Tests at `gather.rs:260-294` exercise
`SumGather`, `MaxGather`, `FirstGather` on `i32`. Commits: `72a70fe` (🔴)
+ `8e7f53d` (🟢).

> **New concern, not blocking:** the `NotString` sealed marker is a
> coherence workaround — Rust's orphan rules + `String: Add<&str>`
> (not `Add<String>`) force it. It works, but it is not extensible by
> downstream users: a third-party type `Foo: Add<Output=Foo>` that
> someone wants to gather via `SumGather` will need to implement the
> sealed `Accumulate` trait, which is private. The alternative
> ("rename `SumGather` to `ConcatGather` and ship a separate `AddGather`
> for the numeric case") would have been cleaner. The current design
> paints a sub-Turing corner: eight types are sealed in, everything
> else is sealed out. Worth filing as a follow-up refactor task. Not
> a blocker because the sealed trait is clearly marked and the tests
> demonstrate the only downstream path users currently need.

**m4 — `is_lossless()` alias — ADDRESSED.**
`src/loss.rs` now exposes `is_lossless`; `src/beam.rs:62` uses it
(`self.loss.is_zero()`). The integration test
`compose_two_idprisms_end_to_end_through_full_pipeline` calls
`out.loss.is_lossless()` at line 144 — the alias is already in use
at the call site Seam recommended it for. Commit: `c7fe545`.

## New concerns (introduced by the fixes)

**N1 — OpticPrism's `extract` inherent method still returns `Option<A>`.**
Not on the trait surface, documented as "internal convenience API," but
a downstream reader who imports `OpticPrism` and types `.extract(` will
get an `Option<A>` back. This is one rename or comment-on-the-method
away from clean. Seam flags, not blocking.

**N2 — SetterCrystal carries `S` past the Prism boundary.**
The other five optics crystallise to phantom data. Setter crystallises
to a real `S`. This is deliberate (the prior review asked for it) but
worth documenting at the Prism layer so a user who reaches for
`refracted.result.witnessed` on a Setter vs an Iso is not surprised.

**N3 — Sealed `Accumulate` trait paints a closed set of numeric types
into `SumGather`.** Works today. Not extensible by downstream users.
Document as a known limitation or refactor into `ConcatGather` +
`AddGather`. The prior review did not ask for this because SumGather
was String-only at the time; the generalisation was Seam+Taut's
recommendation, and the execution introduces the workaround.
Follow-up, not blocker.

**N4 — Marker types for PhantomCrystal.** Each optic uses a distinct
marker (`TraversalMarker<A, B>`, `OpticPrismMarker<S, A>`, etc.) instead
of `PhantomCrystal<Self>`. That is fine — it avoids the "marker must
be Clone" headache when the optic itself cannot be `Clone` because it
owns `Box<dyn Fn>`. Seam reviewed and this is sound. No action.

**N5 — `Compose::refract` control flow is
`first.refract → second.focus → second.project → second.refract`.**
The original Seam concern in the prior review ("`P1`'s focus and
project are bypassed because the base trait's `apply` is not used")
is still the case structurally, but this is the correct semantics for
`Compose`: the inner `Compose::focus` already calls `first.focus` and
`project` calls `first.project`, and `refract` picks up after
`project` has run. Re-reading the code confirms `Compose::focus` at
line 41 and `Compose::project` at line 45 delegate to `first`, and
`refract` at line 61 runs `first.refract` and then the second prism's
full focus+project+refract chain. The composition law is: the caller
runs `focus → project → refract` through the composed prism, and the
composed prism folds `first` fully before handing off to `second`.
This is correct. The prior review's flag is discharged by actually
reading the test evidence (`compose_runs_first_then_second_refract`
exercises exactly this path).

## Physics-of-the-ship assessment

> Grounded in the dispatcher's summary of the insight doc; the full
> text was not available to this review session.

**Identity through parallel transport.** The composition layer
preserves the input beam's `path`, `loss`, `precision`, and `recovered`
fields through every optic in the chain. The
`compose_preserves_path_through_both_idprisms` test is the parallel
transport witness: a path with two Oid entries traverses
`Compose<IdPrism, IdPrism>` and arrives intact on the far side. Loss
accumulates when prisms are lossy (verified in the OpticPrism infinite-loss
test). Precision is monotone non-increasing across the chain (MaxGather
takes min, SumGather takes min, identity prisms pass through). This
matches the "curvature accumulates along the path" reading.

**Recursive `Crystal: Prism<Crystal = Self::Crystal>` bound.** The bound
holds at every level in every shipped optic:
- `IdPrism<T>::Crystal = IdPrism<T>` (self-loop)
- `PhantomCrystal<M>::Crystal = PhantomCrystal<M>` (self-loop)
- `Compose<P1, P2>::Crystal = P2::Crystal` — which by induction on
  `P2` is a fixed-point too, because `P2` satisfies the same bound.
- `MetaPrism<P, G>::Crystal = MetaPrism<P, G>` (self-loop)
- `SetterCrystal<S, A>::Crystal = SetterCrystal<S, A>` (self-loop)

The tower admits compatible connections at each level because the
fixed-point is enforced structurally by the type alias at every level.
Seam's concern from the prior review — that the bound is "one level of
fixed-point only, a malicious impl could redirect" — remains true, but
no shipped impl exhibits that behaviour, and a malicious impl in user
code is outside the threat model for the crate itself.

**Refutation propagation.** An `OpticPrism` on a non-matching input
emits `ShannonLoss::new(f64::INFINITY)`. Downstream operations
(`Compose::refract`, `MetaPrism::project`, all passthrough optics)
preserve the loss field unchanged. A reader who checks
`beam.loss.is_lossless()` or `beam.loss.as_f64().is_infinite()` will
see the refutation. The `# Laws` rustdoc on `OpticPrism::new`
documents that consumers MUST check loss before reading result on
infinite-loss beams. This is the correct propagation for "refutation
as curvature singularity along the tower." A silent consumer that
reads `result` without checking loss can still be fooled — no type-level
enforcement — but the contract is now named and the test confirms the
observable encoding.

**Content-addressing through split/gather.** Traversal and Fold now
push index Oids onto children's paths. WordsPrism in the meta tests
pushes `word/i` Oids. When a population is gathered, SumGather takes
the first beam's path (losing the index information), which is a
deliberate design choice — the gather strategy decides how provenance
survives collapse. This is the one place where the bundle's identity
is deliberately dropped: at the collapsing step, by the collapsing
strategy. That is structurally correct (you cannot parallel-transport
a population back to a singleton without losing something), but the
framework does not currently offer a "path-preserving gather" option.
Follow-up for the next iteration: a `PathMergeGather` that collects
children's paths into a joint path. Not blocking.

**Monoid structure holds on non-trivial elements.** The 3-deep
associativity test exercises `MarkerPrism`, `IdPrism<MarkerPrism>`, and
`IdPrism<IdPrism<MarkerPrism>>`. The two groupings produce identical
observable outputs. This is the monoid law witnessed on distinct prism
types, not just on u64 counts. The prior review's C3 blocker was
precisely this; it is discharged.

**Joint assessment:** the composition layer is a faithful
instantiation of the abstract object at the level the code can reach.
The one place where the abstraction leaks is gather: the collapse
step decides provenance policy, and the current strategies are
lossy-by-design at the path level. This is the correct place for the
leak if you have to have one. A future `PathMergeGather` would let
users opt out of the loss.

## Cross-lens tensions

On this pass there is remarkably little Seam/Taut disagreement. Both
signed off on:
- The PhantomCrystal unification (Seam: type-level fingerprint
  preserved via M; Taut: ~300 LoC gone, no runtime cost).
- The three-closure OpticPrism shape (Seam: refutation in Shannon,
  not Option; Taut: still one `Box<dyn Fn>` per closure, no worse
  than before).
- The deletion of `Stage::Joined` (Seam: no drift-habitat; Taut:
  one fewer discriminant to match exhaustively).
- The real `Compose` tests (Seam: laws witnessed; Taut: a measurable
  primitive exists).

One soft disagreement: the sealed `Accumulate` workaround in `gather.rs`.
- **Taut** is fine with it — it's zero runtime cost, the numeric types
  are the ones users reach for, and the alternative (a second gather
  strategy) doubles the API surface.
- **Seam** prefers the alternative — the closed-world sealed trait is
  a sub-Turing corner that users can't extend, and in a framework that
  sells "sub-Turing by default, escape at the boundary" that's a trap
  worth avoiding.

Not worth blocking on. Joint recommendation: file it as a follow-up
task, ship the branch.

## Recommended next actions

1. **Merge to main.** The blockers are discharged. The remaining
   concerns are documented and do not affect the correctness of the
   composition layer.
2. **File N3 (`SumGather` sealed trait refactor)** as a follow-up task.
   Candidate title: "split `SumGather` into `ConcatGather` + `AddGather`;
   remove `sealed::NotString` workaround."
3. **File N1 (`OpticPrism::extract` rename to `try_extract`)** as a
   minor polish. Optional.
4. **File N2 (document `SetterCrystal.witnessed` as a deliberate data
   leak)** as a documentation task.
5. **Proceed to MirrorPrism proper.** The composition foundation is
   sound; the next major step (the real boot fold) can land on top of
   this branch.
6. **When the real boot fold lands, run it in anger** against a hundred
   files and measure: path-clone cost in `Traversal::split`, Box<dyn Fn>
   virtual dispatch in the inner loop, Compose monomorphisation in long
   chains. None of these are current fires; all of them are future fires
   if the layer is used without measurement.
7. **Future: path-preserving gather.** A `PathMergeGather<T>` that
   concatenates child paths into the result beam's path. Would close
   the one remaining abstraction leak in the gather layer.

## Sign-off

**Seam:** **signed**, conditional on N1 and N2 being filed as follow-up
tasks (neither must be done before merge). The layer is now
"readable as correct" — the tests ask the hard questions and the code
answers them. The composition primitive, the refutation channel, the
meta-prism lift, and the provenance-carrying splits all do what the
spec says they do, and there are witnesses. The branch is ready to
merge to main.

**Taut:** **signed**. The composition primitive is measurable because
it exists as a real code path that real tests exercise. Allocation
and monomorphisation concerns are deferrable. The line exists and
I can see it now. Run the boot fold when it's ready and we'll measure
where it actually hurts. The branch is ready to merge to main **and**
ready to carry the next major step (MirrorPrism proper).

**Joint:** both signed. Branch `reed/prism-composition-foundation` at
`8f81d67` is ready to merge to `main`. Follow-ups N1, N2, N3, and the
next iteration's path-preserving gather are filed as non-blocking.

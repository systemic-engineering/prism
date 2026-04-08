# Prism as Monoid — Optics Layer Design

**Date:** 2026-04-08
**Status:** Draft for review
**Related:** prism commit `78d22f9` (five-type trait), mirror's registry work at commit `d1814b7`

## Goal

Add a composition layer to the `prism` crate that makes explicit the monoid structure already implicit in the base trait, and provide a small kit of classical functional optics built on top of it. Keep the base crate minimal by default; make the full layer available behind a `optics` cargo feature.

## Context and recognition

The base `Prism` trait (five operations, five associated types, recursive `Crystal: Prism<Crystal = Self::Crystal>` bound) already has a monoid hiding inside it.

**The recognition:**

For the homogeneous case `type Crystal = Self`, a Prism is fundamentally an endofunction `Beam<T> → Beam<T>`, and endofunctions form a monoid under composition. The five named operations are *structured views* of this endofunction:

- `focus`, `project`, `zoom`, `refract` are all Beam-to-Beam and participate in the monoid
- `split` is the exception: `Beam<T> → Vec<Beam<T>>` leaves the monoid, entering a *different level* where populations of beams live
- The `Crystal = Self` constraint is the **monoid closure property** — composition keeps you in the same type

This gives us a clean multi-level picture:

```
Level 0:  Beam<T>                  ← single beam
Level 1:  Vec<Beam<T>>             ← population of beams
Level 2:  Vec<Vec<Beam<T>>>        ← populations of populations
...

Within each level: Monoid (endofunctions under composition)
Between levels:    Meta-prisms and classical optics (monoid homomorphisms)
```

A `Prism` lives at Level 0. `split` moves you from Level 0 to Level 1. To return, you need a *meta-prism* — an object that takes `Vec<Beam<T>>` and produces `Beam<T>`. That's not a method on the splitter; it's a different object with its own focus/project/refract and its own Crystal type.

**The optics layer makes this structure first-class** — turning the implicit monoid into a named `PrismMonoid` trait, and the cross-level homomorphisms into named optic kinds (Traversal, Lens, Iso, etc.) borrowed from classical functional optics theory.

## Scope

**In scope:**
- A `prism::optics` module, gated behind `feature = "optics"`
- `PrismMonoid` trait witnessing the `Crystal = Self` closure
- `Gather<T>` trait for vector-to-beam strategies
- Concrete gather implementations: `SumGather`, `MaxGather`, `FirstGather`
- `MetaPrism<P, G>` — generic meta-prism parameterized by an inner prism and a gather strategy
- Classical optic shapes: `Traversal`, `Lens`, `Iso`, `Setter`, `Fold`, `OpticPrism` (the semidet optic kind, distinguished by name from the trait)
- `Compose<P1, P2>: Prism` — sequential composition
- `IdPrism` — the monoid identity
- Full test coverage of the monoid laws (identity, associativity)
- Full test coverage of each optic kind against its expected shape

**Out of scope:**
- Changes to the base `Prism` trait shape
- Migration of downstream crates (handled separately, already in progress on branches)
- Performance tuning of the gather strategies (we pick obvious implementations, optimize later if needed)
- A `join` method on the base `Prism` trait — join happens in meta-prisms, not in base prisms
- Higher-kinded generalization to level-2 (populations of populations) — defer until we have a concrete need

## Feature flag

A new cargo feature `optics` added to `prism/Cargo.toml`:

```toml
[features]
default = []
optics = []
```

`default = []` keeps the base crate minimal for consumers who only need the trait and Beam. Downstream crates that want the optics layer opt in via:

```toml
prism = { path = "../prism", features = ["optics"] }
```

All new types and the `optics` module itself are gated by `#[cfg(feature = "optics")]`. The base trait remains in `lib.rs` with no feature gate, unchanged by this work.

## Architecture

### Module layout

```
prism/
├── src/
│   ├── lib.rs              ← unchanged: base trait, Beam re-export, apply
│   ├── beam.rs             ← unchanged
│   ├── content.rs          ← unchanged
│   ├── loss.rs             ← unchanged
│   ├── metal.rs            ← unchanged (note: not MetalPrism; utility module)
│   ├── oid.rs              ← unchanged
│   ├── precision.rs        ← unchanged
│   ├── spectral_oid.rs     ← unchanged
│   └── optics/             ← NEW, feature-gated
│       ├── mod.rs          ← re-exports, module doc
│       ├── monoid.rs       ← PrismMonoid trait + IdPrism + Compose
│       ├── gather.rs       ← Gather trait + Sum/Max/First strategies
│       ├── meta.rs         ← MetaPrism<P, G>
│       ├── traversal.rs    ← Traversal
│       ├── lens.rs         ← Lens
│       ├── iso.rs          ← Iso
│       ├── setter.rs       ← Setter
│       ├── fold.rs         ← Fold
│       └── optic_prism.rs  ← OpticPrism (the semidet optic kind)
```

Each file has one clear responsibility. Inter-file dependencies flow downward from `mod.rs`: `monoid.rs` depends only on the base trait; `gather.rs` depends on `monoid.rs`; `meta.rs` depends on both; the classical optic shapes depend on `meta.rs`.

### Layer 1: the monoid

`prism/src/optics/monoid.rs`:

```rust
use crate::{Beam, Prism};

/// A Prism whose Crystal is itself — the closure property that makes
/// prisms compose into a monoid.
pub trait PrismMonoid: Prism<Crystal = Self> + Sized {
    /// The identity element: refracting through this prism is a no-op.
    /// All Beam fields survive unchanged except stage, which transitions
    /// to Refracted.
    fn identity_prism() -> Self;

    /// Monoid composition: run `self` then `other`.
    /// compose(identity_prism(), p) = compose(p, identity_prism()) = p
    /// compose(compose(a, b), c) = compose(a, compose(b, c))
    fn compose(self, other: Self) -> Self;
}

/// A generic identity prism parameterized by the type it operates on.
/// For a type T, IdPrism<T> is a Prism<Input=T, Crystal=IdPrism<T>> that
/// passes beams through unchanged.
pub struct IdPrism<T> { _phantom: PhantomData<T> }

impl<T> Prism for IdPrism<T> { /* ... pass-through impls ... */ }
impl<T> PrismMonoid for IdPrism<T> { /* ... */ }

/// Sequential composition of two prisms. Compose<P1, P2> is a Prism whose
/// refract is `p2.refract(p1.refract(beam))`. When P1::Crystal = P1 and
/// P2::Crystal = P2 and both agree on Input/Focused/Projected/Part types,
/// Compose is itself a PrismMonoid.
pub struct Compose<P1, P2> { first: P1, second: P2 }

impl<P1, P2> Prism for Compose<P1, P2>
where
    P1: Prism,
    P2: Prism<Input = P1::Crystal>,
{
    /* ... */
}
```

**Testing:** monoid laws as property tests.

```rust
// Identity law: compose(id, p) ≡ p and compose(p, id) ≡ p on all inputs
// Associativity: compose(compose(a, b), c) ≡ compose(a, compose(b, c))
```

### Layer 2: gather strategies

`prism/src/optics/gather.rs`:

```rust
use crate::Beam;

/// A strategy for collapsing Vec<Beam<T>> into Beam<T>.
/// Different strategies make different decisions about how to aggregate
/// loss, combine results, and preserve path provenance.
pub trait Gather<T> {
    fn gather(&self, beams: Vec<Beam<T>>) -> Beam<T>;
}

/// Concatenate results (for types with an Add-like operation), sum losses,
/// take max precision.
pub struct SumGather;

/// Pick the beam with the highest precision, discard the rest.
/// Use when you only care about the best single outcome.
pub struct MaxGather;

/// Pick the first beam. A degenerate gather, mostly for testing.
pub struct FirstGather;
```

`SumGather` is the workhorse; the other two are specialty strategies. Implementations live in `gather.rs` and are generic in `T` where possible (e.g., `T: Add + Clone` for Sum), or concrete where necessary.

**Testing:** each gather strategy has at least one test showing the expected behavior on a small vector of beams with distinct losses/precisions.

### Layer 3: the meta-prism

`prism/src/optics/meta.rs`:

```rust
use crate::{Beam, Prism, Stage};
use super::gather::Gather;

/// A meta-prism operates on populations of beams. Its Input is a beam
/// whose content is itself a Vec<Beam<T>> from an inner prism's split.
/// Its refract gathers the population into a single beam via the provided
/// Gather strategy.
pub struct MetaPrism<P: Prism, G: Gather<P::Part>> {
    pub inner: P,
    pub gather: G,
}

impl<P: Prism, G: Gather<P::Part>> Prism for MetaPrism<P, G> {
    type Input = Beam<P::Projected>;
    type Focused = Vec<Beam<P::Part>>;
    type Projected = Beam<P::Part>;
    type Part = P::Part;
    type Crystal = MetaPrism<P, G>;

    fn focus(&self, beam: Beam<Beam<P::Projected>>) -> Beam<Vec<Beam<P::Part>>> {
        // Unwrap the nested beam, split the inner via P, rewrap the Vec
        let inner_beam = beam.result;
        let parts = self.inner.split(inner_beam);
        Beam::new(parts).with_stage(Stage::Focused)
        // ... preserve path/loss/precision from `beam`
    }

    fn project(&self, beam: Beam<Vec<Beam<P::Part>>>) -> Beam<Beam<P::Part>> {
        // Gather the Vec into a single Beam<Part>
        let gathered = self.gather.gather(beam.result);
        Beam::new(gathered).with_stage(Stage::Projected)
    }

    fn split(&self, beam: Beam<Beam<P::Part>>) -> Vec<Beam<P::Part>> {
        // Degenerate: a meta-prism's split is typically trivial
        vec![beam.result]
    }

    fn zoom(
        &self,
        beam: Beam<Beam<P::Part>>,
        f: &dyn Fn(Beam<Beam<P::Part>>) -> Beam<Beam<P::Part>>,
    ) -> Beam<Beam<P::Part>> {
        f(beam)
    }

    fn refract(&self, beam: Beam<Beam<P::Part>>) -> Beam<MetaPrism<P, G>> {
        // Crystal = a copy of self, carrying forward the population's state
        Beam::new(MetaPrism {
            inner: self.inner.clone(),  // P: Clone requirement
            gather: self.gather.clone(),
        })
        // ... preserve path/loss/precision, transition stage to Refracted
    }
}
```

Note: the meta-prism's types are deeper (`Beam<Beam<P::Projected>>`) because it operates one level up. This is expected. The meta-prism's `refract` emits a new MetaPrism as its Crystal (fixed-point closure).

Clone is required on `P` and `G` because the Crystal must be a fresh instance. This adds `Clone` as a where-clause burden — acceptable, most prisms will be Clone anyway.

**Testing:** feed a known Vec of beams into `project`, assert the gathered result matches what the explicit gather call would produce. Round-trip test: `MetaPrism<P>.project(Vec::new_empty())` vs the inner prism's known identity behavior.

### Layer 4: classical optics

Each classical optic kind is a specific shape of PrismMonoid member or meta-prism. They're built on top of Layer 3.

**`Traversal<P, T>`** — lifts an inner prism `P` over any `Traversable` container type `T`. The canonical multi-focus optic. Implemented by splitting via the inner prism, applying operations uniformly, then gathering with a user-provided gather strategy. Returns a new Traversal as its Crystal.

**`Lens<S, A>`** — total bidirectional single-focus. Parameterized by a `view: Fn(&S) → &A` and a `set: Fn(S, A) → S`. The get side is focus; the set side is zoom with an appropriate inner function. No split (single focus). Crystal = Lens<S, A>.

**`Iso<A, B>`** — total invertible. Parameterized by `forward: Fn(A) → B` and `backward: Fn(B) → A`. The only optic where refract is genuinely lossless and the round-trip `forward ∘ backward = id` is law. Crystal = Iso<A, B>.

**`OpticPrism<S, A>`** — semidet optic kind (the classical Prism, renamed to avoid collision with our trait). Parameterized by `preview: Fn(&S) → Option<A>` and `review: Fn(A) → S`. The preview side corresponds to project with refutation in loss; the review side corresponds to refract's constructor role. Crystal = OpticPrism<S, A>.

**`Setter<S, A>`** — write-only. Parameterized by `modify: Fn(S, &dyn Fn(A) → A) → S`. No get side. Zoom-with-function is the natural operation. Crystal = Setter<S, A>.

**`Fold<S, A>`** — multi read-only. Parameterized by `fold: Fn(&S) → Vec<A>`. Focus + split, no modification. Crystal = Fold<S, A>.

Each of these is a small file (~50-100 lines) implementing the `Prism` trait with the specific types and behaviors. They are all `PrismMonoid` (Crystal = Self) and therefore participate in the monoid structure.

**Testing:** for each optic kind, one test showing the defining law holds. E.g., for Iso: `iso.apply(iso_inverse.apply(x))) = x`. For Traversal: `split_then_join = id` up to gather loss.

## Data flow

A concrete example: `mirror ai abyss |> pathfinder`.

1. **Input:** a .mirror file's source text
2. **`MetalPrism`** (not a monoid member, the seed) refracts the source into a `MirrorPrism` carrying the stdlib bindings
3. **`MirrorPrism`** is a PrismMonoid member. Applying it to the input text via `apply()` runs focus → project → refract
4. **`abyss_prism`** and **`pathfinder_prism`** are specific MirrorPrisms (retrieved from the registry)
5. **`Compose::new(abyss_prism, pathfinder_prism)`** is a new MirrorPrism representing the composition — the monoid operation
6. **`Compose.refract(input_beam)`** runs abyss's refract first, then feeds the result into pathfinder's refract
7. **Split-level operations:** if the user writes `call | cast`, the prism's split produces two beams. A `MetaPrism<inner, SumGather>` gathers them back into a single beam representing the sum type

## Testing strategy

Three kinds of tests:

1. **Monoid law tests** — property tests asserting identity and associativity hold for composed prisms. Use `proptest` if available, otherwise hand-written cases with small input sets.

2. **Per-optic law tests** — one test per classical optic kind asserting its defining law. E.g., Iso round-trips exactly; Lens view-set-view = view; Traversal split-then-gather is the identity up to loss.

3. **Integration tests** — a small "domain prism" (parse a simple DSL, like the existing `StringPrism`) lifted over Traversal, wrapped in MetaPrism, composed with another copy of itself. Assert the end-to-end behavior matches hand-written composition.

All tests live in `#[cfg(test)] mod tests` blocks in their respective files, gated by `#[cfg(feature = "optics")]` so they only run when the feature is enabled.

Test count estimate: 25-40 tests across the whole optics layer.

## Error handling

Consistent with the base crate: no tagged error types, no Option/Result. Refutation and failure are encoded as `ShannonLoss` in the Beam. The gather strategies aggregate loss according to their semantics (Sum: add, Max: forward the max-precision beam's loss, First: forward the first beam's loss and discard the rest's loss).

The `IdPrism` and monoid identity never introduce loss — they are lossless by definition. `Compose` accumulates loss additively across its two constituents.

## Open questions

1. **Where does `PrismMonoid::compose` signature live?** Does it consume both prisms (taking ownership), or take references? Consuming is more monoid-natural (the operation produces a new element and the inputs are "used up"). References are more ergonomic for repeated use. I propose consuming with `Clone` bound on impls that want repeated use.

2. **Should `IdPrism<T>` be generic or one-shot?** A single generic `IdPrism<T>` works for most cases but requires `T: Default` or phantom typing. Per-type hand-written id prisms are cleaner but more boilerplate. I propose generic with `PhantomData`.

3. **Does `MetaPrism::refract`'s Crystal need to clone the inner?** Yes, because each refracted crystal is a fresh value. This forces `P: Clone, G: Clone`. Acceptable burden.

4. **Do we need higher-kinded meta-prisms (level-2, populations of populations)?** Not for the mirror bootstrap use case. Defer until there's a concrete need. A level-2 meta-prism would be `MetaPrism<MetaPrism<P, G1>, G2>` which composes naturally from what this spec provides.

## What this enables

Once landed, the following become expressible:

- **The boot fold** as `boot_files.fold(IdPrism::<MirrorPrism>::new(), |acc, f| acc.compose(compile_file(f)))`
- **Sum types in .mirror files** via `MetaPrism<_, SumGather>` lifting `call | cast` into a single beam
- **Lens access to registry entries** via `Lens<Registry, Entry>` for fine-grained updates
- **Traversal over boot directory** via `Traversal<MirrorPrism, Vec<PathBuf>>` for applying one operation uniformly across all files
- **Iso between .mirror source text and MirrorFragment** for exact parse/emit round-trips
- **Optic-based validation** via Fold + a predicate to assert properties across the registry

## Non-goals

- Making the base `Prism` trait dependent on the optics layer
- Changing the behavior of the existing five operations
- Forcing downstream crates to adopt the optics layer — they can ignore the feature flag entirely
- Implementing every classical optic kind from Haskell's lens library — only the six listed above

## Migration and rollout

1. Land the base feature flag and module skeleton (empty `optics/mod.rs` gated by feature)
2. Implement `IdPrism` and `Compose` with tests — proves the feature flag machinery works
3. Implement `PrismMonoid` trait and verify monoid laws
4. Implement `Gather` strategies
5. Implement `MetaPrism`
6. Implement classical optics one per commit: Iso, Lens, Traversal, OpticPrism, Setter, Fold
7. Integration test with `StringPrism` lifted over Traversal
8. Update module-level docs in `lib.rs` to mention the optional layer

Each step is its own commit with red/green TDD discipline. Downstream crates are unaffected until they opt in via the feature flag.

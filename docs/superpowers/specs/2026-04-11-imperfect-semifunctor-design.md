# Prism Redesign: Imperfect + Semifunctor

**Date:** 2026-04-11

**Status:** Approved

---

## Core Insight

Beam is a semifunctor. Prism is the monoid lifted into it.

Split and zoom are not core operations — they are `smap` in user space. The Prism trait has three methods: focus, project, refract. Three dimensions of space.

## Crate Structure

```
prism/
  Cargo.toml          (workspace)
  imperfect/           standalone crate, no prism dependency
    Cargo.toml
    src/lib.rs
  core/                depends on imperfect
    Cargo.toml
    src/
      lib.rs           Prism trait, Operation structs (Focus, Project, Refract)
      beam.rs          Beam trait (tick, next, smap, apply), PureBeam
      trace.rs         Op (3 variants), Step, Trace, Traced
      oid.rs           Oid, SpectralOid, ContentAddressed
```

## imperfect

An extension on `Result`. Three states: success, partial success, failure.

### Loss trait

```rust
pub trait Loss: Clone + Default {
    fn zero() -> Self;
    fn total() -> Self;
    fn is_zero(&self) -> bool;
    fn combine(self, other: Self) -> Self;
}
```

The shape of `Loss` is discovered through TDD against stdlib types — not prescribed up front. The trait above is the minimum; std interop tests will reveal what's missing.

### Imperfect type

```rust
pub enum Imperfect<T, E, L: Loss = ShannonLoss> {
    Ok(T),
    Partial(T, L),
    Err(E),
}
```

Replaces `Luminosity<V, E>` from the current codebase. Same three-variant shape, but:

- Loss type is parameterized (not hardcoded to `ShannonLoss`)
- Standalone crate, decoupled from beam/prism semantics
- Naming follows `Result` conventions (`Ok`/`Err`, not `Radiant`/`Dark`)

### ShannonLoss

```rust
pub struct ShannonLoss(f64);

impl Loss for ShannonLoss {
    fn zero() -> Self { ShannonLoss(0.0) }
    fn total() -> Self { ShannonLoss(f64::INFINITY) }
    fn is_zero(&self) -> bool { self.0 == 0.0 }
    fn combine(self, other: Self) -> Self { ShannonLoss(self.0 + other.0) }
}
```

Ships with `imperfect` as the default `Loss` implementation. Information loss measured in bits.

### Stdlib interop (feature-gated)

Behind `#[cfg(feature = "std")]` (default on):

- `From<Result<T, E>>` / `Into<Result<T, E>>` for `Imperfect`
- `From<Option<T>>` for `Imperfect`
- Iterator/collect support for `Vec<Imperfect<T, E, L>>`
- `Loss` impls where they make sense (count-based for collection operations)
- `Try` trait impl when stabilized

Core type and `Loss` trait stay `no_std` compatible.

**Implementation strategy:** TDD bottom-up. Tests against `Result`, `Option`, `Vec` reveal the actual shape of `Loss` and `Imperfect`'s API surface. The stdlib interop IS the specification.

## core

### Beam (semifunctor)

```rust
pub trait Beam: Sized {
    type In;
    type Out;
    type Error;
    type Loss: Loss;

    type Next<T>: Beam<In = Self::Out, Out = T, Error = Self::Error, Loss = Self::Loss>;
    type Tick<T, E, L: Loss>: Beam<In = Self::Out, Out = T, Error = E, Loss = L>;

    fn input(&self) -> &Self::In;
    fn result(&self) -> Imperfect<&Self::Out, &Self::Error, Self::Loss>;

    /// The primitive. One tick forward.
    fn tick<T, E, L: Loss>(self, imperfect: Imperfect<T, E, L>) -> Self::Tick<T, E, L>;

    /// Lossless shorthand: tick(Imperfect::Ok(value))
    fn next<T>(self, value: T) -> Self::Next<T>;

    /// Semifunctor map. Derived from tick.
    fn smap<T>(
        self,
        f: impl FnOnce(&Self::Out) -> Imperfect<T, Self::Error, Self::Loss>,
    ) -> Self::Next<T> {
        let imp = match self.result() {
            Imperfect::Ok(v) | Imperfect::Partial(v, _) => f(v),
            Imperfect::Err(_) => panic!("smap on Err beam"),
        };
        // loss composition handled by tick
        ...
    }

    /// Apply an operation. The DSL entry point.
    fn apply<O: Operation<Self>>(self, op: O) -> O::Output {
        op.apply(self)
    }
}
```

`tick` is the fundamental transition — takes an `Imperfect` and advances the beam one step. Loss composition (Partial + Partial accumulates, Partial + Ok carries loss, Err absorbs everything) happens inside `tick`.

`next` is convenience for the lossless case. `smap` is the semifunctor operation derived from `tick`.

### PureBeam

```rust
pub struct PureBeam<In, Out, E = Infallible, L: Loss = ShannonLoss> {
    input: In,
    imperfect: Imperfect<Out, E, L>,
}
```

Production beam. Flat struct. No trace overhead. Implements `Beam`.

### Prism (monoid — three dimensions)

```rust
pub trait Prism {
    type Input:     Beam;
    type Focused:   Beam<In = <Self::Input as Beam>::Out>;
    type Projected: Beam<In = <Self::Focused as Beam>::Out>;
    type Refracted: Beam<In = <Self::Projected as Beam>::Out>;

    fn focus(&self, beam: Self::Input) -> Self::Focused;
    fn project(&self, beam: Self::Focused) -> Self::Projected;
    fn refract(&self, beam: Self::Projected) -> Self::Refracted;
}
```

Three methods. Three dimensions. No split, no zoom.

Monoid laws:
- Composition is associative (testable, not type-enforced)
- Identity prism exists (all three methods call `next` — passthrough)

Blanket impl: `impl<P: Prism> Prism for &P` — non-consuming pipeline.

### Operations

Three structs, three `Operation<B>` impls:

```rust
pub struct Focus<P>(pub P);
pub struct Project<P>(pub P);
pub struct Refract<P>(pub P);
```

DSL:

```rust
beam.apply(Focus(&prism))
    .apply(Project(&prism))
    .apply(Refract(&prism))
```

### Op enum

Shrinks to three variants:

```rust
pub enum Op {
    Focus,
    Project,
    Refract,
}
```

### Split and zoom — user space

Not in the Prism trait. Users write them as `smap` applications:

```rust
// zoom: map through a function
let zoomed = beam.smap(|v| Imperfect::Ok(v * 2));

// split: map to a collection
let split = beam.smap(|v| Imperfect::Ok(v.chars().collect::<Vec<_>>()));
```

Or wrap in named operations for reuse — but that's application code, not core.

## Migration from current codebase

| Current | New |
|---------|-----|
| `Luminosity<V, E>` | `Imperfect<T, E, L>` |
| `Radiant(V)` | `Ok(T)` |
| `Dimmed(V, ShannonLoss)` | `Partial(T, L)` |
| `Dark(E)` | `Err(E)` |
| `advance(luminosity)` | `tick(imperfect)` |
| `next(value)` | `next(value)` (unchanged) |
| `Luminosity::combine` | `Imperfect::compose` |
| `is_light()` / `is_dark()` | `is_ok()` / `is_err()` (follow Result conventions) |
| `loss.rs` (ShannonLoss) | `imperfect/` crate |
| `Split<P, F>` / `Zoom<P, F>` | Removed from core. User-space `smap`. |
| `Op::Split` / `Op::Zoom` | Removed from enum. |

## What this solves

1. **Split/zoom were overspecified** — they're just `smap` with different output types. Removing them from the Prism trait eliminates the design tension around `Vec<Beam>` vs `Beam<Vec<T>>`.

2. **Loss was hardcoded** — `ShannonLoss` as a concrete type in every beam. Now parameterized via the `Loss` trait. Different domains can define their own loss semantics.

3. **Luminosity was coupled to prism** — the three-state result type is general. `imperfect` makes it available to any Rust code, not just beam pipelines.

4. **The algebraic structure is visible** — Beam is a semifunctor, Prism is a monoid. The naming (`tick`, `smap`) and the three-method Prism make the algebra legible without requiring category theory to use.

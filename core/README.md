# prism-core

[![crates.io](https://img.shields.io/crates/v/prism-core.svg)](https://crates.io/crates/prism-core)
[![docs.rs](https://docs.rs/prism-core/badge.svg)](https://docs.rs/prism-core)

Beam (semifunctor) + Prism (monoid). Three operations: focus, project, refract.

## Beam

A `Beam` carries three things through a pipeline:

1. **The current value** -- the output of the last operation.
2. **The input** -- what entered the current step (the previous step's output).
3. **Accumulated loss** -- an `Imperfect` result tracking what was lost along the way.

`tick` is the primitive: advance the beam one step with a new `Imperfect` result. Loss composition happens inside `tick` -- if the beam is already Partial, new losses accumulate automatically.

`next` is the lossless shorthand: `tick(Imperfect::Success(value))`.

`smap` is the semifunctor map: apply a function to the carried value, producing a new `Imperfect`.

## Prism

A `Prism` defines three operations over beams:

- **focus** -- select what matters from the input. The narrowing step.
- **project** -- transform the focused value. The lossy step, where information may not survive (precision cut, eigenvalue threshold, compression).
- **refract** -- produce the output from what survived projection. The reconstruction step.

The associated types form a chain enforced by the compiler:

```
Input -> [focus] -> Focused -> [project] -> Projected -> [refract] -> Refracted
```

Each stage's input type must equal the previous stage's output type. Mismatches are compile errors.

## The DSL

```rust
use prism_core::{Beam, Prism, PureBeam, Focus, Project, Refract};

let result = PureBeam::ok((), input)
    .apply(Focus(&my_prism))
    .apply(Project(&my_prism))
    .apply(Refract(&my_prism));
```

Or use the convenience function:

```rust
let result = prism_core::apply(&my_prism, beam);
```

## The algebra

**Beam is a semifunctor.** You can map over the carried value (`smap`), but the identity law does not hold: mapping the identity function over a Failure beam panics instead of returning the same beam. This is deliberate -- Failure beams have no value to map over, so the type system forces you to check `is_ok()` before transforming.

**Prism is a monoid.** Prisms compose associatively (focus-project-refract chains), and an identity prism exists (passthrough on all three stages). This means pipelines are type-safe by construction.

## Split and zoom

Not core operations. They are `smap` in user space:

```rust
// zoom: transform the value
let zoomed = beam.smap(|&v| Imperfect::Success(v * 2));

// split: expand into a collection
let split = beam.smap(|v| Imperfect::Success(v.chars().collect::<Vec<_>>()));
```

Wrap in named `Operation` implementations for reuse. That's application code, not core.

## Implementations

**`PureBeam`** -- production beam. Flat struct: input + `Imperfect` result. No trace overhead.

**`TraceBeam`** -- forthcoming. Records each step into a `Trace` for debugging and inspection.

## Features

- `optics` -- classical optics (Iso, Lens, Prism, Traversal, etc.) built on the Beam/Prism algebra.

## Dependencies

Depends on `imperfect`. Zero external dependencies.
